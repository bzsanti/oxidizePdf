# #666: continuación C07/F07, 2026-10-04

Issue: #666 — test(text): build normative encoding and glyph-spacing contract batteries — https://github.com/bzsanti/oxidizePdf/issues/666 (OPEN reconfirmada).

## Resultado

Seis contratos nuevos del selector genérico TrueType y diagnóstico público de recuperación truncada. Pasan28/28 pruebas en cuatro targets. No cambia producto; #666, C07 y F07 permanecen incompletas. No se integra ni publica nada.

## Contratos F07

`text_competing_cmap_contract_test.rs` analiza un programa TrueType original con cinco subtables reales. El código65 selecciona GID4/anchura600 en plataforma Unicode, GID2/400 en Windows BMP y GID3/700 en Windows UCS4. Macintosh selecciona GID1; Windows Symbol usaF041→GID2. Las expectativas son literales y FontTools4.60.1 verifica independientemente las tablas, orden de glifos y métricas al generar.

Se comprueba la prioridad documentada de la API `(3,10) > (3,1) > (0,any)` en120 órdenes, y degradación a BMP en24 y Unicode en6. Otros dos órdenes verifican que el fallback no preferido conserva la primera tabla, junto a entrada vacía, ausencia de fusión de mapas y conservación del GID no BMP. Esto cubre152 permutaciones; no establece una prioridad normativa para todos los PDF simbólicos ni añade un oráculo visual.

La fuente y procedencia nuevas viven en `tests/fixtures/text_contracts/symbolic/competing/`. El generador offline exige FontTools4.60.1 y reutiliza únicamente construcción de contornos originales. Dos archivos regenerados idénticos byte a byte. No se alteran las fuentes/PDFs/oráculos anteriores.

## Hallazgo C07 pendiente de issue

1. `extraction_cmap.rs::decode_with_cmap` descarta los bytes no mapeados y `extraction.rs::decode_text` acepta el prefijo utilizable. Una fuente con codespaces explícitos de1–4 bytes produce `ABCD` para entrada completa; `A` seguida de cualquiera de tres colas parciales produce `A`, pero la cola aislada produceU+FFFD. Un código completo no mapeado también desaparece trasA. Dos strings deTJ mantienen la pérdida. Los siete casos se ejecutan en modo estricto y permisivo:14 resultados conservados.

La sonda utiliza PdfReader/TextExtractor públicos y CFF real. Es diagnóstico, no una regresión que consagre la pérdida como correcta. `ParseOptions::strict()` por sí solo no documenta una política universal de texto; queda por definir recuperación/error, aislamiento entre strings y distinción de CID0 intencional. Las issues abiertas consultadas no cubren este defecto; segúnAGENTS.md, TASKS lo registra bloqueado, a cargo de mantenimiento/bzsanti para crear/vincular una issue específica OPEN. No se inicia corrección de producto bajo#666.

## Validación y calidad

- Rust: boundary9, competing6, symbolic8, truetype5:28 PASS/0 FAIL.
- Clippy del nuevo target, formato, trazabilidad y diff-check pasan.
- Dos perturbaciones del programa de fuente: cambiar el GID preferido provoca2 fallos; quitar su tabla provoca3. Control6/6. Son mutaciones de entrada, no de ramas de producto.
- Kripteia Rust95/6 tests. Sus tres avisos de ausencia de llamadas a producción son falsos positivos: las funciones llaman al helper `priority`, que analiza el programa y ejecuta ambos selectores públicos. Expectativas literales deliberadas; unwrap falla el test y no se usa en producto.
- Kripteia Python no detecta tests (0 archivos): su100 no se presenta como cobertura. La generación, relectura independiente y regeneración se ejecutaron realmente. Ambos análisis Security sin alertas; no equivalen a prueba exhaustiva de seguridad.
- Instantánea anterior:560 archivos; único cambio, coverage.json. Producto, PDFs y oráculos previos idénticos. No se repite corpus; sus resultados anteriores conservan fecha/alcance.

Evidencia reproducible: `2026-10-04-issue-666-continuation-evidence/` incluye sonda fuente/log, comando de validación, logs, script de perturbaciones, resultados, resumen y hashes finales. La sonda se compila con rustc edición2021 contra el rlib local indicado y `-L dependency=target/debug/deps`; binarios/variantes en target, sin/tmp.

## Pendiente

F08: FontMatrix Type3 no escalar y avance vectorial. F07: política simbólica sinToUnicode y representación. C07: issue de recuperación antes de corrección. CI/integración y resto de matriz siguen abiertos; este incremento no cierra#666.
