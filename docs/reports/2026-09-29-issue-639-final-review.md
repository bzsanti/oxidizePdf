# Revisión final de #639 y dependencias #653/#654/#655

## Resumen ejecutivo

Base develop `8c3c7fabe34ab33bd379dc2dc048d745f1a5af32`, rama
`fix/issue-639-completion`. El WIP previo se conservó en su clon original.
El manifiesto `2026-09-29-issue-639-final-evidence/source-sha256.json` identifica
el producto revisado. Las cuatro issues se confirmaron OPEN antes de publicar.

La identificación sigue habilitada por defecto, con opt-out público. Producer y
Creator conservan sus setters independientes. Las APIs incrementales conservan
Info, XMP y metadatos de catálogo/página; reemplazar Info requiere una política
explícita. El hash sigue siendo metadata descriptiva, no autenticación.
Los defectos heredados que impedían validar esa conservación tienen issues propias.

## Contratos y evidencia

| Contrato y fuente | Contraejemplo, entrada y consumidor | Evidencia | Estado |
|---|---|---|---|
| #639: opt-out compatible y sin marca oculta | `to_bytes`, `save` y variantes con WriterConfig moderno; default/enabled/disabled; Info leído y bytes inspeccionados | 15 regresiones finales, tres configuraciones; Producer/Creator y campos estándar comparados | Demostrado |
| #639: conservar metadatos no solicitados | Tres APIs incrementales, fuente independiente con Info personalizado/generación 7/ausente, XMP, Lang, strings binarios y nombres escapados | RED de pérdida de Lang y de nombres; GREEN; qpdf compara catálogo, Info y XMP; referencias originales y prefijo preservados | Demostrado |
| #653: páginas hoja y herencia | Árbol plano/anidado, append, reemplazo parcial y overlay; fuentes externas, origen distinto de cero, CropBox/Rotate, anotación indirecta y destino | RED de Parent; GREEN con identidades/contenido; qpdf/Poppler confirman 30 salidas | Demostrado |
| #654: cifrado antes de efectos | Fixture AES-256 de qpdf, tres APIs por dos políticas Info | RED acepta entrada y escribe; GREEN exige error de cifrado, cero bytes y ningún callback | Demostrado |
| #639/#655: identificación y xref efectivos | Matriz compresión on/off por xref clásico/stream | GREEN interno y cuatro salidas qpdf; mutación aislada restaura filtro incorrecto y falla leyendo Info | Demostrado |
| Fallos y revisiones sucesivas | ID agotado, configuración incremental incompatible, EOF sin salto final, append seguido de overlay | Error sin bytes para entradas rechazadas; segunda revisión conserva Info, texto y prefijo | Demostrado |

Comandos principales:

- `cargo test --workspace`: 9931 pasan, 0 fallan, 71 omitidos; 371 suites. Esta
  ejecución precede al endurecimiento final de nombres PDF escapados.
- `cargo test -p oxidize-pdf --test issue_639_build_identification_test`: árbol
  final, 15 pasan. También 15 pasan con `--features internal-testing,unstable-spi,semantic`
  y 15 con `--no-default-features --features compression`.
- `cargo test -p oxidize-pdf --doc` en el árbol final: 220 pasan, 24 omitidos.
- `cargo clippy -p oxidize-pdf --lib --tests -- -D warnings`, formato y diff check pasan.
- Probe público independiente: 42 intentos incrementales, 30 éxitos y 12 rechazos
  (cifrado/IDs agotados) sin salida; cuatro configuraciones de generación.
  qpdf 11.9.0 y Poppler verifican 40 PDFs incluyendo seis fixtures válidos.
  `assertions.py` exige prefijo, páginas, texto, metadatos, XMP y enlaces Parent.

Los tests no dependen de qpdf instalado en CI: incluyen fixtures independientes
pequeños y verifican la API real. El probe externo complementa el parser propio.
`xref-mutation.log` procede de un clon desechable: se retiró exclusivamente la
supresión de Filter para xref sin compresión; el test falla por Info ausente,
no por compilación. `names-red.log` discrimina la pérdida de claves escapadas.

## Hallazgos

No quedan hallazgos abiertos dentro del alcance. Correcciones verificadas:

1. **Identificación incondicional y descripción engañosa**: `document.rs:451` y
   `writer/build_identification.rs` — opt-out público, default explícito y
   descripción del hash como dato editable; las tres claves se omiten juntas.
2. **Info sobrescrito implícitamente**: `writer/pdf_writer/mod.rs:621` y `:3934`
   — conservar referencia/generación o ausencia; Replace es explícito.
3. **Kids confundido con páginas hoja y herencia perdida**: `writer/pdf_writer/mod.rs:456`
   — recorrer hojas, materializar recursos/cajas/rotación y reconstruir Parent;
   reutilizar IDs de páginas conserva destinos y referencias de anotaciones (#653).
4. **Catálogo/páginas reducidos y nombres mal serializados**: `writer/pdf_writer/mod.rs:456`,
   `:606` y `:2401` — preservar diccionarios, strings binarios, referencias y nombres
   escapados; actualizar únicamente entradas controladas por la edición (#639).
5. **Entrada cifrada aceptada sin salida válida**: `writer/pdf_writer/mod.rs:621`
   — rechazar Encrypt antes de copiar bytes o invocar overlay (#654).
6. **Flags de Document distintos de WriterConfig**: `writer/build_identification.rs:89`
   — compresión/xref se obtienen de la configuración efectiva (#639).
7. **FlateDecode sobre xref sin comprimir**: `writer/pdf_writer/mod.rs:3753`
   — eliminar Filter cuando los bytes xref no se comprimen (#655).
8. **Overlay documentado como futuro**: rustdoc de las APIs incrementales —
   ejemplo compilable del callback real y políticas de metadata/fallos descritas.
9. **Pruebas limitadas a páginas vacías**: `tests/issue_639_build_identification_test.rs`
   — fixtures independientes y contenido visible, generaciones, herencia, nombres,
   anotaciones, destinos, errores y actualizaciones sucesivas; oráculo externo.

Límites: no se implementa edición cifrada, saneamiento de revisiones previas,
validación de permisos de firma ni reparación semántica de etiquetado al sustituir
contenido. Preservar referencias no implica esas garantías. El rechazo de
configuraciones legacy incompatibles evita emitir objetos sin xref válido.
La revisión cubre el cambio y sus consumidores, no certifica todo el escritor.
No se ejecuta Clippy all-features/all-targets de ejemplos semánticos: el hallazgo
previo de esa combinación está fuera de alcance y no se declara corregido.

## Calidad de Tests (Kripteia)

Ejecución real del script de quality-review en escritor y regresión final:
199 tests / 12 archivos del escritor, score 96; 15 tests / 1 archivo de regresión,
score 100. Peores scores heredados: dos tests `test_extract_font_references_*`
con 70 y tres con 80 en `content_stream_utils.rs`, ajeno al diff.
Se inspeccionaron los avisos de constantes, diagnósticos y unwrap: las constantes
son el contrato/fixture independiente y unwrap produce fallos de test, no oculta
un Result de producto. El score no sustituye RED/GREEN ni qpdf.

## Análisis de Seguridad (Kripteia Security)

Ambos escaneos finalizan con `No security issues found.` Los logs reales están
junto al informe. Revisión manual: guard de cifrado previo a efectos, conversión
de diccionarios con profundidad máxima 64, ID checked_add, preservación binaria
y escape de delimitadores de nombres. No se añaden red, unsafe, FFI ni secretos.
El análisis estático no demuestra ausencia global de vulnerabilidades.

## Métricas

- Archivos Rust revisados: 15 (8 afectados, regresión y 6 módulos de productores/consumidores).
- Hallazgos cerrados: 9; abiertos en alcance: 0.
- Archivos de producto modificados por la fase final de revisión: 0; correcciones
  anteriores autorizadas y verificadas antes de esta reconciliación.
- Verificaciones locales pendientes: ninguna dentro del alcance indicado.
- CI remota e integración: pendientes; PR https://github.com/bzsanti/oxidizePdf/pull/656.
