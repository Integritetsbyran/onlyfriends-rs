#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // start each run clean
    for f in ["alice.sqlite", "bob.sqlite", "carl.sqlite"] {
        let _ = std::fs::remove_file(f);
    }

    let alice = client_core::Account::open("alice.sqlite", "http://127.0.0.1:3000")?;
    let bob = client_core::Account::open("bob.sqlite", "http://127.0.0.1:3000")?;
    let carl = client_core::Account::open("carl.sqlite", "http://127.0.0.1:3000")?;
    println!("Identities generated.");

    alice.add_friend(&bob.identity.public(), "Bob").await?;
    bob.add_friend(&alice.identity.public(), "Alice").await?;
    alice.add_friend(&carl.identity.public(), "Carl").await?;
    carl.add_friend(&alice.identity.public(), "Alice").await?;
    println!("Friends exchanged.");

    alice.set_profile("Alice", "hi from alice").await?;
    alice.send_post("hello over the wire!").await?;
    println!("Alice set profile and posted.");

    // --- Bob sees both the post and the profile ---
    let bob_result = bob.sync().await?;
    println!("Bob posts: {:?}", bob_result.new_posts);
    assert_eq!(bob_result.new_posts, vec!["hello over the wire!".to_string()]);
    assert_eq!(bob_result.updated_profiles.len(), 1);
    assert_eq!(bob_result.updated_profiles[0].display_name, "Alice");

    // --- Carl sees the same ---
    let carl_result = carl.sync().await?;
    println!("Carl posts: {:?}", carl_result.new_posts);
    assert_eq!(carl_result.new_posts, vec!["hello over the wire!".to_string()]);
    assert_eq!(carl_result.updated_profiles.len(), 1);
    assert_eq!(carl_result.updated_profiles[0].display_name, "Alice");

    // --- last-writer-wins: Alice edits, both friends see the update ---
    alice.set_profile("Alice B.", "updated bio").await?;
    let bob_result2 = bob.sync().await?;
    assert_eq!(bob_result2.updated_profiles[0].display_name, "Alice B.");
    let carl_result2 = carl.sync().await?;
    assert_eq!(carl_result2.updated_profiles[0].display_name, "Alice B.");
    println!("Both friends saw the profile update.");

    // --- idempotency: re-syncing finds nothing new (cursor + version guard) ---
    let bob_again = bob.sync().await?;
    assert!(bob_again.new_posts.is_empty() && bob_again.updated_profiles.is_empty());
    println!("Re-sync correctly found nothing new.");

    println!("All checks passed!");
    Ok(())
}
