#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATADIR="${BITCOIN_REGTEST_DATADIR:-$HOME/btc-regtest-minerbtc-nonempty}"
RPC_PORT="${BITCOIN_RPC_PORT:-18444}"
RPC_USER="${BITCOIN_RPC_USER:-user}"
RPC_PASS="${BITCOIN_RPC_PASS:-pass}"
RPC_URL="http://127.0.0.1:${RPC_PORT}"
WALLET="miner"

cli() {
  bitcoin-cli -regtest -datadir="$DATADIR" -rpcport="$RPC_PORT" -rpcuser="$RPC_USER" -rpcpassword="$RPC_PASS" "$@"
}

wallet_cli() {
  bitcoin-cli -regtest -datadir="$DATADIR" -rpcport="$RPC_PORT" -rpcuser="$RPC_USER" -rpcpassword="$RPC_PASS" -rpcwallet="$WALLET" "$@"
}

json_get() {
  python3 -c 'import json,sys; print(json.load(sys.stdin)[sys.argv[1]])' "$1"
}

cleanup() {
  cli stop >/dev/null 2>&1 || true
}
trap cleanup EXIT

command -v bitcoind >/dev/null
command -v bitcoin-cli >/dev/null

rm -rf "$DATADIR"
mkdir -p "$DATADIR"
bitcoind -regtest -datadir="$DATADIR" -server=1 -rpcport="$RPC_PORT" \
  -rpcuser="$RPC_USER" -rpcpassword="$RPC_PASS" -fallbackfee=0.0001 \
  -daemon >/dev/null

for _ in $(seq 1 120); do
  if cli getblockchaininfo >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
cli getblockchaininfo >/dev/null

cli createwallet "$WALLET" >/dev/null
MINER_ADDR="$(wallet_cli getnewaddress "miner-btc-nonempty-miner" bech32)"
MINER_SCRIPT_HEX="$(wallet_cli getaddressinfo "$MINER_ADDR" | json_get scriptPubKey)"

# Mature coinbase funds. Regtest coinbase maturity is 100 blocks.
cli generatetoaddress 101 "$MINER_ADDR" >/dev/null
RECIPIENT_ADDR="$(wallet_cli getnewaddress "miner-btc-nonempty-recipient" bech32)"
TXID="$(wallet_cli sendtoaddress "$RECIPIENT_ADDR" 1.0)"
MEMPOOL_COUNT="$(cli getrawmempool | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')"
if [ "$MEMPOOL_COUNT" -lt 1 ]; then
  echo "ERROR: expected at least one transaction in mempool after sendtoaddress txid=$TXID" >&2
  exit 1
fi

TEMPLATE_SUMMARY="$(cd "$ROOT_DIR" && \
  BITCOIN_RPC_URL="$RPC_URL" BITCOIN_RPC_USER="$RPC_USER" BITCOIN_RPC_PASS="$RPC_PASS" \
  cargo run --quiet -- template)"
printf '%s\n' "$TEMPLATE_SUMMARY"
if ! grep -Eq 'transactions: [1-9]' <<<"$TEMPLATE_SUMMARY"; then
  echo "ERROR: getblocktemplate did not expose non-empty transaction set" >&2
  exit 1
fi

BEFORE="$(cli getblockcount)"
OUTPUT="$(cd "$ROOT_DIR" && \
  BITCOIN_RPC_URL="$RPC_URL" BITCOIN_RPC_USER="$RPC_USER" BITCOIN_RPC_PASS="$RPC_PASS" \
  cargo run --quiet -- regtest --payout-script-hex "$MINER_SCRIPT_HEX" --max-nonce 5000000 --submit)"
printf '%s\n' "$OUTPUT"

AFTER="$(cli getblockcount)"
BEST="$(cli getbestblockhash)"
BLOCK_JSON="$(cli getblock "$BEST")"
HEIGHT="$(printf '%s' "$BLOCK_JSON" | json_get height)"
TX_COUNT="$(printf '%s' "$BLOCK_JSON" | json_get nTx)"

if ! grep -q 'submitblock result: None' <<<"$OUTPUT"; then
  echo "ERROR: submitblock was not accepted for non-empty block" >&2
  exit 1
fi
if [ "$AFTER" -ne $((BEFORE + 1)) ]; then
  echo "ERROR: expected height $((BEFORE + 1)), got $AFTER" >&2
  exit 1
fi
if [ "$TX_COUNT" -lt 2 ]; then
  echo "ERROR: expected non-empty mined block with nTx>=2, got nTx=$TX_COUNT" >&2
  exit 1
fi
POST_MEMPOOL_COUNT="$(cli getrawmempool | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')"
if [ "$POST_MEMPOOL_COUNT" -ne 0 ]; then
  echo "ERROR: mempool not empty after mining transaction into block: $POST_MEMPOOL_COUNT" >&2
  exit 1
fi

echo "REGTEST_NONEMPTY_OK txid=$TXID before=$BEFORE after=$AFTER height=$HEIGHT nTx=$TX_COUNT best=$BEST"
