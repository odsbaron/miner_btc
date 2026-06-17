use anyhow::Result;

use crate::block::CandidateBlock;
use crate::rpc::RpcClient;

pub fn submit_candidate(rpc: &RpcClient, block: &CandidateBlock) -> Result<Option<String>> {
    rpc.submit_block(&block.to_hex())
}
