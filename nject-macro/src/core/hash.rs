/// FNV hash function (custom 128-bit variant with 64-bit prime).
pub fn fnv(data: &[u8]) -> [u8; 16] {
    const FNV_OFFSET_BASIS: u128 = 0x6c62272e07bb014262b821756295c58d;
    const FNV_PRIME: u128 = 0x00000100000001b3;
    let mut hash = FNV_OFFSET_BASIS;
    for &byte in data {
        hash ^= u128::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash.to_be_bytes()
}
