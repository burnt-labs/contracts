# XION Testnet Marketplace v0.2.0 Deployment

## Deployment

- Network: `xion-testnet-2`
- RPC: `https://rpc.xion-testnet-2.burnt.com:443`
- Contract: `xion-nft-marketplace` (crate version `0.2.0`)
- Code ID: `2315`
- Transaction: `F8B4ED065714A0859EA996385D2D1E4A8A8F4B93880F3C05C640FEAEC5CC6F46`
- Block height: `18235820`
- Timestamp: `2026-09-15T17:42:10Z`
- Deployer: `xion1haem6exxh8yahudm33zcy4r778hjthe0sfwlc9`
- Instantiate permission: `Everybody`
- Transaction fee: `112731uxion`
- Gas wanted: `4509238`
- Gas used: `3493582`

## Artifact

- Source commit: `e82deac8981acb18ea17fbab23f76a8c7f9d9795`
- Artifact: `artifacts/xion_nft_marketplace.wasm`
- Artifact size: `551956` bytes
- SHA-256: `65b2da0691d332a24d81ff499a448a2d6542459142ea4033e8b937e5b52bba76`
- Optimizer: `cosmwasm/optimizer:0.17.0` (`linux/amd64`)

The local artifact checksum was verified against the checksum reported by
`xiond query wasm code-info 2315` after the transaction was committed.

## Changes Since Code ID 2312

- PR #136 (`97efd23`): paginated `listings` and `listings_by_seller` queries.
- PR #138 (`e82deac`): listings-by-collection index (`lc`) and `listings_by_collection`
  query; crate version `0.1.0` -> `0.2.0`; `migrate` now records the cw2 version.
  The new index is not backfilled on migration; fresh instantiation is recommended.

## Verification

CI (`Basic` workflow, Rust `1.88.0`: `cargo test`, `cargo fmt --check`,
`cargo clippy -D warnings`, wasm build) passed on the source commit:
https://github.com/burnt-labs/contracts/actions/runs/34993128604

Smoke test instance:

- Contract: `xion1mxugvexrscyu367f65vwgaecwm3pf3n68e60swf974sgt7x3mc7s3fz4tk`
- Label: `marketplace-v0.2.0-smoke-2026-09-15`
- cw2 `contract_info`: `{"contract":"xion-nft-marketplace","version":"0.2.0"}`
- `listings`, `listings_by_seller`, `listings_by_collection` queries returned
  empty pages without error on the fresh instance.

## Asset Contract

No new Asset contract code was uploaded. The marketplace changes are additive
queries and input guards and remain wire-compatible with existing Asset
collections.
