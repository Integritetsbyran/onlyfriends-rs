#[tokio::main]
async fn main() {
    let alice = keystone::Identity::generate();
    let bob = keystone::Identity::generate();
    let carl = keystone::Identity::generate();

    println!("Identities generated for two users.");

    // pretend they exchanged cards via QR already:
    let alice_friend_of_bob = keystone::friend::add_friend(&bob, &alice.public(), "Alice");
    let bob_friend_of_alice = keystone::friend::add_friend(&alice, &bob.public(), "Bob");
    let carl_friend_of_alice = keystone::friend::add_friend(&alice, &carl.public(), "Carl");
    let alice_friend_of_carl = keystone::friend::add_friend(&carl, &alice.public(), "Alice");

    println!("Exchanged the tokens via QR or similar");

    let relay = client_core::relay_client::RelayClient::new("http://127.0.0.1:3000");

    println!("Connected to the relay");

    // Alice posts to Bob's mailbox.
    client_core::send_post(
        &relay,
        &alice,
        "hello over the wire!",
        &[bob_friend_of_alice, carl_friend_of_alice]
    )
    .await
    .unwrap();

    println!("Alice posts into bobs mailbox");

    // Bob fetches and decrypts.
    let posts = client_core::fetch_posts(&relay, &bob, &alice.public(), &alice_friend_of_bob, 0, 2)
        .await
        .unwrap();

    println!("Bob sees: {posts:?}");
    assert_eq!(posts, vec!["hello over the wire!".to_string()]);

    let posts = client_core::fetch_posts(&relay, &carl, &alice.public(), &alice_friend_of_carl, 0, 2)
        .await
        .unwrap();

    println!("Carl sees: {posts:?}");
    assert_eq!(posts, vec!["hello over the wire!".to_string()]);
}
