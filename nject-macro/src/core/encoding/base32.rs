pub fn encode(input: &[u8]) -> Vec<u8> {
    data_encoding::BASE32_NOPAD.encode(input).into_bytes()
}
