use client_core::account::SyncResult;
use keystone::{media::Media, post::PostContent};
use onlyfriends_time::seconds_since_epoch;
use std::sync::Arc;
use storage_sqlite::SqliteStorage;
use tokio::sync::Mutex;

#[tokio::main]
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for f in ["alice.sqlite", "bob.sqlite", "carl.sqlite"] {
        let _ = std::fs::remove_file(f);
    }

    let alice_db = Arc::new(Mutex::new(SqliteStorage::open("alice.sqlite")?));
    let bob_db = Arc::new(Mutex::new(SqliteStorage::open("bob.sqlite")?));
    let carl_db = Arc::new(Mutex::new(SqliteStorage::open("carl.sqlite")?));

    let mut alice = client_core::Account::create_new(alice_db, "http://127.0.0.1:3000").await?;
    let mut bob = client_core::Account::create_new(bob_db, "http://127.0.0.1:3000").await?;
    let mut carl = client_core::Account::create_new(carl_db, "http://127.0.0.1:3000").await?;

    let bob_id = bob.identity.public();
    let carl_id = carl.identity.public();

    println!("Identities generated.");

    alice.add_friend(&bob.identity.public(), "Bob").await?;
    bob.add_friend(&alice.identity.public(), "Alice").await?;
    alice.add_friend(&carl.identity.public(), "Carl").await?;
    carl.add_friend(&alice.identity.public(), "Alice").await?;
    println!("Friends exchanged.");

    alice.set_profile("Alice", "hi from alice").await?;
    let content = PostContent {
        body: "hello over the wire!".to_string(),
        created_at: seconds_since_epoch(),
        media: vec![Media {
            mime: "image/webp".parse().unwrap(),
            bytes: include_bytes!("../../../ui/ui-common/assets/frog.webp").to_vec(),
        }],
    };
    let post_id = alice.send_post(&content).await?.expect("alice has friends");
    println!("Alice set profile and posted.");

    // --- Bob sees the post and the profile ---
    let bob_result = bob.sync().await?;
    assert_eq!(texts(&bob_result), vec!["hello over the wire!"]);
    assert_eq!(bob_result.updated_profiles[0].display_name, "Alice");
    assert_eq!(bob_result.new_posts[0], content);

    // --- Carl sees the same ---
    let carl_result = carl.sync().await?;
    assert_eq!(texts(&carl_result), vec!["hello over the wire!"]);
    assert_eq!(carl_result.updated_profiles[0].display_name, "Alice");
    assert_eq!(carl_result.new_posts[0], content);

    // --- profile edit propagates ---
    alice.set_profile("Alice B.", "updated bio").await?;
    let bob_result2 = bob.sync().await?;
    assert_eq!(bob_result2.updated_profiles[0].display_name, "Alice B.");
    carl.sync().await?;
    println!("Both friends saw the profile update.");

    // --- Bob reacts, Carl comments — both to Alice, the post owner ---
    bob.react(post_id, &alice.identity.public().sign_pub, "👍")
        .await?;
    carl.comment(post_id, &alice.identity.public().sign_pub, "nice post!")
        .await?;
    println!("Bob reacted, Carl commented.");

    // --- Alice syncs: receives both responses, vouches, rebroadcasts ---
    let alice_result = alice.sync().await?;
    assert_eq!(alice_result.new_responses.len(), 2);
    println!(
        "Alice saw {} new responses.",
        alice_result.new_responses.len()
    );

    // --- Carl syncs: should now see Bob's reaction via Alice's rebroadcast ---
    let carl_result2 = carl.sync().await?;
    let carl_sees_bobs_reaction = carl_result2.new_responses.iter().any(|r| {
        matches!(&r.body, keystone::ResponseBody::Reaction { emoji } if emoji == "👍")
            && r.author == bob.identity.public().sign_pub
    });
    assert!(
        carl_sees_bobs_reaction,
        "Carl should receive Bob's reaction via Alice's rebroadcast"
    );

    // --- Bob syncs: should see Carl's comment via Alice's rebroadcast ---
    let bob_result3 = bob.sync().await?;
    let bob_sees_carls_comment = bob_result3.new_responses.iter().any(|r| {
        matches!(&r.body, keystone::ResponseBody::Comment { text } if text == "nice post!")
            && r.author == carl.identity.public().sign_pub
    });
    assert!(
        bob_sees_carls_comment,
        "Bob should receive Carl's comment via Alice's rebroadcast"
    );
    println!("Both friends saw each other's responses via owner-as-hub rebroadcast.");

    // --- idempotency: re-syncing finds nothing new across the board ---
    let alice_again = alice.sync().await?;
    assert!(
        alice_again.new_posts.is_empty()
            && alice_again.updated_profiles.is_empty()
            && alice_again.new_responses.is_empty()
    );
    println!("Re-sync correctly found nothing new.");

    for (name, account) in [
        ("Alice", &mut alice),
        ("Bob", &mut bob),
        ("Carl", &mut carl),
    ] {
        println!("Checking {name}s feed");

        let feed = account.load_feed().await?;
        assert_eq!(
            feed.len(),
            1,
            "{name} should have exactly one post in their feed"
        );

        let post = &feed[0];
        assert_eq!(post_id, post.id);
        assert_eq!(content, post.content);

        assert_eq!(post.reactions.len(), 1, "{name} should see Bob's reaction");
        assert_eq!(post.reactions[0].emoji, "👍");
        assert_eq!(post.reactions[0].author, bob_id.sign_pub);

        assert_eq!(post.comments.len(), 1, "{name} should see Carl's comment");
        assert_eq!(post.comments[0].text, "nice post!");
        assert_eq!(post.comments[0].author, carl_id.sign_pub);

        println!(
            "{name}'s feed: 1 post, {} reaction(s), {} comment(s) — matches.",
            post.reactions.len(),
            post.comments.len()
        );
    }

    println!("All checks passed!");
    Ok(())
}

fn texts(sync_result: &SyncResult) -> Vec<&str> {
    sync_result
        .new_posts
        .iter()
        .map(|p| p.body.as_str())
        .collect()
}
