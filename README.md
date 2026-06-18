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
- Stratum share target / difficulty math.
- fixed-width little-endian extranonce2 rolling.
- CPU worker dispatcher for educational share scanning.
- reconnect policy/runtime state for an async Stratum client loop.
- mock public-pool Stratum loop that subscribes, authorizes, consumes notify, and submits shares.
- ntime rolling and BIP320-style version rolling mask helper.
- dry-run cgminer / Bitaxe / Avalon adapter interfaces.
- live-write command builders for cgminer / Bitaxe / Avalon APIs gated by explicit opt-in.
- local-first hardware/dashboard abstraction inspired by MinerWatch / Avalon Q Controller.
- dashboard bearer-token auth primitive and JSON metrics persistence.
- Docker Compose dashboard deployment with health endpoint.
- unit and integration tests for critical serialization / hashing / protocol boundaries.

Not implemented yet:

- production Stratum share submission loop against public pools.
- direct live HTTP/TCP writes to physical ASICs from the default dashboard; current live API builders require explicit opt-in and should be wired only after confirming the target device.
- full transaction policy engine; this trusts Bitcoin Core's template.
- fully autonomous profitable mainnet mining; this remains a research/scaffold project unless connected to real ASIC capacity and a real pool.
- authenticated dashboard write actions; status/auth/metrics primitives are present, but dangerous device writes stay gated.

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
  stratum.rs    Stratum v1 parser, difficulty target, extranonce, reconnect state
  worker.rs     CPU worker dispatcher/share scanner
  hardware.rs   local-first miner-device/dashboard abstraction
  dashboard.rs  small local HTTP dashboard and health endpoint
scripts/
  regtest_acceptance.sh      coinbase-only submitblock acceptance test
  regtest_nonempty_block.sh  mempool transaction inclusion acceptance test
deploy/
  umbrel/umbrel-app.yml      Umbrel-style app metadata
docs/
  bitcoinminer-topic-integration.md
  docker-deployment.md
Dockerfile
docker-compose.yml
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

## Docker dashboard deployment

```bash
docker compose up --build -d
python3 - <<'PY'
import socket
s=socket.create_connection(('127.0.0.1',8080),3)
s.sendall(b'GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n')
print(s.recv(4096).decode())
PY
```

Open <http://127.0.0.1:8080/> for the local dashboard. The compose file binds only to localhost by default.

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
