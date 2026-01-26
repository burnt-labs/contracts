#!/bin/bash

# zkShuffle Contract - Test All Execute Methods
# This script runs all proof verification tests in sequence

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "========================================"
echo " zkShuffle Contract - Execute Methods Test Suite"
echo "========================================"
echo ""

# Test 1: VerifyShuffleProof
echo ">>> Test 1: VerifyShuffleProof"
echo "========================================"
if bash "$SCRIPT_DIR/test_verify_shuffle_proof.sh"; then
    echo ""
    echo "✓ VerifyShuffleProof test PASSED"
    echo ""
else
    echo ""
    echo "✗ VerifyShuffleProof test FAILED"
    echo ""
    exit 1
fi

# Wait between tests
sleep 3

# Test 2: VerifyDecryptProof
echo ">>> Test 2: VerifyDecryptProof"
echo "========================================"
if bash "$SCRIPT_DIR/test_verify_decrypt_proof.sh"; then
    echo ""
    echo "✓ VerifyDecryptProof test PASSED"
    echo ""
else
    echo ""
    echo "✗ VerifyDecryptProof test FAILED"
    echo ""
    exit 1
fi

# Summary
echo "========================================"
echo " All Tests PASSED!"
echo "========================================"
echo ""
echo "Summary:"
echo "  ✓ VerifyShuffleProof - Shuffle encryption proof verification"
echo "  ✓ VerifyDecryptProof - Card decryption proof verification"
echo ""
