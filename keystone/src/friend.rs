use crate::{
    crypto,
    identity::{Identity, PublicIdentity},
    labels,
};

#[derive(Clone, PartialEq)]
pub struct Friend {
    pub public: PublicIdentity,
    pub nickname: String,
    pub pairwise_root: [u8; 32],
}

pub fn add_friend(me: &Identity, their: &PublicIdentity, nickname: &str) -> Friend {
    let shared = me
        .dh_secret()
        .diffie_hellman(&(&their.dh_pub).into())
        .to_bytes();
    let me_pub = me.public();
    let (lo, hi) = if me_pub.dh_pub <= their.dh_pub {
        (me_pub.dh_pub, their.dh_pub)
    } else {
        (their.dh_pub, me_pub.dh_pub)
    };
    let info = [labels::PAIRWISE, &lo.to_bytes(), &hi.to_bytes()].concat();
    let pairwise_root = crypto::derive32(&shared, &info);
    Friend {
        public: their.clone(),
        nickname: nickname.to_string(),
        pairwise_root,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;

    #[test]
    fn both_sides_derive_the_same_root() {
        let (a, b) = (Identity::generate(), Identity::generate());
        let a_sees_b = add_friend(&a, &b.public(), "B");
        let b_sees_a = add_friend(&b, &a.public(), "A");
        assert_eq!(a_sees_b.pairwise_root, b_sees_a.pairwise_root);
    }
}
