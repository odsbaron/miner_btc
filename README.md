# miner_btc

Rust layout for moving the previous Bitcoin mining simulation toward a real mining workflow.

This repository is **not** a profitable mainnet CPU miner. It is a real-protocol scaffold for local Bitcoin Core `regtest` / `signet` experiments:

```text
Bitcoin Core getblocktemplate
→ build BIP34 + SegWit coinbase
→ compute wtxid witness commitment
→ compute txid merkle root
→ mine block header nonce locally
→ optionally submitblock
```

## Why this exists

The earlier assignment-style project generated `out.txt` from static mempool fixtures. This repo reorganizes the logic into a production-shaped Rust codebase where the boundary is Bitcoin Core RPC instead of local JSON fixtures.

## Current status

Implemented:

- Rust crate layout by mining domain.
- Bitcoin Core JSON-RPC client.
- `getblocktemplate` parsing.
- Coinbase builder with:
  - BIP34 block height push;
  - miner extranonce/tag;
  - payout output;
  - SegWit witness commitment output.
- txid / wtxid handling.
- Merkle root and witness commitment calculation.
- 80-byte block header serialization.
- nonce mining against template target.
- full block serialization.
- optional `submitblock`.
- automated Bitcoin Core regtest acceptance script.
- automated non-empty mempool block script (`nTx >= 2`).
- Stratum v1 message/parser skeleton with local mock-pool handshake test.
- local-first hardware/dashboard abstraction inspired by MinerWatch / Avalon Q Controller.
- unit and integration tests for critical serialization / hashing / protocol boundaries.

Not implemented yet:

- production Stratum reconnecting miner loop.
- ASIC/cgminer/Bitaxe/Avalon device adapters.
- full transaction policy engine; this trusts Bitcoin Core's template.
- dynamic extranonce / ntime rolling after nonce space exhaustion.
- production observability / metrics dashboard.

## Repository layout

```text
src/
  main.rs       CLI entrypoint
  config.rs     clap CLI and env config
  rpc.rs        Bitcoin Core JSON-RPC client
  template.rs   getblocktemplate response types
  coinbase.rs   BIP34 + SegWit coinbase builder
  tx.rs         varint and witness stripping helpers
  merkle.rs     txid/wtxid merkle roots and witness commitment
  header.rs     block header serialization and nonce search
  block.rs      candidate block assembly
  miner.rs      orchestration from RPC template to candidate block
  submit.rs     submitblock wrapper
  crypto.rs     sha256d, hash endian conversion, target comparison
  stratum.rs    Stratum v1 parser/client-line skeleton
  hardware.rs   local-first miner-device/dashboard abstraction
scripts/
  regtest_acceptance.sh      coinbase-only submitblock acceptance test
  regtest_nonempty_block.sh  mempool transaction inclusion acceptance test
docs/
  bitcoinminer-topic-integration.md
```

## Build and test

```bash
cargo fmt
cargo test
cargo clippy -- -D warnings
cargo run -- doctor
./scripts/regtest_acceptance.sh
./scripts/regtest_nonempty_block.sh
```

## Regtest usage

Start Bitcoin Core regtest with RPC enabled. Example `bitcoin.conf`:

```ini
regtest=1
server=1
rpcuser=user
rpcpassword=pass
fallbackfee=0.0001
```

Create a regtest address and inspect its scriptPubKey:

```bash
bitcoin-cli -regtest createwallet miner || true
ADDR=$(bitcoin-cli -regtest getnewaddress "miner" bech32)
bitcoin-cli -regtest getaddressinfo "$ADDR" | jq -r .scriptPubKey
```

Run dry-run mining:

```bash
MINER_PAYOUT_SCRIPT_HEX=<scriptPubKeyHex> \
BITCOIN_RPC_URL=http://127.0.0.1:18443 \
BITCOIN_RPC_USER=user \
BITCOIN_RPC_PASS=pass \
cargo run -- regtest --max-nonce 5000000
```

Submit a found block:

```bash
cargo run -- regtest \
  --payout-script-hex <scriptPubKeyHex> \
  --max-nonce 5000000 \
  --submit
```

Verify:

```bash
bitcoin-cli -regtest getblockcount
bitcoin-cli -regtest getbestblockhash
```

## Conceptual difference from fixture simulation

| Fixture simulation | This layout |
|---|---|
| Reads static `mempool/*.json` | Calls Bitcoin Core `getblocktemplate` |
| Fake previous block hash | Uses template `previousblockhash` |
| Fixed assignment target | Uses template `target` / `bits` |
| Test-only coinbase | BIP34 + witness coinbase |
| Writes `out.txt` | Serializes full block hex |
| Jest checks | Bitcoin Core can validate with `submitblock` |

## Safety note

Use this for regtest/signet learning. Real mainnet mining requires ASIC hardware or a pool/Stratum integration. CPU mining against mainnet difficulty is economically meaningless.
