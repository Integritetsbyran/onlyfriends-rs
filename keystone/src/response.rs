use ed25519_dalek::Signer;
use serde::{Deserialize, Serialize};

use crate::{
    Identity, PublicKeySealed, Signable, envelope::LetterId, identity::SigningPublicKey,
    signing::Signature,
};

/// All kinds of post responses.
///
/// # Compatibility
/// The postcard wire format is stable, and describes tagged unions (enums) with a `varint(u32)` discriminant.
/// To maintain cross-compatibility between versions, we explicitly set the discriminant for each variant.
/// A discriminant should never be re-used, therefore removing variants is discauraged.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(u32)] // postcard uses u32 to tag enums
pub enum ResponseBody {
    Comment { text: String } = 0,
    Reaction { emoji: String } = 1,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseInner {
    pub letter_id: LetterId,
    /// Responder's public signing key
    pub author: SigningPublicKey,
    pub body: ResponseBody,
    /// Responder's signature
    pub sig: Signature,
}

impl Signable for ResponseInner {
    fn signing_bytes(&self) -> Vec<u8> {
        let Self {
            letter_id,
            author,
            body,
            sig: _, // don't sign the signature
        } = self;
        postcard::to_allocvec(&(letter_id, author, body)).expect("serializes")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseRebroadcast {
    pub inner: ResponseInner,
    /// post owner's signature over `inner`
    pub vouch_sig: Signature,
}

pub fn create_response(
    responder: &Identity,
    letter_id: LetterId,
    body: ResponseBody,
) -> ResponseInner {
    let mut response_inner = ResponseInner {
        letter_id,
        author: responder.public().sign_pub,
        body,
        sig: Signature::invalid(),
    };
    response_inner.sig = response_inner.sign_with(&responder.signing_key());
    response_inner
}

pub fn open_and_vouch(
    owner: &Identity,
    sealed: &PublicKeySealed,
) -> crate::Result<ResponseRebroadcast> {
    let opened = PublicKeySealed::open(&owner.dh_secret(), sealed)?;
    let response_inner = postcard::from_bytes::<ResponseInner>(&opened)?;
    let vk = ed25519_dalek::VerifyingKey::try_from(&response_inner.author)?;

    let sig = ed25519_dalek::Signature::from(response_inner.sig);

    vk.verify_strict(&response_inner.signing_bytes(), &sig)
        .map_err(|_| crate::Error::Signature)?;

    let sig = owner.signing_key().sign(&response_inner.signing_bytes());
    Ok(ResponseRebroadcast {
        inner: response_inner,
        vouch_sig: sig.into(),
    })
}

pub fn open_rebroadcast(
    recipient: &Identity,
    owner: &SigningPublicKey,
    sealed: &PublicKeySealed,
) -> crate::Result<ResponseRebroadcast> {
    let opened = PublicKeySealed::open(&recipient.dh_secret(), sealed)?;
    let rb = postcard::from_bytes::<ResponseRebroadcast>(&opened)?;

    let vk = ed25519_dalek::VerifyingKey::try_from(owner)?;
    let sig = ed25519_dalek::Signature::from(rb.vouch_sig);
    vk.verify_strict(&rb.inner.signing_bytes(), &sig)
        .map_err(|_| crate::Error::Signature)?;

    Ok(rb)
}

#[cfg(test)]
mod tests {
    use crate::identity::DhPublicKey;

    use super::*;

    // Helper: seal a signed ResponseInner to a recipient, the way Account::react would.
    fn seal_response(inner: &ResponseInner, recipient: &DhPublicKey) -> PublicKeySealed {
        let bytes = postcard::to_allocvec(inner).expect("serializes");
        PublicKeySealed::seal(recipient, &bytes)
    }

    // Helper: seal a rebroadcast to a recipient, the way the owner's rebroadcast loop would.
    fn seal_rebroadcast(rb: &ResponseRebroadcast, recipient: &DhPublicKey) -> PublicKeySealed {
        let bytes = postcard::to_allocvec(rb).expect("serializes");
        PublicKeySealed::seal(recipient, &bytes)
    }

    #[test]
    fn full_reaction_round_trip() {
        // Carl reacts to Alice's post; Alice vouches; Bob receives.
        let alice = Identity::generate();
        let carl = Identity::generate();
        let bob = Identity::generate();

        let letter_id = LetterId::from([1u8; 16]);

        // 1. Carl builds + signs a reaction, seals it to Alice (the post owner).
        let inner = create_response(
            &carl,
            letter_id,
            ResponseBody::Reaction {
                emoji: "👍".to_string(),
            },
        );
        let sealed_to_alice = seal_response(&inner, &alice.public().dh_pub);

        // 2. Alice opens it, verifies Carl's sig, and vouches.
        let rb = open_and_vouch(&alice, &sealed_to_alice).unwrap();
        assert_eq!(rb.inner.letter_id, letter_id);
        assert_eq!(rb.inner.author, carl.public().sign_pub);
        match &rb.inner.body {
            ResponseBody::Reaction { emoji } => assert_eq!(emoji, "👍"),
            _ => panic!("expected a reaction"),
        }

        // 3. Alice reseals the rebroadcast to Bob; Bob verifies Alice's vouch.
        let sealed_to_bob = seal_rebroadcast(&rb, &bob.public().dh_pub);
        let received = open_rebroadcast(&bob, &alice.public().sign_pub, &sealed_to_bob).unwrap();

        // The reaction survived intact.
        assert_eq!(received.inner.author, carl.public().sign_pub);
        match received.inner.body {
            ResponseBody::Reaction { emoji } => assert_eq!(emoji, "👍"),
            _ => panic!("expected a reaction"),
        }
    }

    #[test]
    fn only_the_owner_can_open_the_response() {
        // A response sealed to Alice cannot be opened by anyone else.
        let alice = Identity::generate();
        let carl = Identity::generate();
        let mallory = Identity::generate();

        let inner = create_response(
            &carl,
            LetterId::from([1u8; 16]),
            ResponseBody::Reaction { emoji: "x".into() },
        );
        let sealed_to_alice = seal_response(&inner, &alice.public().dh_pub);

        // Mallory isn't the recipient — open should fail at the unseal step.
        assert!(open_and_vouch(&mallory, &sealed_to_alice).is_err());
    }

    #[test]
    fn forged_responder_signature_is_rejected() {
        // Tamper with a signed response so the responder's signature no longer matches.
        let alice = Identity::generate();
        let carl = Identity::generate();

        let mut inner = create_response(
            &carl,
            LetterId::from([1u8; 16]),
            ResponseBody::Reaction {
                emoji: "👍".into()
            },
        );
        // Change the body after signing — sig no longer covers these bytes.
        inner.body = ResponseBody::Reaction {
            emoji: "😈".into()
        };

        let sealed = seal_response(&inner, &alice.public().dh_pub);
        assert!(open_and_vouch(&alice, &sealed).is_err());
    }

    #[test]
    fn rebroadcast_verified_against_wrong_owner_is_rejected() {
        // THE trust check: Bob must verify the vouch against the ACTUAL owner's key.
        // If he checks against the wrong key, verification must fail.
        let alice = Identity::generate();
        let carl = Identity::generate();
        let bob = Identity::generate();
        let impostor = Identity::generate();

        let inner = create_response(
            &carl,
            LetterId::from([1u8; 16]),
            ResponseBody::Reaction {
                emoji: "👍".into()
            },
        );
        let sealed_to_alice = seal_response(&inner, &alice.public().dh_pub);
        let rb = open_and_vouch(&alice, &sealed_to_alice).unwrap();
        let sealed_to_bob = seal_rebroadcast(&rb, &bob.public().dh_pub);

        // Correct owner key: succeeds.
        assert!(open_rebroadcast(&bob, &alice.public().sign_pub, &sealed_to_bob).is_ok());
        // Wrong owner key (impostor): the vouch doesn't verify → rejected.
        assert!(open_rebroadcast(&bob, &impostor.public().sign_pub, &sealed_to_bob).is_err());
    }

    #[test]
    fn comment_body_survives_round_trip() {
        // Same path, Comment variant, to confirm both variants work through one code path.
        let alice = Identity::generate();
        let carl = Identity::generate();
        let bob = Identity::generate();

        let inner = create_response(
            &carl,
            LetterId::from([2u8; 16]),
            ResponseBody::Comment {
                text: "cool comment".to_string(),
            },
        );
        let sealed = seal_response(&inner, &alice.public().dh_pub);
        let rb = open_and_vouch(&alice, &sealed).unwrap();
        let sealed_to_bob = seal_rebroadcast(&rb, &bob.public().dh_pub);
        let received = open_rebroadcast(&bob, &alice.public().sign_pub, &sealed_to_bob).unwrap();

        match received.inner.body {
            ResponseBody::Comment { text } => assert_eq!(text, "cool comment".to_string()),
            _ => panic!("expected a comment"),
        }
    }
}
