# #678 — recuperación de códigos de texto

Issue: #678 — fix(text): preserve undecodable character boundaries during text recovery — https://github.com/bzsanti/oxidizePdf/issues/678.

## Cambio y contrato

Se corrige la pérdida silenciosa reproducida durante #666. La extracción tolerante devuelve U+FFFD por código no decodificable o cola incompleta, conserva el texto sano y no combina bytes de strings Tj/TJ distintos. La política se aplica con ambas configuraciones de ParseOptions; no se añade una excepción estricta de extracción. ResolvedFontResource conserva su rechazo de códigos truncados.

- `extraction_cmap.rs::decode_with_cmap`: un fallo consume el código completo según codespace, o la cola parcial, y añade U+FFFD. Antes podía omitir bytes o reinterpretar un sufijo como otro carácter: `41814141` producía `AAA` en lugar de `A�A`. Se mantiene la recuperación existente de mapas simples con bfchar de un byte y codespace incorrecto de dos bytes; los mapeos explícitos exitosos conservan su precedencia.
- `decode_via_encoding_cmap`: rechaza para recuperación el código incompleto antes de consultar mappings; no recorta su longitud para convertirlo en un código menor válido.
- `decode_with_cid_table`: conserva CID0 como vacío y marca el byte impar final, incluso después de CID0.
- `encoding_cmap.rs::decode_utf16be`: marca el byte final impar de la recuperación para nombres Uni* no vendorizados. La prueba anterior que exigía descartarlo se cambia explícitamente al contrato de #678; no es una recalibración de corpus.

El recorrido avanza al menos un byte y el acceso a rangos se comprueba antes de indexar. El camino ordinario de mappings conserva sus consultas; el recorrido adicional de codespaces sólo se realiza tras un fallo. No se cambia la API pública ni el renderer.

## Evidencia de regresión

`text_recovery_boundary_contract_test.rs` contiene ocho pruebas públicas. RED inicial:1 PASS/4 FAIL. Se añadieron el alias de Encoding CMap y el fallback UTF16; este último obtuvo6 PASS/1 FAIL antes de su corrección. GREEN final:8/8. Una prueba adicional detectó y corrigió el consumo de dos bytes durante recuperación de fuentes simples con codespace incorrecto (7 PASS/1 FAIL antes de ajustar el ancho de recuperación). Se prueban todos los prefijos parciales de códigos de2–4 bytes, código no mapeado con sufijo que sí mapea, strings Tj/TJ separados, códigos completos, vacío, Identity/Adobe-Japan1, CID0 y Unicode no BMP.

La sonda pública original se vuelve a compilar:14 resultados finales en `public-probe-final.log`, con `A�` para colas parciales y códigos sin mapa, `�` para cola aislada y `A��` para dos strings parciales distintos. El control completo sigue dandoABCD. No se altera la evidencia RED anterior.

## Validación

Ejecución final única: **7157 PASS/0 FAIL/3 omitidas preexistentes,34 targets**. Biblioteca6800, contratos246, otras regresiones39 y diferenciales72.

Resultados finales y comandos: `2026-10-04-text-recovery-evidence/summary.json` y logs asociados. Las ejecuciones validation.log, final-contracts.log y corpus-final.log son intermedias; la validación final completa usa `final-validation.log`, tras actualizar la prueba de UTF16 y corregir la compatibilidad de fuentes simples. No se presenta el agregado intermedio como verde.

Clippy, formato, trazabilidad y diff-check se ejecutan. Los oráculos y PDFs anteriores se preservan; hashes documentan el alcance. No se modifica baseline ni umbral del corpus.

## Revisión de pruebas y seguridad

Kripteia del nuevo target:85/8 pruebas; Security sin alertas. Los avisos de “no production code” son falsos positivos del análisis del helper `recovered`; la ruta real es PdfReader→TextExtractor. “Only happy path” tampoco describe los inputs: son colas truncadas, alias inválidos y códigos no mapeados que recuperan con éxito por contrato. La proporción de asserts/fixtures no mide los casos de los bucles. Escaneos adicionales de ambos decodificadores conservados en `kripteia-{decoder,encoding}.log`; no sustituyen la inspección manual ni los resultados ejecutados.

## Estado

La corrección local se sigue en TASKS.md. #678 permanece abierta para revisión/integración; #666 conserva el resto de matriz pendiente. La instrucción del usuario queda incorporada: ante un fallo reproducido, crear/vincular la issue necesaria y corregirlo sobre la marcha.

## Corpus final

```text
differential fusion gate [t3-stress]: compared=1695 skipped=107 fusions=276 candidates=211815 rate=0.001303 content_coverage=0.9728
differential order gate [t3-stress]: compared=1060 skipped=742 pop_words=594863 common=583048 misplaced=98087 misplaced_rate=0.164890 transposed=86272 transposed_rate=0.147967 fidelity_micro=0.8520 median=1.0000 p10=0.8250 docs_below_0.95=213 alignment_coverage=0.9801 content_coverage=0.9942
differential order gate [t3-stress-reading-order]: compared=1060 skipped=742 pop_words=594863 common=583048 misplaced=82512 misplaced_rate=0.138708 transposed=70697 transposed_rate=0.121254 fidelity_micro=0.8787 median=1.0000 p10=0.8657 docs_below_0.95=198 alignment_coverage=0.9801 content_coverage=0.9942
```

Las cifras de orden cambian frente al QR previo (0.185675/0.159549;594837 palabras), con población1060 PDFs. No se afirma equivalencia por documento ni mejora semántica general; los gates pasan sin recalibración.
