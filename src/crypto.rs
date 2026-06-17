use sha2::{Digest, Sha256};

#[must_use]
pub fn sha256d(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    let second = Sha256::digest(first);
    second.into()
}

#[must_use]
pub fn reverse32(mut bytes: [u8; 32]) -> [u8; 32] {
    bytes.reverse();
    bytes
}

pub fn parse_hash_be_to_internal_le(hex_be: &str) -> anyhow::Result<[u8; 32]> {
    let mut bytes: [u8; 32] = hex::decode(hex_be)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("expected 32-byte hash hex, got {} chars", hex_be.len()))?;
    bytes.reverse();
    Ok(bytes)
}

#[must_use]
pub fn internal_le_to_display_be(hash: [u8; 32]) -> String {
    hex::encode(reverse32(hash))
}

pub fn target_from_be_hex(hex_be: &str) -> anyhow::Result<[u8; 32]> {
    let bytes: [u8; 32] = hex::decode(hex_be)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("expected 32-byte target hex, got {} chars", hex_be.len()))?;
    Ok(bytes)
}

/// Compare an internal little-endian sha256d hash against a target displayed in
/// conventional big-endian block-explorer order.
#[must_use]
pub fn hash_meets_target(hash_internal_le: [u8; 32], target_be: [u8; 32]) -> bool {
    let hash_be = reverse32(hash_internal_le);
    hash_be <= target_be
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_display_hash_to_internal_little_endian() {
        let input = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let parsed = parse_hash_be_to_internal_le(input).unwrap();
        assert_eq!(parsed[0], 0x1f);
        assert_eq!(parsed[31], 0x00);
        assert_eq!(internal_le_to_display_be(parsed), input);
    }

    #[test]
    fn target_comparison_uses_display_order() {
        let target =
            target_from_be_hex("0000ffff00000000000000000000000000000000000000000000000000000000")
                .unwrap();
        let good = parse_hash_be_to_internal_le(
            "0000000100000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        let bad = parse_hash_be_to_internal_le(
            "0001ffff00000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        assert!(hash_meets_target(good, target));
        assert!(!hash_meets_target(bad, target));
    }
}
