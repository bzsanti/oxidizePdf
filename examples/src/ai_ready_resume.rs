//! AI-Ready Resume/CV Example
//!
//! Demonstrates creating a professional resume PDF with semantic entity marking for automated
//! resume parsing, skill extraction, and candidate screening.

use oxidize_pdf::semantic::{BoundingBox, EntityType};
use oxidize_pdf::{Color, Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("👤 Creating AI-Ready Resume/CV PDF...\n");

    // Create document
    let mut doc = Document::new();
    doc.set_title("Software Engineer Resume - John Smith");
    doc.set_author("oxidize-pdf");
    doc.set_subject("Professional Resume with Semantic Markup");

    // Create page
    let mut page = Page::a4();
    let width = page.width();
    let mut y = 750.0;

    // Header - Name
    page.text()
        .set_font(Font::HelveticaBold, 24.0)
        .at(50.0, y)
        .write("JOHN SMITH")?;

    let name_id = doc.mark_entity(
        "candidate_name".to_string(),
        EntityType::PersonName,
        BoundingBox::new(50.0, y as f32, 200.0, 25.0, 1),
    );
    doc.set_entity_content(&name_id, "John Smith");
    doc.add_entity_metadata(&name_id, "firstName", "John");
    doc.add_entity_metadata(&name_id, "lastName", "Smith");
    doc.set_entity_confidence(&name_id, 1.0);

    y -= 20.0;
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, y)
        .write("Senior Software Engineer")?;

    y -= 25.0;

    // Contact Information
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("📧 john.smith@email.com")?;

    let email_id = doc.mark_entity(
        "email".to_string(),
        EntityType::Email,
        BoundingBox::new(50.0, y as f32, 180.0, 12.0, 1),
    );
    doc.set_entity_content(&email_id, "john.smith@email.com");
    doc.add_entity_metadata(&email_id, "value", "john.smith@email.com");
    doc.set_entity_confidence(&email_id, 1.0);

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(250.0, y)
        .write("📱 (555) 123-4567")?;

    let phone_id = doc.mark_entity(
        "phone".to_string(),
        EntityType::PhoneNumber,
        BoundingBox::new(250.0, y as f32, 130.0, 12.0, 1),
    );
    doc.set_entity_content(&phone_id, "(555) 123-4567");
    doc.add_entity_metadata(&phone_id, "value", "+15551234567");
    doc.add_entity_metadata(&phone_id, "formatted", "(555) 123-4567");
    doc.set_entity_confidence(&phone_id, 0.98);

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("🌐 github.com/johnsmith")?;

    let website_id = doc.mark_entity(
        "github".to_string(),
        EntityType::Website,
        BoundingBox::new(50.0, y as f32, 180.0, 12.0, 1),
    );
    doc.set_entity_content(&website_id, "github.com/johnsmith");
    doc.add_entity_metadata(&website_id, "url", "https://github.com/johnsmith");
    doc.add_entity_metadata(&website_id, "platform", "GitHub");
    doc.set_entity_confidence(&website_id, 0.95);

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(250.0, y)
        .write("📍 San Francisco, CA")?;

    let location_id = doc.mark_entity(
        "location".to_string(),
        EntityType::Address,
        BoundingBox::new(250.0, y as f32, 150.0, 12.0, 1),
    );
    doc.set_entity_content(&location_id, "San Francisco, CA");
    doc.add_entity_metadata(&location_id, "city", "San Francisco");
    doc.add_entity_metadata(&location_id, "state", "CA");
    doc.set_entity_confidence(&location_id, 0.95);

    y -= 35.0;

    // Professional Summary
    page.graphics()
        .set_fill_color(Color::rgb(0.2, 0.4, 0.7))
        .rectangle(50.0, y - 5.0, width - 100.0, 20.0)
        .fill();

    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(55.0, y)
        .write("PROFESSIONAL SUMMARY")?;

    y -= 25.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Experienced software engineer with 8+ years in full-stack development,")?;

    y -= 13.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("specializing in Rust, Python, and cloud architecture. Proven track record")?;

    y -= 13.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("of delivering scalable systems and leading technical teams.")?;

    y -= 30.0;

    // Work Experience Section
    page.graphics()
        .set_fill_color(Color::rgb(0.2, 0.4, 0.7))
        .rectangle(50.0, y - 5.0, width - 100.0, 20.0)
        .fill();

    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(55.0, y)
        .write("WORK EXPERIENCE")?;

    y -= 25.0;

    // Job 1
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, y)
        .write("Senior Software Engineer")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(400.0, y)
        .write("2020 - Present")?;

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("TechCorp Inc., San Francisco, CA")?;

    let job1_org_id = doc.mark_entity(
        "job1_org".to_string(),
        EntityType::OrganizationName,
        BoundingBox::new(50.0, y as f32, 200.0, 12.0, 1),
    );
    doc.set_entity_content(&job1_org_id, "TechCorp Inc.");
    doc.add_entity_metadata(&job1_org_id, "name", "TechCorp Inc.");
    doc.add_entity_metadata(&job1_org_id, "location", "San Francisco, CA");
    doc.set_entity_confidence(&job1_org_id, 0.98);

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(60.0, y)
        .write("• Led development of microservices architecture serving 10M+ users")?;

    y -= 12.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(60.0, y)
        .write("• Reduced API response time by 60% through optimization")?;

    y -= 12.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(60.0, y)
        .write("• Mentored team of 5 junior engineers")?;

    y -= 12.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(60.0, y)
        .write("• Technologies: Rust, Python, PostgreSQL, AWS, Docker, Kubernetes")?;

    y -= 20.0;

    // Job 2
    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, y)
        .write("Software Engineer")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(400.0, y)
        .write("2018 - 2020")?;

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("StartupXYZ, Palo Alto, CA")?;

    let job2_org_id = doc.mark_entity(
        "job2_org".to_string(),
        EntityType::OrganizationName,
        BoundingBox::new(50.0, y as f32, 180.0, 12.0, 1),
    );
    doc.set_entity_content(&job2_org_id, "StartupXYZ");
    doc.add_entity_metadata(&job2_org_id, "name", "StartupXYZ");
    doc.add_entity_metadata(&job2_org_id, "location", "Palo Alto, CA");
    doc.set_entity_confidence(&job2_org_id, 0.98);

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(60.0, y)
        .write("• Built real-time data processing pipeline handling 1M events/day")?;

    y -= 12.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(60.0, y)
        .write("• Implemented CI/CD pipeline reducing deployment time by 80%")?;

    y -= 12.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(60.0, y)
        .write("• Technologies: Python, Django, Redis, RabbitMQ, Jenkins")?;

    y -= 25.0;

    // Education Section
    page.graphics()
        .set_fill_color(Color::rgb(0.2, 0.4, 0.7))
        .rectangle(50.0, y - 5.0, width - 100.0, 20.0)
        .fill();

    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(55.0, y)
        .write("EDUCATION")?;

    y -= 25.0;

    page.text()
        .set_font(Font::HelveticaBold, 11.0)
        .at(50.0, y)
        .write("Bachelor of Science in Computer Science")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(400.0, y)
        .write("2014 - 2018")?;

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, y)
        .write("Stanford University, Stanford, CA")?;

    let edu_org_id = doc.mark_entity(
        "education_org".to_string(),
        EntityType::OrganizationName,
        BoundingBox::new(50.0, y as f32, 220.0, 12.0, 1),
    );
    doc.set_entity_content(&edu_org_id, "Stanford University");
    doc.add_entity_metadata(&edu_org_id, "institutionType", "University");
    doc.add_entity_metadata(&edu_org_id, "degree", "Bachelor of Science");
    doc.add_entity_metadata(&edu_org_id, "major", "Computer Science");
    doc.add_entity_metadata(&edu_org_id, "graduationYear", "2018");
    doc.set_entity_confidence(&edu_org_id, 0.99);

    y -= 15.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(60.0, y)
        .write("• GPA: 3.8/4.0")?;

    y -= 12.0;
    page.text()
        .set_font(Font::Helvetica, 9.0)
        .at(60.0, y)
        .write("• Dean's List: 2016, 2017, 2018")?;

    y -= 25.0;

    // Skills Section
    page.graphics()
        .set_fill_color(Color::rgb(0.2, 0.4, 0.7))
        .rectangle(50.0, y - 5.0, width - 100.0, 20.0)
        .fill();

    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(55.0, y)
        .write("TECHNICAL SKILLS")?;

    y -= 20.0;
    page.text()
        .set_font(Font::HelveticaBold, 10.0)
        .at(50.0, y)
        .write("Languages:")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(150.0, y)
        .write("Rust, Python, JavaScript, Go, SQL")?;

    y -= 15.0;
    page.text()
        .set_font(Font::HelveticaBold, 10.0)
        .at(50.0, y)
        .write("Frameworks:")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(150.0, y)
        .write("Actix, Django, React, FastAPI")?;

    y -= 15.0;
    page.text()
        .set_font(Font::HelveticaBold, 10.0)
        .at(50.0, y)
        .write("Cloud & DevOps:")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(150.0, y)
        .write("AWS, Docker, Kubernetes, Terraform, CI/CD")?;

    y -= 15.0;
    page.text()
        .set_font(Font::HelveticaBold, 10.0)
        .at(50.0, y)
        .write("Databases:")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(150.0, y)
        .write("PostgreSQL, Redis, MongoDB, Elasticsearch")?;

    doc.add_page(page);

    // Save PDF
    let pdf_path = "examples/results/ai_ready_resume.pdf";
    doc.save(pdf_path)?;
    println!("✅ PDF saved to: {}", pdf_path);

    // Export semantic entities as JSON-LD
    #[cfg(feature = "semantic")]
    {
        let json_ld = doc.export_semantic_entities_json_ld()?;
        let json_path = "examples/results/ai_ready_resume_entities.jsonld";
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
    println!("   Personal Info: 5 (name, email, phone, GitHub, location)");
    println!("   Work Experience: 2 organizations");
    println!("   Education: 1 institution");

    println!("\n🎯 Use Case:");
    println!("   This resume can now be processed by:");
    println!("   - Automated Applicant Tracking Systems (ATS)");
    println!("   - AI-powered candidate screening tools");
    println!("   - Resume parsing and skill extraction systems");
    println!("   - Talent management platforms");

    Ok(())
}
