#\!/usr/bin/env python3
"""Test width calculation for characters"""

# For character 'A' (Unicode 65 = 0x0041)
# In W array: "65 66 666" means CIDs 65-66 have width 666
# DW = 556 (default width)

char_A = 0x0041  # Unicode for 'A'
width_A = 666     # From W array
font_size = 24    # Font size in points

# PDF units calculation
# Width in text space = width_A / 1000 * font_size
text_space_width = (width_A / 1000) * font_size
print(f"Character 'A':")
print(f"  Unicode: {char_A} (0x{char_A:04X})")
print(f"  Width from W array: {width_A}")
print(f"  Font size: {font_size} pt")
print(f"  Calculated width in points: {text_space_width:.2f} pt")

# If using 2-byte encoding, each character takes 2 bytes
print(f"\nIn hex string: <{char_A:04X}> (4 hex digits = 2 bytes)")

# Compare with default width
default_width = 556
default_text_width = (default_width / 1000) * font_size
print(f"\nDefault width (DW): {default_width}")
print(f"Default width in points: {default_text_width:.2f} pt")

# If the viewer is using DW instead of W array width
print(f"\nDifference if using DW vs W: {text_space_width - default_text_width:.2f} pt")

# Check if double spacing might be happening
print(f"\nIf spacing is doubled (bug):")
print(f"  Expected: {text_space_width:.2f} pt")
print(f"  Actual (doubled): {text_space_width * 2:.2f} pt")
