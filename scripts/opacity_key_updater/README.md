Opacity Key Updater (off-chain)

This is a small Rust CLI that periodically polls Opacity’s public signing key endpoint and, when a change is detected, submits an admin execute to the on-chain `opacity_verifier` contract to update its allowlist.

Features
- Polls an HTTP endpoint for the list of Ethereum addresses used by Opacity.
- Normalizes addresses to lowercase, without `0x`, and validates hex length (20 bytes).
- Detects changes vs a locally persisted snapshot and only submits when needed.
- Dry-run mode by default (prints the execute message JSON and stores the new snapshot).
- On-chain submission is enabled at runtime by setting DRY_RUN=false (requires chain config env vars).
- Endpoint shape: accepts either a top-level JSON array of strings, or a JSON object with the array located at the configured field name (set via KEYS_FIELD).

Build
- Default (no on-chain submission code):
  cargo build -p opacity_key_updater


Run (dry-run default)
By default, the tool runs in dry-run mode and will not broadcast a transaction.

Environment variables:
- KEYS_URL: The public keys endpoint. Default: https://verifier.opacity.network/api/public-keys
- KEYS_FIELD: Field name within a JSON object that contains the array of keys (e.g., `allowlistedKeys`). If the endpoint returns an object and KEYS_FIELD is not set, the tool will error. Not used for top-level arrays. Default: unset.
- POLL_INTERVAL_SECS: Poll interval seconds. Default: 5
- STATE_PATH: Path to a local state file. Default: .opacity_keys_state.json
- DRY_RUN: Set to `false` or `0` to enable submissions (requires chain config below).

Chain submission (requires DRY_RUN=false):
- CONTRACT_ADDRESS: Address of the deployed opacity_verifier contract (bech32).
- RPC_ENDPOINT: Tendermint RPC endpoint (e.g., https://rpc.xion-testnet-1.burnt.com).
- GRPC_ENDPOINT: gRPC endpoint (optional; defaults to derived from RPC).
- CHAIN_ID: Chain id (e.g., xion-testnet-1).
- GAS_DENOM: Gas denom (e.g., uxion).
- GAS_PRICE: Gas price (e.g., 0.025uxion). Default: 0.025{GAS_DENOM}.
- BECH32_PREFIX: Bech32 prefix for addresses (e.g., xion).
- ADMIN_MNEMONIC: Mnemonic of the admin address (must match the contract admin on-chain).

Examples
1) Run as a dry-run one-shot (Ctrl+C to stop):
   DRY_RUN=true cargo run -p opacity_key_updater

2) Run with real submissions:
   DRY_RUN=false \
   CONTRACT_ADDRESS="xion1..." \
   RPC_ENDPOINT="https://rpc.xion-testnet-1.burnt.com" \
   CHAIN_ID="xion-testnet-1" \
   GAS_DENOM="uxion" \
   BECH32_PREFIX="xion" \
   ADMIN_MNEMONIC="<your mnemonic phrase here>" \
   cargo run -p opacity_key_updater

Note: The tool persists a snapshot of the keys in `STATE_PATH` and only submits when changes are detected. It also normalizes uploaded keys to exactly match the contract’s normalization logic.
