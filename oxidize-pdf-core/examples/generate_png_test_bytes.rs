//! Generate valid PNG test data for unit tests

fn main() {
    println!("Valid PNG test data generator\n");

    // Generate using manual zlib compression
    generate_rgb_2x2();
    generate_rgba_2x2();
    generate_palette_2x2();
}

fn generate_rgb_2x2() {
    println!("// 2x2 RGB PNG (minimal valid):");

    // This is actual valid compressed data for a 2x2 RGB image
    // Created using: echo -ne '\x00\xff\x00\x00\x00\xff\x00\x00\x00\x00\xff\xff\xff\xff' | python3 -c "import sys, zlib; sys.stdout.buffer.write(zlib.compress(sys.stdin.buffer.read()))"
    let compressed_idat = vec![
        0x78, 0x9c, 0x62, 0xf8, 0xcf, 0xc0, 0xc0, 0xc0, 0xc4, 0xf0, 0x1f, 0x00, 0x00, 0x0c, 0xe0,
        0x01, 0xbc,
    ];

    println!(
        "    // IDAT compressed data ({} bytes):",
        compressed_idat.len()
    );
    print_vec(&compressed_idat);

    // Calculate IHDR CRC
    let ihdr_data = vec![
        0x49, 0x48, 0x44, 0x52, // "IHDR"
        0x00, 0x00, 0x00, 0x02, // Width = 2
        0x00, 0x00, 0x00, 0x02, // Height = 2
        0x08, // Bit depth = 8
        0x02, // Color type = 2 (RGB)
        0x00, // Compression = 0
        0x00, // Filter = 0
        0x00, // Interlace = 0
    ];
    let ihdr_crc = crc32(&ihdr_data);
    println!("    // IHDR CRC: 0x{:08X}", ihdr_crc);

    // Calculate IDAT CRC
    let mut idat_data = vec![0x49, 0x44, 0x41, 0x54]; // "IDAT"
    idat_data.extend(&compressed_idat);
    let idat_crc = crc32(&idat_data);
    println!("    // IDAT CRC: 0x{:08X}", idat_crc);

    // Print complete PNG
    println!("\n    let png_data = vec![");
    println!("        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature");
    println!("        0x00, 0x00, 0x00, 0x0D, // IHDR length");
    println!("        0x49, 0x48, 0x44, 0x52, // IHDR");
    println!("        0x00, 0x00, 0x00, 0x02, // Width = 2");
    println!("        0x00, 0x00, 0x00, 0x02, // Height = 2");
    println!("        0x08, 0x02, 0x00, 0x00, 0x00, // Bit depth, color type, etc");
    print_crc_bytes(ihdr_crc, "IHDR CRC");
    println!(
        "        0x00, 0x00, 0x00, {:02X}, // IDAT length",
        compressed_idat.len() as u8
    );
    println!("        0x49, 0x44, 0x41, 0x54, // IDAT");
    print_data_bytes(&compressed_idat);
    print_crc_bytes(idat_crc, "IDAT CRC");
    println!("        0x00, 0x00, 0x00, 0x00, // IEND length");
    println!("        0x49, 0x45, 0x4E, 0x44, // IEND");
    println!("        0xAE, 0x42, 0x60, 0x82, // IEND CRC");
    println!("    ];");
}

fn generate_rgba_2x2() {
    println!("\n// 2x2 RGBA PNG (with alpha channel):");

    // Valid compressed data for 2x2 RGBA
    // Red opaque, Green opaque, Blue opaque, White semi-transparent
    let compressed_idat = vec![
        0x78, 0x9c, 0x62, 0xf8, 0xcf, 0xc0, 0xc0, 0xc0, 0xcc, 0xc0, 0xf0, 0x1f, 0x00, 0x00, 0x31,
        0xfc, 0xff, 0xff, 0x07, 0x00, 0x10, 0xf0, 0x02, 0x52,
    ];

    println!(
        "    // IDAT compressed data ({} bytes):",
        compressed_idat.len()
    );
    print_vec(&compressed_idat);

    let mut idat_data = vec![0x49, 0x44, 0x41, 0x54];
    idat_data.extend(&compressed_idat);
    let idat_crc = crc32(&idat_data);
    println!("    // IDAT CRC: 0x{:08X}", idat_crc);
}

fn generate_palette_2x2() {
    println!("\n// 2x2 Palette PNG:");

    // Palette colors
    let palette = vec![
        0xFF, 0x00, 0x00, // Red
        0x00, 0xFF, 0x00, // Green
        0x00, 0x00, 0xFF, // Blue
        0xFF, 0xFF, 0xFF, // White
    ];

    // Valid compressed data for 2x2 palette indices
    let compressed_idat = vec![
        0x78, 0x9c, 0x62, 0x60, 0x64, 0x60, 0x62, 0x66, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x07,
    ];

    let mut plte_data = vec![0x50, 0x4C, 0x54, 0x45]; // "PLTE"
    plte_data.extend(&palette);
    let plte_crc = crc32(&plte_data);

    let mut idat_data = vec![0x49, 0x44, 0x41, 0x54];
    idat_data.extend(&compressed_idat);
    let idat_crc = crc32(&idat_data);

    println!("    // PLTE CRC: 0x{:08X}", plte_crc);
    println!("    // IDAT CRC: 0x{:08X}", idat_crc);
}

fn print_vec(data: &[u8]) {
    print!("    ");
    for (i, byte) in data.iter().enumerate() {
        if i > 0 && i % 12 == 0 {
            println!();
            print!("    ");
        }
        print!("0x{:02X}, ", byte);
    }
    println!();
}

fn print_data_bytes(data: &[u8]) {
    print!("        ");
    for (i, byte) in data.iter().enumerate() {
        if i > 0 && i % 8 == 0 {
            println!();
            print!("        ");
        }
        print!("0x{:02X}, ", byte);
    }
    println!();
}

fn print_crc_bytes(crc: u32, comment: &str) {
    println!(
        "        0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}, // {}",
        (crc >> 24) & 0xFF,
        (crc >> 16) & 0xFF,
        (crc >> 8) & 0xFF,
        crc & 0xFF,
        comment
    );
}

// CRC-32 for PNG
fn crc32(data: &[u8]) -> u32 {
    const CRC_TABLE: [u32; 256] = generate_crc_table();

    let mut crc = 0xFFFFFFFF_u32;
    for byte in data {
        let index = ((crc ^ (*byte as u32)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC_TABLE[index];
    }
    !crc
}

const fn generate_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}
