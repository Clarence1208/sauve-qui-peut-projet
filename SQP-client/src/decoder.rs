use crate::error::{Error, DecodeError};

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

pub(crate) fn decode(input: &str) -> Result<Vec<u8>, Error> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidSize.into());
    }

    let mut values = Vec::new();
    for c in input.chars() {
        match char_to_value(c) {
            Some(v) => values.push(v),
            None => return Err(DecodeError::UnauthorizedCharacter(c).into()),
        }
    }

    let mut output = Vec::new();
    let mut i = 0;

    while i < values.len() {
        let chunk_len = std::cmp::min(4, values.len() - i);
        if chunk_len < 2 {
            return Err(DecodeError::InvalidSegmentSize.into());
        }

        let v0 = values[i];
        let v1 = if i + 1 < values.len() { values[i + 1] } else { 0 };
        let v2 = if i + 2 < values.len() { values[i + 2] } else { 0 };
        let v3 = if i + 3 < values.len() { values[i + 3] } else { 0 };

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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_valid() {
        assert_eq!(decode("aa"), Ok(vec![0]));
        assert_eq!(decode("gq"), Ok(vec![25]));
        assert_eq!(decode("gG"), Ok(vec![26]));
        assert_eq!(decode("mW"), Ok(vec![51]));
        assert_eq!(decode("na"), Ok(vec![52]));
        assert_eq!(decode("pq"), Ok(vec![61]));
        assert_eq!(decode("pG"), Ok(vec![62]));
        assert_eq!(decode("pW"), Ok(vec![63]));
        assert_eq!(decode("sgvSBg8SifDVCMXKiq"), Ok(b"Hello, World!".to_vec()));
        assert_eq!(
            decode(
                "aaecaWqfbGCicqOlda0odXareHmufryxgbKAgXWDhH8GisiJjcuMjYGPkISSls4VmdeYmZq1nJC4otO7pd0+p0bbqKneruzhseLks0XntK9quvjtvfvwv1HzwLTCxv5FygfIy2rLzMDOAwPRBg1UB3bXCNn0Dxz3EhL6E3X9FN+aGykdHiwgH4IjIOUmJy6pKjgsK5svLPEyMzQBNj2EN6cHOQoKPAANQkMQQ6YTRQ+WSBkZTlw2T7I5URU8VB6/WmhcW8tfXSFiYCRlZm3oZ9dr0Tpu1DBx2nNA29ZD3T/G4ElJ5oxM5+JP6UVS7E7V8phY8/t19VF4+FR7/p3+/W"
            ),
            Ok((0..=255).collect())
        );
    }

    #[test]
    fn test_decode_invalid_character() {
        assert!(decode("a*a").is_err());
    }
}
