pub fn encode(bytes: &[u8]) -> Vec<u8> {
    data_encoding::BASE64.encode(bytes).into_bytes()
}
