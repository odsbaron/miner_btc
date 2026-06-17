use anyhow::Result;

use crate::block::{build_and_mine_candidate, CandidateBlock};
use crate::rpc::RpcClient;

pub fn mine_from_rpc_template(
    rpc: &RpcClient,
    payout_script_hex: &str,
    max_nonce: u32,
) -> Result<Option<CandidateBlock>> {
    let template = rpc.get_block_template()?;
    let payout_script = hex::decode(payout_script_hex)?;
    build_and_mine_candidate(&template, payout_script, max_nonce)
}
