pub fn encode(input: &[u8]) -> Vec<u8> {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    const BITS_PER_CHAR: usize = 5;
    let mut output = Vec::with_capacity(input.len() * 8 / 5 + 2);
    let mut buffer: u64 = 0;
    let mut bits_in_buffer: usize = 0;
    for &byte in input {
        buffer = (buffer << 8) | (byte as u64);
        bits_in_buffer += 8;
        while bits_in_buffer >= BITS_PER_CHAR {
            let index = (buffer >> (bits_in_buffer - BITS_PER_CHAR)) as usize;
            output.push(ALPHABET[index]);
            buffer &= (1 << (bits_in_buffer - BITS_PER_CHAR)) - 1;
            bits_in_buffer -= BITS_PER_CHAR;
        }
    }
    if bits_in_buffer > 0 {
        buffer <<= BITS_PER_CHAR - bits_in_buffer;
        let index = buffer as usize;
        output.push(ALPHABET[index]);
    }
    output
}

#[cfg(test)]
pub fn decode(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(input.len());
    for chunk in input.chunks(8) {
        let mut buffer = [0u8; 8];
        for (i, &c) in chunk.iter().enumerate() {
            let index = match c {
                50..=55 => c - 24,
                65..=90 => c - 65,
                _ => return Err(format!("Invalid char: {c}")),
            };
            buffer[i] = index;
        }
        let b0 = (buffer[0] << 3) | (buffer[1] >> 2);
        let b1 = ((buffer[1] & 0b11) << 6) | (buffer[2] << 1) | (buffer[3] >> 4);
        let b2 = ((buffer[3] & 0b1111) << 4) | (buffer[4] >> 1);
        let b3 = ((buffer[4] & 0b1) << 7) | (buffer[5] << 2) | (buffer[6] >> 3);
        let b4 = ((buffer[6] & 0b111) << 5) | buffer[7];
        output.push(b0);
        output.push(b1);
        output.push(b2);
        output.push(b3);
        output.push(b4);
    }
    output.truncate(input.len() * 5 / 8);
    Ok(output)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encode_with_data_should_return_encoded_data() {
let data = "lib1 :: Module";
let encoded = encode(data.as_bytes());
let encoded = String::from_utf8(encoded).unwrap();
        assert_eq!(&encoded, "NRUWEMJAHI5CATLPMR2WYZI")
    }

    #[test]
    fn decode_with_encoded_data_should_return_decoded_data() {
let encoded = "NRUWEMJAHI5CATLPMR2WYZI";
let decoded = decode(encoded.as_bytes()).unwrap();
let decoded = String::from_utf8(decoded).unwrap();
        assert_eq!(&decoded, "lib1 :: Module")
    }

    #[test]
    fn encode_with_data_should_return_decodable_string() {
        let cases = vec![
            "A",
            "0",
            "1",
            "One",
            "Hello, World !!!",
            "lib1::Module1",
            "my::very::special_crate_123::Module456<Complex>",
        ];
        for case in cases {
        let data = case;
        let encoded = encode(data.as_bytes());
        let expected_encoded_len = data.len() * 8 / 5 + 2;
            assert!(
                encoded.len() <= expected_encoded_len,
                "{} <= {}",
                encoded.len(),
                expected_encoded_len
            );
            let decoded = decode(&encoded).unwrap();
            let decoded = String::from_utf8(decoded).unwrap();
            assert_eq!(&decoded, data)
        }
    }
}
