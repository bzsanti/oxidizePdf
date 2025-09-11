//! Debug the XRef stream decoding issue in MADRIDEJOS PDF

use oxidize_pdf::parser::reader::PdfReader;
use oxidize_pdf::parser::ParseOptions;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 DEBUGGING XREF STREAM DECODING");
    println!("=================================");

    let pdf_path = std::path::Path::new("~/Downloads/ocr/MADRIDEJOS_O&M CONTRACT_2013.pdf")
        .expand()
        .expect("Failed to expand path");

    if !pdf_path.exists() {
        println!("❌ MADRIDEJOS PDF not found");
        return Ok(());
    }

    println!("📄 File: {}", pdf_path.display());

    let file = File::open(&pdf_path)?;
    let mut reader = BufReader::new(file);

    // Go to the XRef stream object (311) that we know exists
    println!("\n🔍 Looking for XRef stream object 311...");

    // Search for "311 0 obj"
    reader.seek(SeekFrom::Start(0))?;
    let mut buffer = vec![0u8; 8192];
    let mut found_position: Option<u64> = None;

    loop {
        let position = reader.stream_position()?;
        let bytes_read = reader.read(&mut buffer)?;

        if bytes_read == 0 {
            break;
        }

        let content = String::from_utf8_lossy(&buffer[..bytes_read]);
        if let Some(obj_pos) = content.find("311 0 obj") {
            found_position = Some(position + obj_pos as u64);
            break;
        }

        // Move back a bit to avoid missing boundary matches
        if bytes_read == buffer.len() {
            reader.seek(SeekFrom::Current(-100))?;
        }
    }

    match found_position {
        Some(pos) => {
            println!("✅ Found object 311 at position: {}", pos);

            // Read the object header and dictionary
            reader.seek(SeekFrom::Start(pos))?;
            let mut obj_buffer = vec![0u8; 1024];
            let bytes_read = reader.read(&mut obj_buffer)?;
            obj_buffer.truncate(bytes_read);

            let obj_content = String::from_utf8_lossy(&obj_buffer);
            println!("\n📋 Object 311 content:");
            // Safe substring that respects char boundaries
            let safe_end = obj_content
                .char_indices()
                .nth(400)
                .map(|(i, _)| i)
                .unwrap_or(obj_content.len());
            println!("{}", &obj_content[..safe_end]);

            // Look for stream data
            if let Some(stream_start) = obj_content.find("stream") {
                let stream_start_pos = pos + stream_start as u64 + 6; // Skip "stream"

                // Skip any newlines after "stream"
                reader.seek(SeekFrom::Start(stream_start_pos))?;
                let mut nl_buffer = [0u8; 2];
                reader.read(&mut nl_buffer)?;

                let actual_stream_start = if nl_buffer[0] == b'\r' && nl_buffer[1] == b'\n' {
                    stream_start_pos + 2
                } else if nl_buffer[0] == b'\n' || nl_buffer[0] == b'\r' {
                    stream_start_pos + 1
                } else {
                    stream_start_pos
                };

                reader.seek(SeekFrom::Start(actual_stream_start))?;

                // Read stream data (length is 64 according to the dictionary)
                let mut stream_data = vec![0u8; 64];
                reader.read_exact(&mut stream_data)?;

                println!("\n📊 Stream data (64 bytes):");
                println!("Raw bytes: {:?}", stream_data);
                println!("Hex: {}", format_hex(&stream_data));

                // Try to decode it manually
                println!("\n🔧 Attempting manual decode...");

                // Test the improved FlateDecode function directly
                match try_decode_stream(&stream_data) {
                    Ok(decoded) => {
                        println!("✅ Successfully decoded {} bytes", decoded.len());
                        println!("Decoded hex: {}", format_hex(&decoded));

                        // Now apply Predictor 12 with Columns 4
                        match apply_png_predictor(&decoded, 4) {
                            Ok(final_data) => {
                                println!("✅ Successfully applied PNG predictor");
                                println!("Final data: {:?}", final_data);
                                println!("Final hex: {}", format_hex(&final_data));
                            }
                            Err(e) => {
                                println!("❌ PNG predictor failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Decode failed: {}", e);
                    }
                }
            }
        }
        None => {
            println!("❌ Object 311 not found in file");
        }
    }

    Ok(())
}

fn try_decode_stream(data: &[u8]) -> Result<Vec<u8>, String> {
    use flate2::read::{DeflateDecoder, ZlibDecoder};
    use std::io::Read;

    // Strategy 1: Standard zlib
    if let Ok(result) = try_zlib_decode(data) {
        return Ok(result);
    }

    // Strategy 2: Raw deflate
    if let Ok(result) = try_deflate_decode(data) {
        return Ok(result);
    }

    // Strategy 3: Skip headers
    for skip in 1..=5 {
        if data.len() > skip {
            if let Ok(result) = try_zlib_decode(&data[skip..]) {
                return Ok(result);
            }
            if let Ok(result) = try_deflate_decode(&data[skip..]) {
                return Ok(result);
            }
        }
    }

    Err("All decode strategies failed".to_string())
}

fn try_zlib_decode(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

fn try_deflate_decode(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    let mut decoder = DeflateDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

fn apply_png_predictor(data: &[u8], columns: usize) -> Result<Vec<u8>, String> {
    if data.is_empty() || columns == 0 {
        return Ok(data.to_vec());
    }

    let bytes_per_row = columns + 1; // +1 for predictor byte

    if data.len() % bytes_per_row != 0 {
        return Err(format!(
            "Data length {} not divisible by row length {}",
            data.len(),
            bytes_per_row
        ));
    }

    let num_rows = data.len() / bytes_per_row;
    let mut result = Vec::with_capacity(data.len() - num_rows);
    let mut prev_row: Option<Vec<u8>> = None;

    for row_idx in 0..num_rows {
        let row_start = row_idx * bytes_per_row;
        let predictor = data[row_start];
        let row_data = &data[row_start + 1..row_start + bytes_per_row];

        let decoded_row = match predictor {
            0 => row_data.to_vec(),                                      // No predictor
            1 => apply_sub_predictor(row_data),                          // Sub
            2 => apply_up_predictor(row_data, prev_row.as_deref()),      // Up
            3 => apply_average_predictor(row_data, prev_row.as_deref()), // Average
            4 => apply_paeth_predictor(row_data, prev_row.as_deref()),   // Paeth
            _ => return Err(format!("Unknown PNG predictor: {}", predictor)),
        };

        result.extend_from_slice(&decoded_row);
        prev_row = Some(decoded_row);
    }

    Ok(result)
}

fn apply_sub_predictor(data: &[u8]) -> Vec<u8> {
    let mut result = data.to_vec();
    for i in 1..result.len() {
        result[i] = result[i].wrapping_add(result[i - 1]);
    }
    result
}

fn apply_up_predictor(data: &[u8], prev_row: Option<&[u8]>) -> Vec<u8> {
    if let Some(prev) = prev_row {
        data.iter()
            .zip(prev.iter())
            .map(|(a, b)| a.wrapping_add(*b))
            .collect()
    } else {
        data.to_vec()
    }
}

fn apply_average_predictor(data: &[u8], prev_row: Option<&[u8]>) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());

    for i in 0..data.len() {
        let left = if i > 0 { result[i - 1] } else { 0 };
        let up = prev_row.map(|p| p[i]).unwrap_or(0);
        let average = ((left as u16 + up as u16) / 2) as u8;
        result.push(data[i].wrapping_add(average));
    }

    result
}

fn apply_paeth_predictor(data: &[u8], prev_row: Option<&[u8]>) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());

    for i in 0..data.len() {
        let left = if i > 0 { result[i - 1] } else { 0 };
        let up = prev_row.map(|p| p[i]).unwrap_or(0);
        let up_left = if i > 0 {
            prev_row.map(|p| p[i - 1]).unwrap_or(0)
        } else {
            0
        };

        let paeth = paeth_predictor(left, up, up_left);
        result.push(data[i].wrapping_add(paeth));
    }

    result
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i16 + b as i16 - c as i16;
    let pa = (p - a as i16).abs();
    let pb = (p - b as i16).abs();
    let pc = (p - c as i16).abs();

    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

fn format_hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

trait PathExpansion {
    fn expand(&self) -> std::io::Result<std::path::PathBuf>;
}

impl PathExpansion for std::path::Path {
    fn expand(&self) -> std::io::Result<std::path::PathBuf> {
        if let Some(s) = self.to_str() {
            if s.starts_with("~/") {
                if let Some(home) = std::env::var_os("HOME") {
                    let mut path = std::path::PathBuf::from(home);
                    path.push(&s[2..]);
                    return Ok(path);
                }
            }
        }
        Ok(self.to_path_buf())
    }
}
