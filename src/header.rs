use anyhow::Result;

use crate::crypto::{hash_meets_target, parse_hash_be_to_internal_le, sha256d, target_from_be_hex};

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub version: i32,
    pub previous_block_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub time: u32,
    pub bits: u32,
    pub nonce: u32,
}

impl BlockHeader {
    pub fn from_template_fields(
        version: i32,
        previousblockhash: &str,
        merkle_root: [u8; 32],
        time: u64,
        bits_hex: &str,
    ) -> Result<Self> {
        Ok(Self {
            version,
            previous_block_hash: parse_hash_be_to_internal_le(previousblockhash)?,
            merkle_root,
            time: time as u32,
            bits: u32::from_str_radix(bits_hex, 16)?,
            nonce: 0,
        })
    }

    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(80);
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&self.previous_block_hash);
        out.extend_from_slice(&self.merkle_root);
        out.extend_from_slice(&self.time.to_le_bytes());
        out.extend_from_slice(&self.bits.to_le_bytes());
        out.extend_from_slice(&self.nonce.to_le_bytes());
        out
    }

    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        sha256d(&self.serialize())
    }
}

pub fn mine_header(
    mut header: BlockHeader,
    target_be_hex: &str,
    max_nonce: u32,
) -> Result<Option<BlockHeader>> {
    let target = target_from_be_hex(target_be_hex)?;
    for nonce in 0..=max_nonce {
        header.nonce = nonce;
        if hash_meets_target(header.hash(), target) {
            return Ok(Some(header));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_serializes_to_80_bytes() {
        let header = BlockHeader {
            version: 4,
            previous_block_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            time: 1,
            bits: 0x207f_ffff,
            nonce: 42,
        };
        assert_eq!(header.serialize().len(), 80);
    }
}
