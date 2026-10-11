## Release 5.4.2 — 2026-10-10

- Issue: #705 — release: publish oxidize-pdf 5.4.2 — https://github.com/bzsanti/oxidizePdf/issues/705
- Estado: activa; publicación solicitada explícitamente. Responsable: Codex. Prioridad P1. Base develop befc1a2, árbol idéntico al head revisado 5d554e3 de #704. Checkout existente target/issue690-review; WIP raíz preservado.
- Alcance: #700–#703 integradas; cambio patch de versión, README y changelog. Coste residual de gráficos fraccionarios sin compresión (3,6%) explícito; sin cambios adicionales de código ni dependencias.
- Cierre: paquete verificado, CI del PR de release aprobada, merge main y tag v5.4.2, workflow Release exitoso, artefacto crates.io y GitHub Release verificados, sincronización main→develop.
- Siguiente acción: verificar paquete y revisión proporcional, publicar PR de release a main y completar las puertas de publicación. Reutilizar evidencia vigente sobre las mismas fuentes de #704.

## Implementación autorizada de rendimiento — 2026-10-09

- Orden: #700 → #702 → #701 → #703. Responsable: Codex. Prioridad P2. Estado: las cuatro implementadas, medidas y revisadas localmente. Integración/CI aún pendientes para todas. Issues OPEN confirmadas; títulos/URLs y criterios particulares en sus entradas siguientes.
- Issue: #700 — perf(text): avoid cloning complete font metric tables for each width lookup — https://github.com/bzsanti/oxidizePdf/issues/700
- Base: checkout existente limpio target/issue690-review, rama perf/issues-700-703 desde origin/develop actualizado. Raíz/WIP preservados; sin nuevos clones ni worktrees.
- Requisitos/bloqueantes finitos: (1) métricas sin copias, (2) bytes exactos sin buffers intermedios, (3) tracking completo con menos hashing, (4) números exactos con ruta rápida: demostrados localmente con pruebas discriminantes y comparación externa. #700: RED 100 asignaciones / GREEN 0; layout estándar 52,2% y flow comprimido 25,1% menos tiempo. #702: oráculo BMP/suplementarios, 868 unitarias y cinco pares externos; mejora incremental 1,7–9,2%. #701: 9 unitarias y 7 contratos writer, cinco pares externos; acentos 17,9–26,4%, flow 11,1%. #703: oráculo numérico, mutación -0, cuatro pares externos; enteros 44,8–72,5% menos tiempo. Primer fallback (7,8–9,1% más lento) corregido con formateo conjunto; candidato final conserva 3,6% más tiempo en raw fraccionario (IC pareado de reducción −5,4 a −1,0), declarado como tradeoff de revisión, no ocultado ni contado como mejora. El criterio de #703 exige medir/declarar ese coste, no garantiza mejora universal.
- Validación conjunta: 10.434 PASS / 0 FAIL / 71 ignorados existentes, incluyendo doctests; exit 0 sobre las fuentes finales. Pendientes finitos: publicar PR a develop con evidencia, CI y revisión/integración. Formato, Clippy all-targets/internal-testing y los dos análisis Kripteia pasan/revisados. No se añaden dependencias ni C/FFI.
- Validación inicial: evidencia 5.4.1 y controles binarios preservados en target/performance-700-703; controles intactos. Siguiente acción: publicar PR a develop y verificar CI. Evidencia incremental: target/performance-700-703/.
- #690: usuario informa fallo qpdf, pero no puede confirmar ahora la versión; sigue OPEN. No atribuirlo todavía a 5.4.1 ni afirmar aceptación independiente. Investigación diferida por instrucción del usuario.

## Registro de rendimiento y cierre de issues resueltas — 2026-10-09

- Estado: registro y cierres administrativos completados por petición del usuario. Responsable: Codex. La autorización vigente incluye cerrar las issues de oshtivi y sustituye la reserva anterior de #662/#663.
- #662 CLOSED/COMPLETED: PR #664 MERGED en develop (`136fa170`), incluido en v5.4.1; evidencia retenida de cinco contratos MacRoman y CI aprobada. #663 CLOSED/COMPLETED: PR #665 MERGED en develop (`4a2c38a8`), incluido en v5.4.1; evidencia retenida de trece contratos de tracking y CI aprobada. Comentarios de cierre en inglés publicados; estados y ascendencia al tag verificados, sin atribuir nuevas ejecuciones de tests.
- Se mantienen abiertas #690 (falta aceptación de Studio con 5.4.1), #642 (aceptación de seguridad pendiente), #581 (evaluación oficial TEDS/layout pendiente), #583 (recuperación opt-in/medición pendiente) y #294 (proveedor VLM pendiente). Sus cuerpos/comentarios actuales no acreditan cierre.
- Evidencia remota: `docs/reports/2026-10-09-performance-review-evidence/issues-after-registration-and-closure.json`. Sin cambios de producto ni implementación activada.

### Métricas de fuentes sin copias completas

- Issue: #700 — perf(text): avoid cloning complete font metric tables for each width lookup — https://github.com/bzsanti/oxidizePdf/issues/700
- Estado: Implementada y validada localmente; integración/CI pendientes. Prioridad P2. Responsable: Codex.
- Cierre: eliminar copias de mapas por consulta conservando resolución/anchuras/aislamiento; mejora medida en layout real, regresiones y revisión/CI aprobadas.
- Última validación: RED/GREEN de asignaciones, anchuras/aislamiento/reemplazo, layout estándar 52,2% y flow comprimido 25,1% menos tiempo; evidencia 700-* en target/performance-700-703. Siguiente acción: regresión conjunta y PR a develop.

### Registro de caracteres con menos hashing

- Issue: #701 — perf(text): reduce repeated character hashing while preserving font usage tracking — https://github.com/bzsanti/oxidizePdf/issues/701
- Estado: Implementada y validada localmente; integración/CI pendientes. Prioridad P2. Responsable: Codex.
- Cierre: reducir coste conservando todos los caracteres, Unicode, unión por nombre/contexto/página, registro tardío y subsetting; medición y regresiones/revisión/CI.
- Última validación: 9 unitarias y 7 contratos writer pasan; mutación que elimina Unicode detectada. Acentos 17,9–26,4%, flow 11,1% menos tiempo; CJK raw no concluyente. Siguiente acción: regresión conjunta y PR a develop.

### Codificación y escape sin buffers intermedios

- Issue: #702 — perf(text): eliminate intermediate encoding buffers in show-text construction — https://github.com/bzsanti/oxidizePdf/issues/702
- Estado: Implementada y validada localmente; integración/CI pendientes. Prioridad P2. Responsable: Codex.
- Cierre: menos asignaciones/tiempo con bytes, escapes, UTF-16 y semántica exactos; capacidades seguras, muestras ASCII/acentos/CJK, regresiones/revisión/CI.
- Última validación: 868 unitarias pasan, oráculo BMP/suplementarios y cinco pares qpdf/bbox/píxeles idénticos. Mejora incremental 1,7–9,2%; reserva inicial insuficiente corregida. Siguiente acción: regresión conjunta y PR a develop.

### Formateo numérico de operadores

- Issue: #703 — perf(graphics): optimize numeric operator formatting without changing serialized semantics — https://github.com/bzsanti/oxidizePdf/issues/703
- Estado: Implementada y validada localmente; integración/CI pendientes. Prioridad P2. Responsable: Codex.
- Cierre: mejora medida conservando bytes/precisión/redondeo/no finitos/orden/geometría y píxeles; pruebas diferenciales, revisión y CI.
- Última validación: 16 unitarias pasan con oráculo numérico y mutación -0 detectada. Primer candidato con regresión fraccionaria descartado; fallback conjunto e inline medidos en la comparación final. Siguiente acción: publicar PR a develop y verificar CI, declarando el coste residual fraccionario.

El bloqueo por falta de issue del diagnóstico siguiente queda resuelto por #700–#703; su texto conserva la evidencia anterior al registro. El candidato de las cuatro issues se valida según el bloque activo superior; sus criterios siguen requiriendo integración y CI, no solo documentación, para el cierre.

## Revisión de rendimiento adicional — 2026-10-09

- Estado: Implementada y validada localmente; integración/CI pendientes. Prioridad P2. Responsable: Codex.
- Issue: pendiente. GitHub consultado: siete OPEN (#294/#581/#583/#642/#662/#663/#690), ninguna aplicable. #661 ya está cerrada e integrada; no se reutiliza para activar trabajo nuevo.
- Alcance: generación y layout de 5.4.1 en checkout existente `target/issue690-review`, HEAD `8a92dce`, árbol idéntico al tag. Raíz/WIP previos preservados; sin cambios de producto, ramas, publicaciones ni métricas históricas.
- Hallazgos: copia completa de métricas por consulta; hash por carácter repetido; buffers de codificación/escape; formateo numérico de operadores. Informe y evidencias: [2026-10-09-performance-review.md](docs/reports/2026-10-09-performance-review.md).
- Última validación: 16 unitarias pasan con oráculo numérico y mutación -0 detectada. Primer candidato con regresión fraccionaria descartado; fallback conjunto e inline medidos en la comparación final. Siguiente acción: publicar PR a develop y verificar CI, declarando el coste residual fraccionario.
- Dependencia externa y criterio de desbloqueo: mantenimiento crea/vincula issues específicas y confirma estado OPEN con alcance/aceptación; registrar número, título y URL antes de corregir.
- Criterio de cierre de cada optimización: ahorro medido frente a control comparable, misma semántica de métricas/Unicode/registro tardío/serialización según alcance, regresiones discriminantes, revisión y validación proporcional. No cerrar por completar el diagnóstico.
- Siguiente acción exacta: crear/vincular primero la issue de copias de `text/metrics.rs:286–308`; probar préstamos/Arc conservando resolución por documento y medir layout real. Después abordar los otros hallazgos bajo sus issues.
- Restricciones: no corregir ni recalibrar bajo una entrada sin issue; no omitir tracking estándar ni cambiar precisión/defaults de compresión para ganar el benchmark. No se repitió suite completa del core ni Clippy; las verificaciones de un candidato futuro siguen pendientes.

## #690 — inherited zero-offset xref entries — 2026-10-08

## Release 5.4.1 — 2026-10-08

- Issue: #697 — release: publish oxidize-pdf 5.4.1 — https://github.com/bzsanti/oxidizePdf/issues/697
- Status: active. Priority: P1. Owner: maintenance.
- Closure: #696 integrated; reviewed 5.4.1 package; green release PR merged into main; tag and successful publication verified against registry checksum and source provenance; main synchronized into develop.
- Validation: #696 merged as `4420acc3f4fc14259c30c866dacd8690ddeb6620`; its entire tree is identical to reviewed HEAD `3e47216125b25e43918893569cf0a361edfa4c5a`. PR CI: 29 successful checks, 2 configured skips. Reusable product evidence: 10,430 tests passed, 19 PDFs accepted by qpdf without warnings, Kripteia quality/security review.
- Remaining blockers: verify version/package changes; release PR CI; tagged publication/provenance; branch synchronization. #690 Studio acceptance remains separate and open.
- Package validation: passed; extracted package compiled and 1,427 source/test files plus README match the checkout. Kripteia: 98/100, no security findings. Evidence: `docs/reports/2026-10-08-release-5.4.1-review.md`.
- Next action: publish the release PR against main and wait for CI before merging.

- Issue: #690 — fix(operations): complete nested tagged split and preflight for the #621/#677 input — https://github.com/bzsanti/oxidizePdf/issues/690
- Status: implementation, focused/full regression and local original-input acceptance complete; review and Clippy passed; delivery pending; issue confirmed OPEN. Priority P1. Owner: Codex.
- Scope: safely retire unreachable in-use zero-offset entries in a preserving incremental revision, without masking reachable corruption or changing source/content/page-tree identities. Existing clean checkout target/issue690-review reused, branch fix/issue-690-unused-xref from origin/develop 592f771; root WIP preserved.
- Acceptance / evidence / blockers:
  - Clean prepared and split xref: demonstrated on candidate: 19 qpdf exit-0 outputs; baseline consumer 5.4.0 had exit 3. Evidence: docs/reports/2026-10-08-issue-690-xref-evidence/external-final.json.
  - Preservation and permission policy: demonstrated on candidate by native contracts and original-input consumer, including text/pixels, prefixes and permission/metadata negatives.
  - Safe reachability and revision handling: demonstrated: complete trailer reachability, generation/alias rejection, missing compressed-container rejection, table/stream/hybrid and repeat-revision checks.
  - Proportional regression, Clippy, quality/security review: complete locally on final sources (10,430 PASS/0 FAIL; Clippy and both Kripteia analyses). Published-candidate Studio acceptance remains external after delivery.
- Last validation: public follow-up https://github.com/bzsanti/oxidizePdf/issues/690#issuecomment-6057868240 read; baseline checkout clean and develop fetched.
- Latest candidate evidence: 10/10 new public contracts, 39 prior tagged contracts including 48 combinations; 15 synthetic PDFs pass qpdf without warnings and two 12-page cohorts preserve Poppler text/pixels. Initial RED: retirement inventory absent and trailer reference accepted. An additional negative test was corrected to expect the existing DocMDP rejection of nonzero corrupt offsets; no product guard was weakened.
- Final validation so far: workspace 10,430 PASS/0 FAIL/71 existing ignored; format passed. Candidate original-input public consumer passes 3/3/6 with unchanged source and prefixes; 19 PDFs pass qpdf clean, and original plus both synthetic workflows preserve all 12 text/raster pages at 72 dpi. Both Kripteia analyses completed across affected code/tests; no security alerts.
- Review: docs/reports/2026-10-08-issue-690-xref-review.md; exact hashes and reproducible external verifier archived.
- Next action: publish the reviewed change against develop and verify candidate CI. No issue closure before published-candidate Studio acceptance.

## Release 5.4.0

- Issue: #693 — release: prepare and publish 5.4.0 — https://github.com/bzsanti/oxidizePdf/issues/693
- Estado: paquete 5.4.0 validado; PR de release/CI pendientes. Prioridad P1. Responsable: Codex.
- Cierre: versión/changelog coherentes, paquete validado, PR de release integrado en main con CI aprobado, tag y publicación GitHub/crates.io verificados, sincronización en develop.
- Última validación: base develop `90c16c0`, PR #692 integrado con 29 checks aprobados y 2 omitidos; producto con 10.419 tests aprobados y QR completo.
- Siguiente acción: abrir el PR de release hacia main; comprobar CI antes de integrar, etiquetar y publicar.
- Restricción: #690 permanece abierta por aceptación en Studio y avisos qpdf heredados; #658 queda fuera de la release.

## Consolidar preparación en el editor incremental — #690

- Issue: #690 — fix(operations): complete nested tagged split and preflight for the #621/#677 input — https://github.com/bzsanti/oxidizePdf/issues/690
- Estado: corrección integrada en develop mediante #692 (`90c16c0`); CI29 aprobados/2 omitidos. Prioridad P1. Responsable: Codex.
- Cierre: retirar las APIs específicas no publicadas y reutilizar el editor, sus mutaciones y validación sin perder los contratos demostrados de #690.
- Última validación: corrección:10.419 PASS/0 FAIL, Clippy y QR; consumidor compatible y original3/3/6 con12/12 textos/píxeles idénticos. [Evidencia](docs/reports/2026-10-07-issue-690-review.md).
- Siguiente acción: verificar aceptación en Studio con el paquete publicado por #693.
- Pendiente de aceptación: Studio/API publicada y avisos xref heredados. Corrección integrada; aceptación externa pendiente.

## Continuación autorizada de los cuatro frentes — #661 — 2026-10-07

- Issue: #661 — perf(writer): implement measured CPU and allocation improvements from generation profiling — https://github.com/bzsanti/oxidizePdf/issues/661
- Estado: cuatro frentes implementados y QR completado; publicación/CI de continuación e integración pendientes. Issue confirmada OPEN; alcance autorizado explícitamente por el usuario. Prioridad P1. Responsable Codex.
- Control: commit 5621e4cb25690c7618981edc03deb15883e4dbc9 del PR #689, base develop. Históricos y WIP raíz preservados.
- Criterio de cierre: evaluar los cuatro frentes con implementaciones independientes, conservar únicamente mejoras medidas sin regresiones de contratos, documentar costes/limitaciones, QR calidad/seguridad y CI antes de integración.

| Frente de #661 | Estado | Criterio y siguiente acción |
| --- | --- | --- |
| Registro de caracteres | Demostrado:242 llamadas/doc evitadas | Eliminar agregaciones/copias evitables sin perder uso por fuente ni registros custom con nombres estándar; medir contra control y probar subsetting/CJK/reutilización |
| Copias de páginas y streams | Demostrado:540 llamadas/doc adicionales evitadas | Prestar contenido y mover buffers sin alterar orden de pintado, colisiones, headers/footers, metadatos ni saves repetidos |
| Formateo | Demostrado:158 llamadas/doc adicionales evitadas; bytes preservados | Evitar temporales conservando exactamente precisión, redondeo, escapes, nombres y orden determinista; comparar bytes y mediciones |
| Compresión | Demostrado con política conservadora; penalización pequeña en vacío documentada | Evaluar reutilización de estado/buffers con datos vacíos, texto, fuentes e imágenes; mantener nivel/backend/default y dependencia Rust; justificar con tiempo/tamaño/asignaciones |

- Última validación: suite conjunta inicial10.397 PASS/0 FAIL/71 ignorados; tras ajustar compresión, finales1 prueba encoder+14 recursos/memoria PASS y Clippy all-targets/internal-testing -D warnings PASS. QR manual y ambas herramientas Kripteia pasan;42 PDFs representativos externos qpdf/texto/raster sin avisos. Paquete1422 fuentes/tests idénticos. La suite completa inicial no se atribuye a una segunda ejecución local final.
- Medición final respecto del primer PR5621e4cb: facturas1/10/100 páginas comprimidas −5,2%/−48,3%/−13,2% tiempo; sin comprimir −8,9%/−12,5%/−12,1%.10 páginas:3484→2481 alloc/realloc y4.390.720→1.092.106 bytes solicitados, PDF9483 bytes intacto. No son RSS ni garantías universales.
- Compresión incondicional descartada por regresiones en páginas cortas/imagen. Final: solo multipágina y streams de al menos2048 bytes, reset diferido hasta siguiente uso. Se conserva penalización observada en vacío frente a formatting:1 página+3,45µs/+2,37%;10 páginas+13,94µs/+1,28% con intervalo pareado que incluye cero en10 páginas. Limitación no bloqueante de los contratos PDF, destino #661/informe; no afirmar mejora universal ni ocultar la primera cohorte.
- Evidencia: docs/reports/2026-10-07-issue-661-review.md y evidencia/fronts, incluidos controles, parches por incremento, muestras e incertidumbre. Históricos intactos, WIP raíz preservado. CI primera versión29 SUCCESS/2 SKIPPED; no valida todavía esta continuación.
- Siguiente acción exacta: publicar continuación del PR #689 contra develop, comprobar CI sobre su nuevo HEAD y preparar integración. #661 permanece OPEN. CFF pequeño y build sin features quedan registrados por separado sin corrección ni issue inventada.

## Reanudación del rendimiento del writer — #661 — 2026-10-07

- Issue: #661 — perf(writer): implement measured CPU and allocation improvements from generation profiling — https://github.com/bzsanti/oxidizePdf/issues/661
- Estado: implementación local y revisión completadas; CI/integración pendientes; OPEN y lista de issues comprobadas en GitHub. Prioridad P1. Responsable Codex.
- Base: develop `9dcf8188a906132991e14946eec228efe98ada01`, checkout aislado `target/issue661-writer`, rama `perf/issue-661-writer`. Checkout raíz y WIP anterior preservados.
- Criterio de cierre: mejoras implementadas e integradas con medición comparable de tiempos/asignaciones, contratos PDF preservados, validación externa, QR calidad/seguridad y CI aprobados; históricos intactos.
- Última validación: release 5.3.0 publicada, workflow 37527011742 SUCCESS. Paquete verificado contra índice oficial: no retirado, checksum correcto, commit ca9c662, 465 fuentes y README idénticos, MSRV 1.88 y sin recipient-encryption. API crates.io devuelve403; índice y archivo oficiales accesibles. Evidencia: docs/reports/2026-10-07-issue-661-evidence/baseline-registry.json.

| Requisito y fuente | Estado y evidencia | Bloqueante / siguiente acción |
| --- | --- | --- |
| Base reproducible, #661 | Demostrado: develop y paquete 5.3.0 verificados | Fijar consumidor/lock y control inalterado |
| Reducir CPU/asignaciones, #661 | Demostrado: preasignación y recursos compartidos; producto medido identificado por SHA256 | Conservar tracking por compatibilidad con registros custom del mismo nombre |
| Preservar contratos, #661 | Demostrado: Raw/custom/CJK, colisiones, importación y reutilización; qpdf/Poppler y raster | Evidencia en informe 2026-10-07-issue-661-review.md |
| Mejora medida, #661 | Demostrado:192 muestras;10 páginas −10,6% tiempo comprimido/−21,8% sin comprimir;4816→3484 alloc/realloc | No afirmar mejora concluyente en1 página ni confundir bytes acumulados con RSS |
| Revisión e integración, #661/AGENTS | Pendiente | Regresiones, QR calidad/seguridad y CI antes de integración en develop |

- Última validación de implementación:113 writer tests y37 contratos pasan; suite completa10.393 PASS/1 FAIL/71 ignorados, único fallo por mínimo de20KB ahora sustituido por25 páginas y46 líneas/página exactas. Final12/12 en ambos targets afectados; Clippy completo/final, formato, QR Kripteia/Security y28 PDFs externos pasan. Sin corpus externo en el checkout aislado; no atribuir una ejecución nueva T0–T6/OmniDocBench. Producto y dependencias permanecen idénticos al candidato medido.
- Dos hallazgos de tests corregidos (fixtures fuera del crate y proxy de tamaño); embedding CFF pequeño preexistente registrado aparte y sin corrección. Informe y evidencia: docs/reports/2026-10-07-issue-661-review.md y directorio contiguo.
- Siguiente acción exacta: paquete extraído5/5 validado; publicar PR contra develop y validar CI antes de integración. Mantener #661 OPEN hasta demostrar integración.

## Hallazgo separado durante #661 — compilación sin ninguna feature

- Issue: pendiente; lista abierta confirmada sin issue aplicable. Estado: bloqueado para corrección por falta de issue específica. Prioridad P2. Responsable de crear/vincular: mantenimiento/bzsanti.
- Evidencia: `--no-default-features` falla con18 errores de flate2 ausente tanto en control5621e4cb como en candidato; cinco módulos causales son idénticos byte a byte. Logs e identidad en evidencia fronts/no-features-*. No lo introduce la reutilización del encoder.
- Dependencia externa y desbloqueo: crear/vincular issue aplicable confirmada OPEN. Criterio de cierre: definir/validar el contrato sin features y corregir los imports/features bajo esa issue.
- Siguiente acción exacta: mantenimiento debe crear/vincular la issue; no modificar dependencias, features ni esos módulos bajo este hallazgo antes de ello.
- No bloquea #661: el perfil mínimo soportado por la CI usa `--no-default-features --features compression` (tools/text_contracts/run_fast.py); sus13 contratos focales pasan, al igual que los caminos runtime con compress=false. No afirmar validación de un build sin compression.

## Hallazgo separado durante #661 — OpenType/CFF pequeño sin subsetting

- Issue: pendiente; la lista abierta consultada no contiene una issue aplicable al embedding de programas CFF pequeños. #661 optimiza recursos, no cambia este camino de incrustación.
- Estado: bloqueado para corrección por falta de issue específica. Prioridad P2. Responsable de crear/vincular: mantenimiento/bzsanti.
- Evidencia: fixture OTF de3328 bytes, Poppler avisa `Mismatch between font type and embedded font file` tanto en candidato como en control retenido de release5.3.0; Unicode extraído idéntico. `write_font_with_unicode_support` no cambia en #661. Artefactos: docs/reports/2026-10-07-issue-661-evidence/small-cff-probe/.
- Criterio de cierre: programa completo/subconjunto y Subtype coherentes, sin avisos externos en OTF/CFF pequeño/grande y regresiones discriminantes.
- Dependencia externa: issue dedicada creada o vinculada por mantenimiento; desbloqueo: issue aplicable confirmada OPEN.
- Siguiente acción exacta: crear/vincular issue y reproducir los límites/fallbacks del embedding antes de corregir. Restricción: no corregir ni recalibrar métricas bajo esta entrada sin issue.
- No bloquea el cambio de recursos: defecto presente en control y candidato, código causal intacto; las salidas finales del alcance de subsetting pasan la validación externa sin avisos.


## Release 5.3.0 — 2026-10-06

- Issue: #684 — release: publish 5.3.0 and reconcile release history — https://github.com/bzsanti/oxidizePdf/issues/684
- Estado: activa, candidato preparado y validado localmente. Prioridad P1. Responsable Codex.
- Base: develop 36ef67e3444dc37d5426e8c3951dec427efc836c; main/v5.2.0 9ab7ac6530d6e064d48a3b274568af1b6d901462. Trabajo aislado en target/release530; cambios locales anteriores preservados.
- Criterio de cierre: historial de releases conservado, versión/documentación 5.3.0 coherentes, producto idéntico a develop, QR y validación del paquete aprobados, PR de release integrado en main, tag/GitHub/crates.io verificados y metadatos sincronizados a develop por PR.

| Requisito | Estado / evidencia | Bloqueante y siguiente acción |
|---|---|---|
| Historial y versiones coherentes | Demostrado: 5.3.0 coherente e historial anterior preservado | Ningún bloqueo local |
| Producto revisado conservado | Demostrado: identidad exacta con develop y lock salvo versión propia | #642/#658 y #661 excluidos |
| Validación de candidato y QR | Demostrado: 10.389 PASS/0 FAIL/71 ignorados, Clippy, paquete signatures y Kripteia | CI remota pendiente |
| Publicación y sincronización | Pendiente | CI/merge de release a main, tag, registros y PR hacia develop |

- Última validación: suite completa exit0, Clippy y paquete con signatures aprobados; QR cerrado para el delta. Evidencia: docs/reports/2026-10-06-release-5.3.0-review.md y directorio contiguo. Siguiente acción: commit con hook original, PR de release a main y CI.

## Integración autorizada de PRs verdes — 2026-10-06

- Issue: #680 — TextExtractor::extract_from_page fails with StreamDecodeError("FlateDecode incomplete or truncated zlib stream") on truncated streams — https://github.com/bzsanti/oxidizePdf/issues/680
- Issue: #677 — Tagged split rejects missing page StructParents keys in 5.2.0 (#621) — https://github.com/bzsanti/oxidizePdf/issues/677
- Estado: integración solicitada completada. PR #683 y PR #682 MERGED en develop, cada uno con 29 SUCCESS/2 SKIPPED y ningún check pendiente al fusionar. Prioridad P1. Responsable Codex.
- #683: HEAD3cdf880, merge 091d4abf7b2686e22c36427b1759b3ec11484193, 2026-10-06T13:51:20Z.
- #682: HEADaa19e39f19d80bf6d23770a378a31c45c04fdf44, merge 36ef67e3444dc37d5426e8c3951dec427efc836c, 2026-10-06T15:05:08Z.
- Conflicto secuencial resuelto: al fusionar #683, #682 entró en conflicto únicamente en TASKS.md. Conservados ambos bloques e historial; diez fuentes relevantes idénticas a las revisadas. 69 contratos combinados y hook original aprobados (formato/Clippy/build y biblioteca6.801 PASS/0 FAIL/3 ignorados). Nueva CI esperada hasta concluir Windows; merge protegido por HEAD exacto, sin bypass.
- Última validación: GitHub confirma ambos merges y solo #658 abierto/borrador, base develop. #658 conserva aceptación de seguridad RSA pendiente y queda excluido de esta integración aunque sus checks automáticos estén aprobados. No se cambió main ni se realizó release o cierre administrativo de issues.
- Criterio de cierre de esta orden: cumplido. Evidencia: docs/reports/2026-10-06-pr-682-evidence/integration.json, after683-tests.log, after683-commit.log y final-ci-watch.log. La CI posterior al merge es una ejecución separada; no se atribuye aquí su resultado.
- Siguiente acción exacta: ninguna fusión pendiente entre los PR autorizados y listos. #661 conserva implementación pendiente; #658 depende de aceptación de seguridad. Checkout raíz y cambios preexistentes preservados.

## Base de integración de #682 — 2026-10-06

- Issue: #680 — TextExtractor::extract_from_page fails with StreamDecodeError("FlateDecode incomplete or truncated zlib stream") on truncated streams — https://github.com/bzsanti/oxidizePdf/issues/680
- Estado: base remota cambiada a develop por instrucción del usuario; conflictos documentales resueltos localmente, validación/publicación de la conciliación pendientes. Prioridad P1. Responsable Codex.
- Regla permanente: todo PR de trabajo apunta a develop; main recibe únicamente merges de nuevas releases.
- Alcance: preservar autoría e historial del PR; conservar ambas aclaraciones rustdoc (recuperación explícita y geometría), seguimiento de develop y pruebas ya revisadas. Excluidos del diff los cambios de versiones/documentación de releases heredados de main.
- Criterio de cierre: diff contra develop limitado a #680, contratos focales aprobados, hook original y rama publicada sin conflictos. No merge del PR, release ni cierre de issue.
- Siguiente acción: validar y publicar la conciliación; los checks anteriores sobre main no acreditan el nuevo HEAD/base.
- Validación de conciliación: 42/42 contratos focales pasan; las cuatro fuentes Rust del PR coinciden byte a byte con el snapshot develop ya revisado y probado. Código ejecutable y manifiestos iguales a develop; solo docs/tests/evidencia de #680 en el diff. Se reutilizan QR y ambos análisis de seguridad/calidad vigentes. Hook original y publicación pendientes.

## Corrección de PR #682 — 2026-10-06

- Issue: #680 — TextExtractor::extract_from_page fails with StreamDecodeError("FlateDecode incomplete or truncated zlib stream") on truncated streams — https://github.com/bzsanti/oxidizePdf/issues/680
- Estado: cinco hallazgos corregidos y revisión local completada; publicación/CI pendientes. Prioridad P1. Responsable Codex. Usuario solicita corregir los hallazgos del PR.
- Decisión: retirar el builder que ocultaba diagnósticos y el buffer adicional; documentar y probar la API existente extract_from_page_with_recovery. Código ejecutable idéntico a develop tras conciliar la base; tests y docs son el alcance final. Autoría e historia de los dos commits originales conservadas.
- Criterio de cierre local: diagnósticos observables, límites independientes, un buffer expandido, documentación coherente y regresiones discriminantes; demostrado en pruebas focalizadas y revisión calidad/seguridad.
- Validación: 42/42 en base main y 42/42 sobre develop; Clippy/formato y ejemplo rustdoc pasan. RED de memoria 2.140.544 bytes; GREEN 1.091.968 para salida 1 MiB. Mutaciones: borrar diagnósticos provoca cinco fallos; confundir límites provoca un fallo. Seis alcances Kripteia/Security, allocator de tests auditado. Evidencia y revisión: docs/reports/2026-10-06-pr-682-review.md y directorio contiguo.
- Siguiente acción: publicar corrección aditiva en #682 y comprobar CI del nuevo HEAD. No merge, release ni cierre de #680. #677/#661 no iniciadas; #662/#663 siguen reservadas a oshtivi.
- Restricciones: preservar cambios locales anteriores; recuperación explícita no implica integridad/completitud. Primera ejecución con caché compartida descartada; validación final en targets exclusivos.

## Recuperación segura de tagged split — #677 — 2026-10-06

- Issue: #677 — Tagged split rejects missing page StructParents keys in 5.2.0 (#621) — https://github.com/bzsanti/oxidizePdf/issues/677
- Estado: implementación, QR y validación local terminados; publicación pendiente, issue OPEN. Prioridad P1. Responsable Codex.
- Base: develop 9c3f4b57969dd69fc9efa128495a8b0a7c7f4f74; checkout aislado target/issue677-fix. Retomada la secuencia tras entregar las correcciones de #682; #661 continúa pendiente.
- Criterio de cierre: reconstruir únicamente claves ausentes desde propietarios MCID inequívocos; preservar contenido, ActualText, referencias y permisos; salidas 3/3/6 correctas y fallos sin publicación parcial; regresiones discriminantes y revisión de calidad/seguridad.
- Bloqueantes delimitados: asignación determinista sin colisiones con páginas/OBJR; conservar las claves reparadas al reescribir páginas; verificar rechazo de ambigüedad/contenido inconsistente y atomicidad.
- Última validación: inspección confirma rechazo en associate_mcid y segunda escritura de diccionario de página en reorder; pruebas nuevas pendientes.
- Siguiente acción: fixture compartido con ActualText, demostrar RED, implementar y validar los contratos de salida y rechazo.
- Avance validado: RED inicial 3 PASS/6 FAIL (cuatro casos de recuperación rechazados y dos guardas ocultas); GREEN final 63 PASS/1 ignorado preexistente en siete suites. Trece contratos #677 incluyen ActualText, propietarios exactos, MCR/inherencia, colisiones, permisos y conservación de destinos existentes.
- Revisión en curso: tres mutaciones detectadas por fallo funcional (colisiones, doble escritura, validación de tags); control restaurado13/13. Seis análisis Kripteia y Security ejecutados sin alertas de seguridad; pendientes validación de workspace y cierre del informe. La escritura duplicada detectada con páginas anidadas se corrigió componiendo las modificaciones antes de escribir.
- Siguiente acción: completar validación obligatoria, informe QR y preparar PR contra develop; sin merge/release ni cierre de issue.

- Suite completa terminada: 10.380 PASS/0 FAIL/71 ignorados preexistentes, 412 resúmenes, --workspace --features internal-testing --no-fail-fast. Fuentes coinciden con hashes revisados; qpdf acepta tres salidas y Poppler conserva texto/orden frente a fuente. Informe: docs/reports/2026-10-06-issue-677-review.md.
- Clippy all-targets con internal-testing y -D warnings aprobado. Se prepara commit con hook original completo y PR contra develop; no se da por aprobada CI remota todavía.

## T5 — Integración y mantenimiento (terminada)

- Issue: #666 — test(text): build normative encoding and glyph-spacing contract batteries — https://github.com/bzsanti/oxidizePdf/issues/666
- Estado: implementación, QR y aceptación CI completados; seis hallazgos corregidos y revalidados. Prioridad P1. Responsable Codex.
- Criterio de cierre: cumplido; baterías deterministas conectadas a CI, perfiles default/minimal (`compression`)/SPI, MSRV 1.88, Linux/Windows/macOS y corpus manual/programado demostrados. Merge/release excluidos del alcance por la issue.
- Última validación: código a2562e2; CI 29 SUCCESS/0 FAILURE y dos jobs programados omitidos en PR. Matriz nativa 12/12: 403 PASS en 40 targets default/minimal, 417 en 41 SPI, cero fallos/ignorados/vacíos; 34 checks Python. Suite local completa: 10.367 PASS/0 FAIL/71 ignorados preexistentes. Corpus manual 37382367497 SUCCESS, incluidos T2/T3, fusión/orden y contenido; 1.802 PDFs, 6.934 páginas comparadas. Las 6.892 páginas con PDF/hash común al snapshot local tienen métricas idénticas. Informe: docs/reports/2026-10-05-t5-review.md.
- Siguiente acción: revisión de mantenimiento e integración del PR #681; no quedan fases de implementación, hallazgos QR ni validaciones obligatorias de #666 pendientes. La issue permanece abierta hasta su integración; merge/release no ejecutados.
- Procesos actuales: ninguno pendiente de aceptación. La evidencia final identifica el commit de código validado; el cierre documental no modifica producto, tests, oráculos ni workflows.
- Restricciones: preservar WIP, oráculos y baselines; no comparar agregados de poblaciones distintas como si fueran iguales. Corregir regresiones inmediatamente bajo #666.

### Criterios de aceptación de T5

| Requisito y fuente | Estado y evidencia | Siguiente acción |
| --- | --- | --- |
| Suite rápida portable, issue666/T5 | Demostrado: runner con 40/41 targets efectivos, cero ignorados/vacíos | Conservar evidencia |
| Default/minimal/SPI y MSRV 1.88, plan T5 | Demostrado: matriz nativa run37382295498, 12/12 | Ninguna corrección pendiente |
| Linux/Windows/macOS, plan T5 | Demostrado: tres perfiles por plataforma y CI general SUCCESS | Ninguna verificación pendiente |
| Corpus pesado manual/programado, issue666/T5 | Demostrado: run37382367497 SUCCESS; mismo job para nightly | Ejecución periódica ya configurada |
| QR y hallazgos resueltos, usuario | Demostrado: fases anteriores y T5 cerradas, seis hallazgos T5 revalidados | Ningún bloqueante pendiente |
| Entrega versionada, aceptación issue666 | Demostrado: PR681 y evidencia publicada; merge/release excluidos | Revisión e integración por mantenimiento |

## T4 — Discriminación y corpus (terminada localmente)

- Issue: #666 — test(text): build normative encoding and glyph-spacing contract batteries — https://github.com/bzsanti/oxidizePdf/issues/666
- Estado: implementación y QR pasados, ocho hallazgos corregidos y revalidados. Prioridad P1. Responsable Codex.
- Criterio de cierre: cumplido; mutaciones discriminantes, población congelada y métricas separadas con errores/exclusiones visibles.
- Última validación: 7.453 PASS/0 FAIL/3 ignoradas en62 targets; tres gates históricos pasan. Medidor13/13, nueve mutaciones de producto, cinco del parser y seis del medidor discriminan. Corpus1.802 PDFs,6.937 páginas comparables; hashes intactos. Clippy, formato, Kripteia/Security y trazabilidad pasan. Informe: docs/reports/2026-10-05-t4-review.md.
- Correcciones de producto: NUL como whitespace; longitud/padding y parámetros tipados de imágenes inline; regresión incremental de whitespace virtual resuelta. Tres páginas reales recuperan texto. Los440 reemplazos Type3 del PDF diagnosticado corresponden a nombres C0…C255 sin ToUnicode, conforme a política.
- Siguiente acción exacta: T5 completada según el registro superior; conservar evidencia T4. Ningún proceso T4 pendiente.
- Restricciones: preservar WIP y baselines; no afirmar igualdad con Poppler ni interpretar exclusiones como mejora. Sin merge/release autorizados. #666 sigue abierta.

## #668 sobre develop integrado — 2026-10-01

- Issue: #668 — fix(text): decode normative simple-font encodings and built-in Symbol/Zapf — https://github.com/bzsanti/oxidizePdf/issues/668
- Estado: `[-]` TDD activo. Prioridad P1. Responsable Codex.
- Alcance: cinco codificaciones simples; conservar intacta la función MacRoman de Omer y el tracking integrado. Rama fix/issue-668-simple-encodings desde develop 4a2c38a, independiente de #669.
- Criterio de cierre: tablas y selección/precedencias correctas, recuperación visible, métricas conservadas, RED/GREEN, mutaciones discriminantes, QR y corpus/CI.
- Última validación: issue OPEN confirmada; #669 en CI sin fallos reportados al iniciar.
- TDD: RED 13 PASS/14 FAIL en 27 contratos. Adaptadas cinco tablas y selección/precedencias; función decode_macroman comparada byte a byte con HEAD, sin cambios. GREEN y consumidores en ejecución.
- Validación: 27 contratos + 46 regresiones pasan; tres mutaciones detectadas y restauración 28/28; biblioteca 6776/0/3; mínimo 27/27 y SPI 41/41; QR 97/90 tests, Security sin alertas. Corpus 72/72, pero orden compara 1054 PDFs frente a 1058 en base: diagnóstico por documento en curso antes de publicar.
- Diagnóstico resuelto con TDD: recuperación personalizada Type3/intrínseca conservada; dos RED por aserción y 29 contratos + 46 regresiones GREEN. MuPDF 1.26.10 confirma los cuatro Type3. QR actualizado 97/92 tests, Security sin alertas; biblioteca 6776/0/3, mínimo 29 y SPI 43 pasan.
- Corpus final 72/72: recuperados 1058 comparados, 594837 palabras, orden plano 110427 (0.185642), reading-order 94887 (0.159518), transposiciones iguales a base; fusiones 278/211815. Sin cambios de umbrales. Detalle de 16 PDFs y avisos MuPDF en corpus-diagnosis.json.
- Publicación: cbd579660c708f95fe20273c73442bc481d252ba en origin/fix/issue-668-simple-encodings; PR #670 abierto en borrador contra develop, https://github.com/bzsanti/oxidizePdf/pull/670. Hook completo aprobado.
- CI #670: Ubuntu falla en tres pruebas #476 que trataban bytes WinAnsi indefinidos como CR/LF. Reproducido localmente (1 PASS/3 FAIL). Fixture actualizado con ToUnicode explícito, aserciones previas conservadas; añadidos NormalizeLineEnding y control WinAnsi sin mapeo. Seis pruebas pasan; integración completa de targets versionados en curso.
- Validación del fixture: 6/6 pruebas CR, 29/29 contratos, 220 doctests (24 omitidos), Clippy/formato y QR 93/6 tests sin alertas de seguridad. Batería ampliada terminada: 375 targets de integración + biblioteca, 9766 PASS/0 FAIL/47 omitidas; exit 0. Evidencia integration-final.json e integration.log.gz.
- Corrección publicada en #670: a8b595ecafc5988038361a567808053815570aa8; hook completo aprobado.
- Integración de base: develop 8c0c803 (#669) incorporado; conflictos limitados a CHANGELOG/TASKS, conservadas ambas entradas y ambas implementaciones sin cambios Rust manuales.
- Validación conjunta: 65/65 contratos pasan; archivos Rust entrantes idénticos a develop y archivos de codificación idénticos al HEAD revisado de #670.
- Siguiente acción: publicar el merge de develop y fusionar #670 cuando su nueva CI esté aprobada.
- Restricciones: sin /tmp ni worktrees; no cambiar codecs del sistema operativo, oráculos, umbrales ni solución MacRoman de #664.


## #667 sobre develop integrado — 2026-10-01

- Issue: #667 — fix(parser): decode PDF document strings with PDFDocEncoding and version-scoped UTF-8 — https://github.com/bzsanti/oxidizePdf/issues/667
- Estado: `[x]` implementada, validada e integrada en develop mediante PR #669. Prioridad P1. Responsable Codex.
- Alcance: adaptar el parche #667 previamente revisado a develop 4a2c38a, preservando las soluciones integradas de Omer; PDFDocEncoding, UTF-16 y UTF-8 PDF 2.0 en consumidores documentales.
- Criterio de cierre: 30 contratos documentales pasan; regresiones de #664/#665 y consumidores conservadas; revisión de calidad/seguridad y validación de configuraciones/corpus/CI proporcionada.
- Última validación: GitHub confirma #667 OPEN; implementación alternativa anterior separada, no trasladar #668 ni reemplazos de MacRoman/tracking.
- Última ejecución TDD en el repositorio actual: PDFDocEncoding RED 6 PASS/6 FAIL; PDF 2.0 RED de compilación por APIs ausentes. Parche aislado de #667 aplicado sin conflictos sobre 4a2c38a; GREEN y regresiones de #664/#665 en ejecución. Evidencia: docs/reports/2026-10-01-issue-667-develop-evidence/.
- Validación final local: 30 contratos y 50 regresiones pasan; biblioteca 6776/0/3; compression mínima 30/30; SPI+semantic 44/44; corpus diferencial 72/72, métricas idénticas a la base (278/211815 fusiones, orden plano 0.185653 y reading-order 0.159528), umbrales intactos. Clippy completo/formato/diff-check pasan; Kripteia 94/340 tests/10 archivos, Security sin alertas.
- Publicación: PR #669 abierto en borrador contra develop, https://github.com/bzsanti/oxidizePdf/pull/669; implementación 2217f6d.
- CI: Windows falló por transformación LF→CRLF del TSV; 16 checks pasaron y dos se omitieron por workflow. Reproducido con checkout-index y core.autocrlf=true: hash exacto del fallo. Añadido *.tsv text eol=lf; mismo checkout conserva el hash normativo y cero CRLF. Evidencia: docs/reports/2026-10-01-issue-667-windows-checkout.json.
- Validación de corrección: 30/30 contratos documentales pasan; no cambian código Rust ni valores del oráculo.
- Corrección publicada: aa06f27069cfe79473129284729dae7112cd0db8 en #669; hook completo aprobado (formato, Clippy, build y biblioteca 6776/0/3).
- Integración: issue #667 CLOSED como completed; PR #669 MERGED, commit 8c0c80321f236a139e783722d8a337251a7ceaf2; HEAD aa06f27 validado con 17 SUCCESS y dos jobs programados SKIPPED. Linux, Windows y macOS aprobados.
- Siguiente acción: continuar #668/#670 y el resto de #666; este bloque no completa el plan general.
- Restricciones: no usar /tmp ni worktrees. Restos locales obsoletos de #637 eliminados por instrucción del usuario, sin stash nuevo. Conservar tablas/oráculos; ninguna recalibración del corpus ni cierre de #666 por este bloque.

## Resolución #641 y desarrollo #642 — 2026-09-29

- Issue: #641 — docs(signatures): clarify basic support and keep PAdES outside library scope — https://github.com/bzsanti/oxidizePdf/issues/641
- Estado: `[-]` en curso. Prioridad P2. Responsable Codex.
- Criterio de cierre: README, rustdoc, CLAIMS y casos de uso describen firmas básicas, flujo externo y PAdES fuera de alcance; guard documental y ejemplo ejecutable validados; PR integrado.
- Última validación: contrato y ejemplo ya presentes en develop c4e08c7; se precisa exclusión de PAdES y se actualiza referencia a contención #640.
- Última validación final #641: dos guards documentales y cinco doctests pasan (dos omitidos); Clippy focalizado y revisión calidad/seguridad pasan.
- Siguiente acción: publicar PR #641, verificar CI e integrar; #642 continúa por separado.
- Issue: #642 — feat(encryption): implement interoperable certificate-based PDF recipient encryption — https://github.com/bzsanti/oxidizePdf/issues/642
- Estado: `[-]` análisis de perfil e interoperabilidad. Prioridad P2. Responsable Codex.
- Criterio de cierre: writer/reader públicos con cifrado real por certificados, múltiples destinatarios, rechazos, aleatoriedad, interoperabilidad bidireccional, no-C y revisión de seguridad.
- Última validación: #640 mantiene shell legacy no soportado; #642 confirmada OPEN. Implementación autorizada expresamente por el usuario.
- Siguiente acción: fijar perfil soportado y construir fixtures con implementación independiente antes de integrar la criptografía.
- Restricciones: PAdES fuera de la librería; extensiones externas disponibles a cualquiera. Sin PKI gestionada ni ampliación implícita de R6 por contraseña.

## Validación final #639/#653/#654/#655 — 2026-09-29

- Issues abiertas confirmadas en GitHub: #639 (identificación), #653 (árbol de páginas), #654 (entrada cifrada), #655 (filtro xref). Enlaces y criterios individuales debajo.
- Estado: `[-]` implementación y revisión local terminadas; PR #656 publicado, CI/integración pendientes. Prioridad P1. Responsable Codex.
- Última validación: workspace 9931 pasan/0 fallos/71 omitidos antes del endurecimiento final de nombres PDF; árbol final 15 regresiones en tres configuraciones (default, internal-testing+unstable-spi+semantic y compression-only). Clippy biblioteca/tests y formato pasan. RED/GREEN y mutación #655 discriminantes.
- Verificación independiente final: 30 salidas incrementales y 4 configuraciones nuevas; qpdf/Poppler pasan; 40 PDFs incluyendo fixtures sin advertencias. Info, XMP, claves escapadas, metadatos de página/catálogo, recursos heredados, contenido, Parent y prefijo verificados.
- QR calidad/seguridad completado: sin hallazgos abiertos dentro del alcance; limitaciones explícitas en docs/reports/2026-09-29-issue-639-final-review.md. Checksums del producto en evidencia contigua.
- Criterio de cierre restante: PR revisable, CI del HEAD publicado aprobada, integración verificada.
- PR: https://github.com/bzsanti/oxidizePdf/pull/656. Producto validado en 5a4e3b0; este seguimiento no cambia el código.
- Última validación adicional: 220 doctests del árbol final pasan, 24 omitidos.
- Siguiente acción: verificar CI del HEAD publicado de #656, resolver fallos y confirmar integración.

## Dependencia de validación #655

- Issue: #655 — fix(writer): omit FlateDecode when xref stream compression is disabled — https://github.com/bzsanti/oxidizePdf/issues/655
- Estado: `[-]` en curso. Prioridad P1. Responsable Codex.
- Criterio de cierre: xref sin compresión no anuncia FlateDecode; Info legible y qpdf limpio.
- Última validación: prueba de configuración efectiva #639 falla leyendo Info; filtro incondicional confirmado.
- Siguiente acción: retirar filtro cuando no se comprime, repetir matriz y qpdf.

## Finalización #639 y dependencias — 2026-09-29

- Issue: #639 — fix(writer): allow callers to suppress generated build and feature fingerprints — https://github.com/bzsanti/oxidizePdf/issues/639
- Estado: `[-]` en curso. Prioridad P1. Responsable Codex.
- Criterio de cierre: opt-out público, metadatos preservados por defecto, identificación efectiva, regresiones y revisión aprobadas, PR integrado.
- Última validación: WIP recuperado sobre develop 8c3c7fa; revisión anterior identifica defectos pendientes.
- Siguiente acción: reproducir y corregir pérdida de metadatos y dependencias #653/#654; validar con qpdf/Poppler.
- Issue: #653 — fix(writer): preserve inherited resources and leaf pages in legacy incremental updates — https://github.com/bzsanti/oxidizePdf/issues/653
- Estado: `[-]` en curso. Prioridad P1. Responsable Codex.
- Criterio de cierre: append/reemplazo parcial conservan hojas, recursos y cajas heredadas; qpdf y contenido verificables.
- Siguiente acción: regresiones con árboles planos y anidados y corrección compartida de escritura incremental.
- Issue: #654 — fix(writer): reject encrypted input before legacy incremental writes — https://github.com/bzsanti/oxidizePdf/issues/654
- Estado: `[-]` en curso. Prioridad P1. Responsable Codex.
- Criterio de cierre: las tres APIs y ambas políticas rechazan cifrado sin escribir bytes.
- Siguiente acción: prueba con fixture AES-256 y rechazo antes de copiar.
- #650/#651 fusionados en develop (00c4b73/8c3c7fa); issues #648/#649 cerradas. Validación conjunta: 77 regresiones pasan.

## Corrección de revisión #649 / PR #651

- Issue: #649 — bug(text): TJ negative kerning (adjustment > 0.2 em) injects spurious spaces when glyph advance widths are encoded as kerning — https://github.com/bzsanti/oxidizePdf/issues/649
- Estado: `[-]` rediseño publicado; actualizado con #637/#648, integración autorizada. Prioridad P1. Responsable Codex.
- Criterio de cierre: métricas cero verificadas, Type1/Type0/CID cubiertos, huecos reales preservados, revisión/corpus/CI aprobados.
- Última validación: RED 6 fallos/7 controles; GREEN 32 regresiones, 6780 biblioteca (3 omitidos), 72 diferenciales; Clippy y QR pasan. Incluye Type0/CMap/CID y controles de fuentes normales. Informe: docs/reports/2026-09-29-pr-651-correction-review.md.
- Siguiente acción: validar conjunto #637/#648/#649 y fusionar #651.

## Corrección de revisión #648 / PR #650

- Issue: #648 — bug(text): ShowTextArray boundary space threshold (0.7 em) suppresses word separator when baseline shifts across text runs — https://github.com/bzsanti/oxidizePdf/issues/648
- Estado: `[-]` integrada en develop mediante #650 (00c4b73), issue #648 cerrada. Prioridad P1. Responsable Codex.
- Criterio de cierre: distancia y guard vertical en la misma proyección; regresiones, corpus, revisión y CI aprobados.
- Última validación: RED cizallamiento sobre fa4f944; GREEN 27 regresiones, 6780 biblioteca (3 omitidos), 72 diferenciales; Clippy, formato y QR pasan. Evidencia: docs/reports/2026-09-29-pr-650-correction-review.md.
- Siguiente acción: validar combinación #637/#648 y fusionar #650; después integrar #651.

## Publicación #637 — PR #652

- Issue: #637 — fix(parser): reject exhausted FlateDecode recovery instead of reporting empty success — https://github.com/bzsanti/oxidizePdf/issues/637
- Estado: `[-]` PR abierto contra develop; integración pendiente. Prioridad P1. Responsable Codex.
- PR: https://github.com/bzsanti/oxidizePdf/pull/652
- Implementación validada: 8a93b20770bba1bfe8728c18ac9f6c60ce13f631.
- Última validación: workspace 9890 pasan/0 fallos/72 omitidos; gates T3 y orden pasan sin rebajar umbrales; QR cerrado, Clippy y formato pasan.
- Criterio de cierre: revisión y CI remotos aprobados, integración verificada.
- Siguiente acción: revisar checks del PR #652 y atender fallos; merge no ejecutado.

## Recuperación explícita #637 — 2026-09-29

- Issue: #637 — fix(parser): reject exhausted FlateDecode recovery instead of reporting empty success — https://github.com/bzsanti/oxidizePdf/issues/637
- Estado: `[-]` implementación validada; bloqueo técnico resuelto, integración pendiente. Prioridad P1. Responsable Codex.
- Autorización: diagnóstico por documento y recuperación explícita aprobada
  tras confirmar daño en los PDFs de entrada con qpdf/zlib independiente.
- Base: develop 0fb6444; rama fix/issue-637-explicit-recovery, clon aislado
  /tmp/oxidize-issue-637-recovery-20260929. Checkout original y #639 preservados.
- Alcance: APIs estrictas conservan rechazo; APIs nuevas devuelven contenido
  con diagnóstico de recuperación/incompletitud/omisión. Límites y predictores
  siguen siendo errores. Los gates distinguen recuperación e integridad;
  umbrales y baselines numéricos intactos.
- Criterio de cierre: RED/GREEN público, límites y diagnósticos verificables,
  T3 y diferenciales aprobados bajo protocolo explícito, QR y árbol final validados.
- Última validación: RED 10 fallos/1 control; GREEN 72 focalizadas; T3 1613/1761
  (91,6%), 54 recuperados con 136 diagnósticos. Orden plano 0,202859 y lectura
  0,178390, ambos pasan los límites previos. Clippy all-targets pasa.
  Consumidor externo compression-only pasa; mutación de diagnósticos/errores
  produce siete fallos previstos. Seguridad automática sin hallazgos.
- Diagnóstico: docs/reports/2026-09-29-issue-637-diagnosis.md y evidencia
  contigua. El caso previo no determinista qpdf_issue-99 requiere issue propia,
  responsabilidad mantenimiento/bzsanti; no se corrige ni recalibra aquí.
- Validación final: workspace aislado 9890 pasan/0 fallos/72 omitidos; features
  52 pasan. QR cerrado y Clippy del arnés final aprobado. Evidencia y manifiestos
  en docs/reports/2026-09-29-issue-637-validation-evidence/.
- Publicación de rama y PR autorizada expresamente el 2026-09-29.
- Siguiente acción: abrir PR contra develop y verificar su HEAD y checks remotos.
  Integración pendiente; issue continúa abierta.

## Integración signature-preparation #646 — 2026-09-28

- Issue: #646 — feat(signatures): integrate participant signature preparation for next release — https://github.com/bzsanti/oxidizePdf/issues/646
- Estado: `[x]` integrada en develop; issue cerrada. Prioridad: P1. Responsable: Codex.
- Petición expresa: integrar feature/signature-preparation contra develop y
  disponer de sus APIs en la próxima release. Requisito en PR y Unreleased.
- Criterio de cierre: rama actualizada, regresiones de preparación/firma/permisos,
  QR completo, PR contra develop y nota Unreleased. Integración completada; publicación de release pendiente.
- Última validación: develop 93e6796 integrado sin conflictos de código;
  TASKS combina ambos historiales. QR corrigió cuatro hallazgos con RED/GREEN.
  Workspace final: 9895 pasan, cero fallos, 72 omitidos; corpus T0–T6
  y diferenciales pasan. Focalizadas 46; default 10; interop externa 3 pasan.
  Consumidor compression-only pasa. Clippy/formato pasan; Kripteia 91/99/96,
  seguridad sin hallazgos. Mutaciones de DocMDP/FieldMDP detectadas por los tests.
- Evidencia: docs/reports/2026-09-28-issue-646-{quality-review.md,validation.json,consumer.rs}.
- PR: https://github.com/bzsanti/oxidizePdf/pull/647 — MERGED contra develop.
  Merge 32e380324062c20606c83089d4f6c5c9bf61c8d0, 2026-09-28T20:59:41Z.
  Head revisado d502249f1733c3d25d0565a36b31c62c53c0fc4d.
- CI final: 17 SUCCESS / 0 FAILED / 2 SKIPPED (corpus programado,
  validado localmente). Issue #646 cerrada tras verificar el merge.
- Siguiente acción: incluir esta integración en la próxima release;
  la publicación de la release sigue pendiente.

# Seguimiento #603 y #641 — 2026-09-28

- Issue: #603 — docs(adoption): reconcile public claims and establish audited adoption monitoring — https://github.com/bzsanti/oxidizePdf/issues/603
- Issue: #641 — docs(signatures): delimit basic support and PAdES extension points — https://github.com/bzsanti/oxidizePdf/issues/641
- Estado: en curso. Prioridad: P2. Responsable: Codex.
- Criterio de cierre: documentación publicada coherente, inventario con evidencia
  y límites, ejemplo ejecutable y guard documental; aceptación de monitorización
  registrada sin presentar datos sintéticos como adopción real.
- Base de integración: develop b76f738; inventario y medición de release 5.1.5.
  PR #644 corregida a develop; cambios de #640 y demás correcciones preservados.
- Última validación: 35 tests de firmas, dos guards y cinco doctests pasan;
  revisión final y Kripteia tests/security completados (log/informe adjuntos).
- Instrucción vigente del usuario: PAdES queda como extensión para quien quiera
  implementarlo; se retiran las referencias comerciales de esta propuesta.
- Rendimiento: el usuario solicita medir; stats incorpora pdf-generation-v1,
  comparativa ampliada a cinco motores (pdf_oxide 0.3.78) y panel separado. README usa resultados
  observados, incluidos los desfavorables; artefactos reproducibles adjuntos al informe.
- Publicación aprobada por el usuario el 2026-09-28.
- Siguiente acción: publicar PR, verificar CI e integración, desplegar stats y
  comprobar aceptación antes de cerrar #603. Batir a pdf_oxide no es un objetivo.

# Seguimiento diario

## Corrección #638 — 2026-09-28

- Issue: #638 — fix(parser): propagate invalid predictor errors in strict stream decoding — https://github.com/bzsanti/oxidizePdf/issues/638
- Estado: `[-]` PR #645 abierto; corrección de CI validada localmente. Prioridad: P1. Responsable: Codex.
- Activación: OPEN confirmada en GitHub; clon aislado /tmp/oxidize-issue-638,
  rama fix/issue-638-predictor-errors desde develop b76f738.
- Criterio de cierre: predictores inválidos/unsupported producen error explícito;
  dimensiones/arithmetic validadas; muestras PNG válidas exactas en ambas APIs
  y DecodeParms directos/indirectos; RED/GREEN, QR y validación proporcional.
- Alcance: predictores Flate/LZW. #637 permanece intacta en el checkout original;
  sin recalibración de métricas ni recuperación implícita de predictores.
- Última validación: RED de la suite final exacta sobre base inalterada:
  11 fallos/7 controles. GREEN final: 18 regresiones #638 y 6 de #514.
  QR corrigió pérdida de texto en Form por una ampliación de DecodeParms
  (retirada) y preservó defaults null según la especificación, con RED/GREEN.
  Consumidor externo compression-only pasa. Clippy all-targets y formato pasan.
  Kripteia 91/100 filtros y 98/100 nuevos tests; security sin hallazgos.
  Workspace final: 9847 pasan, cero fallos, 73 omitidos; exit 0.
  Corpus T0–T6 y diferenciales pasan; T3 1613/1761 (91,6%).
  QR del autor cerrado. Baselines y umbrales intactos.
- PR: https://github.com/bzsanti/oxidizePdf/pull/645 — base develop.
- Revisión de CI: run 36471334187 falla al compilar dos comparaciones con []
  con internal-testing,unstable-spi,semantic (E0282/E0283). Sustituidas por
  is_empty() sin cambios del parser; GREEN 18 regresiones #638 + 6 de #514.
  Kripteia 98/100 y Security sin hallazgos nuevos. Issue OPEN reconfirmada.
- Integración: develop 32e3803 incorporado; único conflicto en TASKS resuelto
  conservando ambos historiales. Código fusionado automáticamente.
- Siguiente acción: comprobar CI completa del PR actualizado;
  integración pendiente. #647 ya integrada en develop en tarea separada.

## Integración local de upstream para Studio — 2026-09-24

- Estado: `[x]` integración local validada.
- Prioridad: `P1`.
- Responsable: Codex.
- Solicitud: actualizar upstream y renderizador y corregir la firma de Studio.
- Criterio de cierre: conservar la preparación de firmas por participantes e incorporar 5.1.3 y #606; suite completa y Clippy sin warnings.
- Resultado: origin/develop 7115feee integrado en feature/signature-preparation mediante c0ba713d; se conservan los cuatro documentos locales anteriores.
- Validación: 9.630 tests correctos, 47 ignorados por la configuración existente y 119 ejecutables de ejemplos completados; firma y previsualización verificadas en Studio, con OpenSSL y comprobación visual de los artefactos.
- Análisis estático: Clippy completo correcto con -D warnings; registro target/signature-update-clippy.log.
- Autorización: el usuario confirma commit y push de las ramas del motor y renderizador el 2026-09-24.
- Siguiente acción: publicar esta rama y fijar su revisión Git en Studio y el renderizador.

## Contención #640 — 2026-09-28

- Issue: #640 — fix(encryption): fail closed for simulated public-key security handler operations — https://github.com/bzsanti/oxidizePdf/issues/640
- Estado: `[-]` implementación y QR local completados; PR pendiente. Prioridad: P1. Responsable: Codex.
- Activación: GitHub confirma OPEN y alcance de contención; base develop
  `089a92776dc52c6091767edd03caf3668f9a4cd6`. Cambios locales previos preservados.
- Criterio de cierre: operaciones simuladas rechazadas explícitamente, sin
  generar/devolver semillas como supuesto cifrado, todas las entradas SecurityHandler cubiertas,
  documentación pública veraz, RED/GREEN y revisión de calidad/seguridad.
- Alcance: API de cifrado por destinatarios; sin implementación de #642,
  sin PAdES avanzado ni cambios en criptografía por contraseña o firmas.
- Última validación: RED 13 fallos/1 control; GREEN 16 regresiones por API
  pública con y sin signatures. 18 pruebas CMS/certificados y siete de apariencia
  pasan. Clippy all-targets -D warnings, formato y build pasan. Biblioteca:
  6780 pasan/3 omitidos (11 tests de simulación sustituidos por nuevas regresiones).
  Diferenciales de fusión/orden pasan 34/38. Workspace: 9829 pasan/73 omitidos,
  incluidos 217 doctests aprobados. T0–T6 pasan. QR del autor cerrado antes de PR.
  Kripteia 96/100 (16 nuevos) y 100/100 (3 retenidos); security sin hallazgos.
- Evidencia: docs/reports/2026-09-28-issue-640-{quality-review.md,validation.json}.
- Siguiente acción: publicar PR hacia develop y comprobar CI multiplataforma.
  Sin release ni integración todavía; #640 permanece abierta.


## Corrección #633 y #634 — 2026-09-27

- Issue: #633 — Outline writer creates invalid sibling links when preceding items have children (5.1.5) — https://github.com/bzsanti/oxidizePdf/issues/633
- Issue: #634 — Reconstructive extract/split retain metadata while reporting DocumentInfo Discarded (5.1.5) — https://github.com/bzsanti/oxidizePdf/issues/634
- Estado: implementación y QR locales completados; integración/CI pendientes.
  Prioridad: P1. Responsable: Codex.
- Alcance: core, rama fix/issues-633-634 desde develop ff263dba en este checkout.
  Reservar IDs por lista de hermanos; propagar ReconstructMetadataPolicy al
  extractor compartido por extract/split. Sin cambios en Python ni release.
- Criterio de cierre: round-trip de jerarquías raíz/anidadas con destinos y
  estilos; extract/split Discard y FirstInputWins concuerdan con metadata real
  e informe en cada parte; RED/GREEN, QR y CI multiplataforma antes de integración.
- Última validación: RED 4 fallos/3 controles; GREEN 9 tests nuevos y 37 tests
  existentes (#539, #548, #621), 46 en total. Kripteia 98/100 sobre 141 tests;
  security sin hallazgos. Biblioteca: 6791 pasan/3 omitidos; Clippy all-targets
  -D warnings, formato y diff check pasan. QR cerrado antes de crear PR.
- Seguimiento previo: docs/reports/2026-09-27-issues-633-634-review.md.
- Preparación: retirada release/v5.1.3-606 local/remota tras comprobar PR #608
  integrado sin commits pendientes; archivos locales preservados byte por byte,
  copia /tmp/oxidize-checkout-cleanup-20260927 y stash checkout cleanup.
- Siguiente acción: terminar QR/validaciones, crear PR hacia develop y verificar
  Linux/Windows/macOS antes de integrar y cerrar ambas issues.

## Corrección #621 — 2026-09-27

- Issue: #621 — Split fails on reachable zero-offset catalog references; tagged structure needs per-part preservation — https://github.com/bzsanti/oxidizePdf/issues/621
- Estado: `[-]` en curso por petición del usuario. Prioridad: P1. Responsable: Codex.
- Alcance: core exclusivamente, clon persistente ../oxidize-pdf-621,
  rama fix/issue-621-tagged-split desde develop f9132e16. Sin cambios en Studio,
  Python, estadísticas ni otras issues.
- Criterio de cierre: fixture sintético 12 páginas dividido 1–3/4–6/7–12,
  contenido/orden/etiquetas y ParentTree/IDTree coherentes, diagnóstico de
  referencias malformadas, fuente y permisos preservados, publicación atómica;
  tests RED/GREEN, regresiones y QR antes de PR.
- Última validación: RED 4 fallos/1 control; GREEN 14 regresiones nuevas,
  37 pruebas existentes de operaciones/etiquetas y 6791 unitarias (3 omitidas).
  Seis partes verificadas con qpdf exit 0; Clippy all-targets sin avisos.
  Proyección incremental de K/ParentTree/IDTree, diagnóstico de objeto offset0,
  rechazo de alias/ciclos/índices corruptos y permisos/atomicidad conservados.
  QR completado antes de PR; Kripteia 94/100 en 117 tests, security sin hallazgos.
- Siguiente acción: registrar QR sobre fuente final, crear PR y validar CI
  multiplataforma antes de integrar. Sin publicación de release bajo esta tarea.

## Integración y cierre de #627 — 2026-09-26

- Issue: #627 — fix(architecture): enforce the mandatory no-C dependency requirement across signature verification and bindings — https://github.com/bzsanti/oxidizePdf/issues/627
- Estado: `[-]` en curso por petición del usuario. Prioridad: P1. Responsable: Codex.
- Criterio de cierre: PR #631 integrado con CI verde, core publicado desde GitHub,
  Python fijado a versión del registro sin C y wheel/sdist verificados; criterios
  originales de criptografía, gate y QR conservados. No cierre anticipado.
- Última validación: issue OPEN, #631 draft en b8e8f173, develop 410358d8;
  main/release v5.1.4, Python main usa =5.1.3 y contiene cambios documentales ajenos.
- Siguiente acción: publicar actualización revisada del PR, validar CI nativa,
  integrar y preparar release del core; después actualizar binding en checkout
  separado preservando cambios locales. Publicaciones mediante GitHub Actions.

## Repositorio y publicación de oxidize-webpki — 2026-09-26

- Issue: #627 — fix(architecture): enforce the mandatory no-C dependency requirement across signature verification and bindings — https://github.com/bzsanti/oxidizePdf/issues/627
- Estado: `[x]` traslado, repositorio público MIT, publicación desde GitHub y
  consumidor del registro completados. Prioridad: P1. Responsable: Codex / bzsanti.
- Ubicación autorizada: `/home/santi/repos/BelowZero/oxidize-webpki`, repositorio
  Git propio limpio. Sustituye el crate alojado inicialmente dentro del clon PDF.
- GitHub: https://github.com/bzsanti/oxidize-webpki, PUBLIC, MIT, main en
  64972dcb49e1acb1e7b1eae33703f25e73cb3082. Workflows CI/publicación activos.
- Publicación: https://crates.io/crates/oxidize-webpki/0.1.0, confirmada no retirada
  por índice oficial; checksum 462e96cb3710b049a843bb9f1da726053b6709d67c267fc5bf94fa54cdb887aa.
  Publicada EXCLUSIVAMENTE desde GitHub Actions, nunca desde local.
- Run: https://github.com/bzsanti/oxidize-webpki/actions/runs/36258653754,
  completed/success. Cinco gates (Linux/Windows/macOS, MSRV 1.88, lint) y job
  publish pasan. CI de push 36258641966 también success sobre mismo commit.
- Criterio de cierre satisfecho: paquete publicado verificable, licencia y ruta
  correctas, consumidor compilado con fuente registry y checksum actualizado.
- Última validación: descarga real de crates.io; código/documentos/licencia y
  vectores contrastados byte por byte (27 archivos); cargo check y 27 tests de
  firmas/certificados pasan. Consumidor externo con CC/CXX=false y sin dev-deps
  acepta cadena válida y rechaza revocada; gate normal/build sin errores.
- Consumidor en clon de #627: dependencia `oxidize-webpki = { version = "0.1.0",
  optional = true }`, sin path/git. Cargo.lock fija fuente registry y checksum.
- Autenticación: scope workflow habilitado por usuario; secreto cifrado
  CARGO_REGISTRY_TOKEN configurado en GitHub para paso de publicación, sin
  exponerlo en informes ni habilitarlo en validación de pull requests.
- Evidencia: docs/reports/2026-09-26-oxidize-webpki-publication.json y
  2026-09-26-oxidize-webpki-registry-consumer.patch; informe sin secretos.
- Siguiente acción de #627: actualizar PR #631 con consumidor del registro y
  verificar CI completa del core antes de integrar/publicar core y fijar Python.
  Esta publicación no afirma que ese PR esté actualizado ni que #627 esté cerrada.
- Restricciones: cambios originales preservados; sin sustitutos git/path para
  el proveedor. No se modificó el sibling Python sucio.

## Extracción de oxidize-webpki — 2026-09-26

- Issue: #627 — fix(architecture): enforce the mandatory no-C dependency requirement across signature verification and bindings — https://github.com/bzsanti/oxidizePdf/issues/627
- Estado: `[x]` extracción local completada, validada y aprobada en revisión
  independiente. Prioridad: P1. Responsable: Codex / bzsanti.
- Alcance: `oxidize-webpki` 0.1.0, crate independiente de PDF/TLS/WebPKI con
  metadatos propios, API pública ALL_VERIFICATION_ALGS, MIT y 24 fixtures;
  consumidor oxidize-pdf actualizado y CI extendida. Sin cambios de primitivas.
- Criterio de cierre: paquete autónomo verificable, contrato/vectores preservados,
  MSRV 1.88, consumidor/bindings validados y revisión independiente completada.
  Integración/publicación del core y pin Python son seguimiento separado de #627.
- Última validación: cinco tests y un doctest del proveedor, MSRV, package y tests
  del paquete extraído; 27 tests de consumidor; 11 del gate; diez grafos en cinco
  targets y builds Linux del proveedor/producto con CC/CXX=false pasan.
  Consumidor externo acepta cadena válida y rechaza revocada; formato/Clippy
  pasan. Wheels Python directo y desde sdist compilan sin C/C++ y pasan 4 smoke
  tests cada uno; snapshot con path, no versiones publicadas.
- Windows: reproducidos exactamente los dos fallos anteriores mediante CRLF
  en cms_content.bin; corregidos atributos Git binary. Checkout autocrlf conserva
  bytes y las 18 pruebas #526 pasan. CI nativa remota posterior aún no ejecutada.
- Revisión del autor y Kripteia tests/security completadas (98/100, sin hallazgos
  automáticos de seguridad); cfg desplazado detectado/corregido, default check
  pasa. Revisión independiente autorizada y completada por agente separado:
  40 entradas revisadas, cero hallazgos nuevos, tests/MSRV/package/consumidor
  repetidos con éxito; informe independiente conservado.
- Nombre: API crates.io responde 403; índice oficial responde 404. Sin reserva
  ni publicación. Aviso spin retirado permanece bajo el bloqueo previo.
- Evidencias: docs/reports/2026-09-26-oxidize-webpki-{validation.json,
  quality-review.md,implementation.patch}. Implementación en clon aislado
  `/tmp/oxidize-issue-627`, base b8e8f173; PR #631 remoto sin actualizar.
- Siguiente acción exacta de #627: actualizar PR #631 y verificar CI nativa
  multiplataforma antes de integrar; preparar publicación del proveedor antes
  del core, y después actualizar pin/binding. Esta extracción no publica
  paquetes ni cierra el resto de #627.
- Restricciones: cambios originales preservados; sin commits/push/publicación,
  sin alterar sibling Python sucio ni corregir el booleano ajeno a esta tarea.

## Issue #627 — certificados sin dependencias C

- Issue: #627 — fix(architecture): enforce the mandatory no-C dependency requirement across signature verification and bindings — https://github.com/bzsanti/oxidizePdf/issues/627
- Estado: `[-]` en curso. Prioridad: P1. Responsable: Codex / bzsanti.
- Base: develop `410358d8`; clon independiente, sin worktrees.
- Criterio de cierre: eliminar C del producto y bindings conservando algoritmos,
  cadenas, confianza, vigencia, uso de clave y revocación; TDD, gate de
  dependencias por feature/target, interoperabilidad y QR tests/security.
- Alcance ratificado: certificados obligatorios; Tesseract opcional no bloquea
  esta sustitución. No retirar verificación ni modificar #620/baselines.
- Última validación: proveedor Rust integrado; 6.821 pruebas de biblioteca,
  27 pruebas de cadenas/firmas, 3 de interoperabilidad, 11 del gate y 3 de
  binding instalado pasan. Clippy all-targets y formato pasan. Gate RED
  reproduce ring/cc; GREEN 20 configuraciones (5 targets × 4 selecciones).
  Compilación máxima del producto con CC/CXX=false pasa en Linux.
  Consumidor sin dev-dependencies acepta cadena válida y rechaza revocada
  con compression,signatures; gate y ejecución usan el lock copiado del core.
- Wheel Linux y sdist autocontenido reconstruido pasan con el core candidato;
  grafo real del binding sin ring/cc. Override de path solo en copia de
  validación de oxidize-python `81a74e6b`; no altera su árbol local sucio.
  Primer sdist con path absoluto descartado como prueba de aislamiento;
  el definitivo usa path relativo y compila su propia copia del core.
- QR: manual más Kripteia Rust 93/100 focalizado (22 tests), 91/100 del
  módulo completo (127 tests); Security sin hallazgos automáticos. Python
  no reconoce unittest (0 tests detectados), inspección y 11+3 tests reales.
  Mutación que omite el gate detectada. Hallazgo preexistente del wrapper
  Python registrado separadamente y bloqueado sin issue.
- Siguiente acción: conservar informe/evidencia, preparar PR del core; después
  integrar, publicar core y fijar esa versión en Python antes de cerrar #627.
  La CI multiplataforma está añadida, todavía no ejecutada remotamente.
  No se afirma cumplimiento de wheels ya publicados ni cierre de la issue.

## Issue #620 — contrato de serialización OmniDocBench

- Issue: #620 — benchmark(quality): define a consistent OmniDocBench text serialization contract — https://github.com/bzsanti/oxidizePdf/issues/620
- Estado: `[-]` los 7 hallazgos corregidos y validados localmente; integración
  en curso; stats permanece local por instrucción del usuario. Prioridad: P2. Responsable: Codex / mantenimiento (`bzsanti`).
- Criterio de cierre: exportadores conformes e integrados en ambos repositorios,
  comparaciones incompatibles rechazadas, equivalencia histórica de 981
  predicciones y evaluación oficial local con evidencia por página.
- Plan: `docs/plans/2026-09-24-issue-620-serialization-tdd.md`.
- Implementación: normalización histórica predeterminada, variante preservada
  explícita, serializador/vectores compartidos, identidad v2 y verificador de
  bytes. Adaptación de stats autorizada; cambios previos preservados.
- Última validación: 106 tests pasan (33 gate, 6 exportador, 8 exportador stats,
  5 release, 42 aplicación stats, 12 dashboard). Formato y Clippy pasan en ambos
  repositorios; diff check pasa. Cuatro mutaciones de tests detectadas.
- Correcciones: contrato tipado y persistente con migración v14 de stats;
  comparación por identidad en dashboard; ejecución vinculada a scores;
  YAML de una pasada; hashes de fuentes copiadas; procedencia estricta;
  tests de resumen y vectores reforzados.
- Informe de correcciones: `docs/reports/2026-09-24-issue-620-qr-fixes.md` y
  hashes en el JSON homónimo. Migración probada solo en bases de tests.
- Evidencia: 981/981 predicciones históricas idénticas; 994 archivos históricos
  sin cambios. Ambos exportadores coinciden en 981 archivos sobre develop.
  Evaluador fijado: texto 0,47311686306064565; orden 0,29228846821555693;
  921 páginas puntuables, 780 nativas, un error de extracción y un fallback
  oficial de timeout conservados. Sin recalibrar baseline.
- Procedencia: base `e1792e83dbf0fdb42f65742fbc705dc8de906e25`; los 141 objetos
  LFS materializados coinciden con la revisión del dataset. Código rastreado
  del evaluador sin cambios; entornos virtuales no rastreados documentados.
- Informe: `docs/reports/2026-09-24-issue-620-implementation.md`; hashes y
  resultados en `docs/reports/2026-09-24-issue-620-validation.json`.
- Dependencia externa: integración coordinada en oxidize-stats (`bzsanti`).
- QR: `docs/reports/2026-09-24-issue-620-quality-review.md`. Se repitieron
  46 tests, formato y Clippy: pasan. Kripteia Rust 98/100 (12 tests únicos);
  Python no detecta unittest (0 tests). Security sin hallazgos automáticos.
  Siete hallazgos manuales/reproducidos: contrato descartado por stats,
  scores sin vínculo a ejecución, YAML con rutas corruptas, hash de fuente
  viva distinto al harness, campos obligatorios vacíos y dos pruebas débiles.
- Integración: PR #628 — https://github.com/bzsanti/oxidizePdf/pull/628.
  Stats permanece local: desplegados web/scheduler/worker, esquema v14,
  11 resultados históricos y todos los datos anteriores preservados; health e
  integrity_check pasan. Backup online y versiones anteriores conservados.
- Validación oficial final: base e1792e8 y candidato b7fe817 (develop 4c479b2)
  completan export/evaluate/summarize/compare; 981 predicciones, 921 puntuables,
  780 nativas, un fallo y fallback oficial. Delta global/nativo 0.0.
  Exportadores idénticos en 981 archivos. Wrapper 84e274e conserva el entorno
  virtual; test RED reproducido y suite GREEN.
- Informe de integración: `docs/reports/2026-09-24-issue-620-integration.md`
  y JSON homónimo con hashes y evidencia del despliegue.
- Siguiente acción: esperar CI del último commit de #628 y fusionar en develop;
  confirmar merge y cerrar #620. Stats no requiere PR ni publicación remota.
- Restricciones: no alterar históricos, corpus, evaluador o baselines; todo
  el trabajo local está en este worktree y su target, sin usar /tmp. No se
  autoriza commit/PR/fusión de oxidize-pdf y despliegue local de stats;
  no crear remoto ni publicar stats.

## Premortem de aceptación masiva

- Estado: `[-]` en curso — promovida a `main`; release cancelada por el usuario
  para incorporar #606 antes de publicar `v5.1.3`.
- Prioridad: `P1`
- Responsable: mantenimiento/producto del repositorio (`bzsanti`)
- Issue: #603 — `docs(adoption): reconcile public claims and establish audited
  adoption monitoring` — https://github.com/bzsanti/oxidizePdf/issues/603
- Alcance: reconciliar las afirmaciones públicas de PDF/A, firmas y soporte
  empresarial; mantener el inventario versionado y el contrato de monitorización
  para `oxidize-stats`. La materialización de ese contrato se coordina con la
  release, no bloquea técnicamente este cambio local ni cambia métricas.
- Criterio de cierre verificable: README y guía de casos no se contradicen;
  cada claim rastreado tiene fuente, fecha, versión/commit, evidencia y límite;
  el contrato exige eventos auditables, privacidad, reglas deterministas y
  experimentos completos, y `oxidize-stats` lo valida en su propio árbol.
- Última validación: el 2026-09-19 `cargo test --workspace` pasó por completo
  (incluidos corpus T1--T6 y 216 doctests), `cargo clippy --workspace
  --all-targets -- -D warnings` pasó y `cargo package --locked -p
  oxidize-pdf --allow-dirty --no-verify` generó el paquete `5.1.3`. La prueba
  documental específica pasó (1/1), también tras normalizar CRLF en Windows;
  fallaría al retirar una capacidad o límite compartido. El contrato exige log
  append-only con hash encadenado, y el QR no encontró hallazgos de seguridad.
- Handoff 2026-09-19: #604 se fusionó en `develop` como `0311883` tras CI
  verde. El PR #605 hacia `main` está abierto con `6aa97a8`, que reconcilia los
  cuatro conflictos de metadatos de versión conservando `5.1.3`; su CI
  multiplataforma, interoperabilidad y T0/T1 seguía en curso al cerrar sesión.
- Siguiente acción concreta: resolver y validar #606 antes de preparar
  una nueva promoción y etiqueta `v5.1.3`; `oxidize-stats` debe después
  materializar y validar el contrato.
- Revisión 2026-09-20: GitHub confirma #603 abierta y PR #605 abierto,
  fusionable y con CI verde en `6aa97a8` (Linux/Windows/macOS, MSRV,
  SemVer, ejemplos, T0/T1 e interoperabilidad). Pasan 27 pruebas focalizadas,
  las 3 pruebas externas de firmas, formato y Clippy con warnings denegados.
  Kripteia: 90/100 en 211 tests de los archivos Rust cambiados, sin hallazgos
  de seguridad. El análisis general dio 94/100 en 9.758 tests, también sin
  hallazgos de seguridad. Informe: `docs/reports/2026-09-20-pr-605-review.md`.
- Release 2026-09-20: #605 fusionado como `b3c55bc3ded51f0b0a06cddb86fd28f34a82ac07`;
  su árbol coincide con el HEAD revisado. CI, corpus e interoperabilidad del
  merge pasan. Etiqueta anotada `v5.1.3` publicada sobre ese merge;
  Release `35508177202` superó condiciones y CI.
- Cancelación 2026-09-20 solicitada por el usuario: workflow `35508177202`
  confirmado `completed/cancelled` durante `Run tests`. Los pasos de GitHub
  Release y crates.io quedaron `skipped`; `gh release view v5.1.3` devuelve
  `release not found`. Se retiraron las etiquetas remota y local `v5.1.3`.
  #605 permanece fusionado; no se revirtió código ni se alteraron los cambios
  locales preexistentes. No reanudar la publicación antes de integrar #606.
- Restricciones de seguridad o arquitectura: no iniciar trabajo correctivo,
  recalibrar métricas ni usar salidas probabilísticas como evidencia de calidad
  hasta que existan indicadores observados y una calibración aprobada.
- Cierre de sesión: `cargo-sweep 0.8.0` previsualizó y limpió 2,95 GiB de
  artefactos de `target/` con más de cinco días mediante `cargo sweep --time 5
  /home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf`. Los diez elementos no
  rastreados del árbol se preservan: son cambios preexistentes o de propiedad
  incierta.

## Issue #606 — ajuste de apariencias de firma antes de 5.1.3

- Estado: `[-]` implementación local validada — hallazgo del QR corregido;
  integrada en `develop` mediante #607; pendiente de promoción a `main`.
- Prioridad: `P1`
- Responsable: Codex / mantenimiento (`bzsanti`).
- Issue: #606 — fix(signatures): render logo behind text and fit visible
  signature text in both dimensions — https://github.com/bzsanti/oxidizePdf/issues/606
- Alcance: logo centrado detrás del texto con proporción y opacidad conservadas;
  texto con todo el ancho disponible, saltos de línea y ajuste en ambas
  dimensiones; cálculo compartido entre validación y generación.
- Criterio de cierre verificable: regresiones con/sin logo, nombres largos,
  varias líneas y rectángulos insuficientes; error explícito cuando no cabe
  a tamaño mínimo, sin omisiones; apariencia cubierta por la firma.
- Implementación 2026-09-20: `SignatureAppearance::layout(rect)` expone las
  líneas, tamaño, margen, primera línea base e interlineado utilizados por el
  generador. Ajusta Helvetica entre 12 y 6 puntos usando sus métricas, conserva
  el contenido y saltos explícitos, y rechaza el texto que no cabe antes de
  acceder al PDF o invocar al firmante. Logo centrado detrás del texto, sin
  reservar columnas; mantiene proporción y opacidad. Changelog actualizado.
- Última validación 2026-09-20: 7 regresiones nuevas y 10 pruebas existentes
  de firmas pasan; 3 pruebas externas con qpdf/OpenSSL/pdftoppm pasan; la
  finalización conserva los bytes firmados de la apariencia. Pasan 6.789
  pruebas de biblioteca (3 ignoradas), el doctest de la API pública, formato,
  `git diff --check` y Clippy `--all-targets -- -D warnings`.
- QR 2026-09-20: Kripteia 92/100 en 34 tests de los cuatro archivos Rust
  afectados; nuevas regresiones 100/100; Security sin hallazgos. Se repitieron
  20 tests focalizados incluyendo interoperabilidad, formato y Clippy: pasan.
  La inspección confirmó que `measure_char` clona la tabla completa por
  carácter en el nuevo preflight. Informe:
  `docs/reports/2026-09-20-issue-606-quality-review.md`.
- Corrección del QR 2026-09-20: el preflight obtiene una referencia estática
  a Helvetica una sola vez y usa `char_width_unicode`, sin clones por carácter.
  Pasan las 20 pruebas focalizadas (incluida interoperabilidad y anchuras de
  `W`, `i` y texto acentuado), formato, `git diff --check` y Clippy
  `--all-targets -- -D warnings`. Kripteia repetido: 92/100, nuevas regresiones
  100/100; Security sin hallazgos. Hallazgo 1 cerrado en el informe del QR.
- Integración: commit `34f3be8` publicado en `fix/issue-606-signature-layout`;
  PR #607 — https://github.com/bzsanti/oxidizePdf/pull/607. Hooks de commit:
  formato, Clippy, build y 6.789 tests de biblioteca pasan (3 ignorados).
- CI de #607: primer intento macOS falló en el test preexistente
  `test_forms_memory_integration` (indicador 205 → 2000), ajeno a #606.
  Se solicitó repetir únicamente el job `106104724188` del run `35520915224`;
  no se modificó el test ni se relajaron gates.
- Resultado final de CI: Linux, Windows y macOS pasan; también MSRV, SemVer,
  ejemplos, corpus T0/T1 e interoperabilidad. La repetición macOS terminó
  correctamente en el job `106108205113` (7m14s).
- #607 fusionado en `develop` como `7115feee6352e678ab7b33c1630a5727f439296f`.
  Rama de promoción: `release/v5.1.3-606`, creada desde ese merge.
- Siguiente acción concreta: abrir y validar el PR de promoción hacia `main`.
  Mantener la issue abierta
  hasta la integración. Por petición del usuario, la release sigue pausada
  hasta una instrucción posterior.
- Restricciones: preservar los cambios locales; no volver a publicar el commit
  `b3c55bc` como 5.1.3 sin integrar esta corrección.

## Indicador temporal inestable en el test de memoria de formularios

- Estado: `[!]` bloqueada — falta issue abierta aplicable.
- Prioridad: `P2`
- Responsable: mantenimiento (`bzsanti`), responsable de crear la issue.
- Issue: pendiente; consulta de issues abiertas no encontró una aplicable.
- Hallazgo: `forms_cross_module_integration_test.rs:915` cuenta asignaciones
  durante 10 ms, no memoria; `:512` trata su variación como crecimiento de memoria.
- Última validación: CI macOS de #607, run `35520915224`, job `106104724188`,
  falló con 205 → 2000 (crecimiento 1795); código y log confirman la dependencia
  del tiempo. Archivo sin cambios en #606.
- Criterio de cierre verificable: prueba con medición/propiedad de memoria
  real y determinista, estable ante variaciones de carga del runner.
- Dependencia externa: mantenimiento debe crear o vincular una issue abierta.
- Criterio de desbloqueo: issue específica confirmada abierta en GitHub.
- Siguiente acción concreta: crear/vincular la issue y sustituir el indicador
  temporal por una comprobación de memoria válida.
- Restricciones: no corregir ni relajar el gate sin issue; repetir el job
  conserva el test y su configuración originales.

## Restaurar historial del changelog de 5.1.2

- Estado: `[!]` bloqueada — falta issue abierta aplicable.
- Prioridad: `P2`
- Responsable: mantenimiento (`bzsanti`), responsable de crear la issue.
- Issue: pendiente; ninguna de las issues abiertas consultadas el 2026-09-20
  cubre la pérdida del historial de releases.
- Hallazgo: `CHANGELOG.md:37` salta de 5.1.3 a 5.1.1 y elimina la entrada
  de 5.1.2 publicada el 2026-09-16; el cambio no afecta al código distribuido.
- Criterio de cierre verificable: restaurar la sección 5.1.2 y su corrección
  de widgets combinados conservando íntegra la sección 5.1.3.
- Última validación: diff de #605 y `gh release view v5.1.2` confirman
  la omisión y la publicación, respectivamente.
- Dependencia externa: mantenimiento debe crear o vincular una issue abierta.
- Criterio de desbloqueo: issue específica confirmada abierta en GitHub.
- Siguiente acción concreta: crear/vincular la issue y después restaurar
  la entrada histórica desde `v5.1.2:CHANGELOG.md`.
- Restricciones: no iniciar corrección bajo esta entrada sin la issue;
  no recalibrar métricas. No bloquea la publicación del código 5.1.3.

## Reforzar dos aserciones preexistentes de xref

- Estado: `[!]` bloqueada — falta issue abierta aplicable.
- Prioridad: `P2`
- Responsable: mantenimiento (`bzsanti`), responsable de crear la issue.
- Issue: pendiente; ninguna issue abierta consultada cubre estas aserciones.
- Hallazgo: `oxidize-pdf-core/src/parser/xref.rs:3184` y `:3318` aceptan
  tanto `Ok` como `Err`; no detectan cambios del resultado del parser.
- Criterio de cierre verificable: comprobar el resultado contractual y sus
  datos/error; una mutación que invierta el resultado debe hacer fallar el test.
- Última validación: advertencias Kripteia contrastadas con el código;
  las dos aserciones existían antes de #605.
- Dependencia externa: mantenimiento debe crear o vincular una issue abierta.
- Criterio de desbloqueo: issue específica confirmada abierta en GitHub.
- Siguiente acción concreta: crear/vincular issue y sustituir las tautologías
  por comprobaciones del resultado esperado de cada fixture.
- Restricciones: no corregir ni recalibrar métricas sin issue vinculada.

## T3 differential-fusion diagnosis

- Estado: `[x]` completada
- Prioridad: `P1`
- Responsable: Codex
- Issue: [#602](https://github.com/bzsanti/oxidizePdf/issues/602) —
  `fix(text): investigate T3 differential word-fusion regression`.
- Alcance: diagnosticar la regresión T3 del workflow de corpus de GitHub
  Actions `35198171008`; no recalibrar la línea base sin identificar los PDFs
  y pares de palabras responsables.
- Criterio de cierre verificable: una ejecución T3 completa conserva el
  resultado del gate y, si falla, su log enumera los PDFs principales y pares
  fusionados; la instrumentación focalizada compila, pasa pruebas y Clippy.
- Última validación: ejecución externa `cargo test --locked --test
  differential_fusion_test flat_extraction_does_not_fuse_more_words_than_poppler
  -- --nocapture` falló como se esperaba con el diagnóstico: 510/211.815
  fusiones (`0,002408`), 1.695 PDFs comparados y 107 omitidos. El principal
  responsable fue `format-corpus/preserve_027613.pdf` con 292 fusiones.
- Siguiente acción concreta: completada; el análisis correctivo continúa en
  la siguiente tarea.

## T3 differential-fusion remediation analysis

- Estado: `[x]` completada
- Prioridad: `P1`
- Responsable: Codex
- Issue: [#602](https://github.com/bzsanti/oxidizePdf/issues/602) —
  `fix(text): investigate T3 differential word-fusion regression`.
- Alcance: analizar las fusiones de `preserve_027613.pdf` y los demás
  principales responsables, identificar la regresión de separación y preparar
  una corrección sin actualizar la línea base.
- Criterio de cierre verificable: se identifica una causa reproducible, se
  añade una prueba de regresión focalizada y el gate T3 completo no aumenta la
  tasa respecto de la base vigente.
- Última validación: el QR detectó dos coberturas pendientes: la fuente del
  último glifo no se propagaba por la recursión de Form XObjects, y faltaba el
  límite negativo de `0,3 em` tras un cambio de fuente. Ambas se corrigieron
  con las regresiones `form_xobject_preserves_the_last_shown_font_for_tj_boundaries`
  y `a_subthreshold_font_change_boundary_stays_welded`; las 16 pruebas de
  #458 y Clippy pasan. La ejecución externa T3 posterior al QR también pasa
  con 285/211.815 fusiones (`0,001346`), 1.695 PDFs comparados y 107 omitidos,
  sin actualizar la base. La comparación anterior de la página 14 de
  `preserve_027613.pdf` reveló un salto de `0,333 em` tras cambiar de fuente
  entre `Tj` y `TJ` (`STAFF identifies`, `ACCOUNTS is`). La prueba focalizada
  `a_narrow_font_change_boundary_becomes_a_space` y el conjunto de 14 pruebas
  de #458 pasan. La ejecución externa completa de T3 pasa con 285/211.815
  fusiones (`0,001346`), frente a la base de `0,001412`, con 1.695 PDFs
  comparados y 107 omitidos; no se actualizó la base. `cargo fmt --check`,
  `cargo clippy --locked -p oxidize-pdf --all-targets -- -D warnings` y
  `cargo test --locked -p oxidize-pdf --lib` también pasan tras la corrección
  del QR (6.789 pruebas, 3 ignoradas).
- Siguiente acción concreta: completada; conservar la instrumentación de
  diagnóstico de pares en el gate diferencial para futuras regresiones.
- Restricciones de seguridad o arquitectura: no recalibrar la línea base ni
  aplicar cambios correctivos sin una issue vinculada y sin preservar el
  diagnóstico reproducible.

## Issue #581 — adaptadores estructurales OmniDocBench

- Estado: `[-]` en curso
- Prioridad: `P1`
- Responsable: Codex
- Issue: [#581](https://github.com/bzsanti/oxidizePdf/issues/581) —
  `benchmark(quality): add official OmniDocBench adapters for tables, layout,
  and formulas`.
- Alcance: exportador reproducible de tablas/layout mediante API pública;
  CDM queda N/A hasta que exista una API pública de fórmulas a LaTeX.
- Criterio de cierre verificable: adaptador y manifiesto validados, más
  resultados oficiales TEDS/layout y artefactos del evaluador fijado.
- Última validación: `../oxidize-stats/benchmark/datasets/omnidocbench-`
  `f5f559bddf50e36f7f9899d842d0006f13ce8afc` está disponible en solo lectura;
  `DATASET_INFO` y el checkout confirman la revisión fijada, 981 anotaciones y
  981 PDFs originales. El árbol de `oxidize-stats` está modificado, por lo que
  no se reutilizarán ni alterarán sus resultados históricos. El nuevo ejemplo
  `omnidocbench_structural_jobs` generó correctamente 981 jobs ordenados desde
  ese dataset en una ruta temporal. Sus 2 pruebas, las 5 pruebas del exportador
  estructural, formato y Clippy para ejemplos pasan.
- Siguiente acción concreta: usar el flujo de `oxidize-stats` para generar los
  jobs con `omnidocbench_structural_jobs` y ejecutar el evaluador fijado;
  importar o enlazar sus artefactos TEDS y layout sin mezclar sus métricas con
  texto/orden.
- Dependencia externa o equipo responsable: `oxidize-stats` es responsable de
  materializar el evaluador OmniDocBench y ejecutar su protocolo fijado en
  `337cc26965893db3ef53ddc119a6d6bb5bde096f`.
- Criterio de desbloqueo: `oxidize-stats` expone los artefactos verificables
  del evaluador (comando, configuración, resultados por página y resumen).
- Restricciones de seguridad o arquitectura: no inventar LaTeX, OCR ni
  estructuras que la API pública no exponga.

## Issue #619 — fixtures y contrato del helper (2026-09-24)

- Issue: #619 — test(parser): build valid PDF fixtures and assert multistream TJ operands — https://github.com/bzsanti/oxidizePdf/issues/619
- Estado: implementación local validada. Prioridad: P2. Responsable: Codex.
- Criterio de cierre: fixtures válidos y aserción exacta del helper sensible a pérdida de TJ.
- TDD: siete fixtures fallan en modo estricto antes del cambio; después pasan
  7+5+7 tests. Mutación a parseo independiente hace fallar el test del helper.
- Siguiente acción: revisar/publicar el cambio y continuar con #616.
- Restricción: #620 pertenece al usuario en otra sesión.

## Issue #616 — composición del footer (2026-09-24)

- Issue: #616 — fix(page): delimit preserved content before appending generated footer — https://github.com/bzsanti/oxidizePdf/issues/616
- Estado: implementación local validada. Prioridad: P1. Responsable: Codex.
- Criterio de cierre: preservar FOOTER tras operador/comentario sin EOL.
- TDD: RED 2 fallos y 1 control pasa; GREEN 3/3 más 7/7 multistream.
- Cambio: separador LF antes del footer generado; contenido original intacto.
- Siguiente acción: revisión final e integración; continuar con #615.

## Issue #615 — límites de objetos ante puntuación (2026-09-24)

- Issue: #615 — fix(text): preserve positioned text boundaries before punctuation and cover TJ — https://github.com/bzsanti/oxidizePdf/issues/615
- Estado: implementación local validada; corpus/OmniDocBench pendientes.
- Prioridad: P1. Responsable: Codex.
- Criterio de cierre: separar objetos posicionados y conservar supresión válida con Tj/TJ.
- TDD: RED reproduce Right.Left; GREEN 4 nuevas y 5 originales pasan.
  Mutación retirando supresión TJ falla en cambio de fuente (exit 101).
- Siguiente acción: validación conjunta local; continuar con #617.

## Issue #617 — recuperación por límite de stream (2026-09-24)

- Issue: #617 — fix(parser): recover valid later streams after a malformed content stream — https://github.com/bzsanti/oxidizePdf/issues/617
- Estado: corrección local; validación conjunta pendiente. Prioridad: P1. Responsable: Codex.
- Criterio de cierre: recuperar streams sanos y conservar operandos entre límites válidos.
- TDD: RED 3 fallos / 1 control; GREEN 4/4 nuevas y 7/7 originales.
  APIs predeterminada/plaintext/preserve_layout cubiertas. ParseOptions controla
  estructura PDF; parse_strict continúa rechazando contenido malformado.
- Siguiente acción: validar corpus y benchmark conjunto; continuar #618.

## Issue #618 — callbacks incrementales (2026-09-24)

- Issue: #618 — fix(streaming): emit callbacks before materializing all page content operations — https://github.com/bzsanti/oxidizePdf/issues/618
- Estado: corrección local; validación conjunta pendiente. Prioridad: P1. Responsable: Codex.
- Criterio de cierre: entrega temprana, cancelación y estado entre streams, sin materializar toda la página.
- TDD: RED asignaciones antes del callback 2.706 → 9.696.385 bytes al crecer
  la cola; GREEN 447 → 447, tanto stream único como varios. Cancelación conserva
  OperationCancelled; entrega completa comprueba texto/posición de 10.001 chunks.
- Regresión adicional RED/GREEN: imágenes inline divididas no emiten texto
  espurio desde sus datos. Pasan 9 multistream, 4 recuperación, 2 posicionamiento
  y 1 test de asignaciones/cancelación. No es medición de RSS ni pico vivo.
- Siguiente acción: quality-review, T2–T6, diferenciales y OmniDocBench local.

## QR — buffer por lotes vaciado al superar el límite (2026-09-24)

- Estado: `[!]` bloqueada; falta issue abierta aplicable. Prioridad: P2.
- Responsable: mantenimiento (`bzsanti`), responsable de crear/vincular la issue.
- Issue: pendiente. #618 cubre callbacks incrementales; este defecto preexiste
  en `TextStreamer::check_buffer_size` y afecta a la API por lotes.
- Hallazgo: `total_size` no se actualiza dentro del bucle de eliminación; cuando
  excede el máximo se eliminan todos los elementos, incluidos los que cabrían.
- Última validación: con límite 5 y chunks AAAA/BBBB se emiten ambos pero el
  buffer queda vacío; debería conservar BBBB. Código idéntico en develop base.
  Reproducción: `/tmp/oxidize-fixes-validation/buffer_probe.log`.
- Criterio de cierre: conservar los chunks recientes que caben y probar el
  límite sin depender del tiempo ni confundirlo con RSS.
- Dependencia externa: mantenimiento debe crear o vincular una issue específica.
- Criterio de desbloqueo: issue aplicable confirmada abierta en GitHub.
- Siguiente acción: crear/vincular issue antes de corregir esta API.
- Restricción: no implementar corrección bajo esta entrada sin issue.

## QR y validación final — fixes #619/#616/#615/#617/#618 (2026-09-24)

- Estado: implementación y QR completados; PR #622–#626 abiertos.
- Responsable: Codex. Prioridad: P1 (#619 P2). Issues vinculadas en las entradas anteriores.
- Criterio de cierre: TDD reproducido, QR completado antes del PR y gates locales aprobados.
- Código validado: `e28e4f54eb69057bafc4d8e67c8bd3680f2b26db`; base `e1792e83`.
- Resultado: 63 focalizadas, T2–T6 (23/22/25/26/23), diferenciales (34/38),
  biblioteca (6.791, 3 ignoradas), Clippy all-targets y formato pasan.
  T4/T5 omiten precisión por falta de ground truth. Baselines intactas.
- QR: 91/100 en 360 tests; seguridad auditó tres métodos unsafe del allocator
  de tests. Tres hallazgos de implementación corregidos; defecto preexistente
  del buffer por lotes bloqueado sin issue, registrado arriba.
- OmniDocBench local: 981 predicciones finales idénticas a la base integrada;
  texto global 0,473116863, nativo 0,378039828, orden 0,292288468 en ambos.
  Evaluación oficial completa sobre entradas idénticas, reutilización trazable
  en manifiestos y compare del gate aprobado. No se modifica #620.
- Evidencia: `docs/reports/2026-09-24-fixes-615-619-quality-review.md` y JSON asociado.
- Siguiente acción: publicar cinco PR separados, dependientes en el orden
  #619 → #616 → #615 → #617 → #618. Ninguno se crea antes de este QR completo.

## Publicación posterior al QR — 2026-09-24

- Estado: fixes implementados con TDD, QR cerrado y cinco PR confirmados abiertos.
- Responsable: Codex. Prioridad: P1 (#619 P2). Issues #615–#619 siguen vinculadas
  a sus entradas anteriores; #620 pertenece al usuario en otra sesión.
- PR #622 / issue #619: https://github.com/bzsanti/oxidizePdf/pull/622 — base develop.
- PR #623 / issue #616: https://github.com/bzsanti/oxidizePdf/pull/623 — base fix/issue-619-review-fixtures.
- PR #624 / issue #615: https://github.com/bzsanti/oxidizePdf/pull/624 — base fix/issue-616-imported-footer.
- PR #625 / issue #617: https://github.com/bzsanti/oxidizePdf/pull/625 — base fix/issue-615-positioned-punctuation.
- PR #626 / issue #618: https://github.com/bzsanti/oxidizePdf/pull/626 — base fix/issue-617-stream-recovery.
- Última validación: GitHub confirma ramas/HEAD y PR abiertos. #622 tiene CI
  en curso (SemVer ya pasa); #623–#626 aún sin checks remotos en sus bases
  temporales. La validación local completa está publicada con el informe QR.
- Criterio de cierre de implementación: cumplido (TDD, QR y gates locales);
  integración remota pendiente. No se afirma CI remota completa ni issues cerradas.
- Siguiente acción exacta: revisar #622 y verificar su CI; tras integrarlo,
  retargetear #623 a develop, validar CI y repetir en el orden indicado.
- Los cambios ya están publicados; no dependen exclusivamente de /tmp.
  Código medido: e28e4f54eb69057bafc4d8e67c8bd3680f2b26db; documentación de QR
  publicada en 380272fa8c08cbb448d0efbb5df09c4f7accd5ef. Los commits posteriores
  solo registran publicación y no alteran ese árbol Rust ni los resultados.

## Hallazgo del binding Python durante #627 — valid=True para PDF alterado

- Estado: `[!]` bloqueada; falta issue abierta aplicable. Prioridad: P1.
- Responsable: bzsanti / mantenimiento de oxidize-python, crear/vincular issue.
- Issue: pendiente. #627 sustituye el proveedor; este defecto del wrapper es
  preexistente e independiente, no se corrige bajo esta entrada.
- Evidencia: `oxidize-python/src/parser.rs:1913` usa `.is_ok()` como `valid`
  en `verify_pdf_signatures`. El fixture `signed_rsa_altered.pdf` devuelve
  valid=True tanto en el entorno Python previo como en el wheel candidato.
- Criterio de cierre: el wrapper informa del resultado de integridad, firma y
  certificados conforme a su contrato; tests positivos y negativos con PDFs
  reales distinguen errores de ejecución de resultados inválidos.
- Dependencia externa: issue de oxidize-python creada/vinculada por bzsanti.
- Criterio de desbloqueo: issue específica confirmada OPEN.
- Última validación: reproducción en ambos wheels, código del binding idéntico;
  las pruebas Rust de #526/#627 detectan correctamente las alteraciones.
- Siguiente acción: crear/vincular issue; después corregir wrapper y contrato.
- Restricciones: no presentar el booleano de este wrapper como prueba de
  validación completa ni modificar código del binding sin la issue aplicable.

## Configuración sin compression — hallazgo durante #627

- Estado: `[!]` bloqueada; falta issue abierta aplicable. Prioridad: P2.
- Responsable: mantenimiento (`bzsanti`), crear/vincular issue.
- Issue: pendiente; las issues abiertas consultadas no cubren este defecto.
- Hallazgo: `--no-default-features --features signatures` como dependencia real
  falla por referencias a flate2 sin cfg y try_standard_zlib_decode ausente.
  `compression.rs:7` y los demás imports son idénticos a develop base;
  los tests pueden ocultarlo al disponer de flate2 como dev-dependency.
- Última validación: consumidor aislado sin dev-dependencies falla con 19
  errores; log `/tmp/oxidize-627-product-probe-no-compression.log`.
- Criterio de cierre: configurar correctamente la dependencia obligatoria o
  implementar la opción sin compresión, con prueba desde consumidor externo.
- Dependencia externa y desbloqueo: issue específica confirmada OPEN.
- Siguiente acción: mantenimiento crea/vincula issue antes de corregir.
- Restricciones: no corregir bajo #627 ni afirmar que un grafo sin C implica
  compilación correcta. La matriz mínima de producto incluye compression.

## PR #664 follow-up — 2026-10-01

- Issue: #662 — bug(text): decode_macroman in extraction_cmap.rs is incomplete for bytes 0xA0..=0xFF — https://github.com/bzsanti/oxidizePdf/issues/662
- Status: local implementation validated; remote CI/integration pending. Priority P1. Owner: Codex / maintenance.
- Original contribution: Omer Shtivi, 70f28714e2ef7115c0f0898d47a3af6cae3de953; retained unchanged as parent history.
- Scope: Preserve the original match table; correct PDF currency and undefined codes, extend independent public-API coverage.
- Validation: 6 focused tests; 6776 library tests pass, 3 pre-existing ignored; Clippy and formatting pass. Independent RED recorded against the original PR; quality/security review complete within this diff.
- Closure: focused contracts, review and CI approved, integration verified; do not close the issue on local validation alone.
- Next action: append this commit to the existing PR branch without rewriting its history; check remote CI before integration.
- Constraints: no changes from #667/#668 or the general #666 implementation; no merge/release in this follow-up.

## PR #665 follow-up — 2026-10-01

- Issue: #663 — enhancement(text): fallback heuristic for TJ tracking inference in lenient mode when DescendantFonts cannot be resolved — https://github.com/bzsanti/oxidizePdf/issues/663
- Status: local implementation validated; remote CI/integration pending. Priority P1. Owner: Codex / maintenance.
- Original contribution: Omer Shtivi, c7031f7645d947de03264485c8e38c4452feebe3; retained unchanged as parent history.
- Scope: Preserve GlyphZeroWidthStatus and the original inference flow; restrict unknown-metric recovery by mode, font type, descendant, source code and ToUnicode evidence.
- Validation: 40 focused tests; 6776 library tests pass, 3 pre-existing ignored; Clippy and formatting pass. Independent RED recorded against the original PR; quality/security review complete within this diff.
- Closure: focused contracts, review and CI approved, integration verified; do not close the issue on local validation alone.
- Next action: append this commit to the existing PR branch without rewriting its history; check remote CI before integration.
- Constraints: no changes from #667/#668 or the general #666 implementation; no merge/release in this follow-up.

## Historial de preparación de releases conservado desde main

Las siguientes entradas son registros históricos; el estado vigente de 5.3.0 figura al inicio.

## Preparación del release 5.2.0 — 2026-09-29

- Issue: #659 — release: prepare oxidize-pdf 5.2.0 — https://github.com/bzsanti/oxidizePdf/issues/659
- Estado: `[-]` preparación en curso. Prioridad P1. Responsable Codex.
- Autorización: usuario solicita iniciar release tras CI #658 y selecciona 5.2.0.
- Base: develop 48d8b8f; historial main/v5.1.5 se conserva mediante merge.
- Criterio de cierre de preparación: versiones/notas coherentes, validación proporcional y PR de promoción a main revisable.
- Última validación: workspace 9934 pasan/0 fallos/71 omitidas; firmas 43 pasan/0 fallos/3 omitidas; Clippy, empaquetado y compilación de ejemplos pasan. Runtime RAG omitido por falta de corpus. #658 permanece borrador por RSA y se excluye del release.
- Siguiente acción: publicar PR a main y verificar su CI. Tag/publicación requieren CI verde y merge verificado conforme al proceso.
- Restricciones: preservar WIP original; sin #642, sin PAdES y sin cambios a métricas/baselines.


## Release 5.1.5 para #627 — 2026-09-26

- Issue: #627 — fix(architecture): enforce the mandatory no-C dependency requirement across signature verification and bindings — https://github.com/bzsanti/oxidizePdf/issues/627
- Estado: `[-]` preparación en curso. Prioridad: P1. Responsable: Codex.
- #631 integrado en develop f9132e16 con CI completa verde sobre b8ae9af9.
- Criterio de cierre: PR de promoción main con CI verde, etiqueta y publicación
  GitHub/crates.io confirmadas; después binding fijado a la release del registro.
- Base: develop integrado + main v5.1.4; conflicto de TASKS resuelto conservando
  ambas entradas. Changelog 5.1.4 preservado; versión 5.1.5 y proveedor publicado.
- Siguiente acción: validar paquete y QR, crear PR de release, verificar CI antes
  de integrar y publicar exclusivamente mediante GitHub Actions.
- No cerrar #627 hasta pin y validación final de Python. Cambios ajenos preservados.


## Avisos de dependencias retiradas durante release 5.1.4

- Estado: `[!]` bloqueada; falta issue abierta aplicable. Prioridad: P2.
- Responsable: mantenimiento (`bzsanti`), responsable de crear/vincular issue.
- Issue: pendiente; las issues abiertas consultadas no cubren estos avisos.
- Última validación: `cargo package --locked --offline` avisa de chacha20 0.10.0
  y spin 0.9.8 retirados del registro. Ambas entradas ya estaban en develop;
  la release no cambia el grafo de dependencias. No se infiere vulnerabilidad.
- Dependencia externa y desbloqueo: issue específica confirmada OPEN.
- Criterio de cierre: investigar motivo de retirada y resolver/validar el
  seguimiento de dependencias con evidencia reproducible.
- Siguiente acción: mantenimiento crea/vincula issue antes de corregir.
- Restricciones: no actualizar dependencias ni recalibrar métricas bajo esta
  entrada sin issue; no confundir con la eliminación de C de #627.



## Release 5.1.4 — 2026-09-25

- Issue: #629 — release: publish oxidize-pdf 5.1.4 maintenance fixes — https://github.com/bzsanti/oxidizePdf/issues/629
- Estado: `[-]` preparación y validación en curso. Prioridad: P1.
- Responsable: Codex / bzsanti. Publicación autorizada por el usuario.
- Alcance: develop `410358d`, fixes #609/#610/#613/#615–#619 y contrato #620.
- Última validación: #629 OPEN, última GitHub Release y registro crates.io 5.1.3;
  #620 integrado por #628. Copia independiente: `/tmp/oxidize-release-5.1.4`.
- Criterio de cierre: gates locales y CI verdes, PR fusionado en main,
  etiqueta sobre merge validado, workflow exitoso, GitHub Release y crates.io.
- Validación local final: workspace 9.801 pasan (73 ignorados), Clippy all-targets,
  formato, contrato Python 33/Rust 6 y compilación del paquete pasan. QR cerrado
  antes del PR; Kripteia Rust 94/100 y Security inspeccionado. Corpus oficial:
  evidencia integrada previa, no nueva ejecución. Informes de release en docs/reports.
- Siguiente acción: publicar rama y PR a main, esperar CI verde y fusionar;
  etiquetar el merge validado y verificar workflow, GitHub Release y crates.io.
- Restricciones: preservar cambios locales y baselines; #627 permanece pendiente
  y obligatoria. Esta versión conserva C en signatures/bindings por autorización
  explícita de publicar antes de #627. No corregir hallazgos ajenos sin issue.
