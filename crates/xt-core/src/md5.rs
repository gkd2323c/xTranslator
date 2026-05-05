//! Minimal MD5 implementation for API signing.
//!
//! Used by Baidu and Youdao translation providers for authentication signature.
//! This avoids an external crate dependency.

/// Compute the MD5 hash of the input string and return it as a hex string.
pub fn md5_hex(input: &str) -> String {
    let digest = md5_hash(input.as_bytes());
    format!(
        "{:08x}{:08x}{:08x}{:08x}",
        u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]),
        u32::from_be_bytes([digest[4], digest[5], digest[6], digest[7]]),
        u32::from_be_bytes([digest[8], digest[9], digest[10], digest[11]]),
        u32::from_be_bytes([digest[12], digest[13], digest[14], digest[15]]),
    )
}

fn md5_hash(data: &[u8]) -> [u8; 16] {
    let mut state: [u32; 4] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476];
    let msg = pad_message(data);
    let rounds = msg.len() / 64;

    for chunk_index in 0..rounds {
        let chunk = &msg[chunk_index * 64..(chunk_index + 1) * 64];
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }

        let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);

        for (i, &(f_val, k_val, s_val)) in MD5_STEPS.iter().enumerate() {
            let g = md5_g(i);
            let f = match f_val {
                0 => (b & c) | ((!b) & d),
                1 => (b & d) | (c & (!d)),
                2 => b ^ c ^ d,
                _ => c ^ (b | (!d)),
            };

            let temp = a
                .wrapping_add(f)
                .wrapping_add(k_val)
                .wrapping_add(m[g])
                .rotate_left(s_val)
                .wrapping_add(b);

            a = d;
            d = c;
            c = b;
            b = temp;
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }

    let mut result = [0u8; 16];
    for i in 0..4 {
        let bytes = state[i].to_le_bytes();
        result[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
    }
    result
}

const MD5_STEPS: [(u8, u32, u32); 64] = [
    // round 1
    (0, 0xD76AA478, 7), (0, 0xE8C7B756, 12), (0, 0x242070DB, 17), (0, 0xC1BDCEEE, 22),
    (0, 0xF57C0FAF, 7), (0, 0x4787C62A, 12), (0, 0xA8304613, 17), (0, 0xFD469501, 22),
    (0, 0x698098D8, 7), (0, 0x8B44F7AF, 12), (0, 0xFFFF5BB1, 17), (0, 0x895CD7BE, 22),
    (0, 0x6B901122, 7), (0, 0xFD987193, 12), (0, 0xA679438E, 17), (0, 0x49B40821, 22),
    // round 2
    (1, 0xF61E2562, 5), (1, 0xC040B340, 9), (1, 0x265E5A51, 14), (1, 0xE9B6C7AA, 20),
    (1, 0xD62F105D, 5), (1, 0x02441453, 9), (1, 0xD8A1E681, 14), (1, 0xE7D3FBC8, 20),
    (1, 0x21E1CDE6, 5), (1, 0xC33707D6, 9), (1, 0xF4D50D87, 14), (1, 0x455A14ED, 20),
    (1, 0xA9E3E905, 5), (1, 0xFCEFA3F8, 9), (1, 0x676F02D9, 14), (1, 0x8D2A4C8A, 20),
    // round 3
    (2, 0xFFFA3942, 4), (2, 0x8771F681, 11), (2, 0x6D9D6122, 16), (2, 0xFDE5380C, 23),
    (2, 0xA4BEEA44, 4), (2, 0x4BDECFA9, 11), (2, 0xF6BB4B60, 16), (2, 0xBEBFBC70, 23),
    (2, 0x289B7EC6, 4), (2, 0xEAA127FA, 11), (2, 0xD4EF3085, 16), (2, 0x04881D05, 23),
    (2, 0xD9D4D039, 4), (2, 0xE6DB99E5, 11), (2, 0x1FA27CF8, 16), (2, 0xC4AC5665, 23),
    // round 4
    (3, 0xF4292244, 6), (3, 0x432AFF97, 10), (3, 0xAB9423A7, 15), (3, 0xFC93A039, 21),
    (3, 0x655B59C3, 6), (3, 0x8F0CCC92, 10), (3, 0xFFEFF47D, 15), (3, 0x85845DD1, 21),
    (3, 0x6FA87E4F, 6), (3, 0xFE2CE6E0, 10), (3, 0xA3014314, 15), (3, 0x4E0811A1, 21),
    (3, 0xF7537E82, 6), (3, 0xBD3AF235, 10), (3, 0x2AD7D2BB, 15), (3, 0xEB86D391, 21),
];

fn md5_g(i: usize) -> usize {
    if i < 16 {
        i
    } else if i < 32 {
        (5 * i + 1) % 16
    } else if i < 48 {
        (3 * i + 5) % 16
    } else {
        (7 * i) % 16
    }
}

fn pad_message(data: &[u8]) -> Vec<u8> {
    let original_len_bits = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&original_len_bits.to_le_bytes());
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_empty() {
        assert_eq!(md5_hex(""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_md5_hello() {
        assert_eq!(md5_hex("hello"), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn test_md5_hello_world() {
        assert_eq!(md5_hex("Hello World"), "b10a8db164e0754105b7a99be72e3fe5");
    }

    #[test]
    fn test_md5_length() {
        assert_eq!(md5_hex("test").len(), 32);
    }
}
