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
