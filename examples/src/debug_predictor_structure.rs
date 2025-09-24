fn main() {
    // Los 200 bytes que están causando el problema del predictor
    // Sabemos que empiezan con: [01, 00, 00, 0f, 00, 01, 00, 00, d8, 00, 01, 00, 03, 34, 00, 01, 00, 03, 78, 00]

    let data_size = 200;
    let predictor = 12; // PNG predictor
    let columns = 5;

    println!("🧪 Análisis de estructura del predictor PNG");
    println!("==========================================");
    println!("Datos: {} bytes", data_size);
    println!("Predictor: {} (PNG Up)", predictor);
    println!("Columns: {}", columns);

    // Para PNG predictor, cada fila tiene: columns + 1 byte predictor
    let expected_row_size = columns + 1;
    println!("Tamaño esperado por fila: {} bytes", expected_row_size);

    let remainder = data_size % expected_row_size;
    println!("Resto de división: {} bytes", remainder);

    if remainder != 0 {
        println!("❌ Los datos NO son compatibles con Predictor PNG estándar");
        println!();

        // Intentar diferentes interpretaciones
        println!("🔍 Probando estructuras alternativas:");

        // ¿Tal vez no usa predictor por fila?
        for test_columns in [1, 2, 3, 4, 5, 6, 7, 8, 10, 15, 20, 25, 40, 50] {
            let test_row_size = test_columns + 1;
            if data_size % test_row_size == 0 {
                let num_rows = data_size / test_row_size;
                println!("  ✅ Compatible con {} columnas: {} filas de {} bytes", test_columns, num_rows, test_row_size);
            }
        }

        // ¿Tal vez es directamente sin predictors por fila?
        for test_columns in [5, 10, 20, 25, 40, 50] {
            if data_size % test_columns == 0 {
                let num_rows = data_size / test_columns;
                println!("  ✅ Compatible SIN predictor: {} columnas, {} filas", test_columns, num_rows);
            }
        }
    } else {
        let num_rows = data_size / expected_row_size;
        println!("✅ Los datos SON compatibles: {} filas de {} bytes", num_rows, expected_row_size);
    }

    println!();
    println!("💡 Análisis del problema:");
    println!("1. El stream está marcado con DecodeParms Predictor 12, Columns 5");
    println!("2. Pero los 200 bytes no pueden organizarse en filas de 6 bytes (5+1)");
    println!("3. Posibles causas:");
    println!("   a) Los parámetros DecodeParms son incorrectos");
    println!("   b) Este stream no usa predictor PNG realmente");
    println!("   c) Los datos están truncados o hay un error de parsing");

    // Analizar si los datos tienen estructura de XRef
    println!();
    println!("🔍 ¿Son datos de XRef válidos?");
    if data_size % 5 == 0 {
        let num_entries = data_size / 5;
        println!("✅ Compatible con estructura XRef: {} entradas de 5 bytes", num_entries);
        println!("   Esto sugiere que son datos XRef sin predictor");
    }

    println!();
    println!("🎯 SOLUCIÓN PROPUESTA:");
    println!("   Detectar cuando el predictor PNG falla y usar los datos raw");
    println!("   como XRef entries sin post-procesamiento de predictor");
}