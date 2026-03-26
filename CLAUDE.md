# contracts — CLAUDE.md

CosmWasm smart contracts for the Xion ecosystem.

## GitHub Workflows

### `Basic.yml`

**Triggered by:** Push to `main`, tag push `v*.*.*`, PRs

Installs the pinned Rust toolchain with `wasm32-unknown-unknown` target and runs unit tests (`cargo test --locked`).

### `Release.yml`

**Triggered by:** GitHub release created

Compiles contracts to WASM using `cargo wasm --locked` (with `RUSTFLAGS="-C link-arg=-s"` for size optimization) and uploads the resulting `.wasm` artifacts to the release.

## Upstream Triggers

None — this repo is not triggered by other repos.

## Downstream Triggers

None.

## Development

```bash
# Run unit tests
cargo test --locked

# Compile WASM (matches CI)
RUSTFLAGS="-C link-arg=-s" cargo wasm --locked

# Optimize with cosmwasm-optimizer (if you have just installed)
just optimize
```
