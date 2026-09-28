# Quality review — fixes #619, #616, #615, #617 y #618

## Resumen ejecutivo

Revisión de los cambios desde `e1792e83dbf0fdb42f65742fbc705dc8de906e25`
(develop con #611/#612/#614 integrados) hasta `e28e4f5`, excluyendo #620,
que el usuario desarrolla en otra sesión. Rust edición 2021, MSRV 1.88;
validación con Rust/Cargo 1.96.0. Manifiestos y Cargo.lock sin cambios.
Se revisan cinco archivos de producción y siete de tests, con las mismas APIs
públicas. La tarea de implementación está autorizada; el QR inspecciona su diff
y las correcciones derivadas se verifican por separado antes de cerrar el QR.

**QR completado antes de publicar PR.** Todos los checks de código, corpus y
benchmark local terminan correctamente. Se corrigieron los tres hallazgos
introducidos durante la implementación; queda registrado un defecto preexistente
ajeno a la ruta de callbacks, bloqueado por falta de issue. No impide integrar
estos fixes y no se presenta como resuelto.

## Hallazgos

1. **Delimitador incluido en una imagen inline al cruzar un stream (corregido,
   #618)**: `oxidize-pdf-core/src/parser/content.rs:1230` — el LF virtual antes
   de un `EI` situado al principio del siguiente stream se conservaba como dato.
   La prueba reprodujo `abc\n` en lugar de `abc`; la corrección elimina solo ese
   delimitador. La prueba exacta de bytes pasa y se contrasta el resultado del
   visitante con el parser por lotes en los límites léxicos del programa mixto.
2. **Búsqueda repetida del prefijo en la recuperación (corregido, #617)**:
   `oxidize-pdf-core/src/parser/content.rs:1181` — cada error reiniciaba la
   búsqueda lineal de su siguiente límite, con coste cuadrático para una página
   con errores en todos sus streams. Se usa `partition_point` sobre límites
   ordenados; conserva la misma frontera de recuperación sin recorrer otra vez
   los streams anteriores. No se atribuye una mejora de tiempo medida a este
   cambio; se elimina la causa algorítmica bajo las regresiones de recuperación.
3. **Vector temporal por operación de texto (corregido, #618)**:
   `oxidize-pdf-core/src/streaming/text_streamer.rs:114` — la primera refactorización
   devolvía un Vec para una operación que produce como máximo un chunk, añadiendo
   asignaciones a la API por lotes. Ahora devuelve `Option<TextChunk>` y comparte
   la transición de estado sin esa colección. Las asignaciones acumuladas antes
   del primer callback bajan de 447 a 159 bytes en la prueba específica.
4. **Vaciado completo del buffer por lotes (preexistente, bloqueado por falta de
   issue)**: `oxidize-pdf-core/src/streaming/text_streamer.rs:212` — el bucle de
   `check_buffer_size` no actualiza `total_size` después de retirar un elemento.
   Con límite 5 y chunks AAAA/BBBB, el buffer queda vacío cuando BBBB cabría.
   El método coincide byte por byte con la base; #618 evita ese buffer en la
   ruta de callbacks, pero no corrige la API por lotes. Corrección propuesta:
   descontar el tamaño de cada chunk retirado y probar la retención de los más
   recientes. Registrado en TASKS.md: mantenimiento debe crear/vincular una issue
   antes de implementar. No es una regresión introducida por estos cambios.

## Calidad de Tests (Kripteia)

Ejecución real de `run-kripteia.sh rust <scope>`: **91/100, 360 tests, 12 archivos**.
Los archivos del scope se contrastaron por bytes con el código revisado.
Las advertencias sobre helpers de #616 que supuestamente no llaman a producción
son falsos positivos: `round_trip` importa, genera, reabre y comprueba FOOTER y
los bytes originales. Las constantes esperadas son oráculos independientes.
El test de asignaciones sí cubre un error: exige propagar OperationCancelled.
Los avisos sobre smoke tests preexistentes no prueban defectos del nuevo código.

TDD documentado:

- #619: siete fixtures fallan con parseo estricto antes del cambio; pasan con
  longitudes/offsets calculados. La mutación a parseo independiente hace fallar
  la aserción exacta de TJ. Las suites originales pasan 7+5+7 pruebas.
- #616: dos fallos RED y un control verde; después pasan los tres. qpdf valida
  ambos PDFs regenerados. El contenido importado se conserva.
- #615: RED reproduce Right.Left; GREEN cubre los dos operadores, fachada pública,
  cambios de fuente, retroceso, límite de 0,7 em y espacios explícitos. Retirar
  la supresión TJ hace fallar el test específico.
- #617: tres fallos RED y un control; GREEN conserva texto antes/después del error,
  descarta operandos incompletos y conserva operandos entre streams válidos.
  El parser estricto de contenido sigue rechazando el input malformado.
- #618: RED mide 2.706 → 9.696.385 bytes solicitados antes del callback al crecer
  la cola; GREEN final mide **159 → 159**, con uno y con varios streams.
  La cancelación detiene la entrega; el caso completo comprueba texto y posición
  de 10.001 chunks. Son asignaciones acumuladas, no RSS ni pico de memoria viva.
  Se añade el ciclo RED/GREEN del hallazgo 1 y equivalencia en límites léxicos.

## Análisis de Seguridad (Kripteia Security)

La ejecución real señala tres métodos unsafe del allocator del test de #618:
`alloc`, `dealloc` y `realloc`. Se verificó el contrato: todos delegan sin alterar
puntero, layout ni alineación a System; los contadores son Cell por hilo y usan
try_with para la destrucción de TLS. El wrapper no asigna memoria al contar.
No se añade unsafe de producción, red, credenciales, FFI ni dependencias.

La API stream_text sigue recibiendo los buffers descomprimidos completos; el
arreglo evita la materialización adicional de todos los tokens/operaciones/chunks.
No implica lectura incremental del archivo PDF ni memoria total constante. Un
operando grande o una imagen inline requiere memoria hasta completar su operación.

## Validación local final

| Comprobación | Resultado |
|---|---|
| Regresiones focalizadas (12 suites) | 63 pruebas pasan |
| T2 / T3 / T4 / T5 / T6 | 23 / 22 / 25 / 26 / 23 pasan |
| Diferencial de fusiones / orden | 34 / 38 pasan |
| Biblioteca | 6.791 pasan, 3 ignoradas |
| Clippy all-targets, warnings denegados | pasa |
| Formato y diff-check | pasan |
| qpdf sobre los dos casos de footer | pasa |

T4/T5 omiten precisión por falta de ground truth local; sus retornos tempranos
no se contabilizan como precisión validada. Corpus: 484/1.802/130/2.907/161 PDFs.
Fusiones: 285/211.815, 1.695 comparados y 107 omitidos por elegibilidad.
Orden: fidelidad micro 0,8404 y tasa transpuesta 0,159631 con reordenación.
No se regrabaron baselines aunque el gate sugiera hacerlo.

Las suites de corpus comenzaron durante el QR; tras sus correcciones se repitieron
las suites afectadas de recuperación/streaming y la biblioteca completa y Clippy
sobre el código final. T2/T3 no usan TextStreamer; las correcciones finales de QR
cambian el visitante incremental y sustituyen una búsqueda de límite por su
búsqueda binaria equivalente. Los comandos y sus resultados se conservan en JSON.

## OmniDocBench local

El HEAD de código final `e28e4f54eb69057bafc4d8e67c8bd3680f2b26db` se exportó
con una recompilación limpia del paquete, independiente de la base integrada.
Las **981 predicciones normalizadas coinciden byte por byte con develop base**,
así como identidad, poblaciones y hashes de scores. La comparación del gate pasa.

| Error (menor es mejor) | Base integrada | Fixes |
|---|---:|---:|
| Texto global | 0,473116863 | 0,473116863 |
| Texto nativo | 0,378039828 | 0,378039828 |
| Orden de lectura | 0,292288468 | 0,292288468 |

Dataset `f5f559bddf50e36f7f9899d842d0006f13ce8afc`, evaluador
`337cc26965893db3ef53ddc119a6d6bb5bde096f`, OCR desactivado,
`end2end_eval / quick_match`, normalización histórica explícita en Rust
`split_whitespace().collect::<Vec<_>>().join(" ")`. Esta validación no cambia #620.
921 páginas puntuables, 780 nativas, 60 no puntuables de texto; el mismo fallo
Invalid Filter en `jiaocaineedrop_chap10.pdf_8.md`, conservando la predicción vacía.

La evaluación oficial completa se ejecutó una vez sobre esas entradas idénticas;
los manifiestos de candidatos registran la reutilización y sus hashes. Dos
intentos duplicados se cancelaron al comprobar igualdad. El primer intento de la
base falló al guardar porque faltaba result/ en el runner temporal; se creó el
directorio y se repitió sin alterar predicciones, configuración ni límites.
El intento válido termina exit 0 en 440,6 s y usa el fallback oficial de 30 s
para `newspaper_0b1bb8d03b4287eb95f67b68c2cf9f92_1.jpg`. No se oculta ningún
intento fallido ni se atribuyen scores independientes a los runs cancelados.

Evidencia durable: [comandos, hashes, TDD y resultados](2026-09-24-fixes-615-619-validation.json).
Logs y artefactos completos: `/tmp/oxidize-fixes-validation` y
`/tmp/oxidize-{615,616,617,618,619}-*.log`.

## Métricas

- Archivos Rust revisados: 12 (5 de producción, 7 de tests).
- Hallazgos de esta revisión: 4; 3 corregidos y 1 preexistente bloqueado.
- Código modificado por la inspección de solo lectura: 0; las correcciones
  autorizadas posteriores se registran en los commits de sus issues.
