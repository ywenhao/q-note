#!/usr/bin/env bash
# Q Note Cloud Agent bootstrap. Idempotent and safe to re-run against cached
# state. Prepares the Tauri 2 (Rust) + Vue/Vite (Node) toolchains, then refreshes
# JavaScript dependencies.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# 1. System libraries required to build and run the Tauri 2 shell on Linux.
#    Mirrors the release workflow's Ubuntu dependency list. Skipped when the
#    core WebKitGTK dev package is already present so re-runs stay fast.
if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
  sudo apt-get update
  sudo apt-get install -y --no-install-recommends \
    libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
    patchelf rpm build-essential curl wget file libssl-dev libgtk-3-dev \
    libxdo-dev pkg-config
fi

# 2. Rust stable. Several transitive Tauri crates require edition2024, which
#    needs a newer stable than the image's pinned default toolchain.
if command -v rustup >/dev/null 2>&1; then
  rustup toolchain install stable --profile minimal --no-self-update
  rustup default stable
fi

# 3. Node 24 + pnpm. The image's system Node predates the native TypeScript type
#    stripping that the repo's `node --test *.ts` scripts rely on, and it sits
#    ahead of nvm on PATH. Expose Node 24 and a corepack-pinned pnpm from the
#    first PATH entry so bare `node`/`pnpm` resolve to the correct versions.
export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"
nvm install 24 >/dev/null
nvm alias default 24 >/dev/null
NODE24_BIN="$(dirname "$(nvm which 24)")"

SHIM_DIR="/usr/local/cargo/bin"
mkdir -p "$SHIM_DIR"
ln -sf "$NODE24_BIN/node" "$SHIM_DIR/node"
ln -sf "$NODE24_BIN/npm" "$SHIM_DIR/npm"
ln -sf "$NODE24_BIN/npx" "$SHIM_DIR/npx"
"$NODE24_BIN/corepack" enable --install-directory "$SHIM_DIR"
"$NODE24_BIN/corepack" prepare pnpm@11.20.0 --activate

# 4. JavaScript dependencies, pinned by the committed lockfile.
"$SHIM_DIR/pnpm" install --frozen-lockfile
