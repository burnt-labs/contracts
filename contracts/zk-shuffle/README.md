# ZK Shuffle Contract

CosmWasm contract that verifies shuffle-encrypt and decrypt zero-knowledge proofs for card shuffling. Use the provided testnet artifacts to try it quickly, or build and deploy your own optimized binary.

## Prerequisites

- XION daemon (`xiond`). Install via the official guide: [Setup XION Daemon](https://docs.burnt.com/xion/developers/featured-guides/setup-local-environment/interact-with-xion-chain-setup-xion-daemon)
- Docker (required to build the custom optimizer image)
- `jq` (used by the helper scripts)

## Environment Setup

Create a local env file and set your wallet address:

```bash
cp .env.example .env.local
$EDITOR .env.local   # set WALLET1 to your wallet
source .env.local
```

- `testnet.json` contains a live CODE_ID and CONTRACT_ADDRESS you can use immediately on XION testnet if you just want to interact without deploying.
- `MSG` defaults to `{}`; adjust if you need a custom instantiate message.

## Build (custom optimizer)

The standard CosmWasm optimizer image does not work for this contract, so use the local Dockerfile:

1) Build the optimizer image

```bash
docker build -t wasm_optimizer .
```

2) Produce the optimized wasm artifact

```bash
docker run --rm -v "$(pwd)":/code -v ~/.ssh:/root/.ssh:ro -e SSH_AUTH_SOCK=/ssh-agent \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  wasm_optimizer ./
```

The optimized binary is written to `artifacts/zkshuffle_cw.wasm`.

## Deploy to XION

All commands assume the variables from `.env.local` have been loaded.

1) **Store the contract bytecode**

```bash
RES=$(xiond tx wasm store ./artifacts/zkshuffle_cw.wasm \
  --chain-id "$CHAIN_ID" \
  --gas-adjustment 1.3 \
  --gas-prices 0.1uxion \
  --gas auto \
  -y --output json \
  --node "$RPC_URL" \
  --from "$WALLET1")

echo "$RES"
```

Copy the `txhash` from the output into `TX_HASH` in `.env.local`, then reload it.

2) **Retrieve the Code ID**

```bash
CODE_ID=$(xiond query tx "$TX_HASH" --node "$RPC_URL" --output json | jq -r '.events[-1].attributes[1].value')
echo "$CODE_ID"
```

Save the value to `CODE_ID` in `.env.local` and reload it.

3) **Instantiate the contract**

```bash
xiond tx wasm instantiate "$CODE_ID" "$MSG" \
  --from "$WALLET1" \
  --label "zkshuffle" \
  --gas-prices 0.025uxion \
  --gas auto \
  --gas-adjustment 1.3 \
  -y --no-admin \
  --chain-id "$CHAIN_ID" \
  --node "$RPC_URL"
```

Copy the response `txhash` into `DEPLOY_TXHASH`, reload, and fetch the contract address:

```bash
CONTRACT_ADDRESS=$(xiond query tx "$DEPLOY_TXHASH" \
  --node "$RPC_URL" \
  --output json | jq -r '.events[] | select(.type == "instantiate") | .attributes[] | select(.key == "_contract_address") | .value')

echo "$CONTRACT_ADDRESS"
```

Save `CONTRACT_ADDRESS` in `.env.local` for later use.

## Test proof verification

The scripts read configuration from `.env.local` and require `jq`.

```bash
cd scripts
./test_all_proofs.sh            # runs both execute methods
# or individually:
./test_verify_shuffle_proof.sh
./test_verify_decrypt_proof.sh
```

## Circuit and verification keys

- Shuffle encrypt circuit: `vkey name: shuffle_encrypt`, `vkey id: 3`
- Decrypt circuit: `vkey name: decrypt`, `vkey id: 2`

Verification keys and artifacts are stored under `vkeys/` and `artifacts/` respectively.
