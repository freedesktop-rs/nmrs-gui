#!/usr/bin/env python3
"""Bump nmrs-gui version, update CHANGELOG, and optionally create a git tag."""

import argparse
import re
import subprocess
import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).parent
CARGO_TOML = ROOT / "Cargo.toml"
CHANGELOG = ROOT / "CHANGELOG.md"
REPO = "https://github.com/networkmanager-rs/nmrs-gui"


def current_version() -> str:
    text = CARGO_TOML.read_text()
    m = re.search(r'^version = "([^"]+)"', text, re.MULTILINE)
    if not m:
        sys.exit("Could not find version in Cargo.toml")
    return m.group(1)


def bump_cargo(old: str, new: str) -> None:
    text = CARGO_TOML.read_text()
    updated = text.replace(f'version = "{old}"', f'version = "{new}"', 1)
    if updated == text:
        sys.exit(f"Version {old!r} not found in Cargo.toml")
    CARGO_TOML.write_text(updated)
    print(f"  Cargo.toml: {old} → {new}")


def bump_changelog(old: str, new: str) -> None:
    text = CHANGELOG.read_text()
    today = date.today().isoformat()

    # Promote [Unreleased] to the new version
    text = text.replace(
        "## [Unreleased]",
        f"## [Unreleased]\n\n## [{new}] - {today}",
        1,
    )

    # Update comparison links
    old_unreleased = f"[Unreleased]: {REPO}/compare/v{old}...HEAD"
    new_unreleased = f"[Unreleased]: {REPO}/compare/v{new}...HEAD"
    new_entry = f"[{new}]: {REPO}/compare/v{old}...v{new}"

    text = text.replace(old_unreleased, f"{new_unreleased}\n{new_entry}", 1)

    CHANGELOG.write_text(text)
    print(f"  CHANGELOG.md: promoted [Unreleased] → [{new}]")


def run(cmd: list[str]) -> None:
    subprocess.run(cmd, check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description="Bump nmrs-gui version")
    parser.add_argument("version", help="New version (e.g. 1.6.0)")
    parser.add_argument(
        "--tag",
        action="store_true",
        help="Create and push a git tag after bumping",
    )
    args = parser.parse_args()

    new = args.version.lstrip("v")
    old = current_version()

    if old == new:
        sys.exit(f"Version is already {old}")

    print(f"Bumping {old} → {new}")
    bump_cargo(old, new)
    bump_changelog(old, new)

    if args.tag:
        run(["git", "add", "Cargo.toml", "CHANGELOG.md"])
        run(["git", "commit", "-m", f"chore: release v{new}"])
        run(["git", "tag", f"v{new}"])
        print(f"  Tagged v{new}. Push with: git push && git push origin v{new}")


if __name__ == "__main__":
    main()
