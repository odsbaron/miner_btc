# bitcoinminer topic integration plan

This project intentionally keeps `miner_btc` as a real-protocol Rust miner scaffold instead of copying unsafe or unverifiable "BTC miner" repos.

## Reviewed GitHub topic projects

| Source project | Classification | Integration decision |
|---|---|---|
| `HugoXOX3/PythonBitcoinMiner` | Python Stratum/RPC educational miner | Port the Stratum v1 message flow concept: subscribe, authorize, set_difficulty, notify, submit. Do not copy byte/string mining code. |
| `whatsy12/bitcoinminer` | C++ Bitcoin Core RPC solo miner | Use as a checklist for template refresh and submitblock-oriented solo mining. Current Rust regtest path is already Bitcoin Core validated. |
| `imlenti/minerwatch-app-store` | Umbrel Community App Store for miner dashboards | Borrow local-first deployment shape for future dashboard/Umbrel packaging. |
| `gbechtel-beck/avalon-q-controller` | Avalon Q controller/dashboard | Borrow hardware-control abstraction: status, pool rotation, work mode, standby. |
| `mcschwa/BTCWalletMiner` | Wallet brute-force | Excluded: not mining, unsafe project direction. |
| `rafinhahdc19/BTCWalletMiner` | Wallet brute-force | Excluded: not mining, unsafe project direction. |
| `Lusin333/Bitcoin-Grinder` | Closed/binary "easy bitcoin" miner | Excluded: not auditable. |
| `T0xicDEV/BTC-MINER` | Fake/marketing script | Excluded: no real mining logic. |

## Implemented integration in this repo

1. Regtest acceptance automation
   - `scripts/regtest_acceptance.sh`
   - Starts isolated Bitcoin Core regtest, mines a coinbase-only block through `miner_btc`, calls `submitblock`, verifies height +1.

2. Non-empty block automation
   - `scripts/regtest_nonempty_block.sh`
   - Generates mature regtest funds, sends a real wallet transaction into mempool, mines a block with `nTx >= 2`, verifies `submitblock` acceptance and mempool clearing.

3. Stratum v1 testable skeleton
   - `src/stratum.rs`
   - Parses `mining.set_difficulty`, `mining.notify`, subscribe responses, and builds client JSON-RPC lines for `mining.subscribe`, `mining.authorize`, `mining.submit`.
   - Includes a local mock-pool handshake integration test.

4. Hardware/dashboard abstraction
   - `src/hardware.rs`
   - Adds local-first `MinerDevice` trait and status rendering inspired by MinerWatch/Avalon Q Controller.
   - No real device credentials or cloud dependency are introduced.

## Verification commands

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
./scripts/regtest_acceptance.sh
./scripts/regtest_nonempty_block.sh
```

## Next production steps

- Convert Stratum skeleton into an async reconnecting client.
- Add share-target/difficulty math and extranonce2 rolling.
- Add CPU worker dispatcher for educational shares.
- Add cgminer/Bitaxe/Avalon adapters behind `MinerDevice`.
- Package a dashboard as Docker/Umbrel only after protocol tests remain green.
