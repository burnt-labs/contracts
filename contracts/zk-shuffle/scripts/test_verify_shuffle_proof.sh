#!/bin/bash

# zkShuffle Contract - Test VerifyShuffleProof Execute Method
# This script tests the VerifyShuffleProof execute method with proof data from data/shuffle_encrypt.json

set -e

# Source environment variables
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

if [ -f "$PROJECT_ROOT/.env.local" ]; then
    source "$PROJECT_ROOT/.env.local"
else
    echo "Error: .env.local file not found at $PROJECT_ROOT/.env.local"
    exit 1
fi

# Configuration
CONTRACT_ADDRESS="${CONTRACT_ADDRESS:-$NEXT_PUBLIC_CONTRACT_ADDRESS}"
RPC_URL="${RPC_URL:-$NEXT_PUBLIC_RPC_URL}"
CHAIN_ID="${CHAIN_ID:-xion-testnet-2}"
FROM_ACCOUNT="${FROM_ACCOUNT:-$WALLET1}"

# Validate required environment variables
required_vars=("CONTRACT_ADDRESS" "RPC_URL" "CHAIN_ID" "FROM_ACCOUNT")
for var in "${required_vars[@]}"; do
    if [ -z "${!var}" ]; then
        echo "Error: Required environment variable $var is not set"
        exit 1
    fi
done

echo "=== zkShuffle VerifyShuffleProof Test ==="
echo "Contract: $CONTRACT_ADDRESS"
echo "RPC: $RPC_URL"
echo "Chain ID: $CHAIN_ID"
echo "From: $FROM_ACCOUNT"
echo ""

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required but not installed. Please install jq."
    exit 1
fi

# Check if proof data file exists
PROOF_FILE="$SCRIPT_DIR/data/shuffle_encrypt.json"
if [ ! -f "$PROOF_FILE" ]; then
    echo "Error: Proof data file not found at $PROOF_FILE"
    exit 1
fi

# Query the current shuffle verification count from the contract.
query_shuffle_verifications() {
    xiond query wasm contract-state smart "$CONTRACT_ADDRESS" '{"verification_count": {}}' \
        --node "$RPC_URL" \
        --output json | jq -r '.data.shuffle_verifications'
}

echo "Loading proof data from $PROOF_FILE..."

# Parse proof data from shuffle_encrypt.json
# The proof structure has:
# - pi_a: [x, y, "1"]
# - pi_b: [[x0, y0], [x1, y1], ["1", "0"]]
# - pi_c: [x, y, "1"]

# Read the file and extract values
PROOF_DATA=$(cat "$PROOF_FILE")

# Extract pi_a (first 2 elements, skip the "1")
PI_A_0=$(echo "$PROOF_DATA" | jq -r '.proof.pi_a[0]')
PI_A_1=$(echo "$PROOF_DATA" | jq -r '.proof.pi_a[1]')

# Extract pi_b (2x2 array, skip the last ["1", "0"])
PI_B_0_0=$(echo "$PROOF_DATA" | jq -r '.proof.pi_b[0][0]')
PI_B_0_1=$(echo "$PROOF_DATA" | jq -r '.proof.pi_b[0][1]')
PI_B_1_0=$(echo "$PROOF_DATA" | jq -r '.proof.pi_b[1][0]')
PI_B_1_1=$(echo "$PROOF_DATA" | jq -r '.proof.pi_b[1][1]')

# Extract pi_c (first 2 elements, skip the "1")
PI_C_0=$(echo "$PROOF_DATA" | jq -r '.proof.pi_c[0]')
PI_C_1=$(echo "$PROOF_DATA" | jq -r '.proof.pi_c[1]')

# Extract public signals
PUBLIC_INPUTS_COUNT=$(echo "$PROOF_DATA" | jq -r '.publicSignals | length')
echo "Found $PUBLIC_INPUTS_COUNT public inputs"

# Build public inputs array as JSON string
PUBLIC_INPUTS=$(echo "$PROOF_DATA" | jq -c '.publicSignals | map(tostring)')

echo "Proof data loaded successfully"
echo "Building execute message..."
echo ""

# Build the verify_shuffle_proof message
# Format: {"verify_shuffle_proof": {"proof": {...}, "public_inputs": [...]}}
EXECUTE_MSG=$(jq -n \
    --argjson pi_a "[\"$PI_A_0\", \"$PI_A_1\"]" \
    --argjson pi_b "[[\"$PI_B_0_0\", \"$PI_B_0_1\"], [\"$PI_B_1_0\", \"$PI_B_1_1\"]]" \
    --argjson pi_c "[\"$PI_C_0\", \"$PI_C_1\"]" \
    --argjson public_inputs "$PUBLIC_INPUTS" \
    '{
        verify_shuffle_proof: {
            proof: {
                a: $pi_a,
                b: $pi_b,
                c: $pi_c
            },
            public_inputs: $public_inputs
        }
    }')

echo "Execute Message:"
echo "$EXECUTE_MSG" | jq '.'
echo ""

# Capture the verification count before submitting so we can confirm the
# on-chain execute actually recorded a verification.
echo "Querying shuffle verification count before submitting..."
COUNT_BEFORE=$(query_shuffle_verifications)
if ! [[ "$COUNT_BEFORE" =~ ^[0-9]+$ ]]; then
    echo "✗ Failed to read shuffle_verifications before submitting (got: '$COUNT_BEFORE')"
    exit 1
fi
echo "shuffle_verifications before: $COUNT_BEFORE"
echo ""

# Execute the transaction
echo "Executing VerifyShuffleProof transaction..."
echo "---"

# Broadcast in sync mode and capture the txhash from the JSON output. Sync mode
# only guarantees mempool acceptance, so we must poll for block inclusion below.
TX_OUTPUT=$(xiond tx wasm execute "$CONTRACT_ADDRESS" "$EXECUTE_MSG" \
    --from "$FROM_ACCOUNT" \
    --gas-prices 0.025uxion \
    --gas auto \
    --gas-adjustment 1.3 \
    -y \
    --broadcast-mode sync \
    --output json \
    --node "$RPC_URL" \
    --chain-id "$CHAIN_ID")

echo "$TX_OUTPUT" | jq '.'
echo ""
echo "---"

# The sync broadcast response carries its own code; a non-zero code means the
# transaction was rejected by CheckTx and never entered the mempool.
BROADCAST_CODE=$(echo "$TX_OUTPUT" | jq -r '.code')
if [ "$BROADCAST_CODE" != "0" ]; then
    echo "✗ VerifyShuffleProof transaction was rejected during broadcast (code: $BROADCAST_CODE)"
    echo "$TX_OUTPUT" | jq -r '.raw_log'
    exit 1
fi

TX_HASH=$(echo "$TX_OUTPUT" | jq -r '.txhash')
if [ -z "$TX_HASH" ] || [ "$TX_HASH" == "null" ]; then
    echo "✗ VerifyShuffleProof transaction did not return a txhash"
    exit 1
fi

echo "Transaction broadcast, txhash: $TX_HASH"
echo "Polling for block inclusion..."

# Poll for the transaction to be committed in a block, then assert its
# DeliverTx result code is 0 (success).
TX_QUERY=""
for _ in $(seq 1 30); do
    if TX_QUERY=$(xiond query tx "$TX_HASH" --node "$RPC_URL" --output json 2>/dev/null); then
        break
    fi
    TX_QUERY=""
    sleep 2
done

if [ -z "$TX_QUERY" ]; then
    echo "✗ Transaction $TX_HASH was not committed within the timeout"
    exit 1
fi

TX_CODE=$(echo "$TX_QUERY" | jq -r '.code')
if [ "$TX_CODE" != "0" ]; then
    echo "✗ VerifyShuffleProof transaction failed on-chain (code: $TX_CODE)"
    echo "$TX_QUERY" | jq -r '.raw_log'
    exit 1
fi

echo "✓ VerifyShuffleProof transaction committed successfully!"
echo ""

# Confirm the verification was actually recorded by comparing the counter.
echo "Querying shuffle verification count after commit..."
COUNT_AFTER=$(query_shuffle_verifications)
if ! [[ "$COUNT_AFTER" =~ ^[0-9]+$ ]]; then
    echo "✗ Failed to read shuffle_verifications after commit (got: '$COUNT_AFTER')"
    exit 1
fi
echo "shuffle_verifications after: $COUNT_AFTER"

# Greater-than rather than exactly one more: this runs against a shared
# testnet contract, so someone else's verification landing between the two
# queries legitimately moves the counter further. What this is checking is
# that our own tx was recorded, and any advance proves that.
if [ "$COUNT_AFTER" -le "$COUNT_BEFORE" ]; then
    echo "✗ shuffle_verifications did not advance (before: $COUNT_BEFORE, after: $COUNT_AFTER)"
    exit 1
fi

echo "✓ shuffle_verifications advanced from $COUNT_BEFORE to $COUNT_AFTER"
