# XION Testnet Marketplace Zero-Fee Deployment

## Deployment

- Network: `xion-testnet-2`
- RPC: `https://rpc.xion-testnet-2.burnt.com:443`
- Contract: `xion-nft-marketplace`
- Code ID: `2312`
- Transaction: `93E4A3EBA4F02544DF412878D5699AA90254DE9D560A0E957E94ACDD4376F7BC`
- Block height: `18123293`
- Timestamp: `2026-09-10T14:35:56Z`
- Deployer: `xion1haem6exxh8yahudm33zcy4r778hjthe0sfwlc9`
- Instantiate permission: `Everybody`
- Transaction fee: `4352uxion`
- Gas wanted: `4351185`
- Gas used: `3371923`

## Artifact

- Source commit: `f72b524f7946210cd6cb763b050c9b4dd2bf833d`
- Zero-fee fix commit: `df2ed3c15fa91bcef869bd8b8e05c5eab3775262`
- Artifact: `artifacts/xion_nft_marketplace.wasm`
- Artifact size: `527524` bytes
- SHA-256: `48357b6ac9512c1424a8572e5854bf30bfc9af032d8ba6935ce0328cf23e8fb4`
- Optimizer: `cosmwasm/optimizer:0.17.0` (`linux/amd64`)

The local artifact checksum was verified against the checksum reported by
`xiond query wasm code-info 2312` after the transaction was committed.

## Verification

The Marketplace test suite was run from the source commit above using Rust
`1.88.0`:

```text
67 passed; 0 failed
```

Coverage includes zero-fee behavior for direct purchases, approval-queue
purchases, token offers, and collection offers.

## Asset Contract

No new Asset contract code was uploaded for this deployment. The invalid-coins
failure was caused by the Marketplace attempting to emit a zero-amount bank
send, and the Marketplace fix is sufficient for existing compatible Asset
contracts.
