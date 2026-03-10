#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

export RUSTC_WRAPPER=

echo "==> cargo +nightly fmt -- --check"
cargo +nightly fmt -- --check

if command -v taplo >/dev/null 2>&1; then
  echo "==> taplo fmt --check"
  taplo fmt --check
else
  echo "==> taplo fmt --check (skipped: taplo not installed)"
fi

echo "==> cargo clippy --workspace --all-targets --all-features"
cargo clippy --workspace --all-targets --all-features

echo "==> cargo build --release --all-features --workspace --examples"
cargo build --release --all-features --workspace --examples

echo "==> cargo nextest run --all-features --workspace"
CARGO_TERM_COLOR=never cargo nextest run --all-features --workspace

echo "==> cargo mend"
CARGO_TERM_COLOR=never cargo mend
