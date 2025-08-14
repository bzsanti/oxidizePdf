#!/usr/bin/env python3
"""Generate valid PNG test data using base64 encoded real PNGs."""

import base64

# Real valid PNGs created with image tools and base64 encoded
VALID_PNGS = {
    "1x1_red": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAHl6u2QAAAABJRU5ErkJggg==",
    "2x2_rgb": "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAFElEQVR42mP8z8DwHwEYBuwGMgAAFQMCAPc9mpcAAAAASUVORK5CYII=",
    "2x2_rgba": "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAFklEQVR42mNgAAL/EIBx4G4gCDA0NAAAM10DAJNi5V0AAAAASUVORK5CYII=",
}

def decode_and_format(name, b64_data):
    """Decode base64 PNG and format as Rust vec."""
    data = base64.b64decode(b64_data)
    print(f"            // {name} - {len(data)} bytes")
    print("            let png_data = vec![")
    
    for i in range(0, len(data), 16):
        chunk = data[i:i+16]
        hex_bytes = ", ".join(f"0x{b:02X}" for b in chunk)
        if i == 0:
            print(f"                {hex_bytes}, // PNG signature")
        elif i < 32:
            print(f"                {hex_bytes}, // IHDR")
        elif len(data) - i <= 16:
            print(f"                {hex_bytes}, // IEND")
        else:
            print(f"                {hex_bytes},")
    print("            ];")
    print()
    return data

def main():
    print("// Valid PNG test data generated from real PNG files")
    print()
    
    for name, b64 in VALID_PNGS.items():
        decode_and_format(name, b64)
    
    # Also create a simple valid 2x2 RGB PNG manually
    print("            // Manual 2x2 RGB PNG")
    print("            let png_data = vec![")
    print("                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature")
    print("                0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk")
    print("                0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, // width=2, height=2")
    print("                0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A, // 8-bit RGB, CRC")
    print("                0x73, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk")
    print("                0x54, 0x78, 0x9C, 0x62, 0x00, 0x00, 0x00, 0x02,")
    print("                0x00, 0x01, 0xE5, 0x27, 0xDE, 0xFC, 0x00, 0x00, // IDAT data + CRC")
    print("                0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, // IEND chunk")
    print("                0x60, 0x82,")
    print("            ];")

if __name__ == "__main__":
    main()