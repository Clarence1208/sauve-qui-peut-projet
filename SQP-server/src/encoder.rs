/// Encodes a byte vector to a base64 string using the custom SQP encoding.
/// This function is the inverse of the decode function in the client codebase.
pub fn encode(input: &[u8]) -> String {
    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/";
    let chars: Vec<char> = chars.chars().collect();
    
    let mut result = String::with_capacity((input.len() * 4 + 2) / 3);
    
    let mut i = 0;
    while i < input.len() {
        // Process 3 bytes at a time
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };
        
        // Extract 4 6-bit values from the 3 bytes
        let c0 = (b0 >> 2) & 0x3F;
        let c1 = ((b0 & 0x03) << 4) | ((b1 >> 4) & 0x0F);
        let c2 = ((b1 & 0x0F) << 2) | ((b2 >> 6) & 0x03);
        let c3 = b2 & 0x3F;
        
        // Append the corresponding characters
        result.push(chars[c0 as usize]);
        result.push(chars[c1 as usize]);
        
        // Only add the third character if we have at least 2 bytes of input
        if i + 1 < input.len() {
            result.push(chars[c2 as usize]);
        }
        
        // Only add the fourth character if we have 3 bytes of input
        if i + 2 < input.len() {
            result.push(chars[c3 as usize]);
        }
        
        i += 3;
    }
    
    result
}

// Define an error type for decoding
#[derive(Debug, PartialEq)]
pub enum DecodeError {
    InvalidSize,
    UnauthorizedCharacter(char),
    InvalidSegmentSize,
}

/// This is copied from the client for testing purposes.
pub fn decode(input: &str) -> Result<Vec<u8>, DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidSize);
    }

    // Map characters to their corresponding 6-bit values
    let mut values = Vec::new();
    for c in input.chars() {
        match char_to_value(c) {
            Some(v) => values.push(v),
            None => return Err(DecodeError::UnauthorizedCharacter(c)),
        }
    }

    let mut output = Vec::new();
    let mut i = 0;

    while i < values.len() {
        let chunk_len = std::cmp::min(4, values.len() - i);
        if chunk_len < 2 {
            return Err(DecodeError::InvalidSegmentSize);
        }

        let v0 = values[i];
        let v1 = if i + 1 < values.len() {
            values[i + 1]
        } else {
            0
        };
        let v2 = if i + 2 < values.len() {
            values[i + 2]
        } else {
            0
        };
        let v3 = if i + 3 < values.len() {
            values[i + 3]
        } else {
            0
        };

        let b0 = (v0 << 2) | (v1 >> 4);
        output.push(b0);

        if chunk_len >= 3 {
            let b1 = ((v1 & 0x0F) << 4) | (v2 >> 2);
            output.push(b1);
        }

        if chunk_len == 4 {
            let b2 = ((v2 & 0x03) << 6) | v3;
            output.push(b2);
        }

        i += 4;
    }

    Ok(output)
}

fn char_to_value(c: char) -> Option<u8> {
    match c {
        'a'..='z' => Some((c as u8) - b'a'),
        'A'..='Z' => Some((c as u8) - b'A' + 26),
        '0'..='9' => Some((c as u8) - b'0' + 52),
        '+' => Some(62),
        '/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encode() {
        // These tests match the decoder tests from the client code
        assert_eq!(encode(&[0]), "aa");
        assert_eq!(encode(&[25]), "gq");
        assert_eq!(encode(&[26]), "gG");
        assert_eq!(encode(&[51]), "mW");
        assert_eq!(encode(&[52]), "na");
        assert_eq!(encode(&[61]), "pq");
        assert_eq!(encode(&[62]), "pG");
        assert_eq!(encode(&[63]), "pW");
        assert_eq!(encode(b"Hello, World!"), "sgvSBg8SifDVCMXKiq");
        
        // Test encoding all possible byte values (0-255)
        let all_bytes: Vec<u8> = (0..=255).collect();
        assert_eq!(
            encode(&all_bytes),
            "aaecaWqfbGCicqOlda0odXareHmufryxgbKAgXWDhH8GisiJjcuMjYGPkISSls4VmdeYmZq1nJC4otO7pd0+p0bbqKneruzhseLks0XntK9quvjtvfvwv1HzwLTCxv5FygfIy2rLzMDOAwPRBg1UB3bXCNn0Dxz3EhL6E3X9FN+aGykdHiwgH4IjIOUmJy6pKjgsK5svLPEyMzQBNj2EN6cHOQoKPAANQkMQQ6YTRQ+WSBkZTlw2T7I5URU8VB6/WmhcW8tfXSFiYCRlZm3oZ9dr0Tpu1DBx2nNA29ZD3T/G4ElJ5oxM5+JP6UVS7E7V8phY8/t19VF4+FR7/p3+/W"
        );
    }
    
    #[test]
    fn test_encode_empty() {
        assert_eq!(encode(&[]), "");
    }
    
    #[test]
    fn test_encode_single_byte() {
        assert_eq!(encode(&[65]), "qq");
    }
    
    #[test]
    fn test_encode_two_bytes() {
        assert_eq!(encode(&[65, 66]), "qui");
    }
    
    #[test]
    fn test_encode_three_bytes() {
        assert_eq!(encode(&[65, 66, 67]), "qujd");
    }
    
    #[test]
    fn test_decode() {
        // Test decoding from the client code test cases
        assert_eq!(decode("aa"), Ok(vec![0]));
        assert_eq!(decode("gq"), Ok(vec![25]));
        assert_eq!(decode("gG"), Ok(vec![26]));
        assert_eq!(decode("mW"), Ok(vec![51]));
        assert_eq!(decode("na"), Ok(vec![52]));
        assert_eq!(decode("pq"), Ok(vec![61]));
        assert_eq!(decode("pG"), Ok(vec![62]));
        assert_eq!(decode("pW"), Ok(vec![63]));
        assert_eq!(decode("sgvSBg8SifDVCMXKiq"), Ok(b"Hello, World!".to_vec()));
    }
    
    #[test]
    fn test_decode_error() {
        // Test error cases
        assert_eq!(decode("a"), Err(DecodeError::InvalidSize));
        assert_eq!(decode("a*a"), Err(DecodeError::UnauthorizedCharacter('*')));
    }
    
    #[test]
    fn test_roundtrip() {
        // Test roundtrip encoding/decoding for various inputs
        let test_cases = vec![
            vec![],
            vec![0],
            vec![1, 2, 3],
            vec![255],
            vec![0, 128, 255],
            b"Hello, World!".to_vec(),
            (0..=255).collect(),
        ];
        
        for original in test_cases {
            let encoded = encode(&original);
            let decoded = decode(&encoded);
            assert_eq!(decoded, Ok(original));
        }
    }
} 