use anyhow::{Context, Result};

use crate::coinbase::{build_coinbase, CoinbaseSpec};
use crate::crypto::{internal_le_to_display_be, parse_hash_be_to_internal_le, sha256d};
use crate::header::{mine_header, BlockHeader};
use crate::merkle::{merkle_root, witness_commitment};
use crate::template::{BlockTemplate, TemplateTransaction};
use crate::tx::{push_varint, strip_witness};

#[derive(Debug, Clone)]
pub struct CandidateBlock {
    pub header: BlockHeader,
    pub transactions: Vec<Vec<u8>>,
    pub block_hash: String,
}

impl CandidateBlock {
    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = self.header.serialize();
        push_varint(&mut out, self.transactions.len() as u64);
        for tx in &self.transactions {
            out.extend_from_slice(tx);
        }
        out
    }

    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.serialize())
    }
}

pub fn build_and_mine_candidate(
    template: &BlockTemplate,
    payout_script: Vec<u8>,
    max_nonce: u32,
) -> Result<Option<CandidateBlock>> {
    let decoded = decode_template_transactions(&template.transactions)?;
    let txids: Vec<[u8; 32]> = decoded.iter().map(|tx| tx.txid).collect();
    let mut wtxids: Vec<[u8; 32]> = Vec::with_capacity(decoded.len() + 1);
    wtxids.push([0u8; 32]);
    wtxids.extend(decoded.iter().map(|tx| tx.wtxid));
    let commitment = witness_commitment(wtxids);

    let coinbase = build_coinbase(&CoinbaseSpec {
        height: template.height,
        value_sats: template.coinbasevalue,
        payout_script,
        witness_commitment: commitment,
        extranonce: 0u64.to_le_bytes().to_vec(),
        miner_tag: b"/miner_btc/".to_vec(),
    })?;

    let mut merkle_inputs = Vec::with_capacity(txids.len() + 1);
    merkle_inputs.push(coinbase.txid);
    merkle_inputs.extend(txids);
    let root = merkle_root(merkle_inputs);

    let header = BlockHeader::from_template_fields(
        template.version,
        &template.previousblockhash,
        root,
        template.curtime,
        &template.bits,
    )?;
    let Some(mined_header) = mine_header(header, &template.target, max_nonce)? else {
        return Ok(None);
    };

    let mut transactions = Vec::with_capacity(decoded.len() + 1);
    transactions.push(coinbase.raw);
    transactions.extend(decoded.into_iter().map(|tx| tx.raw));
    let block_hash = internal_le_to_display_be(mined_header.hash());
    Ok(Some(CandidateBlock {
        header: mined_header,
        transactions,
        block_hash,
    }))
}

#[derive(Debug, Clone)]
struct DecodedTemplateTx {
    raw: Vec<u8>,
    txid: [u8; 32],
    wtxid: [u8; 32],
}

fn decode_template_transactions(txs: &[TemplateTransaction]) -> Result<Vec<DecodedTemplateTx>> {
    txs.iter()
        .map(|tx| {
            let raw =
                hex::decode(&tx.data).with_context(|| format!("invalid tx hex: {}", tx.txid))?;
            let txid = txid_from_raw(&raw);
            let expected = parse_hash_be_to_internal_le(&tx.txid)?;
            if txid != expected {
                anyhow::bail!("template txid mismatch for {}", tx.txid);
            }
            Ok(DecodedTemplateTx {
                wtxid: sha256d(&raw),
                txid,
                raw,
            })
        })
        .collect()
}

fn txid_from_raw(raw: &[u8]) -> [u8; 32] {
    let no_witness = strip_witness(raw).unwrap_or_else(|| raw.to_vec());
    sha256d(&no_witness)
}
