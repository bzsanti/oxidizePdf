#!/usr/bin/env python3
"""Generate valid PNG test data for oxidize-pdf tests."""

import base64
from PIL import Image
import io

def generate_1x1_red_png():
    """Generate a 1x1 red PNG."""
    img = Image.new('RGB', (1, 1), color=(255, 0, 0))
    buffer = io.BytesIO()
    img.save(buffer, format='PNG')
    return buffer.getvalue()

def generate_2x2_rgb_png():
    """Generate a 2x2 RGB PNG with different colors."""
    img = Image.new('RGB', (2, 2))
    pixels = img.load()
    pixels[0, 0] = (255, 0, 0)    # Red
    pixels[1, 0] = (0, 255, 0)    # Green
    pixels[0, 1] = (0, 0, 255)    # Blue
    pixels[1, 1] = (255, 255, 0)  # Yellow
    buffer = io.BytesIO()
    img.save(buffer, format='PNG')
    return buffer.getvalue()

def generate_2x2_rgba_png():
    """Generate a 2x2 RGBA PNG with transparency."""
    img = Image.new('RGBA', (2, 2))
    pixels = img.load()
    pixels[0, 0] = (255, 0, 0, 255)    # Red, opaque
    pixels[1, 0] = (0, 255, 0, 192)    # Green, 75% opaque
    pixels[0, 1] = (0, 0, 255, 128)    # Blue, 50% opaque
    pixels[1, 1] = (255, 255, 0, 64)   # Yellow, 25% opaque
    buffer = io.BytesIO()
    img.save(buffer, format='PNG')
    return buffer.getvalue()

def generate_4x4_palette_png():
    """Generate a 4x4 palette PNG."""
    img = Image.new('P', (4, 4))
    # Create a simple palette
    palette = []
    for i in range(256):
        palette.extend([i, 0, 255-i])  # Gradient from red to blue
    img.putpalette(palette)
    
    # Set some pixels
    pixels = img.load()
    for y in range(4):
        for x in range(4):
            pixels[x, y] = (x + y * 4) * 16  # Use different palette indices
    
    buffer = io.BytesIO()
    img.save(buffer, format='PNG')
    return buffer.getvalue()

def bytes_to_rust_vec(data, name):
    """Convert bytes to Rust vec! format."""
    print(f"            // {name}")
    print("            let png_data = vec![")
    for i in range(0, len(data), 16):
        chunk = data[i:i+16]
        hex_bytes = ", ".join(f"0x{b:02X}" for b in chunk)
        if i == 0:
            print(f"                {hex_bytes}, // PNG signature")
        else:
            print(f"                {hex_bytes},")
    print("            ];")
    print()

def main():
    print("Generating valid PNG test data for Rust tests...")
    print()
    
    # Generate 1x1 red PNG
    png_1x1 = generate_1x1_red_png()
    print(f"// 1x1 Red PNG ({len(png_1x1)} bytes)")
    bytes_to_rust_vec(png_1x1, "1x1 Red PNG")
    
    # Generate 2x2 RGB PNG
    png_2x2_rgb = generate_2x2_rgb_png()
    print(f"// 2x2 RGB PNG ({len(png_2x2_rgb)} bytes)")
    bytes_to_rust_vec(png_2x2_rgb, "2x2 RGB PNG")
    
    # Generate 2x2 RGBA PNG
    png_2x2_rgba = generate_2x2_rgba_png()
    print(f"// 2x2 RGBA PNG ({len(png_2x2_rgba)} bytes)")
    bytes_to_rust_vec(png_2x2_rgba, "2x2 RGBA PNG")
    
    # Generate 4x4 Palette PNG
    png_4x4_palette = generate_4x4_palette_png()
    print(f"// 4x4 Palette PNG ({len(png_4x4_palette)} bytes)")
    bytes_to_rust_vec(png_4x4_palette, "4x4 Palette PNG")

if __name__ == "__main__":
    main()