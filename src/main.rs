use anyhow::Result;
use clap::Parser;
use miner_btc::config::{Cli, Command};
use miner_btc::crypto::internal_le_to_display_be;
use miner_btc::dashboard::{
    serve_dashboard_with_auth, DashboardAuth, DashboardSnapshot, MetricsStore,
};
use miner_btc::hardware::{BitaxeAdapter, DeviceEndpoint, MinerDevice};
use miner_btc::miner::mine_from_rpc_template;
use miner_btc::rpc::RpcClient;
use miner_btc::submit::submit_candidate;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Doctor(args) => {
            println!("miner_btc real-mining layout");
            println!("RPC URL: {}", args.rpc_url);
            println!("Mode: Bitcoin Core regtest/signet getblocktemplate -> local PoW -> optional submitblock");
            println!("Safety: default regtest dry-run; mainnet CPU mining is not economically meaningful");
        }
        Command::Template(args) => {
            let rpc = RpcClient::new(args.rpc_url, args.rpc_user, args.rpc_pass);
            let template = rpc.get_block_template()?;
            println!("height: {}", template.height);
            println!("previousblockhash: {}", template.previousblockhash);
            println!("transactions: {}", template.transactions.len());
            println!("coinbasevalue: {} sats", template.coinbasevalue);
            println!("fees: {} sats", template.total_fees());
            println!("weight hint: {}", template.total_weight_hint());
            println!("bits: {}", template.bits);
            println!("target: {}", template.target);
        }
        Command::Regtest(args) => {
            let rpc = RpcClient::new(args.rpc.rpc_url, args.rpc.rpc_user, args.rpc.rpc_pass);
            let Some(candidate) =
                mine_from_rpc_template(&rpc, &args.payout_script_hex, args.max_nonce)?
            else {
                println!("no valid nonce found within --max-nonce={}", args.max_nonce);
                return Ok(());
            };
            println!("mined candidate block");
            println!("hash: {}", candidate.block_hash);
            println!(
                "header_hash_check: {}",
                internal_le_to_display_be(candidate.header.hash())
            );
            println!("tx_count: {}", candidate.transactions.len());
            println!("block_hex_len: {}", candidate.to_hex().len());
            if args.submit {
                let result = submit_candidate(&rpc, &candidate)?;
                println!("submitblock result: {:?}", result);
            } else {
                println!("dry-run only; add --submit to call submitblock");
            }
        }
        Command::Dashboard(args) => {
            let adapter = BitaxeAdapter::new(DeviceEndpoint::new(
                args.device_kind,
                args.device_host,
                args.device_port,
            ));
            let metrics_store = MetricsStore::new(args.metrics_path.clone().into());
            let metrics = metrics_store.load().ok();
            let snapshot = DashboardSnapshot {
                title: "miner_btc dashboard".to_string(),
                devices: vec![adapter.status()?],
                stratum_state: "Disconnected".to_string(),
                metrics,
            };
            let auth = if args.dashboard_token.is_empty() {
                DashboardAuth::disabled()
            } else {
                DashboardAuth::bearer(args.dashboard_token)
            };
            println!("serving miner_btc dashboard on http://{}", args.bind);
            serve_dashboard_with_auth(&args.bind, snapshot, auth)?;
        }
    }
    Ok(())
}
