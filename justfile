toml-format:
    @echo "Formatting TOML files..."
    taplo format .


fmt:
    @echo "Formatting Rust code..."
    cargo fmt --all

# Lint with clippy
lint:
    @echo "Linting with clippy..."
    cargo clippy --all-targets --all-features -- -D warnings

lint-asset:
    @echo "Linting asset with clippy..."
    cargo clippy -p asset --all-targets --all-features --fix --no-deps -- -D warnings

# Lint marketplace package only
lint-marketplace:
    @echo "Linting marketplace with clippy..."
    cargo clippy -p xion-nft-marketplace --all-targets --all-features --no-deps -- -D warnings

# Run tests
test:
    @echo "Running tests..."
    cargo test --all

# The optimizer image fixes the Rust version the contracts must compile under
# (1.86.0 for 0.17.0), which is what .github/workflows/Basic.yml pins its toolchain
# to. Bump this, Basic.yml's RUST_VERSION and Release.yml's OPTIMIZER_VERSION
# together.

# Build the release artifacts into artifacts/
optimize:
    docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    cosmwasm/optimizer:0.17.0
