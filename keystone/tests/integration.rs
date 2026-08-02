use keystone::{message::Message, post::PostContent, Envelope, *};

#[test]
fn alice_posts_bob_reads() {
    let alice = Identity::generate();
    let bob = Identity::generate();

    let alice_id = PublicIdentity::try_from(&alice.public().to_bytes()[..]).unwrap(); // like from qr
    let bob_id = bob.public();

    let a = friend::add_friend(&alice, &bob_id, "Bob");
    let b = friend::add_friend(&bob, &alice_id, "Alice");
    assert_eq!(a.pairwise_root, b.pairwise_root);

    let post = PostContent::from_body("Hello bob 📸");
    let envelopes = Envelope::seal_envelope(&alice, &Message::Post(post), &[bob_id]);
    let Message::Post(post) = envelopes[0].open_envelope(&bob, &alice_id).unwrap() else {
        panic!("expected Post");
    };

    assert_eq!(post.body, "Hello bob 📸")
}

#[test]
fn stranger_and_tampering_are_rejected() {
    let alice = Identity::generate();
    let bob = Identity::generate();
    let mallory = Identity::generate();

    let post = PostContent::from_body("secret");
    let mut envelopes = Envelope::seal_envelope(&alice, &Message::Post(post), &[bob.public()]);
    assert!(envelopes[0].open_envelope(&mallory, &alice.public()).is_err()); // not a recipient
    envelopes[0].post.content_ct[0] ^= 0xff;
    assert!(envelopes[0].open_envelope(&bob, &alice.public()).is_err()); // tampered
}
