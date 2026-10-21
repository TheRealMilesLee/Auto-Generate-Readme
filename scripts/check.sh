#!/usr/bin/env bash
# Local quality gate. No GitHub Actions, no paid CI.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

if cargo fmt --version >/dev/null 2>&1; then
  cargo fmt --check
else
  echo "note: rustfmt not installed, skipping format check" >&2
fi

if cargo clippy --version >/dev/null 2>&1; then
  cargo clippy --all-targets -- -D warnings
else
  echo "note: clippy not installed, skipping lint" >&2
fi

cargo test
cargo build --release

if [[ -f README.md ]] && grep -q 'readme-gen:meta' README.md; then
  ./target/release/readme-gen check --strict
fi
