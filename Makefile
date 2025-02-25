# Target WASM platform
WASM_TARGET = wasm32-unknown-unknown

# Optimization flags
RUSTFLAGS = -C link-arg=-s

# Contract directories
CONTRACTS = treasury zkemail xion-account
COMPILED_CONTRACTS = treasury zkemail xion_account

# Build each contract separately
build: $(CONTRACTS)
	@echo "✅ All contracts built and optimized"

# Compile individual contracts
$(CONTRACTS):
	@echo "🚀 Building $@ contract..."
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --target $(WASM_TARGET) -p $@

# Run wasm-opt for additional optimizations
optimize: $(CONTRACTS)
	@echo "🔧 Optimizing contracts..."
	for contract in $(COMPILED_CONTRACTS); do \
		wasm-opt -Oz -o target/wasm32-unknown-unknown/release/$$contract.wasm target/wasm32-unknown-unknown/release/$$contract.wasm; \
	done
	@echo "✅ Optimization complete"

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	@echo "✅ Clean complete"