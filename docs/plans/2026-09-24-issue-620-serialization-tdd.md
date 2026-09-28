# Plan TDD — contrato de serialización OmniDocBench (#620)

Issue: #620 — benchmark(quality): define a consistent OmniDocBench text serialization contract — https://github.com/bzsanti/oxidizePdf/issues/620

Estado: implementación y validación local completadas; integración pendiente. Prioridad: P2.

Ejecución: `docs/reports/2026-09-24-issue-620-implementation.md`. Las secciones
siguientes conservan el plan original; la evidencia final está en ese informe.
Responsable de implementación: mantenimiento de oxidizePdf; coordinación y
adopción del contrato: mantenimiento de oxidize-stats (`bzsanti`).

## Diagnóstico y evidencia revisada

- GitHub confirma la issue OPEN, sin comentarios, el 2026-09-24.
- `oxidize-pdf-core/examples/omnidocbench_export.rs`, `write_predictions`,
  escribe `result.text` directamente. Su informe solo describe extracción.
- `../oxidize-stats/benchmark/src/bin/quality_oracle.rs`, `extract_text`,
  llama a `normalize_text` antes de escribir predicciones Markdown. Aplica
  `split_whitespace().collect::<Vec<_>>().join(" ")`.
- `tools/benchmarks/omnidocbench_gate.py` tiene hashes de predicciones y
  exportador, pero no un contrato explícito de postprocesamiento en
  `IDENTITY_FIELDS`. `compare_summaries` compara también hashes del exportador,
  lockfile y versiones de herramientas: separar compatibilidad semántica de
  procedencia exige pruebas positivas, además de añadir rechazos.
- `../oxidize-stats/benchmark/scripts/release_oracle.py` registra ejecución,
  runner y artefactos, pero usa otra etiqueta de protocolo
  (`omnidocbench-v1-official`). El gate local etiqueta únicamente texto.
  Compartir serialización no convierte métricas distintas en intercambiables.
- Las pruebas locales del ejemplo comprueban nombres y configuración, no los
  bytes exportados. El test de integración Python comprueba existencia/hash;
  el test de normalización de stats cubre whitespace ASCII.
- La equivalencia histórica de 981 predicciones consta en la issue y en el
  informe local de revisión; no se ha vuelto a ejecutar durante este plan.

El checkout inspeccionado está en `release/v5.1.3-606`, HEAD
`f947ba0a076385d07144514aa992ae40ac6db400`, con cambios previos. GitHub confirma
develop remoto en `e1792e83dbf0fdb42f65742fbc705dc8de906e25`; las referencias
locales están desactualizadas. La reproducción ejecutable sobre esa integración
queda como primer paso. El árbol vecino de stats también tiene cambios previos.

## Decisión propuesta

Adoptar `rust-split-whitespace-v1` como serialización predeterminada del
benchmark en ambos repositorios, compatible con el histórico de stats.
Mantener `preserve-text-v1` como opción explícita para exportar el texto exacto,
con identidad incompatible con la primera. La coordinación debe confirmar
estos nombres y el cambio de predeterminado antes de integrar ambos cambios.

El contrato normalizado escribe UTF-8, elimina whitespace inicial/final,
colapsa cada secuencia reconocida por Rust `split_whitespace` a U+0020 y no
añade EOL ni BOM. Vacío y solo whitespace producen cero bytes. Conserva los
demás caracteres, puntuación, composición Unicode y orden. Fijar en el contrato
el conjunto de caracteres whitespace y cubrirlo con vectores explícitos; un
cambio de semántica Unicode requiere otra versión, no reutilizar el mismo ID.
La variante preservada escribe exactamente los bytes UTF-8 del texto extraído.

Aplicar la transformación en la frontera de exportación del benchmark, sin
cambiar `PlainTextExtractor` ni su API pública. Compartir especificación y
vectores de conformidad versionados entre repositorios; no introducir una
dependencia de la librería PDF en el protocolo del benchmark.

## Secuencia RED → GREEN → REFACTOR

### 0. Preparar la reproducción

- Obtener develop remoto y crear un checkout aislado; registrar SHA. Revisar
  allí las instrucciones y volver a comprobar las rutas y comportamientos.
- Capturar identidad de stats, fuentes efectivas y diff/hash de los archivos
  locales necesarios, sin atribuir sus cambios al commit base ni sobrescribirlos.
- Usar inicialmente un PDF válido de dos líneas y texto crudo controlado.
  Comparar bytes de ambos exportadores con la misma versión/configuración del
  extractor y la misma página. Registrar la divergencia antes de corregir.
- Cada ciclo debe conservar comando, fallo esperado y resultado posterior.
  Fallos de compilación o dependencias no cuentan como reproducción funcional.

### 1. Serialización pura y vectores de conformidad

RED: añadir casos con salidas literales independientes de la implementación.
La ruta actual del exportador local debe fallar para el contrato normalizado.

| Entrada (escapes) | Salida normalizada |
| --- | --- |
| `" A\r\n\tB  C \n"` | `"A B C"` |
| `"A\rB\nC"` | `"A B C"` |
| `"A\u00a0B\u2003C\u202fD\u3000E"` | `"A B C D E"` |
| `"A\u0085B\u2028C\u2029D"` | `"A B C D"` |
| `""` / `" \t\r\n\u00a0"` | `""` |
| `"A\u200bB\ufeffC"` | idéntica; no son separadores del contrato |
| `"árbol, 中文 e\u0301"` | idéntica; sin normalizar composición |

Añadir cobertura de todo el conjunto whitespace fijado, incluyendo VT/FF,
y controles negativos U+001C–U+001F para detectar sustituciones por semánticas
distintas en Python. Para `preserve-text-v1`, todas las entradas son idénticas
byte por byte. Verificar idempotencia como propiedad adicional, no único oráculo.

GREEN: función pequeña de serialización con selección validada; ID desconocido
produce error. Integrarla en la escritura real de ambos exportadores.
REFACTOR: centralizar cada implementación y consumir los mismos vectores
con hash de fixture registrado. No recalcular los resultados esperados con
`split_whitespace` dentro de los tests.

### 2. Integración de los exportadores

RED: ejecutar las rutas reales de escritura con un PDF válido de dos líneas;
comparar archivos con bytes esperados y entre ambos exportadores para cada
contrato. Usar PDFs de una página equivalentes a los slices que usa stats;
no comparar su página 0 con otra página del exportador por jobs.

Incluir texto vacío válido y error de extracción: ambos producen archivo vacío,
pero solo el error incrementa fallos y conserva identificación de página.
Comprobar nombres, población, configuración y contrato efectivos del informe.
Los casos Unicode puros del ciclo 1 no dependen del soporte de fuentes del PDF.

GREEN: propagar la opción desde las CLI hasta la escritura y los informes.
Probar el predeterminado histórico y la opción preservada. REFACTOR: evitar
normalización duplicada; mantener intacta la evaluación aproximada de stats
y las anotaciones originales del evaluador oficial.

### 3. Manifiestos y compatibilidad

RED: añadir pruebas en `omnidocbench_gate_test.py` y
`benchmark/scripts/test_release_oracle.py` de stats. Una pareja válida de
resúmenes sellados debe rechazarse al variar solo serialización, configuración,
dataset, evaluador, OCR, métrica o población. Reseñar y recalcular el sello en
estos tests para que fallen por incompatibilidad, no por un hash roto.

También rechazar contrato ausente/desconocido, incluso si falta en ambos lados.
Manipular bytes de predicción debe fallar en validación de integridad. Añadir
caso positivo: implementaciones con hashes de fuente distintos y conformidad
verificada pueden compararse bajo el mismo contrato; una versión candidata del
extractor y sus predicciones pueden diferir de la base. No exigir igualdad de
hashes de salida como condición general para comparar calidad.

GREEN: versionar el esquema y distinguir:

- Identidad semántica: serialización y sus opciones, extracción efectiva,
  dataset/anotaciones, evaluador/configuración efectiva, OCR, métrica y población.
- Procedencia verificable: commit/estado de fuentes, hashes del exportador y
  módulos de serialización, vectores, lockfile, toolchain y comandos reales.
- Artefactos: hashes de predicciones finales por página y árbol, informe de
  errores, resultados por página y resumen. Calcularlos después de transformar.

Conservar las comprobaciones actuales de integridad/procedencia; no eliminar
globalmente restricciones para permitir que dos herramientas sean compatibles.
Configurar una identidad común explícita para texto y otra para orden, con
huella de configuración que no dependa de las rutas absolutas de ejecución;
guardar además el YAML exacto como evidencia.

REFACTOR: validador común por flujo que use la identidad efectiva del informe,
no valores predeterminados supuestos. Auditar la copia del harness: el gate
actual archiva HEAD y copia solo `omnidocbench_export.rs`; si se añade un módulo
auxiliar, copiarlo, identificarlo y probarlo también sobre la release antigua.

### 4. Compatibilidad histórica y evidencia completa local

RED: prueba de comparación de árboles detecta archivo faltante/extra, mismos
nombres con bytes distintos y distinto conjunto de errores. Separar dos ensayos:

1. **Histórico:** extractor 5.1.3 exacto y el contrato normalizado deben reproducir
   las 981 predicciones de
   `../oxidize-stats/benchmark/results/releases/5.1.3/omnidocbench-v1-official/attempt-1/predictions`.
   Registrar hashes antes/después del histórico para confirmar que no cambia.
2. **Develop integrado:** ambos exportadores con el mismo SHA candidato y
   configuración deben coincidir entre sí; no exigir igualdad con 5.1.3, porque
   develop contiene cambios legítimos del extractor.

GREEN: generar en directorios nuevos y comparar nombre por nombre y byte por
byte, incluidos archivos vacíos y errores. Si el histórico no se reproduce,
diagnosticar versión, opciones, entradas y normalización; no reescribirlo. Una
migración alternativa exige documento y artefactos nuevos con identidad propia.
Los manifiestos antiguos sin contrato no se infieren automáticamente: generar
un sidecar de procedencia y equivalencia comprobada que referencia sus hashes.

Ejecutar después OmniDocBench **localmente**, dataset
`f5f559bddf50e36f7f9899d842d0006f13ce8afc`, evaluador
`337cc26965893db3ef53ddc119a6d6bb5bde096f`, `end2end_eval / quick_match`, OCR
apagado. Conservar las 981 predicciones, 921 páginas puntuables de texto y 780
nativas, así como errores y fallbacks. Obtener artefactos de texto y orden por
página. Registrar cualquier divergencia de scores aun con bytes idénticos;
investigar configuración/runtime/timeouts sin descartar páginas ni recalibrar.
No es necesario evaluar la variante preservada para cerrar si solo se ofrece
como protocolo distinto sin atribuirle scores; si se evalúa, separar su identidad.

## Validación y entregables

Comandos focalizados previstos, desde el checkout de cada repositorio:

```bash
cargo test --locked -p oxidize-pdf --example omnidocbench_export
python3 -m unittest tools/benchmarks/omnidocbench_gate_test.py
cargo fmt --check
cargo clippy --locked -p oxidize-pdf --examples -- -D warnings
```

En stats: pruebas Rust del binario `quality_oracle` con
`--manifest-path benchmark/Cargo.toml`, y
`python3 -m unittest discover -s benchmark/scripts -p 'test_release_oracle.py'`.
Añadir el comando del harness de conformidad entre repositorios al implementar.
Las pruebas ligeras pueden ejecutarse en CI; el corpus oficial completo se
ejecuta exclusivamente en local para esta issue.

Comprobar sensibilidad introduciendo temporalmente estas mutaciones: omitir
normalización, usar `split_ascii_whitespace`, añadir LF final, ignorar el campo
de serialización y omitir un archivo vacío. Cada una debe romper su prueba;
retirarlas antes de la validación final.

Entregables: contrato documentado, vectores compartidos, exportadores e informes
consistentes, validadores y pruebas negativas/positivas, manifiestos nuevos,
informe local con equivalencia de 981 archivos y resultados oficiales por página.
Actualizar la documentación del gate y de stats; conservar los informes
históricos como evidencia fechada. Registrar cada estado y validación en TASKS.md.

Cierre de #620: ambos repositorios adoptan el contrato, pasan conformidad y
rechazos, y están disponibles las evidencias históricas y oficiales locales.
La implementación local aislada no basta para cerrar la issue. Dependencia
externa: adopción en stats, responsable `bzsanti`; si no está disponible,
registrar bloqueo con desbloqueo al disponer de su implementación validada.
Mantener fuera de alcance #581, otros defectos del parser y cambios de baseline.

Validación de este plan: lectura del código local y consulta remota de issue,
backlog y SHA de develop; no se han ejecutado tests ni nuevos benchmarks.
Siguiente acción exacta: preparar el checkout aislado de develop y ejecutar
la reproducción y el primer RED del ciclo 1.
