pub struct StoredPost {
    pub id: [u8; 16],
    pub author: [u8; 32],
    pub body: String, // decrypted
    pub created_at: u64,
}
