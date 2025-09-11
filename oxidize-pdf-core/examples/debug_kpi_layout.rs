use oxidize_pdf::dashboard::*;
use oxidize_pdf::*;

fn main() -> Result<()> {
    // Simular la posición exacta que recibe la KPI card del dashboard
    let component_position = ComponentPosition::new(102.0, 547.0, 463.0, 120.0);

    let kpi = KpiCard::new("Total Revenue", "$2,547,820")
        .with_trend(12.3, TrendDirection::Up)
        .with_subtitle("vs Q3 2024");

    println!("🔍 DEBUG: Posiciones internas de KPI Card");
    println!(
        "Component position: x={}, y={}, w={}, h={}",
        component_position.x,
        component_position.y,
        component_position.width,
        component_position.height
    );

    // Simular calculate_layout (sin poder llamarlo directamente por ser privado)
    let card_area = component_position.with_padding(8.0);
    println!(
        "Card area after padding: x={}, y={}, w={}, h={}",
        card_area.x, card_area.y, card_area.width, card_area.height
    );

    // Mi nueva lógica de layout (bottom-up)
    let bottom_y = card_area.y;
    let mut current_y = bottom_y;
    let line_height = 16.0;
    let padding = 8.0;

    println!("Starting from bottom_y: {}", bottom_y);

    // Sparkline (bottom)
    current_y += padding;
    let sparkline_y = current_y;
    println!("Sparkline Y: {}", sparkline_y);
    current_y += 20.0; // sparkline height

    // Subtitle
    current_y += padding / 2.0;
    let subtitle_y = current_y;
    println!("Subtitle Y: {}", subtitle_y);
    current_y += line_height;

    // Value (main)
    current_y += padding / 2.0;
    let value_y = current_y;
    println!("Value Y: {}", value_y);
    current_y += 24.0; // value height

    // Title (top)
    current_y += padding / 2.0;
    let title_y = current_y;
    println!("Title Y: {}", title_y);

    // Verificar si están dentro del área
    let top_limit = card_area.y + card_area.height;
    println!("Top limit: {}", top_limit);

    println!("\n🚨 ANÁLISIS:");
    println!(
        "Title Y {} {} dentro del área",
        title_y,
        if title_y <= top_limit {
            "ESTÁ"
        } else {
            "NO ESTÁ"
        }
    );
    println!(
        "Value Y {} {} dentro del área",
        value_y,
        if value_y <= top_limit {
            "ESTÁ"
        } else {
            "NO ESTÁ"
        }
    );
    println!(
        "Subtitle Y {} {} dentro del área",
        subtitle_y,
        if subtitle_y <= top_limit {
            "ESTÁ"
        } else {
            "NO ESTÁ"
        }
    );

    Ok(())
}
