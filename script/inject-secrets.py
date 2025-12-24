#!/usr/bin/env python3
"""
Inject Infisical secrets into Helm values file.

Usage: python script/inject_secrets.py <environment>
"""

import json
import sys
import subprocess
from pathlib import Path
from typing import Any, TypedDict
import yaml


class InfisicalSecret(TypedDict):
    """Typed representation of an Infisical secret."""
    secretKey: str
    secretValue: str


class LiteralString(str):
    """Marker class for strings that should be represented as literal blocks in YAML."""
    pass


def literal_presenter(dumper: Any, data: Any) -> Any:
    """Custom presenter for literal multiline strings."""
    if '\n' in data:
        return dumper.represent_scalar('tag:yaml.org,2002:str', data, style='|')
    return dumper.represent_scalar('tag:yaml.org,2002:str', data)


yaml.add_representer(LiteralString, literal_presenter)


def run_command(cmd: list[str]) -> str:
    """Run a shell command and return its output."""
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        return result.stdout
    except subprocess.CalledProcessError as e:
        print(f"Error running command: {' '.join(cmd)}", file=sys.stderr)
        print(f"stderr: {e.stderr}", file=sys.stderr)
        sys.exit(1)


def fetch_secrets(environment: str) -> dict[str, str]:
    """Fetch secrets from Infisical for the given environment."""
    cmd: list[str] = ["infisical", "secrets", "--env", environment, "--output", "json"]
    output: str = run_command(cmd)

    try:
        data: Any = json.loads(output)
        # Infisical returns an array of secret objects with 'secretKey' and 'secretValue' fields
        secrets: dict[str, str] = {}
        if isinstance(data, list):
            for item in data:
                # Type-safe extraction of secretKey and secretValue
                secret: InfisicalSecret = item
                secrets[secret["secretKey"]] = secret["secretValue"]
        else:
            # Handle case where output is an object
            secrets = data
        return secrets
    except json.JSONDecodeError as e:
        print(f"Error parsing Infisical JSON output: {e}", file=sys.stderr)
        sys.exit(1)


def load_base_values(base_path: Path) -> dict[str, Any]:
    """Load the base values.yaml file."""
    try:
        with open(base_path, 'r') as f:
            result: Any = yaml.safe_load(f)
            return result if isinstance(result, dict) else {}
    except FileNotFoundError:
        print(f"Error: Base values file not found at {base_path}", file=sys.stderr)
        sys.exit(1)
    except yaml.YAMLError as e:
        print(f"Error parsing YAML: {e}", file=sys.stderr)
        sys.exit(1)


def update_secrets_in_values(values: dict[str, Any], secrets: dict[str, str]) -> dict[str, Any]:
    """Update the secrets.app.data section in values with fetched secrets."""
    if "secrets" not in values:
        values["secrets"] = {}

    secrets_dict: dict[str, Any] = values["secrets"]
    if "app" not in secrets_dict:
        secrets_dict["app"] = {}

    app_dict: dict[str, Any] = secrets_dict["app"]
    if "data" not in app_dict:
        app_dict["data"] = {}

    # Update the data map with secrets - type-safe assignment
    data_dict: dict[str, str] = app_dict["data"]
    data_dict.update(secrets)

    return values


def save_values(values: dict[str, Any], output_path: Path) -> None:
    """Save the updated values to a new file."""
    try:
        with open(output_path, 'w') as f:
            yaml.dump(values, f, default_flow_style=False, sort_keys=False)
        print(f"✓ Created {output_path}")
    except IOError as e:
        print(f"Error writing to {output_path}: {e}", file=sys.stderr)
        sys.exit(1)


def main() -> None:
    """Main entry point."""
    if len(sys.argv) != 2:
        print("Usage: python script/inject_secrets.py <environment>", file=sys.stderr)
        sys.exit(1)

    environment: str = sys.argv[1]

    # Resolve paths relative to script location
    script_dir: Path = Path(__file__).parent
    helm_dir: Path = script_dir.parent / "helm"
    base_values_path: Path = helm_dir / "values.yaml"
    output_values_path: Path = helm_dir / f"values.{environment}.yaml"

    print(f"Fetching secrets for environment: {environment}")
    secrets: dict[str, str] = fetch_secrets(environment)
    print(f"✓ Retrieved {len(secrets)} secrets")

    print(f"Loading base values from {base_values_path.relative_to(script_dir.parent)}")
    values: dict[str, Any] = load_base_values(base_values_path)

    print(f"Updating secrets.app.data")
    values = update_secrets_in_values(values, secrets)

    save_values(values, output_values_path)
    print(f"✓ Complete!")


if __name__ == "__main__":
    main()
