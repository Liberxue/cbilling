#!/usr/bin/env bash
# cbilling installer — auto-detects OS/arch, downloads latest release.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Liberxue/cbilling/main/scripts/install.sh | bash
#   curl -fsSL ... | bash -s -- --to ~/bin      # custom install dir
#   curl -fsSL ... | bash -s -- --version v0.2.0 # specific version
set -euo pipefail

REPO="Liberxue/cbilling"
BIN_NAME="cbilling"
INSTALL_DIR="/usr/local/bin"
VERSION=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --to)      INSTALL_DIR="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        *)         shift ;;
    esac
done

# Detect latest version
if [[ -z "$VERSION" ]]; then
    VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep tag_name | cut -d '"' -f4)
    if [[ -z "$VERSION" ]]; then
        echo "Error: could not detect latest version" >&2
        exit 1
    fi
fi

# Detect OS
OS=$(uname -s)
case "$OS" in
    Linux)  PLATFORM="unknown-linux-gnu" ;;
    Darwin) PLATFORM="apple-darwin" ;;
    *)      echo "Error: unsupported OS: $OS" >&2; exit 1 ;;
esac

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)        ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *)             echo "Error: unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

FILENAME="${BIN_NAME}-${VERSION}-${ARCH}-${PLATFORM}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${FILENAME}"

echo "Installing ${BIN_NAME} ${VERSION} (${ARCH}-${PLATFORM})..."
echo "  from: ${URL}"
echo "  to:   ${INSTALL_DIR}/${BIN_NAME}"

# Download and extract
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL "$URL" -o "${TMPDIR}/${FILENAME}"
tar xzf "${TMPDIR}/${FILENAME}" -C "$TMPDIR"

# Install
mkdir -p "$INSTALL_DIR"
mv "${TMPDIR}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
chmod +x "${INSTALL_DIR}/${BIN_NAME}"

echo ""
echo "✓ ${BIN_NAME} ${VERSION} installed to ${INSTALL_DIR}/${BIN_NAME}"
echo ""
echo "Run 'cbilling' to launch the TUI dashboard."
