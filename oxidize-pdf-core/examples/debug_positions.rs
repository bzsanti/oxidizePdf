use oxidize_pdf::dashboard::*;
use oxidize_pdf::*;

fn main() -> Result<()> {
    let card = KpiCard::new("Test", "$1,000").with_trend(5.0, TrendDirection::Up);

    // Simular área de una KPI card típica
    let position = ComponentPosition::new(50.0, 400.0, 120.0, 80.0);

    println!("🔍 DEBUG: Posición de KPI Card");
    println!(
        "Position: x={}, y={}, w={}, h={}",
        position.x, position.y, position.width, position.height
    );

    // Llamar al método calculate_layout usando reflection/hack
    // Como es privado, vamos a crear un simple test manual

    let card_area = position.with_padding(8.0);
    println!(
        "Card area after padding: x={}, y={}, w={}, h={}",
        card_area.x, card_area.y, card_area.width, card_area.height
    );

    // Replicar la lógica de calculate_layout
    let top_y = card_area.y + card_area.height;
    let mut current_y = top_y;
    let line_height = 16.0;
    let padding = 8.0;

    println!("Top Y: {}", top_y);
    println!("Initial current_y: {}", current_y);

    // Title area calculation
    current_y -= padding;
    let title_y = current_y - line_height;
    println!("Title Y position: {}", title_y);
    current_y -= line_height;

    // Value area calculation
    current_y -= padding / 2.0;
    let value_height = 24.0;
    let value_y = current_y - value_height;
    println!("Value Y position: {}", value_y);

    println!("🚨 PROBLEMA: Si card_area.y={} y calculamos hacia abajo, las posiciones Y son demasiado altas", card_area.y);
    println!(
        "💡 SOLUCIÓN: Necesitamos calcular desde abajo hacia arriba en lugar de arriba hacia abajo"
    );

    Ok(())
}
