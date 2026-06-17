//! Core modules for a regtest-oriented Bitcoin miner.
//!
//! The crate is intentionally split by mining domain instead of by data type:
//! RPC boundary, block-template parsing, coinbase construction, Merkle roots,
//! header mining, and block submission.

pub mod block;
pub mod coinbase;
pub mod config;
pub mod crypto;
pub mod hardware;
pub mod header;
pub mod merkle;
pub mod miner;
pub mod rpc;
pub mod stratum;
pub mod submit;
pub mod template;
pub mod tx;
