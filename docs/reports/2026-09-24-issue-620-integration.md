# Integración de #620

Issue: #620 — benchmark(quality): define a consistent OmniDocBench text serialization contract — https://github.com/bzsanti/oxidizePdf/issues/620

PR: https://github.com/bzsanti/oxidizePdf/pull/628, hacia `develop`.
`oxidize-stats` permanece exclusivamente local por instrucción del usuario.

## Validación oficial de commits

Se integró develop hasta `4c479b2`. La base del extractor es `e1792e8` y el
candidato probado es `b7fe817`; el wrapper corregido está en `84e274e`.
El corpus completo ejecutó `export → evaluate → summarize → compare` sobre
checkouts limpios del extractor, dataset y evaluador fijados. El intérprete
conserva su ruta de entorno virtual: resolver su symlink perdía las dependencias;
la regresión falló antes de corregirlo y las 33 pruebas pasan después.

Ambas ejecuciones conservan 981 predicciones, 921 páginas puntuables, 780 nativas,
un error de extracción y el fallback de timeout del evaluador oficial. Los
scores están vinculados a manifiestos de ejecución y hashes de las entradas.

- Error de texto global: `0.4731168630606457` en ambas ejecuciones.
- Similitud nativa: `0.6219601722288931` en ambas.
- Delta de similitud global y nativa: `0.0`.
- Exportadores local y stats: 981/981 archivos idénticos y mismo fallo.
- SHA-256 de predicciones: `ca23487e62e2347ce0e1a7edd6d82dddf910435d8490db4debe6dcb83eab433f`.

No se recalibró el baseline. Las métricas de orden y todos los resultados por
página se conservan bajo `target/issue-620/integration/*-evaluation-2/result`.
Los hashes, el resumen sellado y la comparación están en el JSON homónimo.
Los primeros intentos que fallaron por el intérprete no produjeron manifiestos
de ejecución válidos y se conservaron para diagnóstico.

## Despliegue local de stats

Nueva imagen `oxidize-stats:issue-620` compilada desde una copia del código
validado, sin añadir remoto ni publicar ese repositorio. Se actualizaron los
contenedores web/scheduler y el binario del worker. Ambos contenedores están
saludables y el temporizador del worker vuelve a estar activo.

La migración v13→v14 se ensayó primero sobre una copia consistente. Se creó
un backup online antes del despliegue y se conservaron las imágenes y el binario
previos. Tras desplegar, todos los valores anteriores de todas las tablas
coinciden con el backup y `PRAGMA integrity_check` devuelve `ok`. Los 11
resultados históricos se conservan como `legacy-unverified`.

No se hizo un commit global de stats: su exportador, worker y dashboard dependen
de trabajo local previo sin commit. Ese trabajo se preservó y permanece local;
la imagen y el inventario de hashes `stats-source-snapshot.json` registran
exactamente el código desplegado. Los deltas propios de #620 están guardados en
los artefactos de la tarea.

## Validación y siguiente paso

Pasan las 106 pruebas focalizadas y los hooks (6.791 tests de biblioteca,
3 ignorados), formato, Clippy y build. Se añadió un job de CI para el contrato
Rust/Python. Pendiente al escribir este informe: CI del último commit documental
y fusión de #628; no fusionar si un check requerido falla.
