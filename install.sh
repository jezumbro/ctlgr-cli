#!/usr/bin/env sh
# Install ctlgr — downloads the right pre-built binary from GitHub Releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/jezumbro/ctlgr-cli/main/install.sh | sh
#
# Environment variables:
#   VERSION     — specific version to install (default: latest)
#   INSTALL_DIR — installation directory        (default: ~/.local/bin)

set -e

REPO="jezumbro/ctlgr-cli"
VERSION="${VERSION:-}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# ── Detect platform ────────────────────────────────────────────────────────────

OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
  Darwin) OS_PART="apple-darwin" ;;
  Linux)  OS_PART="unknown-linux-musl" ;;
  *)
    echo "error: unsupported OS: $OS" >&2
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64)        ARCH_PART="x86_64" ;;
  arm64|aarch64) ARCH_PART="aarch64" ;;
  *)
    echo "error: unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

TARGET="${ARCH_PART}-${OS_PART}"

# ── Resolve version ────────────────────────────────────────────────────────────

if [ -z "$VERSION" ]; then
  VERSION=$(
    curl -fsSL \
      -H "Accept: application/vnd.github.v3+json" \
      "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' \
    | sed 's/.*"v\([^"]*\)".*/\1/'
  )
fi

if [ -z "$VERSION" ]; then
  echo "error: could not determine latest version — set VERSION manually" >&2
  exit 1
fi

# ── Download ───────────────────────────────────────────────────────────────────

URL="https://github.com/$REPO/releases/download/v${VERSION}/ctlgr-v${VERSION}-${TARGET}"
DEST="$INSTALL_DIR/ctlgr"

echo "installing ctlgr v${VERSION} (${TARGET}) → ${DEST}"
mkdir -p "$INSTALL_DIR"
curl -fsSL "$URL" -o "$DEST"
chmod +x "$DEST"

echo "done."

# ── PATH hint ─────────────────────────────────────────────────────────────────

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo ""
    echo "note: $INSTALL_DIR is not in your PATH."
    echo "      Add the following to your shell profile:"
    echo "      export PATH=\"\$PATH:$INSTALL_DIR\""
    ;;
esac
