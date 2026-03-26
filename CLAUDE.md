# contracts — CLAUDE.md

CosmWasm smart contracts for the Xion ecosystem.

## GitHub Workflows

### `Basic.yml`

**Triggered by:** Push to `main`, PRs, tag push `v*.*.*`

Runs build and tests for all contracts.

### `Release.yml`

**Triggered by:** GitHub release created

Compiles contracts to optimized WASM and uploads artifacts to the release.

## Upstream Triggers

None — this repo is not triggered by other repos.

## Downstream Triggers

None.

## Development

```bash
# Build
cargo build

# Test
cargo test

# Optimize (produces production WASM)
docker run --rm -v "$(pwd)":/code cosmwasm/optimizer:latest
```
