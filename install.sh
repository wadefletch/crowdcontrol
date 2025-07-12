#!/bin/bash

# CrowdControl Installation Script
# This script installs CrowdControl CLI and pulls the Docker image

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
GITHUB_REPO="wadefletch/crowdcontrol"  # Update this with actual repo
DOCKER_IMAGE="crowdcontrol/crowdcontrol:latest"  # Update with actual Docker Hub repo
INSTALL_DIR="$HOME/.local/bin"

# Utility functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Detect OS and architecture
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    case $os in
        linux)
            case $arch in
                x86_64) echo "x86_64-unknown-linux-gnu" ;;
                aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
                *) log_error "Unsupported architecture: $arch"; exit 1 ;;
            esac
            ;;
        darwin)
            case $arch in
                x86_64) echo "x86_64-apple-darwin" ;;
                arm64) echo "aarch64-apple-darwin" ;;
                *) log_error "Unsupported architecture: $arch"; exit 1 ;;
            esac
            ;;
        mingw*|msys*|cygwin*)
            echo "x86_64-pc-windows-msvc"
            ;;
        *)
            log_error "Unsupported operating system: $os"
            exit 1
            ;;
    esac
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check for Docker
    if ! command_exists docker; then
        log_error "Docker is not installed. Please install Docker first."
        log_info "Visit: https://docs.docker.com/get-docker/"
        exit 1
    fi
    
    # Check if Docker is running
    if ! docker info >/dev/null 2>&1; then
        log_error "Docker is not running. Please start Docker first."
        exit 1
    fi
    
    log_success "Prerequisites check passed"
}

# Pull Docker image
pull_docker_image() {
    log_info "Pulling CrowdControl Docker image..."
    
    if docker pull "$DOCKER_IMAGE"; then
        log_success "Docker image pulled successfully"
    else
        log_error "Failed to pull Docker image"
        exit 1
    fi
}

# Install CLI binary
install_cli() {
    log_info "Installing CrowdControl CLI..."
    
    # Create install directory
    mkdir -p "$INSTALL_DIR"
    
    # Detect platform
    local platform=$(detect_platform)
    log_info "Detected platform: $platform"
    
    # Get latest release
    log_info "Fetching latest release..."
    local latest_url="https://api.github.com/repos/$GITHUB_REPO/releases/latest"
    local release_data
    
    if command_exists curl; then
        release_data=$(curl -s "$latest_url")
    elif command_exists wget; then
        release_data=$(wget -qO- "$latest_url")
    else
        log_error "Neither curl nor wget is available. Cannot download CLI."
        exit 1
    fi
    
    # Extract download URL using proper JSON parsing
    local download_url
    if [[ "$platform" == *"windows"* ]]; then
        local archive_name="crowdcontrol-$platform.zip"
    else
        local archive_name="crowdcontrol-$platform.tar.gz"
    fi
    
    # Use a more robust approach to extract the download URL
    download_url=$(echo "$release_data" | grep -o "\"browser_download_url\": *\"[^\"]*$archive_name\"" | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')
    
    if [[ -z "$download_url" ]]; then
        log_error "Could not find release for platform: $platform"
        log_info "Falling back to building from source..."
        install_from_source
        return
    fi
    
    # Download and extract
    log_info "Downloading from: $download_url"
    local temp_dir=$(mktemp -d)
    
    if command_exists curl; then
        curl -L -o "$temp_dir/$archive_name" "$download_url"
    else
        wget -O "$temp_dir/$archive_name" "$download_url"
    fi
    
    # Extract based on file type
    if [[ "$archive_name" == *.zip ]]; then
        if command_exists unzip; then
            unzip -q "$temp_dir/$archive_name" -d "$temp_dir"
        else
            log_error "unzip is not available. Cannot extract CLI."
            exit 1
        fi
        local binary_name="crowdcontrol.exe"
    else
        tar -xzf "$temp_dir/$archive_name" -C "$temp_dir"
        local binary_name="crowdcontrol"
    fi
    
    # Move binary to install directory
    mv "$temp_dir/$binary_name" "$INSTALL_DIR/crowdcontrol"
    chmod +x "$INSTALL_DIR/crowdcontrol"
    
    # Cleanup
    rm -rf "$temp_dir"
    
    log_success "CLI installed to: $INSTALL_DIR/crowdcontrol"
}

# Install from source (fallback)
install_from_source() {
    log_info "Installing from source..."
    
    if ! command_exists cargo; then
        log_error "Rust/Cargo is not installed. Please install Rust first."
        log_info "Visit: https://rustup.rs/"
        exit 1
    fi
    
    # Install using cargo from git repo
    if cargo install --git "https://github.com/$GITHUB_REPO.git" crowdcontrol-cli --root "$HOME/.local"; then
        log_success "CLI installed from source"
    else
        log_error "Failed to install from source"
        exit 1
    fi
}

# Check if CLI is in PATH
check_path() {
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        log_warning "The install directory '$INSTALL_DIR' is not in your PATH."
        log_info "Add this line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo "export PATH=\"\$PATH:$INSTALL_DIR\""
        echo
        log_info "Or run: echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> ~/.bashrc"
        log_info "Then restart your shell or run: source ~/.bashrc"
    fi
}

# Verify installation
verify_installation() {
    log_info "Verifying installation..."
    
    if "$INSTALL_DIR/crowdcontrol" --version >/dev/null 2>&1; then
        log_success "CrowdControl CLI is working correctly"
        log_info "Run 'crowdcontrol --help' to get started"
    else
        log_error "CLI installation verification failed"
        exit 1
    fi
}

# Main installation process
main() {
    echo -e "${BLUE}"
    echo "┌─────────────────────────────────────────┐"
    echo "│          CrowdControl Installer         │"
    echo "└─────────────────────────────────────────┘"
    echo -e "${NC}"
    
    check_prerequisites
    pull_docker_image
    install_cli
    check_path
    verify_installation
    
    echo
    log_success "Installation completed successfully!"
    echo
    log_info "Next steps:"
    echo "  1. Make sure $INSTALL_DIR is in your PATH"
    echo "  2. Run: crowdcontrol --help"
    echo "  3. Create your first agent: crowdcontrol new myapp git@github.com:yourname/myapp.git"
    echo
}

# Run main function
main "$@"