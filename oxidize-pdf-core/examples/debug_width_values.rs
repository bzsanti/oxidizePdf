//! Debug para verificar los valores de ancho que se están generando

use oxidize_pdf::text::fonts::truetype::TrueTypeFont;
use oxidize_pdf::{Document, Result};

fn main() -> Result<()> {
    println!("=== Debug de Valores de Ancho ===\n");

    // Cargar fuente Arial
    let font_path = "/System/Library/Fonts/Supplemental/Arial.ttf";
    let font_data = std::fs::read(font_path)?;

    println!("Analizando fuente: {}", font_path);
    println!("Tamaño de fuente: {} bytes", font_data.len());

    // Parsear la fuente
    let tt_font = TrueTypeFont::parse(font_data)?;
    println!("Units per em: {}", tt_font.units_per_em);

    // Obtener cmap
    let cmap_tables = tt_font.parse_cmap()?;
    let cmap = cmap_tables
        .iter()
        .find(|t| t.platform_id == 3 && t.encoding_id == 1)
        .or_else(|| cmap_tables.iter().find(|t| t.platform_id == 0))
        .expect("No Unicode cmap found");

    // Obtener anchos
    let widths = tt_font.get_glyph_widths(&cmap.mappings)?;

    // Mostrar algunos anchos de ejemplo
    println!("\n--- Anchos de caracteres comunes ---");
    let test_chars = vec![
        ('A', 0x0041),
        ('B', 0x0042),
        ('M', 0x004D),
        ('W', 0x0057),
        ('i', 0x0069),
        ('l', 0x006C),
        ('m', 0x006D),
        ('w', 0x0077),
        (' ', 0x0020),
        ('0', 0x0030),
    ];

    for (ch, unicode) in test_chars {
        if let Some(&width_from_fn) = widths.get(&unicode) {
            // width_from_fn is already scaled to PDF units by get_glyph_widths
            println!(
                "  '{}' (U+{:04X}): scaled_width={:4} (already in PDF units)",
                ch, unicode, width_from_fn
            );
        }
    }

    // Calcular ancho promedio
    let common_chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ";
    let mut total_width = 0;
    let mut count = 0;

    for ch in common_chars.chars() {
        let unicode = ch as u32;
        if let Some(&width) = widths.get(&unicode) {
            // width is already in PDF units
            total_width += width as i64;
            count += 1;
        }
    }

    if count > 0 {
        let avg_width = total_width / count;
        println!("\nAncho promedio (DW): {} unidades PDF", avg_width);
    }

    // Verificar el formato del W array
    println!("\n--- Ejemplo de W array generado ---");
    println!("Para caracteres A-C:");
    let a_width = widths.get(&0x0041).unwrap_or(&0);
    let b_width = widths.get(&0x0042).unwrap_or(&0);
    let c_width = widths.get(&0x0043).unwrap_or(&0);

    // Widths are already in PDF units
    if a_width == b_width && b_width == c_width {
        println!("  W array: [65 67 {}]  (rango)", a_width);
    } else {
        println!(
            "  W array: [65 [{}] 66 [{}] 67 [{}]]  (individuales)",
            a_width, b_width, c_width
        );
    }

    println!("\n✅ Análisis completado");

    Ok(())
}
