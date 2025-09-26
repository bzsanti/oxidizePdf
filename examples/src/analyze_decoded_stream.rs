use flate2::read::ZlibDecoder;
use std::fs;
use std::io::Read;

fn main() {
    // Lee el PDF problemático
    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";
    let pdf_data = fs::read(pdf_path).expect("No se pudo leer el PDF");

    // Buscar el XRef stream en offset 0x44acc (281,292)
    let xref_offset = 0x44acc;
    let xref_data = &pdf_data[xref_offset..];

    if let Some(stream_start) = find_stream_start(xref_data) {
        if let Some(stream_end) = find_stream_end(&xref_data[stream_start..]) {
            let stream_data = &xref_data[stream_start..stream_start + stream_end];

            // Decodificar con zlib estándar
            if let Ok(decoded) = try_standard_zlib_decode(stream_data) {
                println!("Stream decodificado exitosamente: {} bytes", decoded.len());

                // Convertir a string para ver si es texto
                if let Ok(text) = String::from_utf8(decoded.clone()) {
                    println!("\n🎉 EL STREAM ES TEXTO LEGIBLE:");
                    println!("{}", &text[..std::cmp::min(500, text.len())]);
                    if text.len() > 500 {
                        println!("\n[...truncado, total {} chars]", text.len());
                        println!("\nÚltimos 200 chars:");
                        println!("{}", &text[text.len().saturating_sub(200)..]);
                    }
                } else {
                    println!("\n🔍 El stream contiene datos binarios:");
                    analyze_binary_data(&decoded);
                }

                // Analizar para entender por qué oxidize-pdf falla con Predictor 12
                analyze_predictor_issue(&decoded);
            }
        }
    }
}

fn analyze_binary_data(data: &[u8]) {
    println!("Primeros 100 bytes como hex:");
    for (i, chunk) in data[..std::cmp::min(100, data.len())]
        .chunks(16)
        .enumerate()
    {
        print!("{:04x}: ", i * 16);
        for &byte in chunk {
            print!("{:02x} ", byte);
        }
        print!(" | ");
        for &byte in chunk {
            if byte.is_ascii_graphic() || byte == b' ' {
                print!("{}", byte as char);
            } else {
                print!(".");
            }
        }
        println!();
    }
}

fn analyze_predictor_issue(data: &[u8]) {
    println!("\n--- Análisis del problema con Predictor 12 ---");

    // Este stream fue decodificado exitosamente con zlib estándar
    // El problema debe estar en que oxidize-pdf intenta aplicar predictor DESPUÉS
    // de la decompresión, pero este stream puede que NO use predictor.

    println!("✅ El stream zlib se decodifica perfectamente");
    println!("❌ Pero oxidize-pdf falla con 'All FlateDecode strategies failed'");

    // Ver si contiene el objeto XRef esperado
    if let Ok(text) = String::from_utf8(data.to_vec()) {
        if text.contains("obj") || text.contains("xref") || text.contains("trailer") {
            println!("🔍 El stream contiene definiciones de objetos PDF típicas");
        } else {
            println!("🤔 El stream no parece contener objetos PDF estándar");
        }
    }

    // El problema podría ser:
    // 1. oxidize-pdf asume que SIEMPRE debe aplicar el predictor si está especificado
    // 2. Pero este stream específico puede que esté mal etiquetado o no use predictor realmente
    // 3. O el parámetro Predictor 12 está mal interpretado

    println!("\n🔍 Hipótesis del problema:");
    println!("1. El stream se decodifica perfectamente con zlib estándar");
    println!("2. oxidize-pdf ve 'Predictor 12' en el diccionario del stream");
    println!("3. oxidize-pdf intenta aplicar PNG predictor DESPUÉS de decompresión");
    println!("4. Pero este stream NO usa predictor, o usa una configuración diferente");
    println!("5. Por eso falla el post-procesamiento del predictor");
}

fn find_stream_start(data: &[u8]) -> Option<usize> {
    let stream_marker = b"stream";
    for i in 0..data.len().saturating_sub(stream_marker.len()) {
        if &data[i..i + stream_marker.len()] == stream_marker {
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
    let endstream_marker = b"endstream";
    for i in 0..data.len().saturating_sub(endstream_marker.len()) {
        if &data[i..i + endstream_marker.len()] == endstream_marker {
            return Some(i);
        }
    }
    None
}

fn try_standard_zlib_decode(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}
