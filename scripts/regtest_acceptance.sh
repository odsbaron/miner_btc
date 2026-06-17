#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATADIR="${BITCOIN_REGTEST_DATADIR:-$HOME/btc-regtest-minerbtc-acceptance}"
RPC_PORT="${BITCOIN_RPC_PORT:-18443}"
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
ADDR="$(wallet_cli getnewaddress "miner-btc-acceptance" bech32)"
SCRIPT_HEX="$(wallet_cli getaddressinfo "$ADDR" | json_get scriptPubKey)"
BEFORE="$(cli getblockcount)"

OUTPUT="$(cd "$ROOT_DIR" && \
  BITCOIN_RPC_URL="$RPC_URL" BITCOIN_RPC_USER="$RPC_USER" BITCOIN_RPC_PASS="$RPC_PASS" \
  cargo run --quiet -- regtest --payout-script-hex "$SCRIPT_HEX" --max-nonce 5000000 --submit)"
printf '%s\n' "$OUTPUT"

AFTER="$(cli getblockcount)"
BEST="$(cli getbestblockhash)"
BLOCK_JSON="$(cli getblock "$BEST")"
HEIGHT="$(printf '%s' "$BLOCK_JSON" | json_get height)"
TX_COUNT="$(printf '%s' "$BLOCK_JSON" | json_get nTx)"

if ! grep -q 'submitblock result: None' <<<"$OUTPUT"; then
  echo "ERROR: submitblock was not accepted" >&2
  exit 1
fi
if [ "$AFTER" -ne $((BEFORE + 1)) ]; then
  echo "ERROR: expected height $((BEFORE + 1)), got $AFTER" >&2
  exit 1
fi
if [ "$TX_COUNT" -ne 1 ]; then
  echo "ERROR: expected coinbase-only acceptance block, got nTx=$TX_COUNT" >&2
  exit 1
fi

echo "REGTEST_ACCEPTANCE_OK before=$BEFORE after=$AFTER height=$HEIGHT nTx=$TX_COUNT best=$BEST"
