#!/usr/bin/env sh
set -e

REPO="https://github.com/aaron-amelink/lazypost"
MIN_RUST_MAJOR=1
MIN_RUST_MINOR=85

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n'  "$*"; }

# ── Rust / Cargo check ────────────────────────────────────────────────────────

if ! command -v cargo > /dev/null 2>&1; then
    red "cargo not found."
    echo "Install Rust via rustup:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

rust_version=$(rustc --version | awk '{print $2}')
rust_major=$(echo "$rust_version" | cut -d. -f1)
rust_minor=$(echo "$rust_version" | cut -d. -f2)

if [ "$rust_major" -lt "$MIN_RUST_MAJOR" ] || \
   { [ "$rust_major" -eq "$MIN_RUST_MAJOR" ] && [ "$rust_minor" -lt "$MIN_RUST_MINOR" ]; }; then
    red "Rust $MIN_RUST_MAJOR.$MIN_RUST_MINOR+ is required (found $rust_version)."
    echo "Update with:  rustup update stable"
    exit 1
fi

echo "Rust $rust_version — ok"

# ── Build & install ───────────────────────────────────────────────────────────

# If this script is run from inside the repo directory, install from here.
# Otherwise clone a fresh copy into a temp dir.
if [ -f "$(dirname "$0")/Cargo.toml" ] && \
   grep -q 'name = "lazypost"' "$(dirname "$0")/Cargo.toml" 2>/dev/null; then
    INSTALL_DIR="$(dirname "$0")"
    bold "Installing from local source…"
    cargo install --path "$INSTALL_DIR" --locked
else
    bold "Cloning $REPO…"
    TMP=$(mktemp -d)
    trap 'rm -rf "$TMP"' EXIT
    git clone --depth 1 "$REPO" "$TMP/lazypost"
    bold "Building (release)…"
    cargo install --path "$TMP/lazypost" --locked
fi

# ── PATH reminder ─────────────────────────────────────────────────────────────

CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"

green "lazypost installed to $CARGO_BIN/lazypost"

if ! echo "$PATH" | grep -q "$CARGO_BIN"; then
    echo ""
    echo "  $CARGO_BIN is not on your PATH."
    echo "  Add this to your shell rc (~/.bashrc, ~/.zshrc, etc.):"
    echo ""
    echo "    export PATH=\"\$PATH:$CARGO_BIN\""
    echo ""
fi

echo ""
bold "Run 'lazypost' from any project directory to get started."
