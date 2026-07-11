pub fn epoch_now(seconds_per_epoch: u64) -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    now / seconds_per_epoch
}

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

pub fn my_direction(
    sender_pub: &keystone::PublicIdentity,
    recipient_pub: &keystone::PublicIdentity,
) -> u8 {
    if sender_pub.dh_pub <= recipient_pub.dh_pub {
        0
    } else {
        1
    }
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
