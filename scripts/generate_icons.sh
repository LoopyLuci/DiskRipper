#!/bin/bash
# Generate placeholder icons for DiskRipper
# For production, replace with real designed icons

set -e

ICONS_DIR="diskripper-tauri/icons"
mkdir -p "$ICONS_DIR"

echo "Generating placeholder icons..."

# Check for ImageMagick
if command -v convert &> /dev/null; then
    echo "Using ImageMagick to generate icons..."
    
    # Generate PNG icons
    convert -size 32x32 xc:'#3b82f6' -pointsize 20 -fill white -gravity center -annotate +0+0 "DR" "$ICONS_DIR/32x32.png"
    convert -size 128x128 xc:'#3b82f6' -pointsize 80 -fill white -gravity center -annotate +0+0 "DR" "$ICONS_DIR/128x128.png"
    convert -size 256x256 xc:'#3b82f6' -pointsize 160 -fill white -gravity center -annotate +0+0 "DR" "$ICONS_DIR/128x128@2x.png"
    convert -size 512x512 xc:'#3b82f6' -pointsize 320 -fill white -gravity center -annotate +0+0 "DR" "$ICONS_DIR/icon.png"
    
    # Generate ICO (Windows)
    convert "$ICONS_DIR/32x32.png" "$ICONS_DIR/128x128.png" -colors 256 "$ICONS_DIR/icon.ico"
    
    # Generate ICNS (macOS) - requires additional tools
    if command -v png2icns &> /dev/null; then
        png2icns "$ICONS_DIR/icon.icns" "$ICONS_DIR/128x128.png"
    else
        echo "png2icns not found, creating placeholder ICNS"
        cp "$ICONS_DIR/128x128.png" "$ICONS_DIR/icon.icns"
    fi
    
    echo "Icons generated successfully!"
else
    echo "ImageMagick not found. Creating minimal placeholder icons..."
    
    # Create minimal valid PNG files as placeholders
    # These are 1x1 pixel PNGs - replace with real icons for production
    python3 -c "
import struct, zlib

def create_png(width, height, r, g, b):
    def chunk(chunk_type, data):
        c = chunk_type + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
    
    header = b'\\x89PNG\\r\\n\\x1a\\n'
    ihdr = chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
    raw = b''
    for y in range(height):
        raw += b'\\x00' + bytes([r, g, b]) * width
    idat = chunk(b'IDAT', zlib.compress(raw))
    iend = chunk(b'IEND', b'')
    return header + ihdr + idat + iend

# Generate icons
for size, name in [(32, '32x32.png'), (128, '128x128.png'), (256, '128x128@2x.png'), (512, 'icon.png')]:
    with open(f'$ICONS_DIR/{name}', 'wb') as f:
        f.write(create_png(size, size, 59, 130, 246))  # Blue #3b82f6

# Copy for ICO and ICNS (not valid formats, just placeholders)
import shutil
shutil.copy(f'$ICONS_DIR/32x32.png', f'$ICONS_DIR/icon.ico')
shutil.copy(f'$ICONS_DIR/128x128.png', f'$ICONS_DIR/icon.icns')

print('Placeholder icons created. Replace with real icons for production.')
"
fi

echo ""
echo "Icons in $ICONS_DIR:"
ls -la "$ICONS_DIR/"
