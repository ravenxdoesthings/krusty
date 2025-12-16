lint:
	cargo clippy
	cargo fmt --check

format:
	cargo fmt

test:
	cargo nextest run

pre-tag: lint test
	@next_version=$$(svu prerelease --strip-prefix --pre-release alpha); \
	echo "Updating to version $$next_version"; \
	git tag "v$$next_version" 2>/dev/null || true; \
	sed -i'.bkp' -E "s/^version = .+/version = \"$$next_version\"/" Cargo.toml
