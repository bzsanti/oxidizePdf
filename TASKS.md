# Seguimiento diario

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
- Integración en curso: PR #628 abierto; stats desplegado solo localmente en
  Docker, esquema v14 y 11 resultados históricos preservados. Backup consistente
  y temporizador del worker restaurado; sin remoto de stats.
- Regresión adicional observada en ejecución real: resolver el symlink de Python
  perdía el entorno virtual; se conserva ahora la ruta absoluta original.
  Test RED confirmado; validación final en curso.
- Siguiente acción: completar evaluación oficial y CI de #628;
  ejecutar el nuevo flujo export/evaluate/summarize/compare desde revisiones
  limpias antes de usarlo como gate oficial. No se repitió el corpus completo
  tras estas correcciones ni se relabeló la evidencia anterior.
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
