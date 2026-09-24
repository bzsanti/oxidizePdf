# QR de #606 — 2026-09-20

Estado posterior a la revisión: hallazgo 1 corregido por petición del usuario.
El informe original se conserva debajo; la validación de la corrección figura
al final.

## Resumen ejecutivo

Revisión del cambio local de #606 sobre `6aa97a8`: generador de firmas,
exportación de la API, las dos suites de integración afectadas, changelog y
seguimiento. Contexto adicional: métricas Helvetica/WinAnsi, escape PDF,
errores de firma, manifests y árbol de dependencias. Rust 2021, MSRV 1.88;
no se añaden dependencias, runtime asíncrono ni unsafe al cambio.

No se confirmó un defecto funcional en ajuste, watermark o conservación de
los bytes firmados. Se confirmó una copia innecesaria de la tabla de métricas
por carácter. Las 20 pruebas focalizadas pasan, incluidas las tres externas
con qpdf/OpenSSL/pdftoppm, así como formato, Clippy `--all-targets -- -D warnings`
y `git diff --check`. La ejecución previa de 6.789 tests de biblioteca y el
doctest constan en TASKS.md; no se repitió esa suite completa en este QR.

## Hallazgos

1. **El preflight clona la tabla completa de métricas por cada carácter**:
   `oxidize-pdf-core/src/signatures/signing.rs:120` llama a `measure_char`
   dentro del recorrido completo del texto. Esta función llama a `lookup`
   (`src/text/metrics.rs:203`), que usa `.cloned()` para Helvetica (`:303`).
   `FontMetrics` contiene un `HashMap` propio y deriva `Clone`, por lo que
   cada carácter copia la tabla y asigna memoria. Con N caracteres se realizan
   N clones completos antes incluso de rechazar un texto por falta de altura.
   Es trabajo evitable en cada prevalidación y preparación de firma; no se ha
   medido latencia ni se atribuye una ralentización porcentual. Obtener una vez
   `get_standard_font_metrics(&Font::Helvetica)` y medir cada carácter con
   `metrics.char_width_unicode(c) as f64 / 1000.0` conserva la correspondencia
   WinAnsi/AFM sin copiar la tabla. Mantener las regresiones de anchura para
   `W`, `i` y texto acentuado al hacer esa sustitución.

## Calidad de Tests (Kripteia)

Ejecutado el script `quality-review/scripts/run-kripteia.sh rust
/tmp/oxidize-606-qr` sobre copias exactas de los cuatro archivos Rust afectados,
incluido el nuevo test aún no rastreado. Código de salida 0. Resultado real:

```text
Overall Score: 92 | Tests: 34 | Files: 4
src/signatures/mod.rs                                11   84
src/signatures/signing.rs                             3   93
tests/issue_540_incremental_signing_test.rs            13   95
tests/issue_606_signature_layout_test.rs                7  100
```

Peores resultados reportados: `test_full_signature_validation_result_invalid_hash`
y `test_full_signature_validation_result_invalid_signature` (70), seguidos
de `test_full_signature_validation_result_modified_after_signing`,
`test_full_signature_validation_result_invalid_certificate` y
`test_full_signature_validation_result_clone` (80). Todos son preexistentes.
Los avisos de ausencia de llamadas productivas se contrastaron: los primeros
cuatro llaman a `is_valid()`, y el último ejercita `Clone`. El bajo ratio
assert/setup y la similitud de fixtures no demuestran por sí solos un defecto.
Los tests externos penalizados por `#[ignore]` se ejecutaron expresamente
con `--include-ignored` en este QR.

Los nuevos tests comprueban contenido completo, anchos, márgenes, descenders,
rechazo justo por debajo del límite, saltos explícitos, geometría inválida,
igualdad del layout con/sin imagen, centrado, opacidad y orden de dibujo.
La comparación de los bytes finales con los verificados por OpenSSL protege
la conservación de la apariencia durante la finalización. El certificado es
de prueba: la ejecución OpenSSL verifica CMS, no la confianza de su emisor.

Mutaciones que deben romper las pruebas: reservar otra vez el 42% del ancho
para el logo; sustituir los avances reales por anchura uniforme; omitir una
línea al agotar la altura; desplazar el logo a la derecha; o modificar bytes
del rango firmado al finalizar. No se ejecutaron mutaciones durante el QR.

Salida completa: `/tmp/oxidize-606-kripteia.txt`.

## Análisis de Seguridad (Kripteia Security)

Salida real, código 0:

```text
=== Kripteia Security Analysis ===

No security issues found.
```

La herramienta no reportó flujos taint, secretos, unsafe/FFI ni funciones
peligrosas. La inspección manual verificó el escape de literales PDF, la
validación previa al acceso al PDF, geometría y opacidad finitas, dimensiones
RGB con multiplicación comprobada, límite de píxeles, restauración del estado
gráfico tras dibujar el watermark y cobertura de los recursos por ByteRange.
El `expect` al codificar texto queda protegido por la validación previa y
la normalización solo introduce caracteres WinAnsi. La API no recibe claves.
El resultado es una revisión estática y focalizada, no una prueba de ausencia
de vulnerabilidades en toda la biblioteca.

## Métricas

- Archivos del cambio revisados: 6, más dependencias de implementación consultadas.
- Hallazgos totales: 1.
- Archivos de código modificados durante el QR: 0.
- Se actualizó únicamente el seguimiento y este informe; release sin reactivar.

## Corrección posterior del hallazgo 1

`SignatureAppearance::layout` obtiene una única referencia estática mediante
`get_standard_font_metrics(&Font::Helvetica)` y reutiliza
`char_width_unicode(c) / 1000.0`. No queda ninguna llamada a `measure_char`
en el cálculo. La tabla se toma prestada y la consulta Unicode/WinAnsi no
clona mapas ni asigna memoria por glifo.

Las 20 pruebas de #606 y firmas incrementales pasan, incluidas las tres de
interoperabilidad externa. Se conservan las comprobaciones de anchura de `W`,
`i` y texto acentuado, ajuste completo y cobertura de la apariencia por la firma.
Formato y `git diff --check` pasan. La nueva ejecución de ambos analizadores
termina con código 0: Kripteia 92/100 (34 tests, 4 archivos), nuevas regresiones
100/100, y Security `No security issues found.`. Salida completa:
`/tmp/oxidize-606-kripteia-after-fix.txt`.

Clippy `--locked -p oxidize-pdf --all-targets -- -D warnings` también pasa.
Hallazgos pendientes de este QR: 0. La integración y la release siguen pendientes.
