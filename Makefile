lint:
	cargo clippy
	cargo fmt --check

format:
	cargo fmt

test:
	cargo nextest run