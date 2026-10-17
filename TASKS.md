### #731 — codificaciones simples coherentes — 2026-10-17

Issue: #731 — fix(text): preserve StandardEncoding and named simple-font mappings across extraction APIs — https://github.com/bzsanti/oxidizePdf/issues/731
Estado: implementación, validación y QR completos; publicación/CI pendientes. Prioridad: P1. Responsable: Codex.
Cierre: StandardEncoding implícita en Standard14 Type1 no simbólicas y explícita en ambas APIs; nombres soportados coherentes, ToUnicode/Differences y salvaguardas simbólicas preservados, controles no ASCII/Poppler, QR y CI antes de merge.
Última validación: OPEN y criterios confirmados en GitHub; inspección identifica fallback byte-as-char en extractor y TextEncoding incompleto en resolver. Siguiente acción: RED público en checkout /tmp/oxidize-issue731 basado en develop e8dbe772; compartir mapeos existentes sin ampliar #715.

## #741 — XMP namespace-aware PDF/A identifiers — 2026-10-17

Issue: #741 — fix(pdfa): recognize RDF attribute-form PDF/A identifiers — https://github.com/bzsanti/oxidizePdf/issues/741
Estado: implementación reconstruida, validación y QR completos; publicación/CI pendientes. Prioridad: P1. Responsable: Codex.
Cierre: atributos/elementos equivalentes por URI; errores tipados para metadatos inválidos/no soportados; límites XML, regresión, QR y CI verde antes del merge a develop.
Última validación: 85 integración PASS, incluido Python independiente y consumidor #346; 70 unidades PDF/A PASS, formato/diff-check y Clippy all-targets -D warnings PASS. QR manual y Kripteia calidad/seguridad completos sin nuevos hallazgos. Evidencia histórica RED preservada; reconstrucción guardada también fuera de /tmp.
Siguiente acción: publicar PR con informe/evidencia, verificar CI y fusionar según autorización del usuario. Después #763 cubre las demás propiedades y el escape de salida. No es certificación completa PDF/A ni publicación de release.
