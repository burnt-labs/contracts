toml-format:
    @echo "Formatting TOML files..."
    taplo format .


fmt:
    @echo "Formatting Rust code..."
    cargo fmt --all

# Run tests
test:
    @echo "Running tests..."
    cargo test --all
