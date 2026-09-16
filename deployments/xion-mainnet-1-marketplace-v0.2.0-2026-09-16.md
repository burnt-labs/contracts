# XION Mainnet Marketplace v0.2.0 Deployment

## Deployment

- Network: `xion-mainnet-1`
- RPC: `https://rpc.xion-mainnet-1.burnt.com:443`
- Contract: `xion-nft-marketplace` (crate version `0.2.0`)
- Code ID: `69`
- Governance proposal: `62` (expedited) — https://www.mintscan.io/verona/proposals/62
- Proposal submit transaction: `B936D113E09297B4D711AA0B3010C9EF8ACB3DED9812405EB7E43D96091C1DAE`
- Proposal submitted: `2026-09-15T18:21:37Z` by `xion1haem6exxh8yahudm33zcy4r778hjthe0sfwlc9`
- Voting ended / code stored: `2026-09-16T18:21:37Z` (executed at block `23844926`)
- Final tally: yes `29477301.19 XION`, abstain `6557547.44 XION`, no `0`, veto `0`
- Code creator: `xion10d07y265gmmuvt4z0w9aw880jnsr700jctf8qc` (gov module)
- Instantiate permission: `Everybody`
- Deposit: `5000000000uxion` (expedited minimum)
- Submit gas wanted / used: `40000000` / `34309763`, fee `40000uxion`

## Artifact

- Source commit: `e82deac8981acb18ea17fbab23f76a8c7f9d9795`
- Artifact: `artifacts/xion_nft_marketplace.wasm`
- Artifact size: `551956` bytes
- SHA-256: `65b2da0691d332a24d81ff499a448a2d6542459142ea4033e8b937e5b52bba76`
- Optimizer: `cosmwasm/optimizer:0.17.0` (`linux/amd64`)

The checksum reported by `xiond query wasm code-info 69` matches the local
artifact and the testnet deployment (`xion-testnet-2` code ID `2315`, see
`xion-testnet-2-marketplace-v0.2.0-2026-09-15.md`).

## Changes Since Code ID 65 (proposal 53, commit `e3dadc6`)

- PR #136 (`97efd23`): paginated `listings` and `listings_by_seller` queries.
- PR #138 (`e82deac`): listings-by-collection index (`lc`) and
  `listings_by_collection` query; crate version `0.1.0` -> `0.2.0`; `migrate`
  now records the cw2 version. The new index is not backfilled on migration;
  fresh instantiation is recommended.
- PR #118 (`d44067a`): sellers can reclaim expired pending sales; the manager
  can no longer approve an expired pending sale.
- `df2ed3c`: zero-price listings are rejected and zero-amount bank sends are
  no longer emitted.
- `c108728`: listing-status check for a direct-buy edge case.
- PR #132 (`e4663da`): dependency updates.

## Governance Process

- Signaling thread (original): https://discourse.xion.burnt.com/t/deploy-xion-asset-marketplace-contracts-on-xion/109
  (update reply posted on submission).
- Submitted as an expedited proposal (24h voting, 66.7% threshold) following
  the precedent of proposals 59 and 61.

## Asset Contract

No new Asset contract code was uploaded. Mainnet Asset code ID `64` is
unchanged.
