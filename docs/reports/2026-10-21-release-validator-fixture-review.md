# QR — release validator process fixtures (#800)

## Resumen ejecutivo

Issue: [#800](https://github.com/bzsanti/oxidizePdf/issues/800). Scope: only the cfg(test) process-contract helper and new regression in oxidize-pdf-core/src/verification/validators.rs, on top of88b5ce3a. PR806 coverage job119282706297 failed with qpdf execution error Text file busy (ETXTBSY), despite its overall workflow success via continue-on-error. Integration was withheld.

## Contratos y evidencia

Every simulated validator case must independently exercise the production process invocation with its own script/status. The previous helper truncated the same executable path between cases. A deterministic regression keeps a writable handle to the previous case, then invokes the next case through production run_qpdf: old helper RED reproduces ETXTBSY; new helper GREEN returns the expected success, preserves the first case content and then its expected failure status after closing the writer.

Use a distinct NamedTempFile per case, set executable permission and convert into TempPath to close the writer before exec while retaining path cleanup. Existing qpdf/veraPDF argument, exit-code and report assertions remain intact. No retry, suppression or product-code change. All23validator tests pass. Exact scheduling/descriptor provenance in the remote failure is not retrospectively proven; the shared executable lifetime hazard is reproduced and removed. Remote coverage must still validate the final candidate.

## Hallazgos

1. **Mutable executable shared between cases**: oxidize-pdf-core/src/verification/validators.rs:646 — repeated truncation of the same executable couples successive process fixtures and permits ETXTBSY when an earlier handle remains writable. Resolved by independently owned files with writers closed before invocation. RED/GREEN and23validator contracts retained.

## Calidad de Tests (Kripteia)

Mandatory Rust scan:93overall,23tests,1file. Existing helper/tool-availability tests limit external-validator claims; this new regression invokes the actual process boundary with shell fixtures and preserves distinct exit semantics. It does not claim to validate qpdf/PDF-A conformance; the separate interoperability job remains mandatory.

## Análisis de Seguridad (Kripteia Security)

Mandatory scan: No security issues found. Manual review confirms changes stay under cfg(test), use unique temporary executable paths with0700permissions, close writable descriptors before exec, and clean up through TempPath ownership. Product error handling remains unchanged.

## Métricas

- Source files reviewed:1; production behavior changes:0.
- Findings:1 corrected; regression RED then23tests GREEN.
- Clippy lib+tests PASS; format and diff whitespace PASS.
- Pending: final remote CI/coverage, release integration/publication.

## Reapertura: la corrección revisada no resuelve el fallo

El candidato 0d497d0d volvió a fallar ETXTBSY en Linux (run39641274357/job119365129760). La conclusión anterior de resolución queda invalidada. La evidencia local posterior reproduce el error con los contratos existentes, excluyendo la regresión que mantiene un writer abierto: 90/1000 ejecuciones paralelas fallan frente a 0/1000 con tests seriales dentro de cada binario; en ambos casos se ejecutan ocho binarios simultáneos. Esto no establece todavía la identidad del descriptor responsable ni valida una nueva corrección. No se han realizado nuevas subidas.

## Revisión del candidato local con dispatcher fijo

Fuentes: validators.rs (sólo cfg(test)) y tests/fixtures/issue_729/validator-dispatch.sh; hashes en dispatcher-source-sha256.json. El ejecutable fijo se alcanza mediante un symlink privado por caso. Los escenarios se escriben como datos, se leen con /bin/sh y se conservan hasta que termina Command::output. No cambia la ejecución de herramientas en producto.

Contratos y evidencia: los argumentos se transmiten con "$@", sin reinterpretarlos; ${0%/*} mantiene la ruta del escenario correspondiente al symlink. qpdf conserva estados 0/2/3/9 y veraPDF los controles de argumentos, XML válido/inválido y discordancia de status. Los errores de herramienta ausente/no ejecutable siguen distinguiéndose. Todos los 23 tests de validadores pasan. La regresión mantiene abierto para escritura el escenario actual, ejecuta casos sucesivos y comprueba que el primero no cambia: RED ETXTBSY con la fixture anterior y GREEN con el dispatcher. Ningún test abre para escritura el dispatcher del repositorio.

La comparación sin writer forzado ejecuta ocho binarios concurrentes con ocho threads cada uno: 90/1000 fallos antes y 0/1000 después. La suite completa de biblioteca termina con 6731 PASS/3 ignored/0 FAIL. Esto demuestra la eliminación local del fallo observado; no identifica el descriptor concreto del runner remoto ni constituye validación macOS. La conservación de permisos ejecutables en Git es necesaria antes del siguiente checkout. No se serializan tests ni se tolera ETXTBSY en producto.

Hallazgos: el anterior diseño de archivos ejecutables únicos era insuficiente; se sustituye la ejecución de archivos recién escritos por un dispatcher fijo. No aparecen otros hallazgos confirmados en el cambio. El shell sólo consume contenido de test controlado por el repositorio, dentro de un TempDir privado; las rutas y argumentos están entrecomillados. No se incorporan entradas externas, red, dependencias ni cambios de API.

Calidad de Tests (Kripteia): 93, 23 tests, 1 archivo; salida completa issue800-dispatcher-kripteia.log. Los avisos de unwrap son esperables en tests (un error provoca fallo); la supuesta rama vacía de verapdf es un falso positivo al inspeccionar sus dos ramas con aserciones. Las debilidades de tests históricos no cambiados no se usan como evidencia del contrato nuevo.

Análisis de Seguridad (Kripteia Security): No security issues found. Revisión manual adicional del dispatcher, no cubierto por el analizador Rust: expansión de ruta y "$@" entrecomilladas, exec de /bin/sh fijo y escenarios privados de confianza. No es una garantía para scripts externos.

Métricas: dos archivos de implementación/fixture revisados; un hallazgo reabierto abordado; cero cambios de producto fuera de cfg(test). Formato y diff --check PASS. Clippy todavía pendiente: revisión no cerrada hasta recoger su resultado. No se ha subido este candidato ni publicado 5.5.1.

Cierre local: Clippy lib+tests -D warnings terminó0; evidencia issue800-dispatcher-clippy.log. Revisión local completa, sin hallazgos pendientes en el cambio. Aceptación multiplataforma y publicación permanecen pendientes.
