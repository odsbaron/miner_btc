use clap::{Parser, Subcommand};

/// Rust Bitcoin miner layout for real node integration.
///
/// This is designed for local regtest/signet experimentation, not profitable
/// mainnet CPU mining. Mainnet mining requires ASIC hardware or a Stratum pool.
#[derive(Debug, Parser)]
#[command(name = "miner-btc", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check local configuration and print the intended mining mode.
    Doctor(RpcArgs),

    /// Fetch a getblocktemplate response from Bitcoin Core and summarize it.
    Template(RpcArgs),

    /// Build and mine a candidate block from getblocktemplate.
    ///
    /// By default this is dry-run and only prints the candidate block hash.
    /// Add --submit to call submitblock if a valid header is found.
    Regtest(RegtestArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct RpcArgs {
    /// Bitcoin Core RPC URL, e.g. http://127.0.0.1:18443 for regtest.
    #[arg(
        long,
        env = "BITCOIN_RPC_URL",
        default_value = "http://127.0.0.1:18443"
    )]
    pub rpc_url: String,

    /// Bitcoin Core RPC username.
    #[arg(long, env = "BITCOIN_RPC_USER", default_value = "user")]
    pub rpc_user: String,

    /// Bitcoin Core RPC password.
    #[arg(long, env = "BITCOIN_RPC_PASS", default_value = "pass")]
    pub rpc_pass: String,
}

#[derive(Debug, Clone, Parser)]
pub struct RegtestArgs {
    #[command(flatten)]
    pub rpc: RpcArgs,

    /// Miner payout scriptPubKey as hex, for example a regtest address script.
    /// Use `bitcoin-cli -regtest getaddressinfo <addr>` and copy scriptPubKey.
    #[arg(long, env = "MINER_PAYOUT_SCRIPT_HEX")]
    pub payout_script_hex: String,

    /// Submit the mined block with submitblock. Without this flag the program
    /// performs a dry-run and prints the candidate block hex length/hash.
    #[arg(long, default_value_t = false)]
    pub submit: bool,

    /// Maximum nonce attempts before giving up. Regtest templates are easy, but
    /// this prevents accidental infinite CPU loops with high-difficulty targets.
    #[arg(long, default_value_t = 5_000_000)]
    pub max_nonce: u32,
}
