use flate2::read::ZlibDecoder;
use std::fs;
use std::io::Read;

fn main() {
    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";
    let pdf_data = fs::read(pdf_path).expect("No se pudo leer el PDF");

    // Analizar ambos XRef streams
    analyze_xref_stream(&pdf_data, 230, "Primer XRef stream");
    analyze_xref_stream(&pdf_data, 929062, "Segundo XRef stream");
}

fn analyze_xref_stream(pdf_data: &[u8], offset: usize, name: &str) {
    println!("\n=== {} (offset {}) ===", name, offset);

    if pdf_data.len() <= offset {
        println!(
            "ERROR: PDF demasiado pequeño para contener XRef en offset {}",
            offset
        );
        return;
    }

    let xref_data = &pdf_data[offset..];

    // Mostrar contexto del diccionario del stream
    let context_end = std::cmp::min(500, xref_data.len());
    let context = &xref_data[..context_end];

    // Buscar el diccionario completo
    if let Some(dict_start) = find_pattern(context, b"<<") {
        if let Some(dict_end) = find_pattern(&context[dict_start..], b">>") {
            let dict_content = &context[dict_start..dict_start + dict_end + 2];

            println!("📖 Diccionario del stream:");
            let dict_str = String::from_utf8_lossy(dict_content).into_owned();
            println!("{}", dict_str);

            // Extraer parámetros específicos
            extract_decode_params(&dict_str);
        }
    }

    // Buscar y decodificar el stream
    if let Some(stream_start) = find_stream_start(xref_data) {
        if let Some(stream_end) = find_stream_end(&xref_data[stream_start..]) {
            let stream_data = &xref_data[stream_start..stream_start + stream_end];
            println!("\n📊 Stream comprimido: {} bytes", stream_data.len());
            println!(
                "Primeros 20 bytes: {:02x?}",
                &stream_data[..std::cmp::min(20, stream_data.len())]
            );

            // Intentar decodificar
            println!("\n🔧 Intentando decodificar...");
            match try_standard_zlib_decode(stream_data) {
                Ok(decoded) => {
                    println!("✅ Decodificación EXITOSA: {} bytes", decoded.len());

                    // Analizar el contenido decodificado para XRef
                    analyze_xref_content(&decoded);

                    // Simular aplicación de Predictor 12
                    simulate_predictor_12(&decoded);
                }
                Err(e) => println!("❌ Decodificación falló: {}", e),
            }
        } else {
            println!("❌ No se encontró 'endstream'");
        }
    } else {
        println!("❌ No se encontró 'stream'");
    }
}

fn extract_decode_params(dict_str: &str) {
    println!("\n🔍 Parámetros de decodificación:");

    // Buscar DecodeParms
    if dict_str.contains("/DecodeParms") {
        println!("✅ Tiene DecodeParms");

        if dict_str.contains("/Predictor") {
            // Extraer valor del predictor
            if let Some(start) = dict_str.find("/Predictor") {
                let after_predictor = &dict_str[start + "/Predictor".len()..];
                if let Some(number_start) = after_predictor.find(char::is_numeric) {
                    let number_part = &after_predictor[number_start..];
                    if let Some(number_end) = number_part.find(|c: char| !c.is_numeric()) {
                        let predictor_value = &number_part[..number_end];
                        println!("✅ Predictor: {}", predictor_value);
                    }
                }
            }
        }

        if dict_str.contains("/Columns") {
            if let Some(start) = dict_str.find("/Columns") {
                let after_columns = &dict_str[start + "/Columns".len()..];
                if let Some(number_start) = after_columns.find(char::is_numeric) {
                    let number_part = &after_columns[number_start..];
                    if let Some(number_end) = number_part.find(|c: char| !c.is_numeric()) {
                        let columns_value = &number_part[..number_end];
                        println!("✅ Columns: {}", columns_value);
                    }
                }
            }
        }
    } else {
        println!("❌ No tiene DecodeParms");
    }
}

fn analyze_xref_content(data: &[u8]) {
    println!("\n📋 Análisis del contenido XRef decodificado:");

    // Los XRef streams contienen datos binarios estructurados
    // con entradas para cada objeto referenciado

    if data.len() < 5 {
        println!("❌ Contenido muy pequeño para ser XRef válido");
        return;
    }

    println!("Primeros 50 bytes como hex:");
    for (i, chunk) in data[..std::cmp::min(50, data.len())].chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for &byte in chunk {
            print!("{:02x} ", byte);
        }
        println!();
    }

    // Intentar interpretar como entradas XRef
    // Formato típico: [tipo 1 byte][campo1 3 bytes][campo2 1 byte] = 5 bytes por entrada
    if data.len() % 5 == 0 {
        let entries = data.len() / 5;
        println!(
            "✅ Posible estructura XRef: {} entradas de 5 bytes cada una",
            entries
        );

        // Mostrar algunas entradas
        for i in 0..std::cmp::min(entries, 10) {
            let entry = &data[i * 5..(i + 1) * 5];
            let tipo = entry[0];
            let campo1 = u32::from_be_bytes([0, entry[1], entry[2], entry[3]]);
            let campo2 = entry[4];

            println!(
                "  Entrada {}: tipo={}, campo1={}, campo2={}",
                i, tipo, campo1, campo2
            );
        }
    } else {
        println!("❌ No tiene estructura XRef estándar de 5 bytes por entrada");
    }
}

fn simulate_predictor_12(data: &[u8]) {
    println!("\n🧪 Simulando aplicación de Predictor 12...");

    // Predictor 12 es PNG predictor para 5 columnas
    let columns = 5;
    let row_size = columns + 1; // +1 para el byte predictor

    if data.len() % row_size != 0 {
        println!("❌ Los datos NO son compatibles con Predictor 12");
        println!(
            "   Tamaño: {} bytes, esperado múltiplo de {} (columnas + 1)",
            data.len(),
            row_size
        );
        println!("   Resto: {} bytes", data.len() % row_size);
        return;
    }

    println!("✅ Los datos SON compatibles con Predictor 12");
    let num_rows = data.len() / row_size;
    println!("   {} filas de {} bytes cada una", num_rows, row_size);

    // Analizar bytes predictores
    let mut predictor_counts = std::collections::HashMap::new();
    for row in 0..std::cmp::min(num_rows, 20) {
        let predictor_byte = data[row * row_size];
        *predictor_counts.entry(predictor_byte).or_insert(0) += 1;
    }

    println!("   Bytes predictores encontrados: {:?}", predictor_counts);

    // PNG predictors válidos son 0-4
    let valid_predictors: Vec<_> = predictor_counts.keys().filter(|&&b| b <= 4).collect();

    if !valid_predictors.is_empty() {
        println!(
            "✅ Contiene predictores PNG válidos: {:?}",
            valid_predictors
        );
    } else {
        println!("❌ NO contiene predictores PNG válidos (0-4)");
    }
}

fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    (0..data.len().saturating_sub(pattern.len())).find(|&i| &data[i..i + pattern.len()] == pattern)
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
    (0..data.len().saturating_sub(endstream_marker.len()))
        .find(|&i| &data[i..i + endstream_marker.len()] == endstream_marker)
}

fn try_standard_zlib_decode(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}
