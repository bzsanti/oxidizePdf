//! Example demonstrating form field calculations in PDF
//!
//! This example shows how to:
//! - Create forms with calculated fields
//! - Set up field dependencies
//! - Use predefined calculation functions
//! - Handle automatic recalculation
//! - Create invoice/order forms

use oxidize_pdf::forms::calculations::{
    ArithmeticExpression, Calculation, CalculationEngine, CalculationFunction, FieldValue,
};
use oxidize_pdf::forms::javascript_engine::JavaScriptEngine;
use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Form Calculations Examples\n");

    // Example 1: Simple invoice with calculations
    create_invoice_form()?;

    // Example 2: Order form with complex calculations
    create_order_form()?;

    // Example 3: Timesheet with hour calculations
    create_timesheet_form()?;

    // Example 4: Loan calculator
    create_loan_calculator()?;

    println!("\nAll calculation examples completed successfully!");
    Ok(())
}

/// Create a simple invoice form with automatic calculations
fn create_invoice_form() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Invoice with Calculations");
    println!("------------------------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("INVOICE")?;

    // Create calculation engine
    let mut calc_engine = CalculationEngine::new();

    // Invoice items table header
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, 680.0)
        .write("Item")?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(250.0, 680.0)
        .write("Quantity")?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(350.0, 680.0)
        .write("Unit Price")?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(450.0, 680.0)
        .write("Total")?;

    // Draw line
    page.graphics()
        .save_state()
        .set_stroke_color(Color::black())
        .move_to(50.0, 670.0)
        .line_to(540.0, 670.0)
        .stroke()
        .restore_state();

    // Create invoice items
    let items = vec![
        ("Product A", "qty1", "price1", "total1", 650.0),
        ("Product B", "qty2", "price2", "total2", 620.0),
        ("Product C", "qty3", "price3", "total3", 590.0),
    ];

    for (item_name, qty_field, price_field, total_field, y_pos) in items {
        // Item name
        page.text()
            .set_font(Font::Helvetica, 11.0)
            .at(50.0, y_pos)
            .write(item_name)?;

        // Quantity field
        draw_field(&mut page, 250.0, y_pos - 5.0, 60.0, 20.0)?;

        // Unit price field
        draw_field(&mut page, 350.0, y_pos - 5.0, 80.0, 20.0)?;

        // Total field (calculated)
        draw_field(&mut page, 450.0, y_pos - 5.0, 80.0, 20.0)?;

        // Set up calculation for line total
        let expr = ArithmeticExpression::from_string(&format!("{} * {}", qty_field, price_field))?;
        calc_engine.add_calculation(total_field, Calculation::Arithmetic(expr))?;
    }

    // Subtotal, tax, and grand total
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(380.0, 520.0)
        .write("Subtotal:")?;

    draw_field(&mut page, 450.0, 515.0, 80.0, 20.0)?;

    // Calculate subtotal
    let subtotal_calc = Calculation::Function(CalculationFunction::Sum(vec![
        "total1".to_string(),
        "total2".to_string(),
        "total3".to_string(),
    ]));
    calc_engine.add_calculation("subtotal", subtotal_calc)?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(380.0, 490.0)
        .write("Tax (10%):")?;

    draw_field(&mut page, 450.0, 485.0, 80.0, 20.0)?;

    // Calculate tax
    let tax_expr = ArithmeticExpression::from_string("subtotal * 0.1")?;
    calc_engine.add_calculation("tax", Calculation::Arithmetic(tax_expr))?;

    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(380.0, 460.0)
        .write("Total:")?;

    draw_field(&mut page, 450.0, 455.0, 80.0, 20.0)?;

    // Calculate grand total
    let total_expr = ArithmeticExpression::from_string("subtotal + tax")?;
    calc_engine.add_calculation("grandtotal", Calculation::Arithmetic(total_expr))?;

    // Simulate entering values
    println!("  Setting field values...");
    calc_engine.set_field_value("qty1", FieldValue::Number(2.0));
    calc_engine.set_field_value("price1", FieldValue::Number(29.99));
    calc_engine.set_field_value("qty2", FieldValue::Number(1.0));
    calc_engine.set_field_value("price2", FieldValue::Number(49.99));
    calc_engine.set_field_value("qty3", FieldValue::Number(3.0));
    calc_engine.set_field_value("price3", FieldValue::Number(15.99));

    // Show calculated results
    println!("  Calculated values:");
    println!(
        "    Product A total: ${}",
        calc_engine.get_field_value("total1").unwrap().to_string()
    );
    println!(
        "    Product B total: ${}",
        calc_engine.get_field_value("total2").unwrap().to_string()
    );
    println!(
        "    Product C total: ${}",
        calc_engine.get_field_value("total3").unwrap().to_string()
    );
    println!(
        "    Subtotal: ${}",
        calc_engine.get_field_value("subtotal").unwrap().to_string()
    );
    println!(
        "    Tax: ${}",
        calc_engine.get_field_value("tax").unwrap().to_string()
    );
    println!(
        "    Grand Total: ${}",
        calc_engine
            .get_field_value("grandtotal")
            .unwrap()
            .to_string()
    );

    // Show calculation summary
    let summary = calc_engine.get_summary();
    println!("  {}", summary);

    doc.add_page(page);
    doc.save("examples/results/form_invoice.pdf")?;

    println!("  ✓ Created form_invoice.pdf");
    Ok(())
}

/// Create an order form with quantity-based pricing
fn create_order_form() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 2: Order Form with Discounts");
    println!("------------------------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("ORDER FORM")?;

    let mut calc_engine = CalculationEngine::new();

    // Product sections
    let products = vec![
        ("Widget A", 25.00, 500.0),
        ("Widget B", 35.00, 450.0),
        ("Widget C", 45.00, 400.0),
    ];

    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 650.0)
        .write("Products")?;

    for (i, (product, base_price, y_pos)) in products.iter().enumerate() {
        let qty_field = format!("qty_{}", i);
        let price_field = format!("price_{}", i);
        let total_field = format!("total_{}", i);

        page.text()
            .set_font(Font::Helvetica, 11.0)
            .at(50.0, *y_pos)
            .write(&format!("{} (${:.2} each)", product, base_price))?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, y_pos - 15.0)
            .write("Quantity:")?;

        draw_field(&mut page, 110.0, y_pos - 20.0, 60.0, 20.0)?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(200.0, y_pos - 15.0)
            .write("Line Total:")?;

        draw_field(&mut page, 260.0, y_pos - 20.0, 80.0, 20.0)?;

        // Set base price
        calc_engine.set_field_value(&price_field, FieldValue::Number(*base_price));

        // Calculate line total
        let expr = ArithmeticExpression::from_string(&format!("{} * {}", qty_field, price_field))?;
        calc_engine.add_calculation(&total_field, Calculation::Arithmetic(expr))?;
    }

    // Discount calculation
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(350.0, 350.0)
        .write("Order Summary")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(350.0, 320.0)
        .write("Subtotal:")?;

    draw_field(&mut page, 450.0, 315.0, 80.0, 20.0)?;

    // Calculate subtotal
    let subtotal_fields: Vec<String> = (0..3).map(|i| format!("total_{}", i)).collect();
    calc_engine.add_calculation(
        "order_subtotal",
        Calculation::Function(CalculationFunction::Sum(subtotal_fields)),
    )?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(350.0, 290.0)
        .write("Discount:")?;

    draw_field(&mut page, 450.0, 285.0, 80.0, 20.0)?;

    // Volume discount calculation using JavaScript
    let js_code = r#"
        var subtotal = order_subtotal;
        var discount = 0;
        if (subtotal > 500) {
            discount = subtotal * 0.15;
        } else if (subtotal > 250) {
            discount = subtotal * 0.10;
        } else if (subtotal > 100) {
            discount = subtotal * 0.05;
        }
        discount
    "#;

    calc_engine.add_calculation("discount", Calculation::JavaScript(js_code.to_string()))?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(350.0, 260.0)
        .write("Final Total:")?;

    draw_field(&mut page, 450.0, 255.0, 80.0, 20.0)?;

    // Calculate final total
    let final_expr = ArithmeticExpression::from_string("order_subtotal - discount")?;
    calc_engine.add_calculation("final_total", Calculation::Arithmetic(final_expr))?;

    // Test with sample quantities
    println!("  Testing with sample order:");
    calc_engine.set_field_value("qty_0", FieldValue::Number(5.0));
    calc_engine.set_field_value("qty_1", FieldValue::Number(3.0));
    calc_engine.set_field_value("qty_2", FieldValue::Number(2.0));

    println!("    Widget A: 5 units");
    println!("    Widget B: 3 units");
    println!("    Widget C: 2 units");
    println!(
        "    Subtotal: ${}",
        calc_engine
            .get_field_value("order_subtotal")
            .unwrap()
            .to_string()
    );
    println!(
        "    Final: ${}",
        calc_engine
            .get_field_value("final_total")
            .unwrap()
            .to_string()
    );

    doc.add_page(page);
    doc.save("examples/results/form_order.pdf")?;

    println!("  ✓ Created form_order.pdf");
    Ok(())
}

/// Create a timesheet form with hour calculations
fn create_timesheet_form() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 3: Timesheet Calculator");
    println!("-------------------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("WEEKLY TIMESHEET")?;

    let mut calc_engine = CalculationEngine::new();

    // Days of the week
    let days = vec![
        ("Monday", 600.0),
        ("Tuesday", 560.0),
        ("Wednesday", 520.0),
        ("Thursday", 480.0),
        ("Friday", 440.0),
    ];

    // Column headers
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, 650.0)
        .write("Day")?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(150.0, 650.0)
        .write("Start")?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(250.0, 650.0)
        .write("End")?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(350.0, 650.0)
        .write("Break")?;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(450.0, 650.0)
        .write("Total Hours")?;

    let mut daily_totals = Vec::new();

    for (day, y_pos) in days {
        let start_field = format!("{}_start", day.to_lowercase());
        let end_field = format!("{}_end", day.to_lowercase());
        let break_field = format!("{}_break", day.to_lowercase());
        let total_field = format!("{}_total", day.to_lowercase());

        page.text()
            .set_font(Font::Helvetica, 11.0)
            .at(50.0, y_pos)
            .write(day)?;

        // Time fields
        draw_field(&mut page, 150.0, y_pos - 5.0, 70.0, 20.0)?;
        draw_field(&mut page, 250.0, y_pos - 5.0, 70.0, 20.0)?;
        draw_field(&mut page, 350.0, y_pos - 5.0, 70.0, 20.0)?;
        draw_field(&mut page, 450.0, y_pos - 5.0, 80.0, 20.0)?;

        // Calculate daily hours (end - start - break)
        let expr = ArithmeticExpression::from_string(&format!(
            "{} - {} - {}",
            end_field, start_field, break_field
        ))?;
        calc_engine.add_calculation(&total_field, Calculation::Arithmetic(expr))?;

        daily_totals.push(total_field);
    }

    // Weekly summary
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(350.0, 380.0)
        .write("Weekly Summary")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(350.0, 350.0)
        .write("Total Hours:")?;

    draw_field(&mut page, 450.0, 345.0, 80.0, 20.0)?;

    // Calculate weekly total
    calc_engine.add_calculation(
        "weekly_total",
        Calculation::Function(CalculationFunction::Sum(daily_totals.clone())),
    )?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(350.0, 320.0)
        .write("Overtime Hours:")?;

    draw_field(&mut page, 450.0, 315.0, 80.0, 20.0)?;

    // Calculate overtime (hours over 40)
    let overtime_calc = Calculation::Function(CalculationFunction::If {
        condition_field: "weekly_total".to_string(),
        true_value: Box::new(Calculation::Arithmetic(ArithmeticExpression::from_string(
            "weekly_total - 40",
        )?)),
        false_value: Box::new(Calculation::Constant(FieldValue::Number(0.0))),
    });
    calc_engine.add_calculation("overtime", overtime_calc)?;

    // Test with sample data
    println!("  Testing with sample hours:");
    calc_engine.set_field_value("monday_start", FieldValue::Number(9.0));
    calc_engine.set_field_value("monday_end", FieldValue::Number(18.0));
    calc_engine.set_field_value("monday_break", FieldValue::Number(1.0));

    calc_engine.set_field_value("tuesday_start", FieldValue::Number(8.5));
    calc_engine.set_field_value("tuesday_end", FieldValue::Number(17.5));
    calc_engine.set_field_value("tuesday_break", FieldValue::Number(0.5));

    println!(
        "    Monday: {} hours",
        calc_engine
            .get_field_value("monday_total")
            .unwrap()
            .to_string()
    );
    println!(
        "    Tuesday: {} hours",
        calc_engine
            .get_field_value("tuesday_total")
            .unwrap()
            .to_string()
    );

    doc.add_page(page);
    doc.save("examples/results/form_timesheet.pdf")?;

    println!("  ✓ Created form_timesheet.pdf");
    Ok(())
}

/// Create a loan calculator form
fn create_loan_calculator() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nExample 4: Loan Calculator");
    println!("--------------------------");

    let mut doc = Document::new();
    let mut page = Page::a4();

    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("LOAN CALCULATOR")?;

    let mut calc_engine = CalculationEngine::new();
    let mut js_engine = JavaScriptEngine::new();

    // Input fields
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 680.0)
        .write("Loan Parameters")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 640.0)
        .write("Loan Amount ($):")?;

    draw_field(&mut page, 200.0, 635.0, 100.0, 20.0)?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 600.0)
        .write("Annual Interest Rate (%):")?;

    draw_field(&mut page, 200.0, 595.0, 100.0, 20.0)?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 560.0)
        .write("Loan Term (years):")?;

    draw_field(&mut page, 200.0, 555.0, 100.0, 20.0)?;

    // Calculated fields
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, 480.0)
        .write("Payment Details")?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 440.0)
        .write("Monthly Payment:")?;

    draw_field(&mut page, 200.0, 435.0, 100.0, 20.0)?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 400.0)
        .write("Total Interest:")?;

    draw_field(&mut page, 200.0, 395.0, 100.0, 20.0)?;

    page.text()
        .set_font(Font::Helvetica, 11.0)
        .at(50.0, 360.0)
        .write("Total Amount:")?;

    draw_field(&mut page, 200.0, 355.0, 100.0, 20.0)?;

    // Monthly payment calculation using JavaScript
    let payment_js = r#"
        var P = loan_amount;
        var r = interest_rate / 100 / 12;
        var n = loan_term * 12;
        
        if (r == 0) {
            P / n
        } else {
            P * r * Math.pow(1 + r, n) / (Math.pow(1 + r, n) - 1)
        }
    "#;

    // Test calculation with JavaScript engine
    js_engine.set_variable("loan_amount", 100000.0);
    js_engine.set_variable("interest_rate", 5.0);
    js_engine.set_variable("loan_term", 30.0);

    let monthly_payment = js_engine.evaluate(payment_js)?;
    println!("  Sample calculation:");
    println!("    Loan: $100,000");
    println!("    Rate: 5% APR");
    println!("    Term: 30 years");
    println!("    Monthly Payment: ${:.2}", monthly_payment);

    // Set up calculations in the engine
    calc_engine.add_calculation(
        "monthly_payment",
        Calculation::JavaScript(payment_js.to_string()),
    )?;

    // Total interest calculation
    let interest_expr =
        ArithmeticExpression::from_string("monthly_payment * loan_term * 12 - loan_amount")?;
    calc_engine.add_calculation("total_interest", Calculation::Arithmetic(interest_expr))?;

    // Total amount calculation
    let total_expr = ArithmeticExpression::from_string("loan_amount + total_interest")?;
    calc_engine.add_calculation("total_amount", Calculation::Arithmetic(total_expr))?;

    // Test with values
    calc_engine.set_field_value("loan_amount", FieldValue::Number(100000.0));
    calc_engine.set_field_value("interest_rate", FieldValue::Number(5.0));
    calc_engine.set_field_value("loan_term", FieldValue::Number(30.0));

    println!(
        "    Total Interest: ${}",
        calc_engine
            .get_field_value("total_interest")
            .unwrap()
            .to_string()
    );
    println!(
        "    Total Amount: ${}",
        calc_engine
            .get_field_value("total_amount")
            .unwrap()
            .to_string()
    );

    doc.add_page(page);
    doc.save("examples/results/form_loan_calculator.pdf")?;

    println!("  ✓ Created form_loan_calculator.pdf");
    Ok(())
}

/// Helper function to draw a field rectangle
fn draw_field(
    page: &mut Page,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    page.graphics()
        .save_state()
        .set_stroke_color(Color::gray(0.5))
        .set_line_width(0.5)
        .rectangle(x, y, width, height)
        .stroke()
        .restore_state();
    Ok(())
}
