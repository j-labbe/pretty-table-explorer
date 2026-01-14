#!/bin/sh
# install.sh - One-line installer for pte (pretty-table-explorer)
# Usage: curl -fsSL https://raw.githubusercontent.com/j-labbe/pretty-table-explorer/master/install.sh | sh
#
# Environment variables:
#   REPO         - GitHub repository (default: j-labbe/pretty-table-explorer)
#   INSTALL_DIR  - Installation directory (default: /usr/local/bin)

set -e

# Configuration (can be overridden via environment variables)
REPO="${REPO:-j-labbe/pretty-table-explorer}"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="pte"

# Colors for output (if terminal supports them)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Print functions
info() {
    printf "${GREEN}[INFO]${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$1"
}

error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1" >&2
}

# Cleanup on exit
cleanup() {
    if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
    fi
}
trap cleanup EXIT INT TERM

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            error "Supported: Linux, macOS"
            exit 1
            ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)    echo "x86_64" ;;
        aarch64|arm64)   echo "aarch64" ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            error "Supported: x86_64, aarch64/arm64"
            exit 1
            ;;
    esac
}

# Check if command exists
has_command() {
    command -v "$1" >/dev/null 2>&1
}

# Check required commands
check_requirements() {
    # Check for curl or wget
    if ! has_command curl && ! has_command wget; then
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    # Check for sha256sum or shasum
    if ! has_command sha256sum && ! has_command shasum; then
        error "Neither sha256sum nor shasum found. Please install one of them."
        exit 1
    fi
}

# Download file using curl or wget
download() {
    url="$1"
    output="$2"

    if has_command curl; then
        curl -fsSL "$url" -o "$output"
    elif has_command wget; then
        wget -q "$url" -O "$output"
    fi
}

# Verify checksum
verify_checksum() {
    file="$1"
    checksum_file="$2"
    binary_name="$3"

    # Extract expected checksum for our binary
    expected_checksum=$(grep "$binary_name" "$checksum_file" | awk '{print $1}')

    if [ -z "$expected_checksum" ]; then
        error "Could not find checksum for $binary_name in checksums.txt"
        exit 1
    fi

    # Calculate actual checksum
    if has_command sha256sum; then
        actual_checksum=$(sha256sum "$file" | awk '{print $1}')
    elif has_command shasum; then
        actual_checksum=$(shasum -a 256 "$file" | awk '{print $1}')
    fi

    if [ "$expected_checksum" != "$actual_checksum" ]; then
        error "Checksum verification failed!"
        error "Expected: $expected_checksum"
        error "Actual:   $actual_checksum"
        exit 1
    fi

    info "Checksum verified successfully"
}

# Main installation
main() {
    info "Installing pte (pretty-table-explorer)..."

    # Check requirements
    check_requirements

    # Detect platform
    OS=$(detect_os)
    ARCH=$(detect_arch)
    PLATFORM_BINARY="${BINARY_NAME}-${OS}-${ARCH}"

    info "Detected platform: ${OS}-${ARCH}"

    # Create temp directory
    TMP_DIR=$(mktemp -d)

    # Build download URLs
    BASE_URL="https://github.com/${REPO}/releases/latest/download"
    BINARY_URL="${BASE_URL}/${PLATFORM_BINARY}"
    CHECKSUM_URL="${BASE_URL}/checksums.txt"

    # Download binary
    info "Downloading ${PLATFORM_BINARY}..."
    download "$BINARY_URL" "${TMP_DIR}/${PLATFORM_BINARY}"

    # Download checksums
    info "Downloading checksums..."
    download "$CHECKSUM_URL" "${TMP_DIR}/checksums.txt"

    # Verify checksum
    info "Verifying checksum..."
    verify_checksum "${TMP_DIR}/${PLATFORM_BINARY}" "${TMP_DIR}/checksums.txt" "$PLATFORM_BINARY"

    # Create install directory if it doesn't exist
    if [ ! -d "$INSTALL_DIR" ]; then
        warn "Install directory $INSTALL_DIR does not exist, creating it..."
        mkdir -p "$INSTALL_DIR" 2>/dev/null || {
            error "Failed to create $INSTALL_DIR. Try running with sudo or set INSTALL_DIR to a writable path."
            exit 1
        }
    fi

    # Install binary
    info "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
    if ! mv "${TMP_DIR}/${PLATFORM_BINARY}" "${INSTALL_DIR}/${BINARY_NAME}" 2>/dev/null; then
        # Try with sudo
        warn "Permission denied, trying with sudo..."
        sudo mv "${TMP_DIR}/${PLATFORM_BINARY}" "${INSTALL_DIR}/${BINARY_NAME}" || {
            error "Failed to install binary. Try running with sudo or set INSTALL_DIR to a writable path."
            exit 1
        }
    fi

    # Make executable
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}" 2>/dev/null || sudo chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    # Verify installation
    if has_command "$BINARY_NAME"; then
        VERSION=$("$BINARY_NAME" --version 2>/dev/null || echo "unknown")
        info "Successfully installed: $VERSION"
    else
        warn "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
        warn "Make sure ${INSTALL_DIR} is in your PATH"
    fi

    info "Installation complete!"
}

main "$@"
