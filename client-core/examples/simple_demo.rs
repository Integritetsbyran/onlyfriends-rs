#[tokio::main]
async fn main() {
    let alice_storage = client_core::storage::Storage::open("alice.sqlite").unwrap();
    let alice = client_core::load_or_create_identity(&alice_storage);
    let bob_storage = client_core::storage::Storage::open("bob.sqlite").unwrap();
    let bob = client_core::load_or_create_identity(&bob_storage);
    let carl_storage = client_core::storage::Storage::open("carl.sqlite").unwrap();
    let carl = client_core::load_or_create_identity(&carl_storage);

    println!("Identities generated for two users.");

    // pretend they exchanged cards via QR already:
    let alice_friend_of_bob = client_core::add_friend(&bob_storage, &bob, &alice.public(), "Alice");
    let bob_friend_of_alice = client_core::add_friend(&alice_storage, &alice, &bob.public(), "Bob");
    let carl_friend_of_alice = client_core::add_friend(&alice_storage, &alice, &carl.public(), "Carl");
    let alice_friend_of_carl = client_core::add_friend(&carl_storage, &carl, &alice.public(), "Alice");

    println!("Exchanged the tokens via QR or similar");

    let relay = client_core::relay_client::RelayClient::new("http://127.0.0.1:3000");

    println!("Connected to the relay");

    // Alice posts to Bob's mailbox.
    client_core::send_post(
        &relay,
        &alice,
        "hello over the wire!",
        &[bob_friend_of_alice, carl_friend_of_alice],
    )
    .await
    .unwrap();

    println!("Alice posts into bobs mailbox");

    // Bob fetches and decrypts.
    let posts = client_core::fetch_posts(&relay, &bob, &alice.public(), &alice_friend_of_bob, client_core::my_direction(&alice.public(), &bob.public()), 2)
        .await
        .unwrap();

    println!("Bob sees: {posts:?}");
    assert_eq!(posts, vec!["hello over the wire!".to_string()]);

    let posts =
        client_core::fetch_posts(&relay, &carl, &alice.public(), &alice_friend_of_carl, client_core::my_direction(&alice.public(), &carl.public()), 2)
            .await
            .unwrap();

    println!("Carl sees: {posts:?}");
    assert_eq!(posts, vec!["hello over the wire!".to_string()]);
}
