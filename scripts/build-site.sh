#!/bin/bash
# build-site.sh — Builds the Anvil promotional site + mdbook docs into dist/
# Works standalone in any Linux environment (CI, Cloudflare Pages, local).
set -euo pipefail

MDBOOK_VERSION="${MDBOOK_VERSION:-0.4.44}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT_DIR"

# Preflight
if [ ! -d "site" ]; then
  echo "Error: site/ directory not found" >&2
  exit 1
fi

# Install mdbook if not available
if ! command -v mdbook >/dev/null 2>&1; then
  echo "==> Installing mdbook v${MDBOOK_VERSION}"
  mkdir -p "$ROOT_DIR/.bin"
  curl -sSL "https://github.com/rust-lang/mdBook/releases/download/v${MDBOOK_VERSION}/mdbook-v${MDBOOK_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
    | tar xz -C "$ROOT_DIR/.bin"
  export PATH="$ROOT_DIR/.bin:$PATH"
fi
echo "    mdbook $(mdbook --version)"

echo "==> Cleaning dist/"
rm -rf dist
mkdir -p dist

echo "==> Copying promotional site"
cp -a site/. dist/

echo "==> Building mdbook documentation"
mdbook build

echo "==> Copying docs to dist/docs/"
cp -r docs/book dist/docs

echo "==> Build complete (output: dist/)"
echo "    Site:  dist/index.html"
echo "    Docs:  dist/docs/index.html"
