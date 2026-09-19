# Seguimiento diario

## Premortem de aceptación masiva

- Estado: `[-]` en curso — validada para integrar primero en `develop`
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
  documental específica pasó (1/1); fallaría al retirar una capacidad o límite
  compartido. El contrato exige log append-only con hash encadenado, y el QR no
  encontró hallazgos de seguridad.
- Siguiente acción concreta: integrar `release/v5.1.3` con `develop` mediante
  PR y CI verde, y promover el mismo commit a `main` para etiquetar `v5.1.3`;
  la release entrega el contrato a `oxidize-stats` para su materialización.
- Restricciones de seguridad o arquitectura: no iniciar trabajo correctivo,
  recalibrar métricas ni usar salidas probabilísticas como evidencia de calidad
  hasta que existan indicadores observados y una calibración aprobada.

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
