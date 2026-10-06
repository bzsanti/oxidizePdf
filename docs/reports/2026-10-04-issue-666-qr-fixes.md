# Correcciones de los 13 hallazgos del QR #666 — 2026-10-04

## Resumen ejecutivo

Los 13 hallazgos del [QR](2026-10-04-issue-666-quality-review.md) están corregidos localmente. Las correcciones funcionales comparten resolución entre consumidores; las pruebas observan los resultados antes ausentes; se preserva la compatibilidad de los matches exhaustivos del enum anterior. No se cierra la matriz general de #666 ni se afirma CI remota o integración.

Base: `a9c47deec7f805e73533032b9d26c7f7f4c95708`, rama `fix/issue-668-simple-encodings`, con WIP preexistente preservado. Issues #666, #671, #673, #674 y #676 confirmadas OPEN antes de implementar. Evidencia y comandos: [directorio de correcciones](2026-10-04-issue-666-qr-fixes-evidence/).

## Contratos y evidencia

| Comprobación | Resultado |
|---|---|
| Sondas originales convertidas en integración, separando CFF y parent KR por diccionario | Antes: 1 PASS / 11 FAIL; después: 12 PASS. Ampliadas a 15 contratos, todos pasan. |
| Ciclos en fuentes inline/indirectas, página/Form, strict/lenient y control acíclico | Strict devuelve error UseCMap; lenient conserva el hijo X; controles válidos conservan X. |
| Guardas de transformaciones | Control: 9 PASS. Inyectar NaN después de extracción: 9 FAIL; antes del arreglo los nueve pasaban. |
| Observación GID | Control: 6 PASS. Reemplazar GID resultante por CID: fallan los contratos de stream y remapeo; Identity sigue pasando. Es inyección en la copia del test después de llamar al consumidor real, no una compilación mutada del producto. |
| Selección FD | Control del parser con cuatro programas reales: PASS. Copia de `parse_fd_select` forzada a FD0: FAIL. No se modifica el módulo original para esta mutación. |
| Procedencia Adobe | La misma entrada alterada del QR ahora se rechaza antes de escribir. Fuentes originales reproducen idénticos Rust y tres artefactos del oráculo CJK. |
| Compatibilidad | El test de integración compila un match exhaustivo de las cuatro variantes originales; Adobe-KR se resuelve por la nueva API. |

Las expectativas verticales de dos tests antiguos se corrigieron al valor normativo de DW2 ausente, `w1y=-1000`. Se preservaron las fuentes, entradas PDF y tablas Adobe: los tests antes confundían DW horizontal con avance vertical. La prueba independiente con W=544 y W2=-1200 ya demostraba esta divergencia en el QR.

## Hallazgos

1. **Type1 y procedimientos inertes — corregido (#674).** `oxidize-pdf-core/src/text/intrinsic_encoding.rs:35` distingue procedimientos, diccionarios literales y ámbitos begin/end al localizar Encoding. El fixture real BA permanece BA al añadir un procedimiento no ejecutado; controles adicionales cubren comentarios, strings, anidamiento y FontInfo. El lector sigue siendo estático y no ejecuta PostScript arbitrario.

2. **Herencia UseCMap del consumidor — corregido (#671).** `oxidize-pdf-core/src/fonts/resolved.rs:518` y `:546` usan los mismos resolvedores que extracción, con límite de stream de 8 MiB y profundidad 16. Los nueve contratos funcionales existentes de UseCMap también comprueban Unicode y avance por ResolvedFontResource, incluyendo sombra del hijo, cadenas y padres nominales.

3. **Errores estrictos descartados — corregido (#671).** `oxidize-pdf-core/src/text/extraction.rs`, `cache_fonts_from_resources`, propaga el error en strict y registra recuperación en lenient. Las entradas inline e indirectas y los recursos Form comparten esta frontera. Los 16 escenarios del control público pasan.

4. **Alias KR/Korea1 heredado — corregido (#676).** `oxidize-pdf-core/src/text/cmap.rs:496` selecciona KR de forma independiente. Tanto operador como diccionario producen 一 para CID14238; la precedencia explícita de ToUnicode continúa cubierta por la batería CJK.

5. **CIDSystemInfo indirecto y Registry — corregido (#676).** `oxidize-pdf-core/src/text/extraction_cmap.rs:18` centraliza resolución del diccionario y strings indirectos y exige Registry Adobe. Ambos consumidores usan este helper. La extracción UniJIS indirecta ya da á sin espacio espurio; los controles de registros privados/indirectos del consumidor siguen pasando.

6. **Codificación intrínseca en ResolvedFontResource — corregido (#674).** `oxidize-pdf-core/src/fonts/resolved.rs` incorpora el lector compartido de FontFile/FontFile3 y aplica ToUnicode, Differences y base intrínseca en ese orden. PFB/CFF completos y subconjuntos, con sus variantes explícitas, se verifican contra los 16 casos congelados del manifiesto Type1.

7. **Avance vertical del consumidor — corregido (#673).** `oxidize-pdf-core/src/fonts/resolved.rs:364` usa W2/DW2 mediante el parser compartido. Se verifica la diferencia entre ancho horizontal y avance vertical y el predeterminado -1000. No se cambia la API para exponer desplazamientos de dibujo: `advance` sigue representando avance, no origen del contorno.

8. **NaN aceptado por tests — corregido (#666).** `oxidize-pdf-core/tests/text_transform_contract_test.rs:39` exige coordenadas finitas antes de comparar tolerancias. La mutación demuestra que las nueve guardas ahora fallan.

9. **Enum exhaustivo público — corregido (#676).** `oxidize-pdf-core/src/text/cid_to_unicode.rs` conserva `CidCollection` con sus cuatro variantes originales. La nueva `AdobeCidCollection` es no exhaustiva y contiene las cinco colecciones; los consumidores internos usan esta API. El método legacy `from_ordering("KR")` devuelve None para evitar el alias incorrecto; esta corrección de comportamiento y la migración están documentadas. No se promete conservar la semántica errónea de KR.

10. **Generación sin verificar procedencia — corregido (#666/#676).** `tools/generate_adobe_pdf_cid_tables.py:78` y `tools/generate_text_cjk_oracles.py:52` verifican revisiones y hashes fijados en `tools/adobe_cjk_source_pins.json` antes de generar. Tres tests cubren rechazo sin sobrescritura y coherencia con procedencia independiente. La regeneración con archivos originales permanece idéntica.

11. **Cobertura GID/FD declarada sin observarla — corregido (#666).** `oxidize-pdf-core/tests/text_composite_font_contract_test.rs:51` comprueba CIDs y GIDs del consumidor en Identity, stream y remapeo, completos/subconjuntos. `oxidize-pdf-core/src/text/fonts/cff/dict.rs:445` ejercita FDSelect0/3 real y las dos Font DICT. Los nombres y F09/F10 distinguen estas observaciones de selección de contornos por un renderer, que sigue fuera de lo demostrado por estos tests. Las dos mutaciones discriminan las rutas.

12. **Documentación con resultados obsoletos — corregido (#666).** `oxidize-pdf-core/tests/fixtures/text_contracts/README.md:32` enlaza validación actual y etiqueta cifras antiguas como históricas; el README del tooling hace lo mismo. Se conserva la matriz como parcial y no se sustituyen oráculos por salidas del producto.

13. **Comentarios en funciones equivocadas — corregido (#666).** `oxidize-pdf-core/src/text/extraction.rs` asigna cada bloque a `cache_page_font`, `advance_text_pen`, `is_vertical_font`, `cids_for_codes` y `calculate_text_width_from_codes`, separando caché, detección de modo, lookup y unidades de avance.

## Validación conjunta

Resultado consolidado: **7134 PASS / 0 FAIL / 3 omitidas preexistentes**, 31 targets, más **16 unittest Python**. No es una ejecución de todo el workspace.

La ejecución agregada inicial (`validation.log`) terminó con 7132 PASS y dos expectativas verticales antiguas fallidas. Tras corregir únicamente esas aserciones se repitieron sus dos targets: **27/27 PASS**, en `vertical-consumer-final.log`. Los otros 29 targets —7107 PASS/3 omitidas— corresponden al mismo código de producto, que no cambió después de esa ejecución. El resultado consolidado no oculta ni reescribe el log inicial.

Clippy de biblioteca y todos los tests, Clippy posterior de los dos targets ajustados, formato, trazabilidad y diff-check pasan. Las pruebas Python y regeneración se repitieron tras el ajuste final del generador. Evidencia de mutaciones con controles separados en `mutations.json`.

Corpus: **72/72 pruebas diferenciales**. Fusiones: 1695 PDFs, 276/211815, idéntico al incremento previo. Orden: 1060 PDFs frente a 1058 anteriormente; tasas 0.185675 y 0.159549 frente a 0.185646 y 0.159521. Ambos gates pasan sus baselines, sin recalibrarlos. La población cambió: no se afirma equivalencia textual por PDF ni una mejora causal de orden. Los logs conservan diagnósticos y denominadores.

## Calidad de Tests (Kripteia)

Ambos análisis se ejecutaron en 12 alcances afectados; se repitió el generador después de su último ajuste. Text: **92/864 tests/48 archivos**; suite QR: **94/15**; compuestos: **100/6**; transformaciones: **72/9**; UseCMap: **91/10**; CID: **97/9**. Se añaden los dos targets finales CJK y consumidor al inventario.

La puntuación de transformaciones no aumenta por arreglar NaN: el scanner sigue penalizando wrappers, aunque las mutaciones demuestran ahora la guarda. Python y el archivo de implementación resolved detectan cero tests; sus puntuaciones no representan cobertura. Se ejecutaron los 16 unittest Python y los consumidores reales por separado.

## Análisis de Seguridad (Kripteia Security)

Las 12 ejecuciones finales no emiten alertas: `No security issues found.` Inspección manual de errores estrictos, límites compartidos, ausencia de ejecución PostScript y verificación de entradas de generación. No se introducen dependencias ni unsafe/FFI. No se convierte un scan limpio en garantía: no hubo fuzzing prolongado ni medición de presión de memoria en cadenas máximas.

## Métricas y alcance pendiente

- Hallazgos corregidos: **13/13**.
- Regresiones de integración nuevas: **15**; dos tests internos adicionales, más guardas ampliadas en baterías existentes y tres tests Python.
- Producto y tests modificados únicamente en el alcance autorizado; oráculos Adobe y PDFs preexistentes preservados. Nuevo fixture Type1 derivado documentado aparte.
- Compatibilidad de fuente del enum anterior preservada; corrección semántica de KR documentada.
- Pendiente de #666: C07/F07/F08 y resto de la matriz, revisión/integración y CI remota. La corrección del hallazgo de cobertura FD no equivale a implementar un renderer CFF.
- Sin commits, PR, merge, release ni cierre remoto de issues.
