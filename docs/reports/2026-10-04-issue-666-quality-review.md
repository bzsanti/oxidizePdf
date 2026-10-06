# Quality review del WIP #666 — 2026-10-04

## Resumen ejecutivo

Revisión terminada; el candidato todavía no está listo para integración. Se identifican 13 hallazgos: divergencias entre consumidores, pérdida de errores, una lectura incorrecta de Type1, huecos en los tests, compatibilidad pública y documentación. No se han aplicado correcciones.

Árbol revisado: rama `fix/issue-668-simple-encodings`, HEAD `a9c47deec7f805e73533032b9d26c7f7f4c95708`, con cambios locales acumulados de #666 y #671–#676. Las issues se confirmaron OPEN en GitHub durante la revisión. El alcance comprende extracción, CMaps, fuentes resueltas, codificación intrínseca, métricas, tablas generadas, baterías y generadores de contratos. Se excluyen los cambios ajenos de #620/#639 y ejemplos OmniDocBench. Los defectos de consumidores que ya existían se presentan como huecos del candidato acumulado, sin atribuirlos todos al último incremento.

Se hicieron dos pasadas: reconstrucción de contratos y contraejemplos, seguida de inspección de implementación, consumidores y aserciones. Las sondas son copias independientes; producto, tests y fixtures originales permanecen intactos. [Evidencia completa](2026-10-04-issue-666-qr-evidence/) y [huellas del árbol revisado](2026-10-04-issue-666-qr-evidence/source-hashes.json).

## Contratos y evidencia

| Contrato y fuente | Productor → consumidor / entrada | Evidencia y estado |
|---|---|---|
| Heredar Encoding/ToUnicode antes de resolver códigos; fixtures independientes UseCMap | CMap y diccionario de stream → `ResolvedFontResource::from_page/decode_glyphs` | Control plano verde; tres formas heredadas rojas. Demostrado en `probe.log`. |
| Rechazar ciclo UseCMap en modo estricto; contrato de #671 | Resolución de fuente → `PdfDocument::extract_text`, `ParseOptions::strict()` | Devuelve texto `A` con éxito tras perder el error de fuente. Demostrado. Falta validar la corrección futura. |
| KR y Korea1 son colecciones distintas; muestra Adobe fijada CID14238→U+4E00 | Parent CMap nominal → extractor público | Operador `usecmap` produce `쑑`, se esperaba `一`. Demostrado. La segunda variante del bucle, diccionario, no se ejecutó después del primer fallo. |
| Referencia indirecta conserva CIDSystemInfo | Diccionario PDF → extracción con UniJIS | Produce espacio espurio antes de `á`. Demostrado. El consumidor resuelto ya tiene controles positivos de referencias indirectas en su batería existente. |
| Encoding estático ejecutado del programa Type1; FontTools y MuPDF | FontFile → lector intrínseco → extractor público | Añadir un procedimiento no ejecutado cambia BA→AB. Qpdf acepta estructura; FontTools da B,A y MuPDF BA. Poppler da AB: desacuerdo externo registrado, no consenso inventado. |
| Base intrínseca sin Encoding PDF; fixture PFB con B,A | Programa → `ResolvedFontResource::decode_glyphs` | Devuelve A,B. Demostrado para PFB; CFF comparte fallback por inspección, pero el bucle se detuvo antes de ejecutarlo. |
| Avance vertical W2/DW2 en unidades de glifo | Métricas PDF → `DecodedGlyph.advance` | Modo vertical reconocido, devuelve W=544 en vez de W2=-1200. Demostrado. |
| Coordenadas finitas y cercanas al oráculo | Extractor real → helper de transformaciones | Mutación posterior a extracción reemplaza x/y por NaN; los nueve tests siguen verdes. Demostrado sobre copia de tests. |
| Selección GID/FD observable en baterías que la anuncian | Programas compuestos → seis tests F09/F10 | Aserciones observan texto, posiciones y metadatos, no GID/FD seleccionado. Demostrado por trazado; mutación del producto a Identity/FD constante no ejecutada. |
| Procedencia fija de entradas generadas | Directorio upstream → generador Adobe | Entrada GB1 alterada aceptada y etiquetada con revisión fija. Demostrado, sin alterar originales. |
| Compatibilidad de enum público | `CidCollection` → consumidor externo con match exhaustivo | E0004 al añadir Kr. Demostrado; decisión de versionado/migración pendiente. |
| Regresión ordinaria | Seis targets Rust + unittest Python | 69/69 y 13/13 verdes; Clippy y formato pasan. No sustituyen las sondas rojas. |

Las diez sondas adicionales dieron **1 PASS / 9 FAIL**, agrupadas en siete hallazgos funcionales; no son nueve defectos independientes. La mutación NaN dio **9 PASS** y demuestra un falso positivo. Comandos y código: [probe-command.json](2026-10-04-issue-666-qr-evidence/probe-command.json), [probe.rs](2026-10-04-issue-666-qr-evidence/probe.rs), [probe.log](2026-10-04-issue-666-qr-evidence/probe.log), [mutación NaN](2026-10-04-issue-666-qr-evidence/transform-nan-mutation.rs).

La ejecución anterior del mismo código aportaba 7117 PASS / 0 FAIL / 3 omitidas y corpus de 1695 PDFs; se conserva como evidencia previa, no como una ejecución nueva del QR ni como validación de los casos ausentes. No se recalibraron baselines. No se ha probado todavía que una corrección elimine las sondas rojas.

## Hallazgos

1. **El lector Type1 toma una definición dentro de un procedimiento no ejecutado.** `oxidize-pdf-core/src/text/intrinsic_encoding.rs:35` busca el primer token `/Encoding` sin considerar anidamiento y termina en el primer `def`. Insertar `/Unused { /Encoding StandardEncoding def } def` antes de la definición real cambia la extracción de BA a AB aunque la codificación ejecutada sigue siendo B,A. La sonda conserva un programa válido y sus longitudes; FontTools y MuPDF confirman el significado, con discrepancia de Poppler expresamente registrada. Corrección: reconocer el ámbito de la definición estática, omitir cuerpos no ejecutados y rechazar construcciones no soportadas; añadir el par programa original/procedimiento inerte. Issue #674.

2. **El consumidor resuelto omite la herencia UseCMap.** `oxidize-pdf-core/src/fonts/resolved.rs:502` y `:532` llaman a parsers del contenido sin resolver padres del documento. El fixture plano da CID17/29 y AB; las variantes heredadas pierden CIDs o Unicode, y el padre nominal de diccionario decodifica cuatro códigos en vez de dos. Corrección: compartir el resolvedor de cadenas con extracción, incluyendo diccionario del stream, WMode, precedencia del hijo, ciclos y límites; ejecutar los mismos contratos por ambas APIs. Issue #671.

3. **La API pública oculta el error de UseCMap estricto.** `oxidize-pdf-core/src/text/extraction.rs:3434` y `:3468` descartan `Err` al cargar fuentes. Un ToUnicode cíclico que debe rechazarse acaba en `Ok` y texto de fallback `A`. Las pruebas internas de resolución no protegen esta frontera pública. Corrección: propagar el error en modo estricto y definir recuperación explícita en lenient; añadir pruebas por página y Form que entren por la API pública. Issue #671.

4. **La herencia ToUnicode todavía confunde Adobe-KR con Korea1.** `oxidize-pdf-core/src/text/cmap.rs:495` agrupa ambos nombres pese a que la nueva tabla ya distingue las colecciones. El CID14238 heredado produce `쑑` en lugar de `一`. Corrección: resolver `Adobe-KR-UCS2` como KR y cubrir padres por operador y diccionario, secuencias suplementarias y precedencia del hijo. Issue #676.

5. **La extracción no resuelve CIDSystemInfo indirecto.** `oxidize-pdf-core/src/text/extraction_cmap.rs:207` solo admite diccionario y Ordering directos; tampoco comprueba Registry. Un UniJIS válido con CIDSystemInfo indirecto pierde la colección y añade un espacio a `á`. El helper nuevo del consumidor resuelto sí trata estas condiciones, dejando dos políticas distintas. Corrección: reutilizar una resolución común de diccionario, Registry y Ordering, con pruebas de referencias indirectas y registro privado; no aplicar tablas Adobe solo por el nombre Ordering. Issue #676.

6. **ResolvedFontResource ignora la codificación intrínseca Type1.** `oxidize-pdf-core/src/fonts/resolved.rs:394` cae en StandardEncoding cuando no hay Encoding PDF, aunque exista programa embebido. El PFB congelado B,A produce A,B por esta API, mientras pasa por el extractor de texto. Corrección: incorporar la misma base intrínseca Type1/CFF y aplicar Differences sobre ella, manteniendo precedencia de ToUnicode; probar ambos consumidores y ambos formatos. El fallo ejecutado es PFB; CFF requiere aún la sonda separada. Issue #674.

7. **DecodedGlyph.advance usa W horizontal en modo vertical.** `oxidize-pdf-core/src/fonts/resolved.rs:343` y `:560` solo consumen W/DW. Con Identity-V y W2=-1200 devuelve 544 aun reconociendo WritingMode::Vertical. Corrección: compartir el parser W2/DW2 y devolver el avance del eje activo conforme al contrato público; distinguir explícitamente avance y desplazamiento de origen si se amplía la API. Añadir controles verticales con valores distintos de W. Issue #673.

8. **Los nueve tests de transformaciones aceptan NaN.** `oxidize-pdf-core/tests/text_transform_contract_test.rs:39` comprueba error mediante `abs() > epsilon`; con NaN la condición es falsa. La copia de la batería sigue completamente verde tras reemplazar ambas coordenadas por NaN. Corrección: exigir `is_finite()` y proximidad mediante una aserción positiva, por ejemplo `assert!(actual.is_finite() && (actual - expected).abs() <= epsilon)`; conservar esta mutación como control de la guarda. Issue #666.

9. **La nueva variante pública rompe matches exhaustivos existentes.** `oxidize-pdf-core/src/text/cid_to_unicode.rs:14` añade Kr a un enum exhaustivo público. Un consumidor que cubría sus cuatro variantes deja de compilar con E0004. No se propone restaurar el alias semánticamente incorrecto. Corrección: decidir y documentar la ruptura y su versión/migración, o mantener la API previa y usar una representación interna distinta con una nueva interfaz extensible. Añadir un consumidor de compatibilidad; poner `non_exhaustive` ahora también requiere valorar la ruptura. Issue #676.

10. **El generador atribuye entradas alteradas a una revisión fija.** `tools/generate_adobe_pdf_cid_tables.py:60` calcula hashes de lo recibido y publica URLs del REVISION constante sin verificar los hashes esperados. Cambiar GB1 CID99 de U+00B7 a U+0041 se acepta y conserva la atribución. El hash de salida detecta diferencias después, pero no acredita que el input pertenezca a esa revisión. Corrección: verificar entradas contra un inventario fijado antes de generar; separar explícitamente actualización de referencia y reproducción. Añadir prueba de rechazo de entrada alterada y reproducción idéntica. Issue #666/#676.

11. **Los contratos compuestos no observan el GID/FD que anuncian.** `oxidize-pdf-core/tests/text_composite_font_contract_test.rs:38`, `:59` y `:71` verifican texto ToUnicode, posiciones derivadas de W y metadatos congelados; no comparan el GID ni la selección FD producidos por el código. Ignorar esos datos puede mantener las aserciones verdes. Corrección: añadir observación de GID por `decode_glyphs` para Identity/stream/remapeo y una entrada real al selector CFF/FD; donde no exista consumidor, marcar esa parte pendiente y estrechar el nombre/alcance de la prueba. La mutación propuesta de selección constante no se ejecutó. Issue #666.

12. **El README presenta resultados históricos como estado actual.** `oxidize-pdf-core/tests/fixtures/text_contracts/README.md:51` y `:63` aún anuncian 131/149 y dos fallos Type1, mientras el árbol actual tiene 208 contratos verdes. Corrección: etiquetar esos resultados como históricos y enlazar la validación vigente con su alcance; conservar los oráculos y no convertir el verde en afirmación de soporte completo. Issue #666.

13. **Comentarios de contrato quedaron unidos a funciones distintas.** `oxidize-pdf-core/src/text/extraction.rs:3485` documenta caché de fuentes sobre `advance_text_pen`; `:4554` describe decodificación de CIDs y retorno Option sobre `is_vertical_font`, que devuelve bool. Corrección: mover cada bloque a su función y documentar de forma separada detección vertical, avance del pen y lookup de CIDs. Esto evita interpretar unidades y políticas de fallback a partir de documentación incorrecta. Issue #666.

## Calidad de Tests (Kripteia)

Se ejecutó el script oficial de la skill, que invoca ambos análisis, en **44 alcances**: módulo text, consumidor resuelto, 25 targets Rust y 17 alcances Python. [Resultados estructurados](2026-10-04-issue-666-qr-evidence/kripteia-summary.json).

`src/text`: **92/100**, 862 tests en 47 archivos detectados. Los 23 targets de contratos suman 208 tests: transformaciones **72/9**, scripts **85/8**, TJ/Tz **88/4**, Type1 **88/10**, spacing **90/14** y CID **90/5** entre las puntuaciones más bajas. CJK da **94/11** y el consumidor resuelto **96/16** en su suite de integración.

Las advertencias de ausencia de llamadas/asserts en wrappers se contrastaron con los helpers: sí llegan al extractor real. No se elevan automáticamente a hallazgos. La guarda NaN y la falta de observación GID/FD sí se sostienen por evidencia independiente. Los 17 alcances Python y el archivo de implementación resolved reportan cero tests detectados; su 100 no acredita cobertura. Las 13 pruebas Python se ejecutaron realmente fuera del scanner y pasaron.

Los 69 tests focalizados pasan; Clippy con `-D warnings` y formato pasan. El scanner no detectó las divergencias públicas demostradas por las sondas. Falta incorporar las regresiones nuevas, obtener rojo/verde con correcciones y completar los contratos pendientes de la matriz.

## Análisis de Seguridad (Kripteia Security)

Las **44 ejecuciones** finalizaron con `No security issues found.` No emitieron alertas de secretos, taint, funciones peligrosas ni unsafe/FFI en los alcances examinados. Es un resultado del análisis estático, no una garantía de seguridad; su detección limitada de Python queda registrada arriba.

La inspección revisó entradas no confiables de PDF, límites de CMap y parsing intrínseco, errores/ciclos, rutas del tooling y atribución de recursos. No hay una vulnerabilidad adicional demostrada por esta revisión. La pérdida de errores estrictos y la procedencia no verificada están descritas como hallazgos concretos, sin exagerarlas a explotación. No se realizaron pruebas de presión de memoria con cadenas máximas de UseCMap ni fuzzing prolongado. No se añadieron dependencias ni se modificaron servicios externos.

## Métricas

- Archivos revisados: núcleo del cambio de 11 archivos Rust de producto/pruebas internas, 26 archivos Rust de integración/helpers y 15 generadores; revisión complementaria de validadores, metadatos y documentación. El scanner de text abarca 47 archivos y no equivale a inspección manual íntegra de todos ellos.
- Recursos CMap comprobados por hash: 35. Inventario protegido: 554 archivos; las tablas grandes se revisaron mediante generador, muestras y procedencia, no leyendo cada fila manualmente.
- Hallazgos totales: **13**.
- Archivos de producto, tests y fixtures originales modificados por la revisión: **0**.
- Validación nueva: **69 Rust + 13 Python PASS**; sondas **1 PASS / 9 FAIL**; mutación de coordenadas **9 PASS**; **44 pares de análisis Kripteia**.
- Verificaciones pendientes: rojo/verde tras correcciones; CFF y segunda forma de parent KR en sondas independientes; observación/mutación de GID y FD; presión de recursos; decisión de compatibilidad; C07/F07/F08 y resto de matriz; CI/integración. No se declara completada #666 ni ninguna corrección por haber terminado el QR.
