pub struct StoredResponse {
    pub author: [u8; 32],
    pub kind: u8, // 0 reaction, 1 comment
    pub content: String,
}
