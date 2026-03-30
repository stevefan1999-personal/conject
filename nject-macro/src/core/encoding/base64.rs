pub fn encode(bytes: &[u8]) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const PADDING: u8 = b'=';
    let len = bytes.len();
    let out_len = (len + 2).div_ceil(3) * 4;
    let mut out = Vec::with_capacity(out_len);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] >> 2;
        let b = ((chunk[0] & 0b11) << 4)
            | chunk.get(1).map_or(0, |&x| x >> 4);
        let c = chunk.get(1).map_or(0, |&x|
            ((x & 0b1111) << 2) | chunk.get(2).map_or(0, |&y| y >> 6));
        let d = chunk.get(2).map_or(0, |&x| x & 0b111111);
        out.push(ALPHABET[a as usize]);
        out.push(ALPHABET[b as usize]);
        out.push(if chunk.len() > 1 {
            ALPHABET[c as usize]
        } else {
            PADDING
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[d as usize]
        } else {
            PADDING
        });
    }
    out
}

#[cfg(test)]
pub fn decode(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len() * 3 / 4);
    for chunk in data.chunks(4) {
        // Convert the chunk into a 24-bit integer
        let mut n = 0u32;
        for i in 0..4 {
            let c = chunk[i] as u32;
            let index = match c {
                43 => 62,
                47 => 63,
                48..=57 => c + 4,
                61 => 0,
                65..=90 => c - 65,
                97..=122 => c - 71,
                _ => panic!("Invalid char: {c}"),
            };
            n = (n << 6) | index;
        }
        for i in (0..3).rev() {
            output.push(((n >> (i * 8)) & 0xFF) as u8);
        }
        for &c in chunk {
            if c == b'=' {
                output.pop();
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_with_data_should_return_encoded_data() {
let data = "lib1 :: Module";
let encoded = encode(data.as_bytes());
let encoded = String::from_utf8(encoded).unwrap();
        assert_eq!(&encoded, "bGliMSA6OiBNb2R1bGU=")
    }

    #[test]
    fn decode_with_encoded_data_should_return_decoded_data() {
let encoded = "bGliMSA6OiBNb2R1bGU=";
let decoded = decode(encoded.as_bytes());
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
        let expected_encoded_len = (data.len() + 2) / 3 * 4;
            assert_eq!(encoded.len(), expected_encoded_len);
            let expected_decoded_len = encoded.len() * 3 / 4;
            let decoded = decode(&encoded);
            assert!(
                decoded.len() <= expected_decoded_len,
                "{:?} <= {:?}",
                decoded.len(),
                expected_decoded_len
            );
            let decoded = String::from_utf8(decoded).unwrap();
            assert_eq!(&decoded, data)
        }
    }
}
