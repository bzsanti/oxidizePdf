//! Advanced tables demonstration
//!
//! This example showcases the advanced table features including:
//! - Different table styles
//! - Cell spanning
//! - Custom borders and padding
//! - Alternating row colors
//! - Header and footer rows
//! - Different column widths

use oxidize_pdf::{
    AdvancedTable, AdvancedTableCell, AdvancedTableOptions, AlternatingRowColors, CellPadding,
    Color, ColumnDefinition, ColumnWidth, Document, Font, Page, PageTables, Result,
    TableBorderStyle, TableRow, TableStyle, TextAlign, VerticalAlign,
};

fn main() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Advanced Tables Demo");
    doc.set_author("oxidize-pdf");

    // Page 1: Simple styled tables
    create_styled_tables_page(&mut doc)?;

    // Page 2: Advanced table features
    create_advanced_features_page(&mut doc)?;

    // Page 3: Financial report example
    create_financial_report_page(&mut doc)?;

    // Page 4: Product catalog example
    create_product_catalog_page(&mut doc)?;

    doc.save("advanced_tables_demo.pdf")?;
    println!("✅ Created advanced_tables_demo.pdf");

    Ok(())
}

fn create_styled_tables_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Table Styles Demonstration")?;

    // Minimal style table
    let headers = vec!["Name".to_string(), "Age".to_string(), "City".to_string()];
    let data = vec![
        vec![
            "Alice Johnson".to_string(),
            "28".to_string(),
            "New York".to_string(),
        ],
        vec![
            "Bob Smith".to_string(),
            "35".to_string(),
            "London".to_string(),
        ],
        vec![
            "Charlie Brown".to_string(),
            "42".to_string(),
            "Tokyo".to_string(),
        ],
    ];

    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 680.0)
        .write("Minimal Style:")?;

    page.add_styled_table(
        headers.clone(),
        data.clone(),
        50.0,
        650.0,
        500.0,
        TableStyle::minimal(),
    )?;

    // Simple style table
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 500.0)
        .write("Simple Style:")?;

    page.add_styled_table(
        headers.clone(),
        data.clone(),
        50.0,
        470.0,
        500.0,
        TableStyle::simple(),
    )?;

    // Professional style table
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 320.0)
        .write("Professional Style:")?;

    page.add_styled_table(
        headers.clone(),
        data.clone(),
        50.0,
        290.0,
        500.0,
        TableStyle::professional(),
    )?;

    // Colorful style table
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 140.0)
        .write("Colorful Style:")?;

    page.add_styled_table(headers, data, 50.0, 110.0, 500.0, TableStyle::colorful())?;

    doc.add_page(page);
    Ok(())
}

fn create_advanced_features_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Advanced Table Features")?;

    // Create a table with mixed column widths
    let columns = vec![
        ColumnDefinition {
            width: ColumnWidth::Fixed(100.0),
            default_align: TextAlign::Left,
            min_width: None,
            max_width: None,
        },
        ColumnDefinition {
            width: ColumnWidth::Relative(0.4),
            default_align: TextAlign::Center,
            min_width: None,
            max_width: None,
        },
        ColumnDefinition {
            width: ColumnWidth::Relative(0.6),
            default_align: TextAlign::Right,
            min_width: None,
            max_width: None,
        },
    ];

    let mut table = AdvancedTable::new(columns);

    // Configure table options
    let mut options = AdvancedTableOptions::default();
    options.border_style = TableBorderStyle::default();
    options.cell_padding = CellPadding::symmetric(10.0, 8.0);
    options.font_size = 11.0;
    options.alternating_rows = Some(AlternatingRowColors {
        even_color: Color::gray(0.95),
        odd_color: Color::white(),
        include_header: false,
    });
    table.set_options(options);

    // Add header with custom styling
    let header_cells = vec![
        AdvancedTableCell::text("Feature".to_string())
            .with_background(Color::rgb(0.2, 0.3, 0.6))
            .with_text_color(Color::white())
            .with_font(Font::HelveticaBold, 12.0),
        AdvancedTableCell::text("Description".to_string())
            .with_background(Color::rgb(0.2, 0.3, 0.6))
            .with_text_color(Color::white())
            .with_font(Font::HelveticaBold, 12.0),
        AdvancedTableCell::text("Status".to_string())
            .with_background(Color::rgb(0.2, 0.3, 0.6))
            .with_text_color(Color::white())
            .with_font(Font::HelveticaBold, 12.0),
    ];
    table.add_header_row(header_cells)?;

    // Add data rows with various features
    table.add_text_row(vec![
        "Cell Spanning".to_string(),
        "Cells can span multiple columns".to_string(),
        "✓ Implemented".to_string(),
    ])?;

    // Row with column spanning
    let spanning_row = TableRow::new(vec![
        AdvancedTableCell::text("This cell spans two columns".to_string())
            .with_colspan(2)
            .with_background(Color::rgb(0.9, 0.95, 1.0))
            .with_align(TextAlign::Center),
        AdvancedTableCell::text("Normal cell".to_string()),
    ]);
    table.add_row(spanning_row)?;

    // Row with custom borders
    let custom_border_row = TableRow::new(vec![
        AdvancedTableCell::text("Custom Borders".to_string()),
        AdvancedTableCell::text("Different border styles per cell".to_string())
            .with_padding(CellPadding::uniform(15.0)),
        AdvancedTableCell::text("✓ Available".to_string())
            .with_text_color(Color::rgb(0.0, 0.6, 0.0)),
    ]);
    table.add_row(custom_border_row)?;

    // Row with vertical alignment
    let valign_row = TableRow::new(vec![
        AdvancedTableCell::text("Vertical\nAlignment".to_string())
            .with_vertical_align(VerticalAlign::Top),
        AdvancedTableCell::text("Text can be aligned\nto top, middle, or bottom".to_string())
            .with_vertical_align(VerticalAlign::Middle),
        AdvancedTableCell::text("Middle".to_string()).with_vertical_align(VerticalAlign::Bottom),
    ])
    .with_height(60.0);
    table.add_row(valign_row)?;

    // Row with different text colors
    let color_row = TableRow::new(vec![
        AdvancedTableCell::text("Text Colors".to_string()).with_text_color(Color::red()),
        AdvancedTableCell::text("Each cell can have different colors".to_string())
            .with_text_color(Color::blue()),
        AdvancedTableCell::text("✓ Colorful".to_string())
            .with_text_color(Color::rgb(0.0, 0.7, 0.0)),
    ]);
    table.add_row(color_row)?;

    page.add_advanced_table(&table, 50.0, 680.0, 500.0)?;

    // Second table showing border styles
    page.text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 350.0)
        .write("Border Styles:")?;

    let border_columns = vec![
        ColumnDefinition {
            width: ColumnWidth::Fixed(150.0),
            default_align: TextAlign::Center,
            min_width: None,
            max_width: None,
        },
        ColumnDefinition {
            width: ColumnWidth::Fixed(150.0),
            default_align: TextAlign::Center,
            min_width: None,
            max_width: None,
        },
        ColumnDefinition {
            width: ColumnWidth::Fixed(150.0),
            default_align: TextAlign::Center,
            min_width: None,
            max_width: None,
        },
    ];

    let mut border_table = AdvancedTable::new(border_columns);

    // Configure for border demonstration
    let mut border_options = AdvancedTableOptions::default();
    border_options.cell_spacing = 10.0;
    border_options.draw_outer_border = false;
    border_options.border_style = TableBorderStyle::none();
    border_table.set_options(border_options);

    // Different border styles
    let border_demo_row = TableRow::new(vec![
        AdvancedTableCell::text("Solid Borders".to_string())
            .with_padding(CellPadding::uniform(10.0)),
        AdvancedTableCell::text("Dashed Borders".to_string())
            .with_padding(CellPadding::uniform(10.0)),
        AdvancedTableCell::text("Mixed Borders".to_string())
            .with_padding(CellPadding::uniform(10.0)),
    ]);
    border_table.add_row(border_demo_row)?;

    page.add_advanced_table(&border_table, 50.0, 300.0, 450.0)?;

    doc.add_page(page);
    Ok(())
}

fn create_financial_report_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Financial Report - Q4 2024")?;

    // Create financial table
    let columns = vec![
        ColumnDefinition {
            width: ColumnWidth::Fixed(200.0),
            default_align: TextAlign::Left,
            min_width: None,
            max_width: None,
        },
        ColumnDefinition {
            width: ColumnWidth::Fixed(100.0),
            default_align: TextAlign::Right,
            min_width: None,
            max_width: None,
        },
        ColumnDefinition {
            width: ColumnWidth::Fixed(100.0),
            default_align: TextAlign::Right,
            min_width: None,
            max_width: None,
        },
        ColumnDefinition {
            width: ColumnWidth::Fixed(100.0),
            default_align: TextAlign::Right,
            min_width: None,
            max_width: None,
        },
    ];

    let mut table = AdvancedTable::new(columns);

    // Professional financial table style
    let mut options = AdvancedTableOptions::default();
    options.border_style = TableBorderStyle::horizontal_only(1.0, Color::gray(0.8));
    options.cell_padding = CellPadding::symmetric(8.0, 6.0);
    options.font_size = 10.0;
    table.set_options(options);

    // Header
    let header_cells = vec![
        AdvancedTableCell::text("Account".to_string()).with_font(Font::HelveticaBold, 11.0),
        AdvancedTableCell::text("Q2 2024".to_string()).with_font(Font::HelveticaBold, 11.0),
        AdvancedTableCell::text("Q3 2024".to_string()).with_font(Font::HelveticaBold, 11.0),
        AdvancedTableCell::text("Q4 2024".to_string()).with_font(Font::HelveticaBold, 11.0),
    ];
    table.add_header_row(header_cells)?;

    // Revenue section
    let revenue_header = TableRow::new(vec![AdvancedTableCell::text("Revenue".to_string())
        .with_colspan(4)
        .with_font(Font::HelveticaBold, 10.0)
        .with_background(Color::gray(0.95))]);
    table.add_row(revenue_header)?;

    table.add_text_row(vec![
        "  Product Sales".to_string(),
        "$1,234,567".to_string(),
        "$1,456,789".to_string(),
        "$1,678,901".to_string(),
    ])?;

    table.add_text_row(vec![
        "  Service Revenue".to_string(),
        "$345,678".to_string(),
        "$456,789".to_string(),
        "$567,890".to_string(),
    ])?;

    table.add_text_row(vec![
        "  Other Income".to_string(),
        "$45,678".to_string(),
        "$56,789".to_string(),
        "$67,890".to_string(),
    ])?;

    // Total revenue row
    let total_revenue_row = TableRow::new(vec![
        AdvancedTableCell::text("Total Revenue".to_string()).with_font(Font::HelveticaBold, 10.0),
        AdvancedTableCell::text("$1,625,923".to_string()).with_font(Font::HelveticaBold, 10.0),
        AdvancedTableCell::text("$1,970,367".to_string()).with_font(Font::HelveticaBold, 10.0),
        AdvancedTableCell::text("$2,314,681".to_string())
            .with_font(Font::HelveticaBold, 10.0)
            .with_text_color(Color::rgb(0.0, 0.5, 0.0)),
    ])
    .with_background(Color::gray(0.98));
    table.add_row(total_revenue_row)?;

    // Expenses section
    let expenses_header = TableRow::new(vec![AdvancedTableCell::text("Expenses".to_string())
        .with_colspan(4)
        .with_font(Font::HelveticaBold, 10.0)
        .with_background(Color::gray(0.95))]);
    table.add_row(expenses_header)?;

    table.add_text_row(vec![
        "  Cost of Goods Sold".to_string(),
        "$567,890".to_string(),
        "$678,901".to_string(),
        "$789,012".to_string(),
    ])?;

    table.add_text_row(vec![
        "  Operating Expenses".to_string(),
        "$234,567".to_string(),
        "$345,678".to_string(),
        "$456,789".to_string(),
    ])?;

    table.add_text_row(vec![
        "  Marketing".to_string(),
        "$123,456".to_string(),
        "$134,567".to_string(),
        "$145,678".to_string(),
    ])?;

    // Total expenses row
    let total_expenses_row = TableRow::new(vec![
        AdvancedTableCell::text("Total Expenses".to_string()).with_font(Font::HelveticaBold, 10.0),
        AdvancedTableCell::text("$925,913".to_string()).with_font(Font::HelveticaBold, 10.0),
        AdvancedTableCell::text("$1,159,146".to_string()).with_font(Font::HelveticaBold, 10.0),
        AdvancedTableCell::text("$1,391,479".to_string())
            .with_font(Font::HelveticaBold, 10.0)
            .with_text_color(Color::rgb(0.7, 0.0, 0.0)),
    ])
    .with_background(Color::gray(0.98));
    table.add_row(total_expenses_row)?;

    // Net profit row
    let net_profit_row = TableRow::new(vec![
        AdvancedTableCell::text("Net Profit".to_string()).with_font(Font::HelveticaBold, 11.0),
        AdvancedTableCell::text("$700,010".to_string()).with_font(Font::HelveticaBold, 11.0),
        AdvancedTableCell::text("$811,221".to_string()).with_font(Font::HelveticaBold, 11.0),
        AdvancedTableCell::text("$923,202".to_string())
            .with_font(Font::HelveticaBold, 11.0)
            .with_text_color(Color::rgb(0.0, 0.6, 0.0)),
    ])
    .with_background(Color::rgb(0.95, 1.0, 0.95))
    .with_height(30.0);
    table.add_row(net_profit_row)?;

    page.add_advanced_table(&table, 50.0, 650.0, 500.0)?;

    // Add a note
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, 250.0)
        .write("Note: All figures are in USD and subject to audit verification.")?;

    doc.add_page(page);
    Ok(())
}

fn create_product_catalog_page(doc: &mut Document) -> Result<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 20.0)
        .at(50.0, 750.0)
        .write("Product Catalog")?;

    // Create product table with auto-width columns
    let columns = vec![
        ColumnDefinition {
            width: ColumnWidth::Fixed(80.0),
            default_align: TextAlign::Center,
            min_width: None,
            max_width: None,
        },
        ColumnDefinition {
            width: ColumnWidth::Auto,
            default_align: TextAlign::Left,
            min_width: Some(150.0),
            max_width: Some(250.0),
        },
        ColumnDefinition {
            width: ColumnWidth::Fixed(100.0),
            default_align: TextAlign::Center,
            min_width: None,
            max_width: None,
        },
        ColumnDefinition {
            width: ColumnWidth::Fixed(80.0),
            default_align: TextAlign::Right,
            min_width: None,
            max_width: None,
        },
    ];

    let mut table = AdvancedTable::new(columns);

    // Colorful catalog style
    let mut options = AdvancedTableOptions::default();
    options.border_style = TableBorderStyle::default();
    options.cell_padding = CellPadding::uniform(8.0);
    options.font_size = 10.0;
    options.alternating_rows = Some(AlternatingRowColors {
        even_color: Color::rgb(0.98, 0.98, 1.0),
        odd_color: Color::white(),
        include_header: false,
    });
    table.set_options(options);

    // Header
    let header_cells = vec![
        AdvancedTableCell::text("SKU".to_string())
            .with_background(Color::rgb(0.3, 0.5, 0.8))
            .with_text_color(Color::white())
            .with_font(Font::HelveticaBold, 11.0),
        AdvancedTableCell::text("Product Name".to_string())
            .with_background(Color::rgb(0.3, 0.5, 0.8))
            .with_text_color(Color::white())
            .with_font(Font::HelveticaBold, 11.0),
        AdvancedTableCell::text("Category".to_string())
            .with_background(Color::rgb(0.3, 0.5, 0.8))
            .with_text_color(Color::white())
            .with_font(Font::HelveticaBold, 11.0),
        AdvancedTableCell::text("Price".to_string())
            .with_background(Color::rgb(0.3, 0.5, 0.8))
            .with_text_color(Color::white())
            .with_font(Font::HelveticaBold, 11.0),
    ];
    table.add_header_row(header_cells)?;

    // Products
    let products = vec![
        ("PRD-001", "Professional PDF Editor", "Software", "$199.99"),
        ("PRD-002", "PDF Converter Suite", "Software", "$149.99"),
        ("PRD-003", "Document Scanner Pro", "Hardware", "$299.99"),
        ("PRD-004", "E-Signature Platform", "Service", "$29.99/mo"),
        ("PRD-005", "OCR Recognition Engine", "Software", "$399.99"),
        (
            "PRD-006",
            "Cloud Storage Integration",
            "Service",
            "$9.99/mo",
        ),
        ("PRD-007", "Batch Processing Tool", "Software", "$249.99"),
        ("PRD-008", "Mobile PDF Reader", "Mobile App", "$4.99"),
    ];

    for (sku, name, category, price) in products {
        let row = TableRow::new(vec![
            AdvancedTableCell::text(sku.to_string()),
            AdvancedTableCell::text(name.to_string()),
            AdvancedTableCell::text(category.to_string()).with_text_color(match category {
                "Software" => Color::rgb(0.0, 0.4, 0.8),
                "Hardware" => Color::rgb(0.6, 0.3, 0.0),
                "Service" => Color::rgb(0.0, 0.6, 0.3),
                "Mobile App" => Color::rgb(0.7, 0.0, 0.7),
                _ => Color::black(),
            }),
            AdvancedTableCell::text(price.to_string())
                .with_font(Font::HelveticaBold, 10.0)
                .with_text_color(Color::rgb(0.0, 0.5, 0.0)),
        ]);
        table.add_row(row)?;
    }

    // Special offer row
    let special_row = TableRow::new(vec![AdvancedTableCell::text(
        "🎉 SPECIAL OFFER: Bundle all software products for $699.99 (Save $200!)".to_string(),
    )
    .with_colspan(4)
    .with_align(TextAlign::Center)
    .with_background(Color::rgb(1.0, 0.95, 0.8))
    .with_text_color(Color::rgb(0.8, 0.4, 0.0))
    .with_font(Font::HelveticaBold, 12.0)
    .with_padding(CellPadding::uniform(10.0))]);
    table.add_row(special_row)?;

    page.add_advanced_table(&table, 50.0, 650.0, 500.0)?;

    doc.add_page(page);
    Ok(())
}
