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

test-if:
	@echo "Building Flash for if/elif/else functionality tests..."
	@cargo build --release
	@echo "Running if/elif/else functionality tests..."
	@./test_if_functionality.sh

test-all: test test-if
	@echo "All tests completed successfully!"

.PHONY: all run dev lint test test-if test-all