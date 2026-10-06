# #666 F08 — programas Type3 y corrección #675

## Resultado

Ocho contratos nuevos,259 en28 baterías. Ejecución final:7087 PASS/0 FAIL/3 omitidas preexistentes en32 targets; biblioteca6800, contratos259 y consumidores28. Se detectó y corrigió inmediatamente un desbordamiento Type3 bajo #675, confirmada OPEN. F08 queda executable; revisión/CI/integración siguen pendientes.

## Contratos

La API Type3Font conserva dos magnitudes distintas: anchura declarada en Widths y displacement de d0/d1. La discrepancia se mantiene observable (contrato previo de #509), mientras ResolvedFontResource y TextExtractor usan Widths para avanzar. Una anchura de procedimiento900 no sustituye a la declarada500. ToUnicode fi mantiene un solo avance de glifo. Se prueban d0/d1, bbox y operaciones reales, anchuras cero/negativas/fraccionarias y ambos modos del parser.

Ocho programas sintácticamente dañados son rechazados por el consumidor gráfico con nombre/código del glifo: faltan métricas, aridad incorrecta, operadores previos, métricas duplicadas y operandos sobrantes. La extracción puede conservar texto y posiciones desde metadata válida aunque no pueda resolverse el programa gráfico. Esto no afirma que el documento dañado sea renderizable o conforme. Widths cortas/no numéricas también se rechazan por el resolvedor.

## Hallazgo y corrección

1. `src/fonts/type3.rs::apply_encoding_differences` validaba el entero explícito, pero no el contador incrementado antes del cast a u8. Después de255, otro nombre podía convertirse en código0. #675 cubre precisamente estos alias. La guarda añadida rechaza el valor fuera de0..255 antes del cast. RED6 PASS/1 FAIL; GREEN7/7; control final8/8 incluyendo255 válido y reinicios explícitos, en ambos modos. No se modifica la API pública ni se rechaza un reinicio legal del contador.

## Contraste externo

Siete PDFs regenerables y estructuralmente válidos para qpdf. MuPDF1.26.10 coincide exactamente en seis; para Widths125.5 sitúa B en101.25 frente a101.255. La discrepancia se explica por la llamada con anchura float en [pdf-type3.c](https://github.com/ArtifexSoftware/mupdf/blob/1.26.10/source/pdf/pdf-type3.c#L161) a [pdf_add_hmtx con parámetro int](https://github.com/ArtifexSoftware/mupdf/blob/1.26.10/include/mupdf/pdf/font.h#L106). Se conservan expectativa, tolerancia y desacuerdo; no se declara coincidencia7/7. qpdf verifica estructura, no semántica de CharProcs. La primera ejecución externa fallida y la posterior clasificación permanecen en los logs.

## Validación y límites

Comando completo, exit0, logs, scripts, resultados y hashes en `2026-10-04-issue-666-type3-program-evidence/`. Clippy, formato, trazabilidad y diff-check pasan. Kripteia tests99/8, producto98/4; ambos Security sin alertas. Los resultados de análisis estático no equivalen a conformance gráfica.

La búsqueda de consumidores de Type3Font::resolve sólo encuentra ResolvedFontResource. Por ello se ejecutan biblioteca, todas las baterías contractuales y consumidores #509/#513/#573; no se repite corpus TextExtractor. Sus cifras anteriores mantienen fecha y alcance, y no se cambian baselines. Comparación de huellas anteriores: sólo type3.rs y coverage.json cambian; se añade el nuevo target. Parche de producto de cinco líneas conservado en runtime.patch.

Siguiente bloque: F07, política TrueType simbólica sin ToUnicode. #666 y #675 permanecen abiertas; no hay nuevo bloqueo administrativo ni defecto confirmado pendiente de este incremento.
