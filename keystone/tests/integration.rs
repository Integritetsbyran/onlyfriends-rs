use keystone::*;

#[test]
fn alice_posts_bob_reads() {
    let alice = Identity::generate();
    let bob = Identity::generate();

    let alice_id = PublicIdentity::from_bytes(&alice.public().to_bytes()).unwrap(); // like from qr
    let bob_id = bob.public();

    let a = friend::add_friend(&alice, &bob_id, "Bob");
    let b = friend::add_friend(&bob, &alice_id, "Alice");
    assert_eq!(a.pairwise_root, b.pairwise_root);

    let envelopes = post::create_post(&alice, "Hello bob 📸", &[bob_id]);
    let text = post::open_post(&bob, &alice_id, &envelopes[0]).unwrap();

    assert_eq!(text, "Hello bob 📸")
}

#[test]
fn stranger_and_tampering_are_rejected() {
    let alice = Identity::generate();
    let bob = Identity::generate();
    let mallory = Identity::generate();

    let mut envelopes = post::create_post(&alice, "secret", &[bob.public()]);
    assert!(post::open_post(&mallory, &alice.public(), &envelopes[0]).is_err()); // not a recipient
    envelopes[0].post.body_ct[0] ^= 0xff;
    assert!(post::open_post(&bob, &alice.public(), &envelopes[0]).is_err()); // tampered
}
