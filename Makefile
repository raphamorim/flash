all: run

run:
	cargo run --release

dev:
	cargo run

lint:
	cargo fmt -- --check --color always
	cargo clippy --all-targets --all-features -- -D warnings

test:
	make lint
	RUST_BACKTRACE=full cargo test --release
	RUST_BACKTRACE=full cargo test --test integration_tests

# Run in a single thread is often useful to catch infinite loops early on
test-single-thread:
	make lint
	RUST_BACKTRACE=full cargo test --release -- --test-threads=1
	RUST_BACKTRACE=full cargo test --test integration_tests -- --test-threads=1

test-if:
	@echo "Building Flash for if/elif/else functionality tests..."
	@cargo build --release
	@echo "Running if/elif/else functionality tests..."
	@./test_if_functionality.sh

test-case:
	@echo "Building Flash for case/esac functionality tests..."
	@cargo build --release
	@echo "Running case/esac functionality tests..."
	@./test_case_functionality.sh

test-all: test test-if test-case
	@echo "All tests completed successfully!"

# WebAssembly demo targets
wasm-demo-build:
	@echo "Building Flash WebAssembly Demo..."
	@if ! command -v wasm-pack >/dev/null 2>&1; then \
		echo "wasm-pack is not installed. Installing..."; \
		curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh; \
	fi
	@echo "Building WebAssembly module..."
	@cd docs && wasm-pack build --release --target web --out-dir pkg
	@echo "Removing wasm-pack generated .gitignore to allow committing WASM files..."
	@rm -f docs/pkg/.gitignore
	@echo "Build complete!"
	@echo "WASM files are tracked by Git LFS and can be committed."

wasm-demo-serve: wasm-demo-build
	@echo "Starting Flash WebAssembly Demo server..."
	@echo "Open http://localhost:8000 in your browser"
	@echo "Press Ctrl+C to stop the server"
	@echo ""
	@if ! command -v cargo-server >/dev/null 2>&1; then \
		echo "Installing cargo-server..."; \
		cargo install cargo-server; \
	fi
	@cd docs && cargo server --port 8000

wasm-demo-clean:
	@echo "Cleaning WebAssembly demo build artifacts..."
	@rm -rf docs/pkg docs/target

wasm-demo-commit: wasm-demo-build
	@echo "Adding WebAssembly demo files to git..."
	@git add docs/pkg/
	@git add .gitattributes
	@echo "WebAssembly files added to git (tracked by LFS)"
	@echo "You can now commit with: git commit -m 'Add WebAssembly demo files'"

.PHONY: all run dev lint test test-if test-case test-all wasm-demo-build wasm-demo-serve wasm-demo-clean wasm-demo-commit