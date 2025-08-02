#!/usr/bin/env python3

# Calculate correct offsets for a minimal PDF
pdf_lines = [
    "%PDF-1.4",
    "1 0 obj",
    "<< /Type /Catalog /Pages 2 0 R >>",
    "endobj",
    "2 0 obj", 
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "endobj",
    "3 0 obj",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    "endobj",
    "4 0 obj",
    "<< /Length 44 >>",
    "stream",
    "BT",
    "/F1 12 Tf", 
    "100 700 Td",
    "(Hello World) Tj",
    "ET",
    "endstream",
    "endobj",
    "5 0 obj",
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    "endobj",
]

# Calculate byte offsets
offset = 0
obj_offsets = {}
for i, line in enumerate(pdf_lines):
    if ' obj' in line and not line.startswith('<<'):
        obj_num = int(line.split()[0])
        obj_offsets[obj_num] = offset
        print(f"Object {obj_num}: offset {offset}")
    offset += len(line) + 1  # +1 for newline

xref_offset = offset
print(f"\nxref table starts at: {xref_offset}")

# Build the complete PDF
pdf_content = '\n'.join(pdf_lines) + '\n'

# Add xref table
xref_lines = [
    "xref",
    "0 6",
    "0000000000 65535 f",
]

# Add entries for objects 1-5
for i in range(1, 6):
    xref_lines.append(f"{obj_offsets[i]:010d} 00000 n")

# Add trailer
xref_lines.extend([
    "trailer",
    "<< /Size 6 /Root 1 0 R >>",
    "startxref",
    str(xref_offset),
    "%%EOF"
])

# Combine everything
full_pdf = pdf_content + '\n'.join(xref_lines)

# Write to file
with open('debug_manual.pdf', 'wb') as f:
    f.write(full_pdf.encode('ascii'))

print(f"\nCreated debug_manual.pdf")
print(f"Total size: {len(full_pdf)} bytes")

# Verify xref entries are correct format
print("\nXref entries:")
for line in xref_lines[2:8]:
    print(f"'{line}' - length: {len(line)}")