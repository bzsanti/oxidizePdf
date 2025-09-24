use std::fs;

fn main() {
    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";
    let pdf_data = fs::read(pdf_path).expect("No se pudo leer el PDF");

    println!("🧩 Diagnóstico detallado de la confusión de XRef");
    println!("===============================================");

    // Analizar ambos XRef streams que sabemos que existen
    analyze_xref_position(&pdf_data, 230, "Primer XRef stream (offset 230)");
    analyze_xref_position(&pdf_data, 929062, "Segundo XRef stream (offset 929062)");

    // También el objeto que oxidize-pdf está intentando procesar
    analyze_problematic_object(&pdf_data);
}

fn analyze_xref_position(pdf_data: &[u8], offset: usize, name: &str) {
    println!("\n--- {} ---", name);

    if pdf_data.len() <= offset {
        println!("❌ Offset fuera de rango");
        return;
    }

    let context = &pdf_data[offset..std::cmp::min(offset + 1000, pdf_data.len())];

    // Buscar el diccionario del objeto
    if let Some(dict_start) = find_pattern(context, b"<<") {
        if let Some(dict_end) = find_pattern(&context[dict_start..], b">>") {
            let dict_bytes = &context[dict_start..dict_start + dict_end + 2];
            let dict_str = String::from_utf8_lossy(dict_bytes);
            println!("📖 Diccionario: {}", dict_str.trim());

            // Extraer información clave
            let has_filter = dict_str.contains("/Filter");
            let has_flate = dict_str.contains("FlateDecode");
            let has_predictor = dict_str.contains("/Predictor");
            let has_columns = dict_str.contains("/Columns");

            println!("   Filter: {} | FlateDecode: {} | Predictor: {} | Columns: {}",
                     has_filter, has_flate, has_predictor, has_columns);

            // Extraer longitud del stream si está disponible
            if let Some(length_start) = dict_str.find("/Length") {
                let after_length = &dict_str[length_start + "/Length".len()..];
                if let Some(number_match) = after_length.chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<usize>()
                    .ok() {
                        println!("   Stream Length: {} bytes", number_match);

                        // Encontrar el stream real
                        if let Some(stream_start) = find_stream_start(context) {
                            if stream_start + number_match <= context.len() {
                                let stream_data = &context[stream_start..stream_start + number_match];
                                println!("   Stream encontrado: {} bytes", stream_data.len());
                                println!("   Primeros 20 bytes: {:02x?}", &stream_data[..std::cmp::min(20, stream_data.len())]);
                            } else {
                                println!("   ❌ Stream truncado en el contexto");
                            }
                        } else {
                            println!("   ❌ No se encontró 'stream'");
                        }
                    }
            }
        }
    }
}

fn analyze_problematic_object(pdf_data: &[u8]) {
    println!("\n--- Análisis del objeto problemático ---");
    println!("¿De dónde vienen exactamente esos 200 bytes?");

    // Sabemos que oxidize-pdf lee 200 bytes que no pueden decodificar con zlib
    // Estos deben ser datos que oxidize-pdf lee desde algún offset específico

    // Los 200 bytes que fallan empiezan con: [01, 00, 00, 0f, 00, 01, 00, 00, d8, 00...]
    let target_pattern = [0x01, 0x00, 0x00, 0x0f, 0x00, 0x01, 0x00, 0x00, 0xd8, 0x00];

    println!("🔍 Buscando patrón de 200 bytes problemático: {:02x?}", &target_pattern);

    for (i, window) in pdf_data.windows(target_pattern.len()).enumerate() {
        if window == target_pattern {
            println!("✅ Encontrado patrón en offset: 0x{:x} ({})", i, i);

            // Mostrar contexto antes del patrón para entender qué objeto es
            let context_start = i.saturating_sub(200);
            let context_end = std::cmp::min(i + 300, pdf_data.len());
            let context = &pdf_data[context_start..context_end];

            // Buscar el objeto que contiene este stream
            let context_str = String::from_utf8_lossy(context);

            if let Some(obj_match) = context_str.rfind(" obj") {
                let before_obj = &context_str[..obj_match];
                if let Some(num_start) = before_obj.rfind(char::is_numeric) {
                    let before_num = &before_obj[..num_start + 1];
                    if let Some(space_pos) = before_num.rfind(' ') {
                        let obj_spec = &before_obj[space_pos + 1..];
                        println!("📍 Encontrado dentro del objeto: {}", obj_spec);
                    }
                }
            }

            // Mostrar diccionario si existe
            if let Some(dict_start) = context_str.rfind("<<") {
                if let Some(dict_end) = context_str[dict_start..].find(">>") {
                    let dict = &context_str[dict_start..dict_start + dict_end + 2];
                    println!("📖 Diccionario del objeto: {}", dict);
                }
            }

            break;
        }
    }
}

fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    for i in 0..data.len().saturating_sub(pattern.len()) {
        if &data[i..i + pattern.len()] == pattern {
            return Some(i);
        }
    }
    None
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