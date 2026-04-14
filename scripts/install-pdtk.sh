#!/usr/bin/env bash
# install-pdtk.sh — download a pdtk release binary into a local .tools/bin/ directory.
#
# Usage:
#   ./scripts/install-pdtk.sh                  # latest release
#   ./scripts/install-pdtk.sh v0.3.0           # specific version tag
#
# Environment:
#   PDTK_INSTALL_DIR   destination directory  (default: .tools/bin)
#
# The binary is placed at $PDTK_INSTALL_DIR/pdtk and made executable.
# Run from the root of your project.

set -euo pipefail

REPO="ModularDevicesMalleableAudio/pd-toolkit"
VERSION="${1:-latest}"
INSTALL_DIR="${PDTK_INSTALL_DIR:-.tools/bin}"

# ── Platform detection ────────────────────────────────────────────────────────

OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}-${ARCH}" in
    Linux-x86_64)   BINARY="pdtk-x86_64-linux" ;;
    Linux-aarch64)  BINARY="pdtk-aarch64-linux-musl" ;;
    Darwin-arm64)   BINARY="pdtk-aarch64-macos" ;;
    Darwin-x86_64)  BINARY="pdtk-x86_64-macos" ;;
    *)
        echo "error: unsupported platform: ${OS}-${ARCH}" >&2
        echo "       build from source: cargo build --release" >&2
        exit 1
        ;;
esac

# ── Resolve version tag ───────────────────────────────────────────────────────

if [ "${VERSION}" = "latest" ]; then
    echo "Fetching latest release tag…"
    TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
           | grep '"tag_name"' \
           | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
    if [ -z "${TAG}" ]; then
        echo "error: could not determine latest release tag" >&2
        exit 1
    fi
else
    TAG="${VERSION}"
fi

# ── Download ──────────────────────────────────────────────────────────────────

URL="https://github.com/${REPO}/releases/download/${TAG}/${BINARY}"
DEST="${INSTALL_DIR}/pdtk"

echo "Installing pdtk ${TAG} (${BINARY}) → ${DEST}"
mkdir -p "${INSTALL_DIR}"
curl -fsSL --progress-bar "${URL}" -o "${DEST}"
chmod +x "${DEST}"

echo "Done."
echo "  ${DEST} --version"
