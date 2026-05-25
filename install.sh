#!/usr/bin/env sh
# Install script for httpc
# Usage: curl -sSf https://raw.githubusercontent.com/hainet50b/zed-http-client/main/install.sh | sh

set -e

REPO="hainet50b/zed-http-client"
INSTALL_DIR="${HTTPC_INSTALL_DIR:-$HOME/.local/bin}"

err() {
    echo "Error: $1" >&2
    exit 1
}

check_already_latest() {
    if [ -n "${HTTPC_FORCE_INSTALL:-}" ]; then
        return 0
    fi
    if ! command -v httpc >/dev/null 2>&1; then
        return 0
    fi
    _ver_local="$(httpc --version 2>/dev/null | awk '{print $NF}')"
    if [ -z "$_ver_local" ]; then
        return 0
    fi
    _ver_response="$(curl -fsSL --max-time 5 "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null)" || return 0
    _ver_latest="$(printf '%s' "$_ver_response" | sed -n 's/.*"tag_name":[[:space:]]*"v\{0,1\}\([^"]*\)".*/\1/p' | head -n 1)"
    if [ -z "$_ver_latest" ]; then
        return 0
    fi
    if [ "$_ver_local" = "$_ver_latest" ]; then
        echo "httpc $_ver_local is already the latest. Set HTTPC_FORCE_INSTALL=1 to reinstall."
        exit 0
    fi
}

check_already_latest

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS-$ARCH" in
    Linux-x86_64)   TARGET="x86_64-unknown-linux-gnu" ;;
    Linux-aarch64)  TARGET="aarch64-unknown-linux-gnu" ;;
    Darwin-x86_64)  TARGET="x86_64-apple-darwin" ;;
    Darwin-arm64)   TARGET="aarch64-apple-darwin" ;;
    *) err "Unsupported platform: $OS $ARCH" ;;
esac

command -v curl >/dev/null 2>&1 || err "curl is required but not installed"
command -v tar >/dev/null 2>&1 || err "tar is required but not installed"

URL="https://github.com/$REPO/releases/latest/download/httpc-$TARGET.tar.gz"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Downloading httpc for $TARGET..."
curl -fsSL "$URL" -o "$TMP/httpc.tar.gz" || err "download failed: $URL"

echo "Extracting..."
tar -xzf "$TMP/httpc.tar.gz" -C "$TMP" || err "extract failed"

mkdir -p "$INSTALL_DIR"
mv "$TMP/httpc" "$INSTALL_DIR/httpc"
chmod +x "$INSTALL_DIR/httpc"

echo "Installed httpc to $INSTALL_DIR/httpc"

case ":$PATH:" in
    *":$INSTALL_DIR:"*)
        echo ""
        "$INSTALL_DIR/httpc" --version
        ;;
    *)
        cat <<EOF

Note: $INSTALL_DIR is not in your PATH.
Add the following to your shell config (e.g., ~/.bashrc):

    export PATH="$INSTALL_DIR:\$PATH"

Then open a new terminal.
EOF
        ;;
esac
