#!/bin/bash
next_version=$(svu prerelease --pre-release alpha);

cargo_version=$(echo ${next_version} | sed 's/^v//');
echo "Setting Cargo.toml version: ${cargo_version}";
sed -i'.bkp' -E "s/^version = .+/version = \"${cargo_version}\"/" Cargo.toml

cargo clippy;

git commit -am "chore: bump version to ${next_version} [skip ci]";

echo "Creating tag: ${next_version}";
git tag "${next_version}";