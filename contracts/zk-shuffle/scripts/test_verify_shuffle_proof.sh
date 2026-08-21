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

# Execute the transaction
echo "Executing VerifyShuffleProof transaction..."
echo "---"

xiond tx wasm execute "$CONTRACT_ADDRESS" "$EXECUTE_MSG" \
    --from "$FROM_ACCOUNT" \
    --gas-prices 0.025uxion \
    --gas auto \
    --gas-adjustment 1.3 \
    -y \
    --node "$RPC_URL" \
    --chain-id "$CHAIN_ID"

TX_RESULT=$?

echo ""
echo "---"

if [ $TX_RESULT -eq 0 ]; then
    echo "✓ VerifyShuffleProof transaction executed successfully!"
    echo "Sleeping for 10 seconds before querying"
    sleep 10
    # Query verification count
    echo ""
    echo "Querying verification count..."
    QUERY_MSG='{"verification_count": {}}'


    xiond query wasm contract-state smart "$CONTRACT_ADDRESS" "$QUERY_MSG" \
        --node "$RPC_URL" \
        --output json | jq '.'
else
    echo "✗ VerifyShuffleProof transaction failed"
    echo "Please check the error messages above"
    exit 1
fi
