use std::time::{Duration, Instant};

use tracing::Level;

use crate::{Opt, SharedStore};

/// How often to run the cleanup job.
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 30); // 30 min

const MAX_SLEEP_INTERVAL: Duration = Duration::from_secs(300);

/// Periodically clean up the store.
pub(crate) fn start_task(store: &SharedStore, opt: &Opt) {
    let store = store.clone();
    let max_age = Duration::from_secs(opt.mailbox_max_age);
    tokio::spawn(async move {
        loop {
            cleanup(&store, max_age);
            sleep(CLEANUP_INTERVAL).await;
        }
    });
}

#[tracing::instrument(skip_all, level = Level::DEBUG)]
fn cleanup(store: &SharedStore, max_age: Duration) {
    tracing::debug!("running");
    let mut store = store.lock().expect("poisoned");
    let mut removed_entries: usize = 0;
    let mailbox_count = store.mailboxes.len();
    store.mailboxes.retain(|_addr, mailbox| {
        // Remove expired mailbox entries
        let entry_count = mailbox.len();
        mailbox.retain(|entry| entry.uploaded_at.elapsed() <= max_age);
        removed_entries += entry_count.saturating_sub(mailbox.len());

        // Remove empty mailboxes
        !mailbox.is_empty()
    });
    let removed_mailboxes = mailbox_count.saturating_sub(store.mailboxes.len());

    if removed_entries > 0 || removed_mailboxes > 0 {
        tracing::info!("Removed {removed_entries} letters and {removed_mailboxes} mailboxes");
    }
}

/// Waits for the specified interval while taking into account system sleep or suspension.
/// The accuracy is to within about one minute.
async fn sleep(duration: Duration) {
    let started = Instant::now();

    loop {
        let elapsed = Instant::now().duration_since(started);

        if elapsed >= duration {
            return;
        }

        tokio::time::sleep(std::cmp::min(
            MAX_SLEEP_INTERVAL,
            duration.saturating_sub(elapsed),
        ))
        .await;
    }
}
