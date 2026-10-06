# QR F08 — Type3, 2026-10-04

## Resumen ejecutivo

Revisión terminada con **seis hallazgos**; F08 no está listo para aceptación/integración. Alcance: contratos y resolvedores Type3, interpretación de CharProcs, métricas, pruebas y scripts de contraste de los dos incrementos F08. HEAD `a9c47deec7f805e73533032b9d26c7f7f4c95708`, rama `fix/issue-668-simple-encodings`, con WIP preservado. Rust2021/MSRV1.88; configuración predeterminada compression/external-images. Issues#666/#675 confirmadas OPEN.

La guarda nueva de #675 evita el wrap del contador directo y conserva255/reinicios legales. Los cuatro problemas de producto descritos abajo están en rutas ya presentes antes de esa guarda; el QR del comportamiento F08 los descubre, no los atribuye a las cinco líneas del último parche. Dos hallazgos adicionales afectan a la fuerza de las pruebas/evidencia. No se han aplicado correcciones durante este QR.

## Contratos y evidencia

| Contrato | Consumidor y caso adversarial | Evidencia / estado |
|---|---|---|
| La forma indirecta de un Encoding mantiene su significado | Type3Font/ResolvedFontResource; Differences `[65 /B]` directo frente a referencia | Demostrado: B directo, A indirecto; TextExtractor conserva BB en ambos. Ambos modos |
| Falta de CharProc no inventa metadatos de otra codificación | ResolvedFontResource; código65→fi, Widths500, programa ausente | Demostrado: glyph=None, pero decode_glyphs devuelve A/avance0; Widths null también se acepta |
| Métricas entregadas a consumidores finitas o error explícito | Matriz decimal finita10^307; d0 con decimal10^40 | Demostrado: advance=inf y procedure_width=(inf,0), respectivamente, también estricto |
| Límite de recursos efectivo por fuente |128 códigos comparten un programa de2048 operaciones | Demostrado:13.971 bytes de PDF,20.971.520 bytes de capacidad de vectores retenidos,128 buffers distintos |
| Conservación de operaciones, no sólo cantidad | Copia desechable cambia rectángulo/relleno por moveto/stroke | Demostrado: control8/8 y perturbación8/8; no prueba una mutación de rama de producto |
| Discrepancia externa limitada a la conocida | Copia del script cambia anchura fraccionaria125.5→250.5 | Demostrado: B pasa a102.5 frente a esperado101.255, script termina0 |
| Guarda #675 y contratos normales | Cinco targets existentes, incluidos #509/#513/#573 |46/46 PASS; Clippy y formato pasan. No neutraliza los contraejemplos anteriores |

Evidencia reproducible en `2026-10-04-f08-qr-evidence/`: sondas fuente/log, capacidades medidas, controles/perturbaciones, resultados de tests, Clippy, formato y los seis escaneos Kripteia. Binarios y PDFs auxiliares en target. Ningún corpus se recalibra; no se vuelve a ejecutar corpus para esta revisión sin cambios de producto.

## Hallazgos

1. **Encoding indirecto pierde sus overrides y puede eludir rechazos.** `oxidize-pdf-core/src/fonts/type3.rs:251` y `:266`: BaseEncoding se consulta como nombre directo y Differences como array directo; no se resuelven sus referencias. La sonda indirect-diffs resuelve el código65 como A en vez de B mientras TextExtractor devuelve BB. Un BaseEncoding indirecto `/BogusEncoding` se trata como StandardEncoding en vez de recibir el rechazo del equivalente directo. Corrección: pasar el documento a la resolución de nombres, resolver BaseEncoding/Differences y sus elementos y distinguir ausente de tipo incorrecto/referencia fallida. Añadir pares directos/indirectos válidos, inválidos y referencias sin resolver; la guarda #675 también debe alcanzarse por esa ruta.

2. **La ausencia del programa borra metadatos y omite validaciones.** `oxidize-pdf-core/src/fonts/type3.rs:100`, `:119` y `oxidize-pdf-core/src/fonts/resolved.rs:178`: se saltan códigos sin CharProc antes de leer sus anchuras; luego el resolvedor reconstruye tanto los nombres como las anchuras exclusivamente desde los glifos disponibles. Para código65→fi y Widths500 se entrega Unicode A y avance0. Reemplazar Widths por null en esa posición tampoco produce error. Es distinto del comportamiento documentado glyph=None que permite fallback: se está fabricando éxito con metadatos ajenos. Corrección: validar y preservar encoding/Widths por código independientemente del programa; exponer ausencia del dibujo sin sustituir silenciosamente Unicode/avance, o devolver un resultado explícitamente no resuelto si ésa es la política acordada. Pruebas con CharProc presente/ausente y Widths válido/inválido.

3. **Se entregan infinitos como métricas válidas.** `oxidize-pdf-core/src/parser/content.rs:586`, `:1008` y `oxidize-pdf-core/src/fonts/resolved.rs:179`: parse::<f32> acepta un decimal grande como infinito y los productos de FontMatrix/Widths tampoco comprueban finitud. Las sondas usan números decimales finitos, sin notación exponencial ni inyección de objetos Rust:10^40 en d0 produce procedure_width inf y10^307 en FontMatrix produce advance inf. Corrección: comprobar representabilidad/finitud al parsear operandos y después de normalizar métricas, con error contextual en la API que valida el programa; definir recuperación explícita para extracción tolerante. Añadir controles grandes finitos aceptables y desbordamientos, sin convertirlos silenciosamente en cero.

4. **El límite por CharProc no impide amplificación de memoria por alias.** `oxidize-pdf-core/src/fonts/type3.rs:95`, `:108`, `:114`: se decodifica/analiza y retiene otro vector para cada código, incluso si todos apuntan al mismo programa. Con1/32/128 alias, la capacidad retenida de operaciones crece163.840/5.242.880/20.971.520 bytes; los PDFs pesan13.080/13.298/13.971 bytes. No es una estimación de RSS ni un pico de memoria: sólo capacidad viva de esos vectores, excluyendo otros gastos. El espacio de códigos está limitado a256, pero no hay presupuesto acumulado de operaciones/memoria por fuente;8MiB por stream no limita esa amplificación. Corrección: presupuesto acumulado verificado antes de expandir/retener cada programa y reutilización de análisis de CharProcs compartidos donde lo permita la API, preservando compatibilidad. Probar alias repetidos y programas distintos contra el límite global.

5. **El contrato de conservación de operaciones sólo comprueba su cantidad.** `oxidize-pdf-core/tests/text_type3_program_boundary_contract_test.rs:57`: `operations.len()==2` no verifica Rectangle/Fill ni sus operandos/orden. La copia de la batería que cambia `0 0 400 600 re f` por `1 1 m S` sigue pasando8/8. Es una perturbación de fixture que demuestra ausencia de un oráculo de operaciones, no una mutación ejecutada del parser. Corrección: comparar variantes y operandos esperados en orden y separar el oráculo de dibujo del de avances; añadir una perturbación que falle en esa aserción concreta.

6. **El caso fraccionario externo excluye cualquier desviación, no sólo la documentada.** `docs/reports/2026-10-04-issue-666-type3-program-evidence/readers.py:14`: la condición omite toda aserción cuando name=fractional. Una copia con anchura250.5 y expectativa101.255 termina correctamente aunque MuPDF coloque B en102.5; el mismo indicador false ocultaría cualquier otra deriva. Corrección: conservar la expectativa matemática y exigir por separado el resultado específico de la discrepancia conocida101.25 para la versión fijada, fallando ante otros resultados o cambios de versión no revisados. Esto no exige aceptar la cuantización como oráculo de producto ni ampliar tolerancias.

## Calidad de Tests (Kripteia)

Ejecuciones reales: Type3 producto98/4 tests; Resolved100/0 tests detectados; parser de contenido88/93; batería FontMatrix96/10; batería programas99/8; script Python100/0 tests detectados. Los resultados100/0 no demuestran cobertura. Avisos de helpers sin llamadas de producto son falsos positivos contrastados: los helpers invocan PdfReader/TextExtractor. Los avisos por ratio de asserts se evalúan por lo que observan; en el hallazgo5 sí existe una debilidad concreta, aunque el score sea alto.

Las46 pruebas focalizadas y Clippy pasan. Las sondas nuevas no se han convertido en regresiones del árbol revisado; son reproducciones de sólo lectura. Control/perturbación de operaciones8/8 y script externo exit0 documentan dos falsos negativos. La guarda directa #675 conserva el RED/GREEN previo y los controles positivos existentes pasan en esta revisión.

## Análisis de Seguridad (Kripteia Security)

Se ejecutaron ambos análisis en los seis alcances indicados, sin alertas Security. La inspección manual detectó amplificación de recursos y propagación de infinitos que el escáner no señaló. No hay nuevos unsafe/FFI, secretos, red ni ejecución externa en la corrección Type3; los scripts de evidencia ejecutan herramientas locales mediante listas de argumentos. Los tamaños decodificados están limitados por stream, pero esto no soluciona el presupuesto acumulado de fuente del hallazgo4. No se sometió al proceso a un agotamiento real de memoria.

## Métricas

- Archivos revisados:16, con regiones de consumidores/parser delimitadas en summary.json; no es revisión completa del repositorio.
- Hallazgos totales:6.
- Archivos de producto/tests/oráculos modificados por la revisión:0; huellas verificadas al cierre.
- Verificaciones pendientes: correcciones y sus regresiones; límites agregados bajo carga después del arreglo; revisión de otros subtipos afectados si se modifica el tokenizer compartido. CI e integración no forman parte de este QR.

La siguiente implementación debe vincular los defectos a issues específicas antes de corregir, creando las que falten sin convertir ese trámite en espera del usuario. #666 cubre las dos mejoras de evidencia; los defectos de resolvedor/recursos necesitan delimitar sus issues. Esta revisión no activa ni deja una corrección a medio hacer.

## Seguimiento correctivo

Los seis hallazgos históricos anteriores se corrigieron y validaron localmente bajo #679/#666; véase [correcciones y evidencia](2026-10-04-f08-fixes.md). CI/integración pendientes. Este informe conserva las reproducciones del candidato revisado.
