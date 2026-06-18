use std::ops::RangeInclusive;

use anyhow::Result;

use crate::crypto::{hash_meets_target, sha256d, target_from_be_hex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkUnit {
    pub job_id: String,
    pub header_prefix: [u8; 76],
    pub nonce_start: u32,
    pub nonce_end: u32,
    pub target_be_hex: String,
    pub extranonce2: String,
    pub ntime: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Share {
    pub job_id: String,
    pub extranonce2: String,
    pub ntime: String,
    pub nonce: u32,
    pub hash: [u8; 32],
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CpuWorker;

impl CpuWorker {
    pub fn scan(&self, unit: WorkUnit) -> Result<Option<Share>> {
        let target = target_from_be_hex(&unit.target_be_hex)?;
        for nonce in unit.nonce_start..=unit.nonce_end {
            let mut header = Vec::with_capacity(80);
            header.extend_from_slice(&unit.header_prefix);
            header.extend_from_slice(&nonce.to_le_bytes());
            let hash = sha256d(&header);
            if hash_meets_target(hash, target) {
                return Ok(Some(Share {
                    job_id: unit.job_id,
                    extranonce2: unit.extranonce2,
                    ntime: unit.ntime,
                    nonce,
                    hash,
                }));
            }
        }
        Ok(None)
    }
}

#[must_use]
pub fn dispatch_nonce_ranges(start: u32, end: u32, workers: usize) -> Vec<RangeInclusive<u32>> {
    if workers == 0 || start > end {
        return Vec::new();
    }
    let total = end as u64 - start as u64 + 1;
    let chunk = total.div_ceil(workers as u64);
    let mut ranges = Vec::new();
    let mut cursor = start as u64;
    while cursor <= end as u64 {
        let chunk_end = (cursor + chunk - 1).min(end as u64);
        ranges.push(cursor as u32..=chunk_end as u32);
        cursor = chunk_end + 1;
    }
    ranges
}
