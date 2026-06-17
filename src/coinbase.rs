use anyhow::{Context, Result};

use crate::crypto::sha256d;
use crate::tx::{push_bytes, push_varint};

pub const WITNESS_RESERVED_VALUE: [u8; 32] = [0u8; 32];
const WITNESS_COMMITMENT_PREFIX: [u8; 6] = [0x6a, 0x24, 0xaa, 0x21, 0xa9, 0xed];

#[derive(Debug, Clone)]
pub struct CoinbaseSpec {
    pub height: u64,
    pub value_sats: u64,
    pub payout_script: Vec<u8>,
    pub witness_commitment: [u8; 32],
    pub extranonce: Vec<u8>,
    pub miner_tag: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CoinbaseTx {
    pub raw: Vec<u8>,
    pub txid: [u8; 32],
    pub wtxid: [u8; 32],
}

pub fn build_coinbase(spec: &CoinbaseSpec) -> Result<CoinbaseTx> {
    let mut script_sig = bip34_height_push(spec.height);
    push_bytes(&mut script_sig, &spec.extranonce);
    push_bytes(&mut script_sig, &spec.miner_tag);

    let mut commitment_script = Vec::from(WITNESS_COMMITMENT_PREFIX);
    commitment_script.extend_from_slice(&spec.witness_commitment);

    let mut raw = Vec::new();
    raw.extend_from_slice(&2i32.to_le_bytes());
    raw.extend_from_slice(&[0x00, 0x01]); // SegWit marker + flag.

    push_varint(&mut raw, 1);
    // Coinbase input spends the special null outpoint: zero hash + 0xffffffff index.
    raw.extend_from_slice(&[0x00; 32]);
    raw.extend_from_slice(&0xffff_ffffu32.to_le_bytes());
    push_bytes(&mut raw, &script_sig);
    raw.extend_from_slice(&0xffff_ffffu32.to_le_bytes());

    push_varint(&mut raw, 2);
    raw.extend_from_slice(&spec.value_sats.to_le_bytes());
    push_bytes(&mut raw, &spec.payout_script);
    raw.extend_from_slice(&0u64.to_le_bytes());
    push_bytes(&mut raw, &commitment_script);

    // One witness stack item: the 32-byte witness reserved value.
    push_varint(&mut raw, 1);
    push_bytes(&mut raw, &WITNESS_RESERVED_VALUE);
    raw.extend_from_slice(&0u32.to_le_bytes());

    let stripped = strip_coinbase_witness(&raw).context("failed to strip coinbase witness")?;
    Ok(CoinbaseTx {
        txid: sha256d(&stripped),
        wtxid: sha256d(&raw),
        raw,
    })
}

#[must_use]
pub fn bip34_height_push(height: u64) -> Vec<u8> {
    // Match Bitcoin Core's `CScript() << nHeight` encoding used by the BIP34
    // consensus check. Small integers are encoded as OP_N, not as raw pushdata.
    if height == 0 {
        return vec![0x00]; // OP_0
    }
    if (1..=16).contains(&height) {
        return vec![0x50 + height as u8]; // OP_1 .. OP_16
    }

    let mut encoded = height.to_le_bytes().to_vec();
    while encoded.last() == Some(&0) && encoded.len() > 1 {
        encoded.pop();
    }
    let mut script = Vec::with_capacity(encoded.len() + 1);
    script.push(encoded.len() as u8);
    script.extend_from_slice(&encoded);
    script
}

fn strip_coinbase_witness(raw: &[u8]) -> Option<Vec<u8>> {
    if raw.len() < 6 || raw[4] != 0x00 || raw[5] != 0x01 {
        return None;
    }
    // This builder always places witness immediately before locktime.
    let mut no_witness = Vec::new();
    no_witness.extend_from_slice(&raw[0..4]);
    let locktime = raw.get(raw.len().checked_sub(4)?..)?;
    // Remove marker/flag and the fixed coinbase witness stack: 01 20 <32 bytes>.
    let witness_start = raw.len().checked_sub(4 + 1 + 1 + 32)?;
    no_witness.extend_from_slice(&raw[6..witness_start]);
    no_witness.extend_from_slice(locktime);
    Some(no_witness)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bip34_height_matches_bitcoin_core_scriptnum_push() {
        assert_eq!(hex::encode(bip34_height_push(0)), "00");
        assert_eq!(hex::encode(bip34_height_push(1)), "51");
        assert_eq!(hex::encode(bip34_height_push(16)), "60");
        assert_eq!(hex::encode(bip34_height_push(17)), "0111");
        assert_eq!(hex::encode(bip34_height_push(256)), "020001");
    }

    fn sample_spec() -> CoinbaseSpec {
        CoinbaseSpec {
            height: 1,
            value_sats: 50_0000_0000,
            payout_script: vec![0x51],
            witness_commitment: [0x11; 32],
            extranonce: b"ex".to_vec(),
            miner_tag: b"miner_btc".to_vec(),
        }
    }

    #[test]
    fn coinbase_contains_witness_commitment_prefix() {
        let coinbase = build_coinbase(&sample_spec()).unwrap();
        assert!(hex::encode(coinbase.raw).contains("6a24aa21a9ed"));
    }

    #[test]
    fn coinbase_input_uses_null_prevout_hash() {
        let coinbase = build_coinbase(&sample_spec()).unwrap();
        // version(4) + segwit marker/flag(2) + vin_count(1), then 32-byte null prevout hash.
        assert_eq!(&coinbase.raw[7..39], &[0u8; 32]);
        assert_eq!(&coinbase.raw[39..43], &0xffff_ffffu32.to_le_bytes());
    }
}
