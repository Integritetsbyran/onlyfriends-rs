pub struct NormalTime;

impl NormalTime {
    pub fn epoch_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time to move forward")
            .as_secs()
    }
}
