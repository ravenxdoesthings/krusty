#!/usr/bin/env python3
"""
Script to bump RC versions in Cargo.toml and create git tags.
Handles vX.X.X-rc.N format tags.
"""

import subprocess
import re
import sys
import argparse
from pathlib import Path


def run_command(cmd):
    """Execute a shell command and return output."""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error running: {cmd}")
        print(f"stderr: {result.stderr}")
        sys.exit(1)
    return result.stdout.strip()


def get_latest_tag():
    """Get the latest tag using git describe."""
    try:
        tag = run_command("git describe --tags --abbrev=0")
        return tag
    except Exception as e:
        print(f"Error getting latest tag: {e}")
        sys.exit(1)


def parse_rc_version(tag):
    """Parse RC version from tag. Returns (base_version, rc_number) or None if not RC."""
    # Match vX.X.X-rc.N format
    match = re.match(r'^v(\d+\.\d+\.\d+)-rc\.(\d+)$', tag)
    if match:
        base_version = match.group(1)
        rc_number = int(match.group(2))
        return base_version, rc_number
    return None


def bump_rc_version(base_version, rc_number):
    """Create new RC version with incremented RC number."""
    new_rc_number = rc_number + 1
    return f"{base_version}-rc.{new_rc_number}", f"v{base_version}-rc.{new_rc_number}"


def prompt_for_next_version():
    """Prompt user for next version in vX.X.X format and return first RC version."""
    while True:
        user_input = input("Enter the next version (format: vX.X.X): ").strip()
        
        # Validate format: vX.X.X
        if re.match(r'^v\d+\.\d+\.\d+$', user_input):
            base_version = user_input[1:]  # Strip the 'v' prefix
            cargo_version = f"{base_version}-rc.0"
            tag_version = f"{user_input}-rc.0"
            return cargo_version, tag_version
        else:
            print("Invalid format. Please use vX.X.X (e.g., v2.0.0)")


def get_cargo_toml_path():
    """Get path to Cargo.toml relative to script location."""
    script_dir = Path(__file__).parent
    cargo_path = script_dir.parent / "Cargo.toml"
    return cargo_path


def update_cargo_version(cargo_path, new_version, dry_run=False):
    """Update version in Cargo.toml."""
    with open(cargo_path, 'r') as f:
        content = f.read()
    
    # Match version = "X.X.X..." at the start of a line
    updated = re.sub(
        r'^version = "[^"]+"',
        f'version = "{new_version}"',
        content,
        count=1,
        flags=re.MULTILINE
    )
    
    if updated == content:
        print("Warning: Could not find version line in Cargo.toml")
        return False
    
    if not dry_run:
        with open(cargo_path, 'w') as f:
            f.write(updated)
    
    return True


def main():
    """Main execution."""
    parser = argparse.ArgumentParser(description="Bump RC versions in Cargo.toml and create git tags.")
    parser.add_argument("--dry-run", action="store_true", help="Print changes without modifying files or repository")
    args = parser.parse_args()
    
    if args.dry_run:
        print("[DRY RUN] No changes will be made")
        print()
    
    # Get latest tag
    latest_tag = get_latest_tag()
    print(f"Latest tag: {latest_tag}")
    
    # Check if it's an RC version
    rc_info = parse_rc_version(latest_tag)
    if not rc_info:
        print(f"Tag {latest_tag} is not in RC format (vX.X.X-rc.N).")
        new_cargo_version, new_tag = prompt_for_next_version()
        print(f"Creating first RC: {new_tag}")
    else:
        base_version, rc_number = rc_info
        print(f"Current RC version: {base_version}-rc.{rc_number}")
        
        # Bump version
        new_cargo_version, new_tag = bump_rc_version(base_version, rc_number)
    
    print(f"New version: {new_cargo_version}")
    print(f"New tag: {new_tag}")
    
    if args.dry_run:
        print(f"[DRY RUN] Would update {get_cargo_toml_path()} with version: {new_cargo_version}")
        print(f"[DRY RUN] Would run: cargo check")
        print(f"[DRY RUN] Would commit: git commit -am 'chore: bumping version to {new_cargo_version} [skip ci]'")
        print(f"[DRY RUN] Would create tag: git tag {new_tag}")
        return
    
    # Update Cargo.toml
    cargo_path = get_cargo_toml_path()
    if not update_cargo_version(cargo_path, new_cargo_version):
        sys.exit(1)
    print(f"Updated {cargo_path}")
    
    # Run cargo check
    print("Running cargo check...")
    run_command("cargo check")
    print("cargo check completed")
    
    # Commit changes
    commit_message = f"chore: bumping version to {new_cargo_version} [skip ci]"
    print(f"Committing: {commit_message}")
    run_command(f"git commit -am '{commit_message}'")
    print("Commit created")
    
    # Create git tag
    print(f"Creating git tag: {new_tag}")
    run_command(f"git tag {new_tag}")
    print(f"Tag {new_tag} created successfully")


if __name__ == "__main__":
    main()
