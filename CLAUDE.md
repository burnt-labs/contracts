# contracts — CLAUDE.md

CosmWasm smart contracts for the Xion ecosystem.

## GitHub Workflows

### `Basic.yml`

**Triggered by:** Push to `main`, tag push `v*.*.*`, PRs

Three jobs on the pinned toolchain:

- **Test Suite** — `cargo test --locked`.
- **Lints** — `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings`.
- **Contract artifacts** — builds through the same pinned `cosmwasm/optimizer` image the release uses, validates every `.wasm` with `cosmwasm-check`, and uploads them for inspection.

### `Release.yml`

**Triggered by:** GitHub release created

Builds through the pinned `cosmwasm/optimizer` image, validates each artifact with `cosmwasm-check`, then uploads the `.wasm` files and `checksums.txt` to the release.

### Why the workflows look like this

Three constraints are easy to break by "simplifying" them, so change them deliberately:

- **Never build contracts with a workspace-wide `cargo build`/`cargo wasm`.** Cargo unifies features across workspace members, so the `library` feature that `asset-proxy` and the marketplace request on their dependencies also gets enabled on `asset` and `asset-proxyable`, compiling their `entry_point`s out. The result is a ~20 KB wasm that cannot be instantiated. The optimizer builds each package separately and is immune to this.
- **Raw `cargo` wasm output cannot be validated at all.** rustc 1.82+ emits wasm using the reference-types proposal, which the CosmWasm VM rejects (`cosmwasm-check`: "reference-types not enabled: zero byte expected"). `-C target-cpu=mvp` does *not* fix it, because the precompiled std already carries it. Only the `wasm-opt` pass inside the optimizer lowers it. This is why `cosmwasm-check` runs against optimizer output, never against `cargo` output.
- **The Rust version is pinned to the optimizer's, currently 1.86.0.** The optimizer image ships its own Rust and is the real lower bound for the repo: code that only compiles on something newer cannot be released, however green CI looks. Both workflows carry that Rust version alongside the optimizer version and image digest, and the justfile carries the optimizer version; bump them in one change, reading the new Rust version with `docker run --rm cosmwasm/optimizer:<version> cargo --version`. Actions are pinned to commit SHAs and the optimizer image to a digest.

The full reasoning, including how each constraint was discovered, lives in the comments at the top of `.github/workflows/Basic.yml`.

## Upstream Triggers

None — this repo is not triggered by other repos.

## Downstream Triggers

None.

## Development

```bash
# Run unit tests
cargo test --locked

# Lints, as CI runs them
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings

# Build deployable artifacts the way CI and releases do, into artifacts/
just optimize
```

`just optimize` is the only supported way to produce a deployable `.wasm` locally: see the
workflow notes above for why a plain `cargo wasm` build is neither deployable nor checkable.
It writes root-owned files from inside the container, so clean `artifacts/` with the
container rather than a bare `rm -rf`.
