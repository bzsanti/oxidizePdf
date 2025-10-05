//! AI-Ready Contract Example
//!
//! Demonstrates creating a legal contract PDF with semantic entity marking for automated
//! contract analysis, clause extraction, and party identification.

use oxidize_pdf::semantic::{BoundingBox, EntityType, RelationType};
use oxidize_pdf::{Color, Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("📜 Creating AI-Ready Legal Contract PDF...\n");

    // Create document
    let mut doc = Document::new();
    doc.set_title("Software Development Services Agreement");
    doc.set_author("oxidize-pdf");
    doc.set_subject("Legal Contract with Semantic Markup");

    // Create page
    let mut page = Page::a4();
    let width = page.width();
    let mut y = 750.0;

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, y)
        .write("SOFTWARE DEVELOPMENT SERVICES AGREEMENT")?;

    // Mark the entire contract
    let contract_id = doc.mark_entity(
        "contract_main".to_string(),
        EntityType::Contract,
        BoundingBox::new(50.0, 50.0, (width - 100.0) as f32, 750.0, 1),
    );
    doc.set_entity_content(&contract_id, "Software Development Services Agreement");
    doc.add_entity_metadata(&contract_id, "contractType", "Services Agreement");
    doc.add_entity_metadata(&contract_id, "jurisdiction", "California, USA");
    doc.set_entity_confidence(&contract_id, 1.0);

    y -= 40.0;

    // Effective Date
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(50.0, y)
        .write("Effective Date:")?;

    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(160.0, y)
        .write("October 5, 2024")?;

    let effective_date_id = doc.mark_entity(
        "effective_date".to_string(),
        EntityType::EffectiveDate,
        BoundingBox::new(160.0, y as f32, 150.0, 15.0, 1),
    );
    doc.set_entity_content(&effective_date_id, "October 5, 2024");
    doc.add_entity_metadata(&effective_date_id, "value", "2024-10-05");
    doc.add_entity_metadata(&effective_date_id, "format", "ISO8601");
    doc.set_entity_confidence(&effective_date_id, 1.0);
    doc.relate_entities(&effective_date_id, &contract_id, RelationType::IsPartOf);

    y -= 30.0;

    // Parties Section
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("PARTIES")?;

    y -= 25.0;

    // Party 1 (Provider)
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, y)
        .write("1. Service Provider:")?;

    y -= 18.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, y)
        .write("TechCorp Solutions, Inc.")?;

    let provider_id = doc.mark_entity(
        "provider".to_string(),
        EntityType::ContractParty,
        BoundingBox::new(70.0, y as f32, 200.0, 15.0, 1),
    );
    doc.set_entity_content(&provider_id, "TechCorp Solutions, Inc.");
    doc.add_entity_metadata(&provider_id, "partyType", "Provider");
    doc.add_entity_metadata(&provider_id, "legalName", "TechCorp Solutions, Inc.");
    doc.add_entity_metadata(&provider_id, "role", "Service Provider");
    doc.set_entity_confidence(&provider_id, 0.99);
    doc.relate_entities(&provider_id, &contract_id, RelationType::IsPartOf);

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(70.0, y)
        .write("123 Innovation Drive, San Francisco, CA 94105")?;

    let provider_addr_id = doc.mark_entity(
        "provider_address".to_string(),
        EntityType::Address,
        BoundingBox::new(70.0, y as f32, 300.0, 12.0, 1),
    );
    doc.set_entity_content(
        &provider_addr_id,
        "123 Innovation Drive, San Francisco, CA 94105",
    );
    doc.add_entity_metadata(&provider_addr_id, "street", "123 Innovation Drive");
    doc.add_entity_metadata(&provider_addr_id, "city", "San Francisco");
    doc.add_entity_metadata(&provider_addr_id, "state", "CA");
    doc.add_entity_metadata(&provider_addr_id, "postalCode", "94105");
    doc.set_entity_confidence(&provider_addr_id, 0.98);
    doc.relate_entities(&provider_addr_id, &provider_id, RelationType::IsPartOf);

    y -= 25.0;

    // Party 2 (Client)
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, y)
        .write("2. Client:")?;

    y -= 18.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, y)
        .write("Global Industries LLC")?;

    let client_id = doc.mark_entity(
        "client".to_string(),
        EntityType::ContractParty,
        BoundingBox::new(70.0, y as f32, 200.0, 15.0, 1),
    );
    doc.set_entity_content(&client_id, "Global Industries LLC");
    doc.add_entity_metadata(&client_id, "partyType", "Client");
    doc.add_entity_metadata(&client_id, "legalName", "Global Industries LLC");
    doc.add_entity_metadata(&client_id, "role", "Client");
    doc.set_entity_confidence(&client_id, 0.99);
    doc.relate_entities(&client_id, &contract_id, RelationType::IsPartOf);

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(70.0, y)
        .write("456 Business Plaza, New York, NY 10001")?;

    let client_addr_id = doc.mark_entity(
        "client_address".to_string(),
        EntityType::Address,
        BoundingBox::new(70.0, y as f32, 280.0, 12.0, 1),
    );
    doc.set_entity_content(&client_addr_id, "456 Business Plaza, New York, NY 10001");
    doc.add_entity_metadata(&client_addr_id, "street", "456 Business Plaza");
    doc.add_entity_metadata(&client_addr_id, "city", "New York");
    doc.add_entity_metadata(&client_addr_id, "state", "NY");
    doc.add_entity_metadata(&client_addr_id, "postalCode", "10001");
    doc.set_entity_confidence(&client_addr_id, 0.98);
    doc.relate_entities(&client_addr_id, &client_id, RelationType::IsPartOf);

    y -= 35.0;

    // Terms Section
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("TERMS AND CONDITIONS")?;

    y -= 25.0;

    // Clause 1: Scope of Work
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, y)
        .write("1. Scope of Work")?;

    y -= 18.0;
    let clause1_text = "Provider agrees to develop and deliver a custom web application \
                        according to the specifications outlined in Exhibit A, including \
                        backend API development, database design, and frontend interface.";

    // Word wrap the text
    let words: Vec<&str> = clause1_text.split_whitespace().collect();
    let max_width = width - 140.0;
    let mut current_line = String::new();
    let mut line_y = y;

    for word in words {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if test_line.len() as f64 * 5.0 < max_width {
            current_line = test_line;
        } else {
            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(70.0, line_y)
                .write(&current_line)?;
            current_line = word.to_string();
            line_y -= 13.0;
        }
    }

    if !current_line.is_empty() {
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(70.0, line_y)
            .write(&current_line)?;
        line_y -= 13.0;
    }

    let clause1_id = doc.mark_entity(
        "clause_scope".to_string(),
        EntityType::ContractTerm,
        BoundingBox::new(
            50.0,
            (line_y - 5.0) as f32,
            (width - 100.0) as f32,
            (y - line_y + 18.0) as f32,
            1,
        ),
    );
    doc.set_entity_content(&clause1_id, clause1_text);
    doc.add_entity_metadata(&clause1_id, "clauseNumber", "1");
    doc.add_entity_metadata(&clause1_id, "clauseType", "Scope of Work");
    doc.add_entity_metadata(&clause1_id, "category", "Services");
    doc.set_entity_confidence(&clause1_id, 0.95);
    doc.relate_entities(&clause1_id, &contract_id, RelationType::IsPartOf);

    y = line_y - 10.0;

    // Clause 2: Payment Terms
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, y)
        .write("2. Payment Terms")?;

    y -= 18.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, y)
        .write("Total contract value: $150,000 USD, payable in three installments:")?;

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(85.0, y)
        .write("• 30% ($45,000) upon signing")?;

    y -= 13.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(85.0, y)
        .write("• 40% ($60,000) upon milestone completion")?;

    y -= 13.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(85.0, y)
        .write("• 30% ($45,000) upon final delivery")?;

    let contract_value_id = doc.mark_entity(
        "contract_value".to_string(),
        EntityType::ContractValue,
        BoundingBox::new(50.0, (y - 5.0) as f32, (width - 100.0) as f32, 60.0, 1),
    );
    doc.set_entity_content(
        &contract_value_id,
        "Total: $150,000 USD in three installments",
    );
    doc.add_entity_metadata(&contract_value_id, "totalAmount", "150000");
    doc.add_entity_metadata(&contract_value_id, "currency", "USD");
    doc.add_entity_metadata(&contract_value_id, "paymentStructure", "installments");
    doc.add_entity_metadata(&contract_value_id, "installmentCount", "3");
    doc.set_entity_confidence(&contract_value_id, 1.0);
    doc.relate_entities(&contract_value_id, &contract_id, RelationType::IsPartOf);

    y -= 25.0;

    // Clause 3: Term and Termination
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, y)
        .write("3. Term and Termination")?;

    y -= 18.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, y)
        .write("This agreement shall commence on the Effective Date and continue for")?;

    y -= 13.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(70.0, y)
        .write("a period of 12 months, unless terminated earlier as provided herein.")?;

    let term_clause_id = doc.mark_entity(
        "clause_term".to_string(),
        EntityType::ContractTerm,
        BoundingBox::new(50.0, (y - 5.0) as f32, (width - 100.0) as f32, 50.0, 1),
    );
    doc.set_entity_content(&term_clause_id, "12 month term from Effective Date");
    doc.add_entity_metadata(&term_clause_id, "clauseNumber", "3");
    doc.add_entity_metadata(&term_clause_id, "clauseType", "Term and Termination");
    doc.add_entity_metadata(&term_clause_id, "duration", "12 months");
    doc.add_entity_metadata(&term_clause_id, "category", "Duration");
    doc.set_entity_confidence(&term_clause_id, 0.95);
    doc.relate_entities(&term_clause_id, &contract_id, RelationType::IsPartOf);

    y -= 35.0;

    // Signature Section
    page.text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, y)
        .write("SIGNATURES")?;

    y -= 30.0;

    // Provider signature
    page.graphics()
        .set_stroke_color(Color::gray(0.3))
        .set_line_width(0.5)
        .move_to(50.0, y)
        .line_to(250.0, y)
        .stroke();

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, y)
        .write("Service Provider Signature")?;

    let provider_sig_id = doc.mark_entity(
        "provider_signature".to_string(),
        EntityType::Signature,
        BoundingBox::new(50.0, y as f32, 200.0, 30.0, 1),
    );
    doc.set_entity_content(&provider_sig_id, "Service Provider Signature");
    doc.add_entity_metadata(&provider_sig_id, "signatoryParty", "provider");
    doc.add_entity_metadata(&provider_sig_id, "signatureType", "written");
    doc.set_entity_confidence(&provider_sig_id, 0.90);
    doc.relate_entities(&provider_sig_id, &provider_id, RelationType::References);

    // Client signature
    y -= 30.0;
    page.graphics()
        .set_stroke_color(Color::gray(0.3))
        .set_line_width(0.5)
        .move_to(50.0, y)
        .line_to(250.0, y)
        .stroke();

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(50.0, y)
        .write("Client Signature")?;

    let client_sig_id = doc.mark_entity(
        "client_signature".to_string(),
        EntityType::Signature,
        BoundingBox::new(50.0, y as f32, 200.0, 30.0, 1),
    );
    doc.set_entity_content(&client_sig_id, "Client Signature");
    doc.add_entity_metadata(&client_sig_id, "signatoryParty", "client");
    doc.add_entity_metadata(&client_sig_id, "signatureType", "written");
    doc.set_entity_confidence(&client_sig_id, 0.90);
    doc.relate_entities(&client_sig_id, &client_id, RelationType::References);

    doc.add_page(page);

    // Save PDF
    let pdf_path = "examples/results/ai_ready_contract.pdf";
    doc.save(pdf_path)?;
    println!("✅ PDF saved to: {}", pdf_path);

    // Export semantic entities as JSON-LD
    #[cfg(feature = "semantic")]
    {
        let json_ld = doc.export_semantic_entities_json_ld()?;
        let json_path = "examples/results/ai_ready_contract_entities.jsonld";
        std::fs::write(json_path, &json_ld)?;
        println!("✅ JSON-LD entities exported to: {}", json_path);
        println!("\n📄 JSON-LD Preview (first 800 chars):");
        println!("{}", &json_ld[..json_ld.len().min(800)]);
        if json_ld.len() > 800 {
            println!("... (truncated)");
        }
    }

    #[cfg(not(feature = "semantic"))]
    {
        println!("\n⚠️  Run with --features semantic to export JSON-LD");
    }

    // Print summary
    println!("\n📊 Entity Summary:");
    println!("   Total entities: {}", doc.semantic_entity_count());
    println!("   Contract: 1");
    println!("   Parties: 2 (Provider + Client)");
    println!("   Addresses: 2");
    println!("   Contract Terms/Clauses: 3");
    println!("   Financial Terms: 1 (Contract Value)");
    println!("   Signatures: 2");
    println!("   Dates: 1 (Effective Date)");

    println!("\n🎯 Use Case:");
    println!("   This contract can now be processed by:");
    println!("   - Automated contract management systems");
    println!("   - Legal AI assistants for clause extraction");
    println!("   - Contract analytics platforms");
    println!("   - Due diligence automation tools");

    Ok(())
}
