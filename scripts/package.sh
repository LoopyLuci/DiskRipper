#!/bin/bash
# DiskRipper Packaging Script
# Builds distributable packages for Windows, Linux, and macOS

set -e

echo "=== DiskRipper Packaging ==="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
VERSION=$(grep '^version' ../Cargo.toml | head -1 | cut -d'"' -f2)
APP_NAME="DiskRipper"
OUTPUT_DIR="./packages"

echo -e "${GREEN}Building ${APP_NAME} v${VERSION}${NC}"
echo ""

# Create output directory
mkdir -p ${OUTPUT_DIR}

# Step 1: Build frontend
echo -e "${YELLOW}Step 1: Building frontend...${NC}"
cd ../frontend
npm install
npm run build
cd ../diskripper-tauri

# Step 2: Build Rust backend
echo -e "${YELLOW}Step 2: Building Rust backend...${NC}"
cargo build --release

# Step 3: Create icons if they don't exist
echo -e "${YELLOW}Step 3: Creating icons...${NC}"
mkdir -p icons

# Create simple placeholder icons (in production, use real icons)
# For now, we'll create a simple PNG using ImageMagick if available
if command -v convert &> /dev/null; then
    convert -size 32x32 xc:blue icons/32x32.png
    convert -size 128x128 xc:blue icons/128x128.png
    convert -size 256x256 xc:blue icons/128x128@2x.png
    convert -size 512x512 xc:blue icons/icon.png
    echo "Icons created with ImageMagick"
else
    echo -e "${YELLOW}ImageMagick not found. Please create icons manually.${NC}"
    echo "Required icons:"
    echo "  - icons/32x32.png"
    echo "  - icons/128x128.png"
    echo "  - icons/128x128@2x.png"
    echo "  - icons/icon.icns (macOS)"
    echo "  - icons/icon.ico (Windows)"
    # Create empty files as placeholders
    touch icons/32x32.png icons/128x128.png icons/128x128@2x.png
    touch icons/icon.icns icons/icon.ico
fi

# Step 4: Build platform-specific packages
echo -e "${YELLOW}Step 4: Building platform packages...${NC}"

# Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)
        echo "Building for Linux..."
        cargo tauri build --target x86_64-unknown-linux-gnu
        echo -e "${GREEN}Linux package built${NC}"
        echo "Output: src-tauri/target/release/bundle/"
        ;;
    Darwin*)
        echo "Building for macOS..."
        cargo tauri build --target x86_64-apple-darwin
        cargo tauri build --target aarch64-apple-darwin
        echo -e "${GREEN}macOS packages built${NC}"
        echo "Output: src-tauri/target/release/bundle/"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        echo "Building for Windows..."
        cargo tauri build --target x86_64-pc-windows-msvc
        echo -e "${GREEN}Windows package built${NC}"
        echo "Output: src-tauri/target/release/bundle/"
        ;;
    *)
        echo -e "${RED}Unknown OS: ${OS}${NC}"
        exit 1
        ;;
esac

# Step 5: Copy packages to output directory
echo -e "${YELLOW}Step 5: Copying packages...${NC}"
cp -r src-tauri/target/release/bundle/* ${OUTPUT_DIR}/ 2>/dev/null || true

# Step 6: Create checksums
echo -e "${YELLOW}Step 6: Creating checksums...${NC}"
cd ${OUTPUT_DIR}
find . -type f \( -name "*.msi" -o -name "*.exe" -o -name "*.dmg" -o -name "*.AppImage" -o -name "*.deb" -o -name "*.rpm" -o -name "*.tar.gz" \) -exec sha256sum {} \; > checksums.txt
cd ..

echo ""
echo -e "${GREEN}=== Packaging Complete ===${NC}"
echo ""
echo "Packages available in: ${OUTPUT_DIR}/"
echo ""
ls -la ${OUTPUT_DIR}/
echo ""
echo "Checksums:"
cat ${OUTPUT_DIR}/checksums.txt 2>/dev/null || echo "No packages found"
