# Correcciones del QR de #620

Issue: #620 — benchmark(quality): define a consistent OmniDocBench text serialization contract — https://github.com/bzsanti/oxidizePdf/issues/620

Los siete hallazgos están corregidos localmente en el worktree
`fix/issue-620-serialization` y en los archivos afectados de oxidize-stats.
Autorización: «ok, corrigelos todos». Sin commits, publicación o cambios en
históricos, baselines, corpus, evaluador ni base de datos de producción.

1. **Consumidor de stats:** contrato tipado `omnidocbench-contract/v1`, obligatorio
   para importar resultados oficiales; el worker comprueba igualdad con el
   manifiesto. Migración 14 conserva valores históricos como `legacy-unverified`.
   Resúmenes y desgloses usan el contrato canónico en su clave única. API y
   dashboard conservan la identidad y separan comparaciones incompatibles.
   Pruebas: importación manual, worker, persistencia de dos variantes/desgloses,
   migración desde v13, idempotencia y agrupaciones del dashboard.
2. **Vínculo de scores:** nuevo comando `evaluate` ejecuta el evaluador fijado
   sobre copias verificadas en un directorio nuevo. Solo emite el manifiesto
   de ejecución tras éxito y comprobación de entradas/scores. `summarize`
   exige la vinculación con exportación, identidad, configuración y scores;
   `compare` exige el hash del manifiesto de ejecución. Pruebas: scores cambiados,
   serialización incompatible, fallo del proceso y entradas mutadas.
3. **YAML:** sustitución de tokens en una sola pasada en ambos repositorios.
   Pruebas con rutas que contienen el otro token y comillas.
4. **Hash de fuentes:** se calcula sobre los archivos copiados al harness antes
   de compilar; vectores y plantilla también se fijan en esa instantánea.
   Regresión que altera la fuente viva durante una llamada Cargo simulada.
5. **Procedencia obligatoria:** validación de tipo, cadenas no vacías y SHA-256
   de 64 caracteres hexadecimales. Pruebas negativas de valores vacíos,
   espacios, booleanos, números, listas, objetos y hashes mal formados.
6. **Tests de resumen stats:** comprueban contrato completo y correspondencia
   con manifiesto, exportador, vectores, plantilla y protocolos. Retirar el
   contrato del resumen vuelve a hacer fallar el test.
7. **Vectores:** inventario único y completo, 25 separadores explícitos y
   controles negativos con bytes concretos. Vaciar, eliminar un caso necesario
   o duplicar un vector hace fallar las pruebas. Copias idénticas en ambos repos.

## Validación

Pasan **106 tests**: 33 Python gate, 6 Rust exportador local, 8 Rust exportador
stats, 5 Python release, 42 Rust aplicación stats y 12 Node dashboard. Pasan
formato de ambos repositorios, Clippy del ejemplo y de stats con warnings
denegados, y `git diff --check`. Se detectan las cuatro mutaciones controladas;
solo afectaron a copias bajo `target/issue-620/qr-fixes-mutations`.

Logs: `target/issue-620/qr-fixes-*.log`. Hashes y resultados:
`2026-09-24-issue-620-qr-fixes.json`. Copias anteriores y delta aislado de stats:
`target/issue-620/qr-fixes-before` y `qr-fixes-stats-session.diff`.
Los manifiestos Cargo de stats conservan sus cambios preexistentes.

## Límites y siguiente paso

No se repitió el corpus oficial completo: la lógica de extracción/serialización
no cambió y se conserva la evidencia histórica anterior. Las regresiones del
nuevo wrapper de evaluación sustituyen el motor externo de forma controlada;
no acreditan una nueva ejecución oficial de 981 páginas. El resumen anterior
no se ha relabelado ni sellado con una ejecución inexistente.

Preparar la integración coordinada de ambos repositorios y ejecutar el nuevo
flujo `export → evaluate → summarize → compare` desde revisiones limpias antes
de usar sus resultados como gate oficial. La migración solo se ha ejercitado
en bases de pruebas; su aplicación real corresponde al despliegue de stats.
