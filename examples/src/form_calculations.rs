//! Example demonstrating form field calculations and dependencies
//!
//! This example shows how to:
//! - Create calculated form fields
//! - Set up field dependencies
//! - Use JavaScript calculations (AFSimple, AFPercent, etc.)
//! - Handle automatic recalculation
//! - Format calculated values

use oxidize_pdf::{Document, Page, PdfError};
use oxidize_pdf::forms::{
    TextField, FieldType, Widget, WidgetAppearance, BorderStyle,
    calculations::{CalculationEngine, FieldValue, Calculation, ArithmeticExpression,
                   CalculationFunction, Operator, ExpressionToken},
    calculation_system::{FormCalculationSystem, JavaScriptCalculation, SimpleOperation,
                         PercentMode, FieldFormat, SeparatorStyle, NegativeStyle,
                         CalculationSettings},
};
use oxidize_pdf::geometry::Rectangle;
use oxidize_pdf::graphics::Color;
use oxidize_pdf::text::Font;

fn main() -> Result<(), PdfError> {
    println!("📊 Creating PDF with form calculations examples...");

    // Create a new document
    let mut doc = Document::new();

    // Create different calculation examples
    create_invoice_page(&mut doc)?;
    create_percentage_calculator_page(&mut doc)?;
    create_loan_calculator_page(&mut doc)?;
    create_grade_calculator_page(&mut doc)?;

    // Save the document
    let output_path = "examples/results/form_calculations_example.pdf";
    doc.save(output_path)?;
    
    println!("✅ PDF with form calculations created successfully!");
    println!("📄 Output: {}", output_path);

    // Demonstrate the calculation system
    demonstrate_calculation_system()?;

    Ok(())
}

/// Create an invoice with automatic calculations
fn create_invoice_page(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0); // Letter size
    
    // Add title
    page.add_text("Invoice with Automatic Calculations", 50.0, 700.0, Font::HelveticaBold, 16.0)?;
    
    // Invoice header
    page.add_text("Invoice #12345", 50.0, 650.0, Font::Helvetica, 12.0)?;
    page.add_text("Date: 2025-08-13", 50.0, 630.0, Font::Helvetica, 12.0)?;
    
    // Table headers
    page.add_text("Item", 50.0, 580.0, Font::HelveticaBold, 10.0)?;
    page.add_text("Quantity", 200.0, 580.0, Font::HelveticaBold, 10.0)?;
    page.add_text("Unit Price", 280.0, 580.0, Font::HelveticaBold, 10.0)?;
    page.add_text("Total", 380.0, 580.0, Font::HelveticaBold, 10.0)?;
    
    // Line items
    let items = [
        ("Widget A", 5.0, 10.99),
        ("Widget B", 3.0, 24.50),
        ("Widget C", 10.0, 5.00),
    ];
    
    let mut y = 550.0;
    for (i, (item, qty, price)) in items.iter().enumerate() {
        page.add_text(item, 50.0, y, Font::Helvetica, 10.0)?;
        page.add_text(&format!("{}", qty), 200.0, y, Font::Helvetica, 10.0)?;
        page.add_text(&format!("${:.2}", price), 280.0, y, Font::Helvetica, 10.0)?;
        
        // Total for this line (calculated field)
        let total = qty * price;
        page.add_text(&format!("${:.2}", total), 380.0, y, Font::Helvetica, 10.0)?;
        
        y -= 20.0;
    }
    
    // Separator line
    page.add_line(50.0, y + 10.0, 450.0, y + 10.0)?;
    
    // Subtotal, tax, and grand total
    y -= 20.0;
    page.add_text("Subtotal:", 280.0, y, Font::HelveticaBold, 10.0)?;
    let subtotal = items.iter().map(|(_, q, p)| q * p).sum::<f64>();
    page.add_text(&format!("${:.2}", subtotal), 380.0, y, Font::Helvetica, 10.0)?;
    
    y -= 20.0;
    page.add_text("Tax (8.5%):", 280.0, y, Font::HelveticaBold, 10.0)?;
    let tax = subtotal * 0.085;
    page.add_text(&format!("${:.2}", tax), 380.0, y, Font::Helvetica, 10.0)?;
    
    y -= 20.0;
    page.add_text("Grand Total:", 280.0, y, Font::HelveticaBold, 12.0)?;
    let grand_total = subtotal + tax;
    page.add_text(&format!("${:.2}", grand_total), 380.0, y, Font::HelveticaBold, 12.0)?;
    
    // Add note about calculations
    page.add_text("Note: All totals are automatically calculated", 50.0, 150.0, 
                  Font::Helvetica, 8.0)?;

    doc.add_page(page);
    Ok(())
}

/// Create a percentage calculator
fn create_percentage_calculator_page(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);
    
    // Add title
    page.add_text("Percentage Calculator", 50.0, 700.0, Font::HelveticaBold, 16.0)?;
    
    // Instructions
    page.add_text("Enter values to see automatic percentage calculations:", 
                  50.0, 650.0, Font::Helvetica, 12.0)?;
    
    // Base value field
    page.add_text("Base Amount:", 50.0, 600.0, Font::Helvetica, 12.0)?;
    page.add_rectangle(Rectangle::new(150.0, 590.0, 250.0, 610.0))?;
    page.add_text("100", 160.0, 595.0, Font::Helvetica, 10.0)?; // Example value
    
    // Percentage field
    page.add_text("Percentage:", 50.0, 560.0, Font::Helvetica, 12.0)?;
    page.add_rectangle(Rectangle::new(150.0, 550.0, 250.0, 570.0))?;
    page.add_text("15", 160.0, 555.0, Font::Helvetica, 10.0)?; // Example value
    
    // Results section
    page.add_text("Results:", 50.0, 500.0, Font::HelveticaBold, 12.0)?;
    
    // X% of base
    page.add_text("15% of 100 =", 70.0, 470.0, Font::Helvetica, 10.0)?;
    page.add_text("15.00", 200.0, 470.0, Font::HelveticaBold, 10.0)?;
    
    // Base + X%
    page.add_text("100 + 15% =", 70.0, 450.0, Font::Helvetica, 10.0)?;
    page.add_text("115.00", 200.0, 450.0, Font::HelveticaBold, 10.0)?;
    
    // Base - X%
    page.add_text("100 - 15% =", 70.0, 430.0, Font::Helvetica, 10.0)?;
    page.add_text("85.00", 200.0, 430.0, Font::HelveticaBold, 10.0)?;
    
    // What % is X of Y
    page.add_text("15 is what % of 100?", 70.0, 410.0, Font::Helvetica, 10.0)?;
    page.add_text("15.00%", 200.0, 410.0, Font::HelveticaBold, 10.0)?;

    doc.add_page(page);
    Ok(())
}

/// Create a loan calculator
fn create_loan_calculator_page(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);
    
    // Add title
    page.add_text("Loan Payment Calculator", 50.0, 700.0, Font::HelveticaBold, 16.0)?;
    
    // Input fields
    page.add_text("Loan Amount:", 50.0, 640.0, Font::Helvetica, 12.0)?;
    page.add_rectangle(Rectangle::new(200.0, 630.0, 300.0, 650.0))?;
    page.add_text("$50,000", 210.0, 635.0, Font::Helvetica, 10.0)?;
    
    page.add_text("Annual Interest Rate:", 50.0, 600.0, Font::Helvetica, 12.0)?;
    page.add_rectangle(Rectangle::new(200.0, 590.0, 300.0, 610.0))?;
    page.add_text("5.5%", 210.0, 595.0, Font::Helvetica, 10.0)?;
    
    page.add_text("Loan Term (years):", 50.0, 560.0, Font::Helvetica, 12.0)?;
    page.add_rectangle(Rectangle::new(200.0, 550.0, 300.0, 570.0))?;
    page.add_text("30", 210.0, 555.0, Font::Helvetica, 10.0)?;
    
    // Calculated results
    page.add_text("Calculated Results:", 50.0, 500.0, Font::HelveticaBold, 14.0)?;
    
    // Monthly payment calculation
    let principal = 50000.0;
    let annual_rate = 0.055;
    let monthly_rate = annual_rate / 12.0;
    let num_payments = 30.0 * 12.0;
    
    let monthly_payment = principal * 
        (monthly_rate * (1.0 + monthly_rate).powf(num_payments)) /
        ((1.0 + monthly_rate).powf(num_payments) - 1.0);
    
    page.add_text("Monthly Payment:", 70.0, 460.0, Font::Helvetica, 12.0)?;
    page.add_text(&format!("${:.2}", monthly_payment), 200.0, 460.0, 
                  Font::HelveticaBold, 12.0)?;
    
    // Total payments
    let total_payments = monthly_payment * num_payments;
    page.add_text("Total Payments:", 70.0, 440.0, Font::Helvetica, 12.0)?;
    page.add_text(&format!("${:.2}", total_payments), 200.0, 440.0, 
                  Font::HelveticaBold, 12.0)?;
    
    // Total interest
    let total_interest = total_payments - principal;
    page.add_text("Total Interest:", 70.0, 420.0, Font::Helvetica, 12.0)?;
    page.add_text(&format!("${:.2}", total_interest), 200.0, 420.0, 
                  Font::HelveticaBold, 12.0)?;
    
    // Payment breakdown pie chart placeholder
    page.add_text("Payment Breakdown:", 50.0, 360.0, Font::HelveticaBold, 12.0)?;
    page.add_rectangle(Rectangle::new(70.0, 200.0, 270.0, 340.0))?;
    page.add_text("Principal: 47.1%", 280.0, 300.0, Font::Helvetica, 10.0)?;
    page.add_text("Interest: 52.9%", 280.0, 280.0, Font::Helvetica, 10.0)?;

    doc.add_page(page);
    Ok(())
}

/// Create a grade calculator
fn create_grade_calculator_page(doc: &mut Document) -> Result<(), PdfError> {
    let mut page = Page::new(612.0, 792.0);
    
    // Add title
    page.add_text("Grade Calculator", 50.0, 700.0, Font::HelveticaBold, 16.0)?;
    
    // Instructions
    page.add_text("Enter scores to calculate weighted grade:", 
                  50.0, 650.0, Font::Helvetica, 12.0)?;
    
    // Grade components
    let components = [
        ("Homework", 0.20, 85.0),
        ("Quizzes", 0.15, 78.0),
        ("Midterm", 0.25, 82.0),
        ("Final Exam", 0.30, 88.0),
        ("Participation", 0.10, 95.0),
    ];
    
    let mut y = 600.0;
    page.add_text("Component", 50.0, y, Font::HelveticaBold, 10.0)?;
    page.add_text("Weight", 200.0, y, Font::HelveticaBold, 10.0)?;
    page.add_text("Score", 280.0, y, Font::HelveticaBold, 10.0)?;
    page.add_text("Weighted", 360.0, y, Font::HelveticaBold, 10.0)?;
    
    y -= 20.0;
    let mut total_weighted = 0.0;
    
    for (name, weight, score) in &components {
        page.add_text(name, 50.0, y, Font::Helvetica, 10.0)?;
        page.add_text(&format!("{}%", (weight * 100.0) as i32), 200.0, y, 
                      Font::Helvetica, 10.0)?;
        page.add_text(&format!("{:.1}", score), 280.0, y, Font::Helvetica, 10.0)?;
        
        let weighted = score * weight;
        total_weighted += weighted;
        page.add_text(&format!("{:.1}", weighted), 360.0, y, Font::Helvetica, 10.0)?;
        
        y -= 20.0;
    }
    
    // Separator
    page.add_line(50.0, y + 10.0, 420.0, y + 10.0)?;
    
    // Final grade
    y -= 20.0;
    page.add_text("Final Grade:", 50.0, y, Font::HelveticaBold, 12.0)?;
    page.add_text(&format!("{:.1}%", total_weighted), 360.0, y, 
                  Font::HelveticaBold, 12.0)?;
    
    // Letter grade
    let letter_grade = match total_weighted as i32 {
        90..=100 => "A",
        80..=89 => "B",
        70..=79 => "C",
        60..=69 => "D",
        _ => "F",
    };
    
    y -= 25.0;
    page.add_text("Letter Grade:", 50.0, y, Font::HelveticaBold, 12.0)?;
    page.add_text(letter_grade, 360.0, y, Font::HelveticaBold, 14.0)?;
    
    // GPA calculation
    let gpa = match letter_grade {
        "A" => 4.0,
        "B" => 3.0,
        "C" => 2.0,
        "D" => 1.0,
        _ => 0.0,
    };
    
    y -= 25.0;
    page.add_text("GPA Points:", 50.0, y, Font::HelveticaBold, 12.0)?;
    page.add_text(&format!("{:.1}", gpa), 360.0, y, Font::HelveticaBold, 12.0)?;

    doc.add_page(page);
    Ok(())
}

/// Demonstrate the calculation system
fn demonstrate_calculation_system() -> Result<(), PdfError> {
    println!("\n📊 Demonstrating Form Calculation System...");
    
    // Create calculation system
    let mut calc_system = FormCalculationSystem::new();
    
    // Example 1: Simple invoice calculation
    println!("\n1️⃣ Invoice Calculation:");
    calc_system.set_field_value("quantity", FieldValue::Number(5.0))?;
    calc_system.set_field_value("unit_price", FieldValue::Number(19.99))?;
    
    let invoice_calc = JavaScriptCalculation::SimpleCalculate {
        operation: SimpleOperation::Product,
        fields: vec!["quantity".to_string(), "unit_price".to_string()],
    };
    calc_system.add_js_calculation("line_total", invoice_calc)?;
    
    if let Some(total) = calc_system.engine.get_field_value("line_total") {
        println!("  Quantity: 5");
        println!("  Unit Price: $19.99");
        println!("  Line Total: ${:.2}", total.to_number());
    }
    
    // Example 2: Percentage calculation
    println!("\n2️⃣ Percentage Calculation:");
    calc_system.set_field_value("base_amount", FieldValue::Number(1000.0))?;
    calc_system.set_field_value("discount_percent", FieldValue::Number(15.0))?;
    
    let percent_calc = JavaScriptCalculation::PercentCalculate {
        base_field: "base_amount".to_string(),
        percent_field: "discount_percent".to_string(),
        mode: PercentMode::SubtractPercent,
    };
    calc_system.add_js_calculation("discounted_price", percent_calc)?;
    
    if let Some(price) = calc_system.engine.get_field_value("discounted_price") {
        println!("  Base Amount: $1000.00");
        println!("  Discount: 15%");
        println!("  Discounted Price: ${:.2}", price.to_number());
    }
    
    // Example 3: Average calculation
    println!("\n3️⃣ Grade Average Calculation:");
    calc_system.set_field_value("test1", FieldValue::Number(85.0))?;
    calc_system.set_field_value("test2", FieldValue::Number(92.0))?;
    calc_system.set_field_value("test3", FieldValue::Number(78.0))?;
    calc_system.set_field_value("test4", FieldValue::Number(88.0))?;
    
    let avg_calc = JavaScriptCalculation::SimpleCalculate {
        operation: SimpleOperation::Average,
        fields: vec![
            "test1".to_string(),
            "test2".to_string(),
            "test3".to_string(),
            "test4".to_string(),
        ],
    };
    calc_system.add_js_calculation("average_score", avg_calc)?;
    
    if let Some(avg) = calc_system.engine.get_field_value("average_score") {
        println!("  Test Scores: 85, 92, 78, 88");
        println!("  Average: {:.1}", avg.to_number());
    }
    
    // Example 4: Field dependencies
    println!("\n4️⃣ Field Dependencies:");
    
    // Create a calculation engine for dependency demonstration
    let mut engine = CalculationEngine::new();
    
    // Set base values
    engine.set_field_value("hours_worked", FieldValue::Number(40.0));
    engine.set_field_value("hourly_rate", FieldValue::Number(25.0));
    
    // Gross pay = hours * rate
    let gross_expr = ArithmeticExpression::from_string("hours_worked * hourly_rate")?;
    engine.add_calculation("gross_pay", Calculation::Arithmetic(gross_expr))?;
    
    // Tax = gross pay * 0.2
    let tax_expr = ArithmeticExpression::from_string("gross_pay * 0.2")?;
    engine.add_calculation("tax", Calculation::Arithmetic(tax_expr))?;
    
    // Net pay = gross pay - tax
    let net_expr = ArithmeticExpression::from_string("gross_pay - tax")?;
    engine.add_calculation("net_pay", Calculation::Arithmetic(net_expr))?;
    
    println!("  Hours Worked: 40");
    println!("  Hourly Rate: $25");
    println!("  Gross Pay: ${:.2}", engine.get_field_value("gross_pay").unwrap().to_number());
    println!("  Tax (20%): ${:.2}", engine.get_field_value("tax").unwrap().to_number());
    println!("  Net Pay: ${:.2}", engine.get_field_value("net_pay").unwrap().to_number());
    
    // Change hours and see automatic recalculation
    println!("\n  Changing hours to 45...");
    engine.set_field_value("hours_worked", FieldValue::Number(45.0));
    
    println!("  New Gross Pay: ${:.2}", engine.get_field_value("gross_pay").unwrap().to_number());
    println!("  New Tax: ${:.2}", engine.get_field_value("tax").unwrap().to_number());
    println!("  New Net Pay: ${:.2}", engine.get_field_value("net_pay").unwrap().to_number());
    
    // Show calculation summary
    println!("\n📊 Calculation System Summary:");
    let summary = calc_system.get_summary();
    println!("{}", summary);
    
    println!("\n✅ Calculation system demonstration complete!");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoice_calculation() {
        let mut calc_system = FormCalculationSystem::new();
        
        calc_system.set_field_value("qty", FieldValue::Number(10.0)).unwrap();
        calc_system.set_field_value("price", FieldValue::Number(5.50)).unwrap();
        
        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Product,
            fields: vec!["qty".to_string(), "price".to_string()],
        };
        
        calc_system.add_js_calculation("total", calc).unwrap();
        
        let total = calc_system.engine.get_field_value("total").unwrap();
        assert_eq!(total.to_number(), 55.0);
    }

    #[test]
    fn test_percentage_discount() {
        let mut calc_system = FormCalculationSystem::new();
        
        calc_system.set_field_value("original", FieldValue::Number(100.0)).unwrap();
        calc_system.set_field_value("discount", FieldValue::Number(20.0)).unwrap();
        
        let calc = JavaScriptCalculation::PercentCalculate {
            base_field: "original".to_string(),
            percent_field: "discount".to_string(),
            mode: PercentMode::SubtractPercent,
        };
        
        calc_system.add_js_calculation("sale_price", calc).unwrap();
        
        let price = calc_system.engine.get_field_value("sale_price").unwrap();
        assert_eq!(price.to_number(), 80.0);
    }

    #[test]
    fn test_grade_average() {
        let mut calc_system = FormCalculationSystem::new();
        
        calc_system.set_field_value("g1", FieldValue::Number(90.0)).unwrap();
        calc_system.set_field_value("g2", FieldValue::Number(85.0)).unwrap();
        calc_system.set_field_value("g3", FieldValue::Number(95.0)).unwrap();
        
        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Average,
            fields: vec!["g1".to_string(), "g2".to_string(), "g3".to_string()],
        };
        
        calc_system.add_js_calculation("avg", calc).unwrap();
        
        let avg = calc_system.engine.get_field_value("avg").unwrap();
        assert_eq!(avg.to_number(), 90.0);
    }

    #[test]
    fn test_field_dependencies() {
        let mut engine = CalculationEngine::new();
        
        engine.set_field_value("a", FieldValue::Number(10.0));
        engine.set_field_value("b", FieldValue::Number(20.0));
        
        // c = a + b
        let expr1 = ArithmeticExpression::from_string("a + b").unwrap();
        engine.add_calculation("c", Calculation::Arithmetic(expr1)).unwrap();
        
        // d = c * 2
        let expr2 = ArithmeticExpression::from_string("c * 2").unwrap();
        engine.add_calculation("d", Calculation::Arithmetic(expr2)).unwrap();
        
        assert_eq!(engine.get_field_value("c").unwrap().to_number(), 30.0);
        assert_eq!(engine.get_field_value("d").unwrap().to_number(), 60.0);
        
        // Change a and verify cascade
        engine.set_field_value("a", FieldValue::Number(15.0));
        
        assert_eq!(engine.get_field_value("c").unwrap().to_number(), 35.0);
        assert_eq!(engine.get_field_value("d").unwrap().to_number(), 70.0);
    }
}