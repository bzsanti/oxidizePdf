# CLAUDE.local.md - Quick Reference

## 🔍 Search Patterns
```bash
# Find implementation
grep -r "impl.*StructName" src/
# Find trait usage  
grep -r "trait.*TraitName" src/
# Find TODOs
grep -r "TODO\|FIXME" src/
```

## 📁 Key Files (when needed)
- **Parser**: `src/parser/document.rs` (1886 lines)
- **Writer**: `src/writer/pdf_writer.rs` (3912 lines) 
- **Graphics**: `src/graphics/pdf_image.rs` (2006 lines)
- **OCR**: `src/text/ocr.rs` (1879 lines)

## 🐛 Common Issues & Solutions
- **PNG tests failing**: Non-critical, compression data issues
- **Encrypted PDFs**: Expected failures, not supported
- **Borrow checker**: Use blocks `{ }` to limit scope

## 📊 Performance Checks
```bash
# Run benchmarks
cargo bench
# Profile with release build
cargo build --release && time ./target/release/oxidize-pdf
# Check binary size
du -h target/release/oxidize-pdf
```

## 🔗 lib.rs Feed Issues
Check: https://lib.rs/~bzsanti/dash.xml

## 💡 Context Optimization Tips
- Use Grep before Read
- Read specific line ranges
- Keep this file under 50 lines
- Reference docs/ instead of copying
- los analisis estrategicos siempre son confidenciales y se tienen que almacenar en el directorio .private
- Cuando trabajes en oxidize-pdf:
1. SIEMPRE lee .private/roadmap_detailed_executable.md PRIMERO
2. NO implementes nada que no esté en ese roadmap
3. Si una feature parece necesaria pero no está:
   - Pregunta al humano
   - No la implementes sin confirmación