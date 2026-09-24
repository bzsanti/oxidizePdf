# Seguimiento diario

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
