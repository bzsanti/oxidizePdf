# oxidize-pdf Benchmarks

Benchmarks honestos y reproducibles para oxidize-pdf.

## 📊 Métricas Reales (Agosto 2025)

### Parser Performance
- **Corpus**: 759 PDFs reales (1.1GB total)  
- **Success Rate**: 98.8% (750/759)
- **Speed**: 42.6 PDFs/segundo promedio
- **Breakdown por tamaño**:
  - Small files (<100KB): 80.6 PDFs/seg
  - Medium files (100KB-1MB): 72.7 PDFs/seg  
  - Large files (>1MB): 12.2 PDFs/seg

### Writer Performance 

#### Documento Simple (2 líneas texto/página)
- 100 páginas: 4,851 páginas/seg
- 500 páginas: 12,647 páginas/seg
- 1,000 páginas: 19,895 páginas/seg

*Nota: Estos números son para contenido mínimo (2 líneas de texto por página). 
El rendimiento será significativamente menor con contenido real.*

## 🚀 Ejecutar Benchmarks

### Parser (lectura de PDFs)
```bash
python3 tools/benchmarks/benchmark_parser.py
```

### Writer (generación de PDFs)
```bash
python3 tools/benchmarks/benchmark_writer.py
```

## 📋 Resultados

Los benchmarks generan archivos JSON con resultados detallados:
- `parser_results.json` - Resultados del parser
- `writer_results.json` - Resultados del writer

## 🎯 Claims Honestos vs Anteriores

| Métrica | Claim Anterior | Medición Real | Status |
|---------|----------------|---------------|---------|
| Parser success rate | 97.2% | 98.8% | ✅ Mejorado |
| Parser speed | 215 PDFs/seg | 42.6 PDFs/seg | ❌ 5x más lento |
| Writer speed | 21,379 pág/seg | ~12,000 pág/seg | ❌ 2x más lento |

## ⚠️ Limitaciones Actuales

1. **Writer benchmarks** solo miden contenido muy simple
2. **PDFs complejos** (con tablas, gráficos, imágenes) tendrán rendimiento menor  
3. **Comparaciones** con otras librerías aún no implementadas
4. **Casos de uso reales** requieren benchmarks adicionales

## 🔧 Metodología

### Medición
1. **Múltiples iteraciones**: 3 ejecuciones por prueba
2. **Builds release**: Compilación optimizada
3. **I/O real**: Incluye escritura a disco
4. **Tiempo wall-clock**: Medición externa del proceso

### Métricas
- **Tiempo de ejecución**: Tiempo total del proceso
- **Throughput**: PDFs/segundo o páginas/segundo  
- **Success rate**: Porcentaje de éxito
- **Breakdown I/O**: Tiempo generación vs escritura

### Entorno
- **OS**: macOS (Darwin 24.6.0)
- **Rust**: 1.85+ optimizado
- **Hardware**: Desarrollo estándar

## ✅ Lo que oxidize-pdf hace bien

- **Confiabilidad**: 98.8% success rate con PDFs reales
- **Calidad**: PDFs válidos que renderizan correctamente
- **Features**: Set completo de funcionalidades
- **Zero deps**: Sin dependencias externas

## 📈 Realidad del Rendimiento

- **42.6 PDFs/seg** para parsing es competitivo
- **12K páginas/seg** para generación simple es respetable
- Performance real probablemente **comparable** a otras librerías Rust
- Claims de "rendimiento extremo" eran **exagerados**

## 🎖️ Filosofía

**Honestidad sobre hype. Data sobre claims. Confiabilidad sobre marketing.**

Estos benchmarks reemplazan claims de marketing con datos reales y reproducibles que los desarrolladores pueden confiar.