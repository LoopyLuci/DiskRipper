#!/usr/bin/env python3
"""
Generate placeholder icons for DiskRipper.
Run: python scripts/generate_icons.py
For production, replace with real designed icons.
"""

import struct
import zlib
import os

def create_png(width, height, r, g, b):
    """Create a minimal PNG file with solid color."""
    def chunk(chunk_type, data):
        c = chunk_type + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
    
    header = b'\x89PNG\r\n\x1a\n'
    ihdr = chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
    raw = b''
    for y in range(height):
        raw += b'\x00' + bytes([r, g, b]) * width
    idat = chunk(b'IDAT', zlib.compress(raw))
    iend = chunk(b'IEND', b'')
    return header + ihdr + idat + iend

def main():
    icons_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), 'diskripper-tauri', 'icons')
    os.makedirs(icons_dir, exist_ok=True)
    
    # Blue color #3b82f6
    r, g, b = 59, 130, 246
    
    print(f"Generating placeholder icons in {icons_dir}...")
    
    # Generate PNG icons
    icons = [
        (32, '32x32.png'),
        (128, '128x128.png'),
        (256, '128x128@2x.png'),
        (512, 'icon.png'),
    ]
    
    for size, name in icons:
        path = os.path.join(icons_dir, name)
        with open(path, 'wb') as f:
            f.write(create_png(size, size, r, g, b))
        print(f"  Created {name} ({size}x{size})")
    
    # Create ICO placeholder (not valid ICO, just a copy)
    ico_path = os.path.join(icons_dir, 'icon.ico')
    with open(os.path.join(icons_dir, '32x32.png'), 'rb') as src:
        with open(ico_path, 'wb') as dst:
            dst.write(src.read())
    print("  Created icon.ico (placeholder)")
    
    # Create ICNS placeholder (not valid ICNS, just a copy)
    icns_path = os.path.join(icons_dir, 'icon.icns')
    with open(os.path.join(icons_dir, '128x128.png'), 'rb') as src:
        with open(icns_path, 'wb') as dst:
            dst.write(src.read())
    print("  Created icon.icns (placeholder)")
    
    print("\nDone! Replace with real icons for production.")
    print(f"\nIcons in {icons_dir}:")
    for f in os.listdir(icons_dir):
        size = os.path.getsize(os.path.join(icons_dir, f))
        print(f"  {f} ({size} bytes)")

if __name__ == '__main__':
    main()
