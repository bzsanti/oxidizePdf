# Issue #638 — revisión del autor

## Resumen ejecutivo

Issue: [#638](https://github.com/bzsanti/oxidizePdf/issues/638). Base
`b76f7382186a704f74c1beb7c48037523ebee715`, rama
`fix/issue-638-predictor-errors`, checkout aislado `/tmp/oxidize-issue-638`.
Rust 2021, MSRV 1.88, biblioteca; features por defecto compression/external-images.
No hay dependencia de los cambios locales pendientes de #637.

La implementación devuelve errores explícitos de predictor en las dos APIs de
stream, conserva identidad/PNG válidos y comprueba tipos, dimensiones y
aritmética antes de procesar las filas. No se añade TIFF ni recuperación tácita.
QR del autor cerrado: revisión manual, Kripteia y workspace/corpus final pasan.
No se ha creado PR; CI remota e integración siguen pendientes.

## Contratos y evidencia

| Contrato | Entrada/consumidor | Evidencia | Estado |
| --- | --- | --- | --- |
| Error de predictor no equivale a bytes decodificados | Flate/LZW, APIs ordinaria/acotada, tres modos | RED de filas truncadas, filtros 5/255 y valores no soportados; GREEN con errores estructurados | Demostrado |
| Parámetros sin reinterpretación por casts/defaults | Predictor, Columns, Colors, BitsPerComponent | Valores negativos, cero, tipos incorrectos, i64::MAX y valor que truncaría a Predictor=1 | Demostrado |
| Aritmética segura antes de filas | PNG pixel/row sizes | checked arithmetic y entradas extremas; sin asignación basada solo en geometría | Demostrado en plataforma local; conversión a usize en 32 bits revisada, no ejecutada |
| PNG mantiene muestras correctas | None/Sub/Up/Average/Paeth; predictores 10–15 | Valores esperados escritos manualmente; packed 1/2/4 bpc y RGB 16 bpc | Demostrado |
| DecodeParms resueltos conservan #514 | PdfDocument, diccionario directo y referencias/arrays | Seis tests existentes y control nuevo de filas válidas/inválidas | Demostrado |
| Consumidor externo real | Dependencia path, default-features=false, compression | Ejecutable independiente comprueba [42] y StreamDecodeError en ambas APIs | Demostrado |
| Límite acotado no se relaja al retirar bytes de filtro PNG | decode_stream_with_limit | Entrada expandida de cuatro bytes y límite tres devuelve error | Demostrado |
| Identidad en Form válido no pierde texto | TextExtractor, referencia indirecta a Predictor=1 | RED de regresión introducida durante implementación; GREEN exige HELLO | Demostrado |

RED sobre la base inalterada y la suite final exacta: **11 fallos y siete
controles válidos**. GREEN sobre la implementación final: **18/18**, más **6/6**
de #514. Las mismas pruebas distinguen los defectos originales de la semántica
válida que se debe conservar, incluidos null e identidad en Forms. Se retienen
además los RED específicos de las dos regresiones detectadas durante QR.

El test no copia la implementación del predictor para producir el resultado
esperado. El encoder zlib prepara datos válidamente comprimidos; las muestras
esperadas son constantes independientes. LZW usa un fixture de códigos
literales de nueve bits y un control válido además del caso malformado.
Reintroducir el catch que retorna bytes sin transformar hace fallar las
regresiones de fila, filtro y LZW: el RED observado lo demuestra.

## Hallazgos

1. **Error de predictor descartado (corregido)**:
   `oxidize-pdf-core/src/parser/filters.rs:1777` — la ruta ordinaria devolvía
   el buffer anterior cuando fallaba PNG. Ambas rutas usan ahora
   `apply_declared_predictor` y propagan `StreamDecodeError`. Una fila de tres
   bytes con Columns=1 deja de devolverse como supuesto contenido decodificado.
2. **Predictores y tipos aceptados sin validación (corregido)**:
   `oxidize-pdf-core/src/parser/filters.rs:1793` y `:1828` — un cast podía
   transformar 4294967297 en 1; tipos incorrectos se ignoraban y valores
   desconocidos devolvían identidad. Se conserva i64 hasta validar, se rechaza
   un tipo no entero y se enumeran las alternativas soportadas 1/10–15.
3. **Geometría de píxel sin protección (corregido)**:
   `oxidize-pdf-core/src/parser/filters.rs:1844` — valores con signo se
   convertían directamente a usize y `bpc * colors` no comprobaba overflow.
   Dimensiones positivas representables, profundidades 1/2/4/8/16 y operaciones
   checked preceden al bucle. El tamaño de salida está acotado por data.len():
   row_bytes < row_size y num_rows = data.len()/row_size.
4. **Ampliación de DecodeParms rompía un consumidor válido (corregido durante QR)**:
   `oxidize-pdf-core/src/text/extraction.rs:2575` consume el decoder sin resolver
   sus referencias mediante PdfDocument. Rechazar referencias globalmente en
   get_filter_params hizo que su `.ok()?` suprimiera el Form (RED: vacío frente
   a HELLO). Se retiró esa ampliación de #638, sin cambiar el consumidor ni la
   resolución general. `get_filter_params` conserva el contrato anterior;
   PdfDocument sigue resolviendo los parámetros de #514. El control público
   permanece para detectar esa regresión de compatibilidad.

5. **Valores null legítimos rechazados (corregido durante QR)**:
   `oxidize-pdf-core/src/parser/filters.rs:1795` — el primer validador trataba
   null como tipo inválido. Un diccionario PDF debe interpretarlo como una
   entrada ausente: se aplican defaults 1/1/1/8 para Predictor/Columns/Colors/BPC.
   RED específico y GREEN por ambas APIs y tres modos. Se corrigieron también
   los oráculos que clasificaban null como inválido. Fuente primaria:
   [Adobe PDF Reference 1.6, 3.2.6 y tabla 3.7](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/pdfreference1.6.pdf#page=57).

No quedan hallazgos nuevos sin corregir dentro de la implementación de #638.
La recuperación Flate y los consumidores que silencian sus errores permanecen
bajo #637; esta revisión no los presenta como corregidos. Los helpers heurísticos
antiguos y sus tests permisivos siguen en esa ruta, fuera de los cambios de
predictores posteriores a una descompresión válida.

## Calidad de Tests (Kripteia)

`run-kripteia.sh rust oxidize-pdf-core/src/parser/filters.rs`: 91/100,
77 tests detectados. `run-kripteia.sh rust
oxidize-pdf-core/tests/issue_638_predictor_errors_test.rs`: 98/100, 18 tests.
Las configuraciones cfg excluyentes explican que el conteo del scanner no sea
el número de unitarias ejecutadas.

Los avisos de bajo ratio de aserciones frente a setup en fixtures PDF y de
constantes esperadas no invalidan su oráculo: se comprueban los bytes o texto
exactos y la variante del error. Los helpers no sustituyen el código productivo.
Los tests existentes permisivos de recuperación Flate no son evidencia de
corrección de #638; las nuevas pruebas ejercitan sus APIs públicas reales.

## Análisis de Seguridad (Kripteia Security)

Ambos archivos analizados: **No security issues found.** Revisión manual de
casts, multiplicación/suma, división por cero, índices, asignaciones y errores
ocultos. No se introduce unsafe, FFI, red, nuevos permisos ni dependencias.
Los límites de descompresión existentes se conservan; no se afirma corregir
las rutas de recuperación de recursos cubiertas por #637.

## Métricas

- Archivos modificados revisados: filters.rs, test de #638, CHANGELOG,
  predictor-decoding.md y TASKS.md; consumidores PdfDocument/TextExtractor y
  los tests de #514 inspeccionados adicionalmente.
- Hallazgos documentados: cinco, corregidos.
- Archivos de producto modificados durante corrección autorizada tras QR: uno.
- Workspace final: 9847 pasan, cero fallos, 73 omitidos; exit 0.
  Corpus T0–T6 disponible, diferenciales de fusión/orden pasan. T3: 1613/1761
  (91,6%) supera 90%; sin cambios de baselines ni umbrales.
- Verificación pendiente: CI del futuro PR contra develop.
- Logs: `/tmp/issue-638-{red,red-final,qr-form-red,green-final,clippy-final,consumer-final,kripteia-filters-final,kripteia-tests-final,workspace-final}.log`.
