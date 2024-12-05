format:
	cargo fmt -- --check
	cargo clippy --no-deps -- -D warnings
	cargo clippy --tests --no-deps -- -D warnings

test:
	cargo test
