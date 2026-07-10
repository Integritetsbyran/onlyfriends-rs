pub mod relay_client;

use crate::relay_client::RelayClient;

pub fn epoch_now(seconds_per_epoch: u64) -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    now / seconds_per_epoch
}

/// `direction` distinguishes "my writes to them" from "their writes to me" so the
/// two directions of a friendship don't collide on the same address.
pub fn mailbox_address(pairwise_root: &[u8; 32], direction: u8, epoch: u64) -> String {
    let info = [
        keystone::labels::MAILBOX,
        &[direction],
        &epoch.to_be_bytes(),
    ]
    .concat();

    let addr_bytes = keystone::crypto::derive32(pairwise_root, &info);

    hex::encode(addr_bytes)
}

pub fn my_direction(me: &keystone::PublicIdentity, friend: &keystone::PublicIdentity) -> u8 {
    if me.dh_pub <= friend.dh_pub { 0 } else { 1 }
}

pub async fn send_post(
    relay: &RelayClient,
    author: &keystone::Identity,
    body: &str,
    friends: &[keystone::Friend],
) -> reqwest::Result<()> {
    let recipients: Vec<_> = friends.iter().map(|f| f.public.clone()).collect();
    let envelopes = keystone::post::create_post(author, body, &recipients);

    for (friend, envelope) in friends.iter().zip(envelopes.iter()) {
        let bytes = postcard::to_allocvec(envelope).expect("serializes");
        let epoch = epoch_now(60 * 60 * 24);
        let addr = mailbox_address(&friend.pairwise_root, my_direction(&author.public(), &friend.public), epoch);

        relay.post_item(&addr, &bytes).await?;
    }
    Ok(())
}

pub async fn fetch_posts(
    relay: &RelayClient,
    recipient: &keystone::Identity,
    author_pub: &keystone::PublicIdentity,
    friend: &keystone::Friend,
    direction: u8,
    lookback_epochs: u64,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let current = epoch_now(60 * 60 * 24);
    let start = current.saturating_sub(lookback_epochs);

    let mut results = Vec::new();

    for e in start..=current {
        let addr = mailbox_address(&friend.pairwise_root, direction, e);
        let items = relay.get_items(&addr, 0).await?;

        for item in items {
            let Ok(envelope) = postcard::from_bytes::<keystone::PostEnvelope>(&item) else {
                continue;
            };

            let Ok(text) = keystone::post::open_post(recipient, author_pub, &envelope) else {
                continue;
            };

            results.push(text);
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_inputs_same_address() {
        let root = [7u8; 32];
        assert_eq!(
            mailbox_address(&root, 0, 100),
            mailbox_address(&root, 0, 100)
        );
    }

    #[test]
    fn direction_and_epoch_change_the_address() {
        let root = [7u8; 32];
        assert_ne!(
            mailbox_address(&root, 0, 100),
            mailbox_address(&root, 1, 100)
        );
        assert_ne!(
            mailbox_address(&root, 0, 100),
            mailbox_address(&root, 0, 101)
        );
    }
}
