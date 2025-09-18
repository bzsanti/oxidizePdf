# 📋 METODOLOGÍA DE TESTEO - Problema de Extracción JPEG

## 🎯 Objetivo
**oxidize-pdf debe extraer JPEGs que funcionen exactamente igual que los extraídos por `pdfimages`**

## ⚠️ Problema Detectado
- Las imágenes JPEG extraídas por oxidize-pdf están malformadas
- Tesseract/Leptonica reporta: "Corrupt JPEG data: 17 extraneous bytes before marker 0xc4"
- La imagen de referencia (pdfimages) funciona correctamente con Tesseract

## 🔬 FASES DE VERIFICACIÓN (Secuenciales)

### **FASE 1: Extracción de Referencia** ✅
```bash
# Crear directorio para imagen de referencia
mkdir -p /tmp/reference

# Extraer imagen con pdfimages (herramienta estándar)
pdfimages -j -f 1 -l 1 "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf" /tmp/reference/fis2
```
**Output esperado**: `/tmp/reference/fis2-000.jpg` (imagen de referencia correcta)

### **FASE 2: Extracción con oxidize-pdf** ✅
```bash
# Ejecutar nuestro extractor con FIS2 PDF
cd /Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf
cargo run --features ocr-tesseract --example test_jpeg_verification
```
**Output esperado**: `oxidize-pdf-core/examples/results/extracted_1169x1653.jpg`

### **FASE 3: Verificación de Procesamiento con Tesseract** 🔍
```bash
# Probar imagen de referencia (debe funcionar)
tesseract /tmp/reference/fis2-000.jpg stdout 2>ref.err
echo "Errores en referencia: $(grep -c 'Corrupt' ref.err || echo 0)"

# Probar imagen de oxidize-pdf (actualmente falla)
tesseract oxidize-pdf-core/examples/results/extracted_1169x1653.jpg stdout 2>oxidize.err
echo "Errores en oxidize: $(grep -c 'Corrupt' oxidize.err || echo 0)"

# Mostrar errores específicos
echo "=== ERRORES REFERENCIA ==="
cat ref.err
echo "=== ERRORES OXIDIZE ==="
cat oxidize.err
```
**GATE**: Si oxidize tiene errores y referencia no → **FALLO**, ir a FASE 6

### **FASE 4: Comparación de Píxeles** 🖼️
```bash
# Convertir ambas a formato raw para comparar píxeles exactos
convert /tmp/reference/fis2-000.jpg -depth 8 rgb:ref.raw
convert oxidize-pdf-core/examples/results/extracted_1169x1653.jpg -depth 8 rgb:oxidize.raw

# Comparar píxeles byte por byte
echo "Comparando píxeles..."
if cmp -s ref.raw oxidize.raw; then
    echo "✅ PÍXELES IDÉNTICOS"
else
    echo "❌ PÍXELES DIFERENTES"
    echo "Primeras 20 diferencias:"
    cmp -l ref.raw oxidize.raw | head -20
fi

# Cleanup
rm -f ref.raw oxidize.raw
```
**GATE**: Si hay diferencias en píxeles → investigar causa

### **FASE 5: Comparación de Texto OCR** 📝
```bash
# Extraer texto de imagen de referencia
tesseract /tmp/reference/fis2-000.jpg stdout 2>/dev/null > ref_text.txt

# Extraer texto de imagen de oxidize-pdf
tesseract oxidize-pdf-core/examples/results/extracted_1169x1653.jpg stdout 2>/dev/null > oxidize_text.txt

# Comparar texto extraído
echo "Comparando texto OCR..."
if diff -q ref_text.txt oxidize_text.txt >/dev/null; then
    echo "✅ TEXTO OCR IDÉNTICO"
else
    echo "❌ TEXTO OCR DIFERENTE"
    echo "Diferencias:"
    diff ref_text.txt oxidize_text.txt
fi

# Mostrar estadísticas
echo "Caracteres en referencia: $(wc -c < ref_text.txt)"
echo "Caracteres en oxidize: $(wc -c < oxidize_text.txt)"
```
**GATE**: Si el texto es diferente → **FALLO**

### **FASE 6: Análisis de Diferencias Estructurales** 🔬
```bash
echo "=== ANÁLISIS DE ESTRUCTURA JPEG ==="

# Verificar existencia de archivos
ls -la /tmp/reference/fis2-000.jpg
ls -la oxidize-pdf-core/examples/results/extracted_1169x1653.jpg

# Comparar tamaños
echo "Tamaño referencia: $(wc -c < /tmp/reference/fis2-000.jpg) bytes"
echo "Tamaño oxidize: $(wc -c < oxidize-pdf-core/examples/results/extracted_1169x1653.jpg) bytes"

# Analizar primeros 200 bytes de cada imagen
echo "=== ESTRUCTURA JPEG - REFERENCIA ==="
xxd -l 200 /tmp/reference/fis2-000.jpg > ref_structure.txt
cat ref_structure.txt

echo "=== ESTRUCTURA JPEG - OXIDIZE ==="
xxd -l 200 oxidize-pdf-core/examples/results/extracted_1169x1653.jpg > oxidize_structure.txt
cat oxidize_structure.txt

# Comparar estructuras
echo "=== DIFERENCIAS EN ESTRUCTURA ==="
diff ref_structure.txt oxidize_structure.txt || echo "No hay diferencias en los primeros 200 bytes"

# Buscar marcadores JPEG específicos
echo "=== MARCADORES JPEG ==="
echo "Referencia - Marcadores:"
xxd /tmp/reference/fis2-000.jpg | grep -E "ff[cd][0-9a-f]" | head -10

echo "Oxidize - Marcadores:"
xxd oxidize-pdf-core/examples/results/extracted_1169x1653.jpg | grep -E "ff[cd][0-9a-f]" | head -10

# Cleanup
rm -f ref_structure.txt oxidize_structure.txt ref_text.txt oxidize_text.txt ref.err oxidize.err
```

## ✅ **CRITERIOS DE ÉXITO**
1. **FASE 3**: Ambas imágenes procesables por Tesseract **SIN ERRORES** ✓
2. **FASE 4**: Píxeles idénticos o visualmente equivalentes ✓
3. **FASE 5**: Texto OCR idéntico ✓

## ❌ **ESTADO ACTUAL CONOCIDO**
- **FASE 3**: ❌ **FALLA** - oxidize-pdf genera "Corrupt JPEG data: 17 extraneous bytes before marker 0xc4"
- **FASE 4**: ⏸️ No aplicable hasta resolver FASE 3
- **FASE 5**: ⏸️ No aplicable hasta resolver FASE 3

## 🎯 **PROBLEMA IDENTIFICADO**
En FASE 3: Tesseract/Leptonica detecta corrupción en el JPEG de oxidize-pdf pero NO en el de pdfimages.

**Error específico**: `Corrupt JPEG data: 17 extraneous bytes before marker 0xc4`

Esto indica que hay 17 bytes extra antes del marcador Huffman Table (0xFFC4) que no deberían estar ahí.

## 📊 **SIGUIENTE ACCIÓN**
Ejecutar **FASE 6** para identificar las diferencias estructurales exactas entre:
- JPEG extraído por pdfimages (correcto)
- JPEG extraído por oxidize-pdf (corrupto)

## 🔧 **SCRIPT DE EJECUCIÓN COMPLETA**
```bash
#!/bin/bash
echo "🧪 EJECUTANDO METODOLOGÍA COMPLETA DE TESTEO JPEG"
echo "=================================================="

# FASE 1
echo "FASE 1: Extrayendo imagen de referencia..."
mkdir -p /tmp/reference
pdfimages -j -f 1 -l 1 "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf" /tmp/reference/fis2

# FASE 2
echo "FASE 2: Extrayendo con oxidize-pdf..."
cd /Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf
cargo run --features ocr-tesseract --example test_jpeg_verification >/dev/null 2>&1

# FASES 3-6
echo "FASE 3: Verificando procesamiento con Tesseract..."
# [comandos de FASE 3]

echo "FASE 4: Comparando píxeles..."
# [comandos de FASE 4]

echo "FASE 5: Comparando texto OCR..."
# [comandos de FASE 5]

echo "FASE 6: Analizando diferencias estructurales..."
# [comandos de FASE 6]

echo "🏁 Metodología completada"
```

---

## 📊 **PROGRESO DEL DEBUGGING**

### ✅ **HALLAZGOS CONFIRMADOS**

#### **Problema Exacto Identificado:**
- **Referencia (pdfimages)**: 38,262 bytes ✅
- **Oxidize-pdf**: 38,279 bytes ❌ (+17 bytes extra)
- **Error Tesseract**: "Corrupt JPEG data: 17 extraneous bytes before marker 0xc4"

#### **Ubicación Exacta del Problema:**
- **EOI debería estar en**: posición 0x9574 (38260 decimal)
- **Referencia**: `28a2 8a28 a28a 28af ffd9` (correcto)
- **Oxidize**: `28a2 8a28 a28a 28a2 8a28 a28a 28a2 8a28 afff d9` (17 bytes duplicados)

#### **Causa Raíz:**
El parser está **duplicando 17 bytes de datos** antes de llegar al marcador EOI. El problema NO es la detección del EOI, sino que hay **duplicación de datos anterior**.

### ❌ **INTENTO DE FIX #1 - FALLIDO**
- **Modificación**: `src/parser/objects.rs` líneas 611-646
- **Objetivo**: Detectar EOI antes de añadir bytes al buffer
- **Resultado**: ❌ Aún produce 38,279 bytes
- **Razón del fallo**: El fix detecta EOI en posición 38279, pero la duplicación ocurre ANTES

### 🔍 **ANÁLISIS DE DUPLICACIÓN**
La duplicación exacta es:
```
Bytes duplicados: 28a2 8a28 a28a 28a2 8a28 (17 bytes)
```

Estos bytes aparecen DOS VECES en el stream, sugiriendo:
1. **Double-read del stream** en algún punto
2. **Buffer corruption** durante el parsing
3. **Referencia incorrecta** al stream data

### 🎯 **SIGUIENTE ACCIÓN REQUERIDA**
Investigar si el problema está en:
1. ❓ Resolución de referencias indirectas del stream length
2. ❓ Múltiples lecturas del mismo stream object
3. ❓ Buffer management en el lexer

### 📊 **PROGRESO ACTUAL - 2025-01-18**

#### **✅ AVANCES LOGRADOS:**

##### **1. Problema de Pérdida de Bytes Resuelto:**
- **Antes**: 37,057 bytes (se cortaba prematuramente)
- **Después**: 38,280 bytes (lee hasta endstream real)
- **Referencia**: 38,262 bytes (diferencia de solo 18 bytes)

##### **2. Mejora en Detección de Endstream:**
- **Problema identificado**: `lexer.peek_token()` perdía bytes al buscar "endstream"
- **Solución implementada**: Detección manual byte por byte en líneas 611-649 de `src/parser/objects.rs`
- **Resultado**: El parser ahora lee el stream completo hasta el marcador "endstream" real

##### **3. Infraestructura de Testing Establecida:**
- Metodología de verificación completa documentada
- Test de comparación con pdfimages implementado
- Proceso reproducible con `cargo run --example test_jpeg_verification`

#### **❌ PROBLEMAS SIN RESOLVER:**

##### **1. JPEG Sigue Corrupto - CRÍTICO**
```bash
$ tesseract extracted_1169x1653.jpg -
Corrupt JPEG data: 17 extraneous bytes before marker 0xc4
Error in pixReadStreamJpeg: read error at scanline 0
```

##### **2. OCR No Funciona**
- **Texto extraído**: `"ti  fh Fe esight alia  t -En ray sy*  em  S+ 7y"`
- **Estado**: Completamente ilegible, no es texto válido
- **Causa**: JPEG corrupto no puede ser procesado correctamente

##### **3. Diferencias Estructurales con Referencia**
- **Comparación binaria**: Archivos difieren desde el byte 87
- **Implicación**: Los datos JPEG internos son fundamentalmente diferentes
- **Hipótesis**: Problema en el pipeline de extracción, no solo en el parser de streams

#### **🔍 ANÁLISIS TÉCNICO:**

##### **El Fix Actual Es Parcial:**
- ✅ Resuelve la pérdida de bytes al final del stream
- ❌ NO resuelve la corrupción interna del JPEG
- ⚠️ Mejora necesaria pero insuficiente

##### **Problema Real Pendiente:**
Los 17 bytes extra antes del marcador 0xc4 indican que:
1. Hay datos adicionales en el stream que no deberían estar ahí
2. O falta algún procesamiento/filtrado del stream DCTDecode
3. O estamos leyendo el objeto incorrecto del PDF

#### **🎯 PRÓXIMOS PASOS REQUERIDOS:**
1. **Investigar pipeline DCTDecode**: ¿Se aplican filtros adicionales?
2. **Comparar con pdfimages**: ¿Cómo procesa exactamente el stream?
3. **Análisis de marcadores JPEG**: Identificar exactamente dónde están los 17 bytes extra
4. **Verificar objeto correcto**: ¿Estamos extrayendo el stream correcto del PDF?

#### **⚠️ ESTADO FINAL HONESTO:**
- **Problema**: Parcialmente diagnosticado y mejorado
- **Solución**: 30% completa (avance en tamaño, fallan datos internos)
- **Testing**: Metodología completa, evidencia clara del problema restante
- **Documentación**: Estado actual documentado honestamente

**❌ EL PROBLEMA NO ESTÁ RESUELTO - OCR SIGUE FALLANDO**

---

**REGLA FUNDAMENTAL**: Las imágenes deben ser **funcionalmente idénticas** - mismos píxeles, mismo comportamiento con Tesseract, mismo texto OCR extraído.