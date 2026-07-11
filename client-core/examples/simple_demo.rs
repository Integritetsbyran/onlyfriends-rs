#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let alice = client_core::Account::open("alice.sqlite", "http://127.0.0.1:3000")?;
    let bob = client_core::Account::open("bob.sqlite", "http://127.0.0.1:3000")?;
    let carl = client_core::Account::open("carl.sqlite", "http://127.0.0.1:3000")?;

    println!("Identities generated for two users.");

    alice.add_friend(&bob.identity.public(), "Bob")?;
    bob.add_friend(&alice.identity.public(), "Alice")?;
    alice.add_friend(&carl.identity.public(), "Carl")?;
    carl.add_friend(&alice.identity.public(), "Alice")?;

    println!("Exchanged the tokens via QR or similar");

    alice.send_post("hello over the wire!").await.unwrap();

    println!("Alice posts into friends");

    let bob_new_posts = bob.sync().await?;
    println!("Bob sees: {bob_new_posts:?}");

    let carl_new_posts = carl.sync().await?;
    println!("Carl sees: {carl_new_posts:?}");

    Ok(())
}
