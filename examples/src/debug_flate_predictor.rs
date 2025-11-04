use flate2::read::ZlibDecoder;
use std::fs;
use std::io::Read;

fn main() {
    // Lee el PDF problemático
    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";
    let pdf_data = fs::read(pdf_path).expect("No se pudo leer el PDF");

    // Buscar el XRef stream en offset 0x44acc (281,292)
    let xref_offset = 0x44acc;
    if pdf_data.len() <= xref_offset {
        println!(
            "ERROR: PDF demasiado pequeño para contener XRef en offset {}",
            xref_offset
        );
        return;
    }

    // Leer desde el offset hasta encontrar el stream
    let xref_data = &pdf_data[xref_offset..];
    println!(
        "Buscando XRef stream en offset 0x{:x} ({})",
        xref_offset, xref_offset
    );

    // Mostrar contexto del objeto
    let context = &xref_data[..std::cmp::min(200, xref_data.len())];
    println!("Contexto del objeto (primeros 200 bytes):");
    for (i, chunk) in context.chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for &byte in chunk {
            if byte.is_ascii_graphic() || byte == b' ' {
                print!("{} ", byte as char);
            } else {
                print!("{:02x} ", byte);
            }
        }
        println!();
    }

    // Buscar el inicio del stream después de la línea "stream"
    if let Some(stream_start) = find_stream_start(xref_data) {
        println!(
            "\n✅ Encontrado inicio de stream en offset relativo: {}",
            stream_start
        );

        // Buscar el final del stream
        if let Some(stream_end) = find_stream_end(&xref_data[stream_start..]) {
            println!(
                "✅ Encontrado final de stream en offset relativo: {}",
                stream_start + stream_end
            );

            let stream_data = &xref_data[stream_start..stream_start + stream_end];
            println!("Tamaño del stream comprimido: {} bytes", stream_data.len());

            // Mostrar los primeros bytes del stream
            println!(
                "Primeros 20 bytes del stream: {:02x?}",
                &stream_data[..std::cmp::min(20, stream_data.len())]
            );

            // Intentar decodificar con zlib estándar
            println!("\n--- Intentando decodificación zlib estándar ---");
            match try_standard_zlib_decode(stream_data) {
                Ok(decoded) => {
                    println!(
                        "✅ Zlib estándar EXITOSO! Tamaño decodificado: {} bytes",
                        decoded.len()
                    );
                    println!(
                        "Primeros 50 bytes decodificados: {:02x?}",
                        &decoded[..std::cmp::min(50, decoded.len())]
                    );

                    // Analizar si tiene formato de XRef stream esperado
                    analyze_xref_stream(&decoded);
                }
                Err(e) => println!("❌ Zlib estándar falló: {}", e),
            }

            // Intentar decodificación raw deflate
            println!("\n--- Intentando decodificación raw deflate ---");
            match try_raw_deflate_decode(stream_data) {
                Ok(decoded) => {
                    println!(
                        "✅ Raw deflate EXITOSO! Tamaño decodificado: {} bytes",
                        decoded.len()
                    );
                    println!(
                        "Primeros 50 bytes decodificados: {:02x?}",
                        &decoded[..std::cmp::min(50, decoded.len())]
                    );
                }
                Err(e) => println!("❌ Raw deflate falló: {}", e),
            }
        } else {
            println!("❌ ERROR: No se encontró el final del stream (endstream)");
        }
    } else {
        println!("❌ ERROR: No se encontró el inicio del stream");
    }
}

fn find_stream_start(data: &[u8]) -> Option<usize> {
    // Buscar "stream\n" o "stream\r\n"
    let stream_marker = b"stream";

    for i in 0..data.len().saturating_sub(stream_marker.len()) {
        if &data[i..i + stream_marker.len()] == stream_marker {
            // Verificar que después viene \n o \r\n
            let mut pos = i + stream_marker.len();
            if pos < data.len() && data[pos] == b'\r' {
                pos += 1;
            }
            if pos < data.len() && data[pos] == b'\n' {
                return Some(pos + 1);
            }
        }
    }
    None
}

fn find_stream_end(data: &[u8]) -> Option<usize> {
    // Buscar "endstream"
    let endstream_marker = b"endstream";

    (0..data.len().saturating_sub(endstream_marker.len())).find(|&i| &data[i..i + endstream_marker.len()] == endstream_marker)
}

fn try_standard_zlib_decode(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

fn try_raw_deflate_decode(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    use flate2::read::DeflateDecoder;
    let mut decoder = DeflateDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

fn analyze_xref_stream(data: &[u8]) {
    println!("\n--- Análisis del XRef stream decodificado ---");

    if data.len() < 10 {
        println!("Stream muy pequeño para ser XRef válido");
        return;
    }

    println!("Tamaño total: {} bytes", data.len());

    // Los XRef streams con Predictor 12 tienen estructura de filas
    // Intentar detectar el patrón del predictor analizando posibles tamaños de fila
    for row_size in [5, 6, 7, 8, 9, 10, 11, 12, 15, 20] {
        if data.len() % row_size == 0 {
            println!(
                "\n🔍 Probando tamaño de fila: {} (total {} filas)",
                row_size,
                data.len() / row_size
            );

            let mut predictor_counts = std::collections::HashMap::new();
            let num_rows = std::cmp::min(data.len() / row_size, 20); // Solo analizar las primeras 20 filas

            for row in 0..num_rows {
                let predictor_byte = data[row * row_size];
                *predictor_counts.entry(predictor_byte).or_insert(0) += 1;
            }

            println!("  Bytes predictores encontrados: {:?}", predictor_counts);

            // PNG predictors válidos son 0, 1, 2, 3, 4
            let valid_predictors: Vec<_> = predictor_counts.keys().filter(|&&b| b <= 4).collect();

            if !valid_predictors.is_empty() {
                println!(
                    "  ✅ Contiene predictores PNG válidos (0-4): {:?}",
                    valid_predictors
                );
            }
        }
    }
}
