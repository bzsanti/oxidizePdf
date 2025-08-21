use std::io::Write;
use flate2::write::ZlibEncoder;
use flate2::Compression;

fn main() {
    // Generate valid PNG test data for different scenarios
    
    // 1. Simple 2x2 RGB image
    generate_rgb_2x2();
    
    // 2. 2x2 RGBA image
    generate_rgba_2x2();
    
    // 3. 2x2 Palette image
    generate_palette_2x2();
    
    // 4. 2x2 RGB 16-bit image
    generate_rgb16_2x2();
}

fn generate_rgb_2x2() {
    println!("// 2x2 RGB PNG:");
    
    // Raw image data: 2x2 pixels, RGB (3 bytes per pixel)
    // Row 1: Red, Green
    // Row 2: Blue, White
    let raw_data = vec![
        0x00, // Filter type for row 1
        0xFF, 0x00, 0x00, // Red pixel
        0x00, 0xFF, 0x00, // Green pixel
        0x00, // Filter type for row 2
        0x00, 0x00, 0xFF, // Blue pixel
        0xFF, 0xFF, 0xFF, // White pixel
    ];
    
    // Compress with zlib
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();
    
    println!("// IDAT compressed data ({} bytes):", compressed.len());
    print_hex(&compressed);
    
    // Calculate CRC-32 for IDAT chunk
    let mut idat_chunk = vec![0x49, 0x44, 0x41, 0x54]; // "IDAT"
    idat_chunk.extend(&compressed);
    let crc = crc32(&idat_chunk);
    println!("// CRC: {:02X} {:02X} {:02X} {:02X}", 
             (crc >> 24) & 0xFF, (crc >> 16) & 0xFF, 
             (crc >> 8) & 0xFF, crc & 0xFF);
}

fn generate_rgba_2x2() {
    println!("\n// 2x2 RGBA PNG:");
    
    // Raw image data: 2x2 pixels, RGBA (4 bytes per pixel)
    let raw_data = vec![
        0x00, // Filter type for row 1
        0xFF, 0x00, 0x00, 0xFF, // Red pixel with alpha
        0x00, 0xFF, 0x00, 0xFF, // Green pixel with alpha
        0x00, // Filter type for row 2
        0x00, 0x00, 0xFF, 0xFF, // Blue pixel with alpha
        0xFF, 0xFF, 0xFF, 0x80, // White pixel with 50% alpha
    ];
    
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();
    
    println!("// IDAT compressed data ({} bytes):", compressed.len());
    print_hex(&compressed);
    
    let mut idat_chunk = vec![0x49, 0x44, 0x41, 0x54];
    idat_chunk.extend(&compressed);
    let crc = crc32(&idat_chunk);
    println!("// CRC: {:02X} {:02X} {:02X} {:02X}", 
             (crc >> 24) & 0xFF, (crc >> 16) & 0xFF, 
             (crc >> 8) & 0xFF, crc & 0xFF);
}

fn generate_palette_2x2() {
    println!("\n// 2x2 Palette PNG:");
    
    // Raw image data: 2x2 pixels, palette indices (1 byte per pixel)
    let raw_data = vec![
        0x00, // Filter type for row 1
        0x00, 0x01, // Palette indices 0, 1
        0x00, // Filter type for row 2
        0x02, 0x03, // Palette indices 2, 3
    ];
    
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();
    
    println!("// IDAT compressed data ({} bytes):", compressed.len());
    print_hex(&compressed);
    
    // Also need PLTE chunk
    let palette = vec![
        0xFF, 0x00, 0x00, // Color 0: Red
        0x00, 0xFF, 0x00, // Color 1: Green
        0x00, 0x00, 0xFF, // Color 2: Blue
        0xFF, 0xFF, 0xFF, // Color 3: White
    ];
    
    println!("// PLTE data:");
    print_hex(&palette);
    
    let mut plte_chunk = vec![0x50, 0x4C, 0x54, 0x45]; // "PLTE"
    plte_chunk.extend(&palette);
    let plte_crc = crc32(&plte_chunk);
    println!("// PLTE CRC: {:02X} {:02X} {:02X} {:02X}", 
             (plte_crc >> 24) & 0xFF, (plte_crc >> 16) & 0xFF, 
             (plte_crc >> 8) & 0xFF, plte_crc & 0xFF);
    
    let mut idat_chunk = vec![0x49, 0x44, 0x41, 0x54];
    idat_chunk.extend(&compressed);
    let crc = crc32(&idat_chunk);
    println!("// IDAT CRC: {:02X} {:02X} {:02X} {:02X}", 
             (crc >> 24) & 0xFF, (crc >> 16) & 0xFF, 
             (crc >> 8) & 0xFF, crc & 0xFF);
}

fn generate_rgb16_2x2() {
    println!("\n// 2x2 RGB 16-bit PNG:");
    
    // Raw image data: 2x2 pixels, RGB 16-bit (6 bytes per pixel)
    let raw_data = vec![
        0x00, // Filter type for row 1
        0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, // Red pixel (16-bit per channel)
        0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, // Green pixel
        0x00, // Filter type for row 2
        0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, // Blue pixel
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // White pixel
    ];
    
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();
    
    println!("// IDAT compressed data ({} bytes):", compressed.len());
    print_hex(&compressed);
    
    let mut idat_chunk = vec![0x49, 0x44, 0x41, 0x54];
    idat_chunk.extend(&compressed);
    let crc = crc32(&idat_chunk);
    println!("// CRC: {:02X} {:02X} {:02X} {:02X}", 
             (crc >> 24) & 0xFF, (crc >> 16) & 0xFF, 
             (crc >> 8) & 0xFF, crc & 0xFF);
}

fn print_hex(data: &[u8]) {
    print!("                ");
    for (i, byte) in data.iter().enumerate() {
        if i > 0 && i % 8 == 0 {
            println!();
            print!("                ");
        }
        print!("0x{:02X}, ", byte);
    }
    println!();
}

// Simple CRC32 implementation for PNG
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFF_u32;
    
    for byte in data {
        crc ^= (*byte as u32) << 24;
        for _ in 0..8 {
            if crc & 0x80000000 != 0 {
                crc = (crc << 1) ^ 0xEDB88320_u32.reverse_bits();
            } else {
                crc <<= 1;
            }
        }
    }
    
    !crc
}