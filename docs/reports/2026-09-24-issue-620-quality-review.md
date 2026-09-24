# QR de #620 — contrato de serialización OmniDocBench

> Actualización posterior: los siete hallazgos se corrigieron por autorización
> explícita del usuario. Véase `2026-09-24-issue-620-qr-fixes.md` para los cambios
> y su validación. El cuerpo siguiente conserva el diagnóstico del QR original.

## Resumen ejecutivo

Revisión de los cambios locales sobre `e1792e83dbf0fdb42f65742fbc705dc8de906e25`
en `fix/issue-620-serialization` y de la adaptación autorizada de oxidize-stats.
Issue #620 confirmada OPEN en GitHub. Las fuentes revisadas coinciden con los
hashes del informe de implementación. No recomiendo integrar todavía: el contrato
se pierde en el consumidor de stats y varios controles no garantizan la
procedencia que anuncian. No se han implementado correcciones.

Se repitieron 46 tests focalizados (6 Rust exportador, 28 Python gate, 8 Rust
stats, 4 Python release), formato y Clippy del ejemplo con warnings denegados:
todos pasan. Las equivalencias históricas y la evaluación completa del turno
anterior conservan su evidencia; no se repitió el corpus durante el QR.
Rust 2021, MSRV 1.88 en oxidizePdf; harnesses síncronos con serde/serde_json,
Python 3.12 y stdlib. La compilación de stats usa el runner aislado del fix;
no cambia el Cargo.toml ni Cargo.lock preexistentes del repositorio vecino.

## Hallazgos

1. **Stats descarta el contrato al importar los resultados**:
   `../oxidize-stats/benchmark/scripts/release_oracle.py:109` añade
   `benchmark_contract`, pero `../oxidize-stats/src/db.rs:260` no lo declara en
   `BenchmarkSummary`. Serde ignora ese campo; el worker (`src/benchmark_worker.rs:106`)
   solo verifica el protocolo antiguo y la revisión. La persistencia
   (`src/db.rs:518`) tampoco guarda el contrato y conserva la clave de conflicto
   `(dataset_revision,library,version,protocol)`. Dos variantes con esa misma
   clave pueden mezclarse o reemplazarse pese a tener serializaciones distintas.
   Confirmado por inspección del productor, deserializador, validaciones y SQL;
   no se escribió en ninguna base de datos. Corrección: hacer obligatorio el
   contrato en el modelo de importación nuevo, validarlo en worker e importación
   manual, persistir su identidad y usarla al comparar o resolver conflictos;
   tratar los históricos sin contrato mediante una migración explícita.

2. **Los scores no están vinculados a las predicciones/configuración evaluadas**:
   `tools/benchmarks/omnidocbench_gate.py:545` carga cualquier `--scores` con las
   páginas válidas. La comprobación de `:558` autentica por hash el YAML del
   manifiesto de exportación, pero no que ese archivo ni esas predicciones se
   usaran para obtener los scores. Reproducción: un archivo separado de scores
   se acepta y se sella como `rust-split-whitespace-v1` sin aportar identidad
   alguna de su ejecución evaluadora. Por tanto, emparejar accidentalmente los
   scores de la variante preservada con un export normalizado elude la garantía
   de incompatibilidad. El hueco de vinculación ya existía en el flujo anterior;
   el nuevo campo de configuración no lo resuelve. Corrección: producir un
   manifiesto de evaluación con hashes de entradas, YAML y salidas, y exigir
   que coincida con el manifiesto de exportación antes de resumir/comparar.

3. **La sustitución de placeholders puede romper rutas válidas del YAML**:
   `tools/benchmarks/omnidocbench_gate.py:528` sustituye `/predictions` después
   de insertar la ruta del dataset. Si esta contiene, por ejemplo,
   `/predictions-dataset/OmniDocBench.json`, la segunda sustitución modifica
   también esa ruta ya insertada y genera comillas anidadas inválidas.
   Reproducido con `yaml.safe_load` del entorno del evaluador: error de parseo
   en la línea 12. El mismo patrón existe en
   `../oxidize-stats/benchmark/scripts/release_oracle.py:98` desde antes del fix.
   Corrección: construir la configuración como estructura YAML o usar sustitución
   de tokens en una sola pasada que no examine los valores interpolados; añadir
   el caso de un directorio de dataset que contenga `/predictions`.

4. **El manifiesto puede registrar un serializador distinto al usado por el harness**:
   `tools/benchmarks/omnidocbench_gate.py:499` copia las fuentes, pero `:521`
   calcula los hashes leyendo otra vez los archivos vivos después de ejecutar
   Cargo. Una edición durante la compilación produce hashes del código nuevo
   asociados a predicciones del anterior. Reproducción aislada con el proceso
   Cargo simulado: el hash del bundle copiado empieza por `69381f20`, mientras
   el manifiesto registra `4bfcc9fe` tras cambiar el serializador vivo. No se
   modificaron fuentes de producción ni se presenta la simulación como una
   compilación real. Corrección: calcular y conservar los hashes de los bytes
   copiados al harness, dentro de su vida útil, y registrar esos valores;
   aplicar la misma disciplina al fixture y a la configuración efectiva.

5. **Se aceptan identidades con campos obligatorios vacíos**:
   `tools/benchmarks/omnidocbench_gate.py:61` solo rechaza claves ausentes o
   `None`. Dos resúmenes sellados con `exporter_sha256`, `source_lock_sha256`,
   `rustc_version`, `cargo_version` y `evaluator_config_sha256` vacíos pasan
   `compare_summaries` y producen un delta 0.0. Esto contradice la procedencia
   obligatoria documentada, incluso sin modificar el sello después de generar
   el resumen. Corrección: validar tipos, strings no vacíos y formato de hashes
   SHA-256; reemplazar placeholders de tests por fixtures válidos y cubrir
   campos vacíos en ambos lados.

6. **Las pruebas del release no protegen la propagación del contrato al resumen**:
   `../oxidize-stats/benchmark/scripts/test_release_oracle.py:75` comprueba el
   manifiesto y que el hash de `summary.json` coincida, pero no su contenido
   contractual. Al eliminar solo la asignación de `benchmark_contract` en una
   copia del productor, los cuatro tests siguen pasando. Corrección: verificar
   el objeto contractual completo del resumen, su coincidencia con el reporte
   efectivo y el rechazo/persistencia en el consumidor real. Esa mutación debe
   hacer fallar el test por ausencia del contrato, no por un hash alterado.

7. **La prueba de conformidad pasa con todos los vectores eliminados**:
   `oxidize-pdf-core/examples/support/omnidocbench_serialization.rs:55` acepta
   `[]` y entra en un bucle vacío sin aserciones. Reproducción en un crate
   aislado que copia el módulo: sus dos tests pasan con el fixture vacío.
   La copia de stats tiene el mismo comportamiento. Corrección: validar que
   existen los casos obligatorios y todos los separadores/controles negativos
   del contrato, con IDs únicos, antes de recorrer los vectores; añadir la
   comprobación de corpus vacío y de eliminación de un caso requerido.

## Calidad de Tests (Kripteia)

Salida real de `run-kripteia.sh rust` sobre tres fuentes únicas del alcance:
**Overall Score: 98 | Tests: 12 | Files: 3**. El serializador idéntico de stats
no se contabilizó dos veces. Puntuaciones por archivo: exportador 98,
serializador 100, quality_oracle 97. Peores tests, con 90:
`effective_configuration_is_reportable`,
`ground_truth_uses_reading_order_and_excludes_non_text_content` y
`whitespace_normalization_is_deterministic`.

Se descartan los avisos de «no llama código de producción» para los tests que
sí invocan `extraction_config` o `normalize_text`. Los esperados literales son
apropiados para un contrato de bytes; `.unwrap()` en estos tests conserva la
información del error al provocar el panic. Los hallazgos 6 y 7 proceden de
mutaciones reproducidas, aunque el analizador otorgue puntuaciones altas.

Salida real para Python: **Overall Score: 100 | Tests: 0 | Files: 0**.
Kripteia no reconoció estos tests unittest; ese 100 no es evidencia de cobertura
Python. Se revisaron manualmente y se ejecutaron sus 32 tests.

## Análisis de Seguridad (Kripteia Security)

Ambas ejecuciones, alcance Rust y Python, terminaron con
**No security issues found.** No reportaron taint flows, secretos, unsafe/FFI
ni funciones peligrosas. La revisión manual no encontró secretos ni nuevos
bloques unsafe/FFI en el diff. Los subprocess usan listas de argumentos y la
extracción del archivo Git usa `filter="data"`. Los defectos de procedencia e
identidad descritos arriba siguen siendo relevantes aunque el escáner no los
clasifique como vulnerabilidades.

## Métricas

- Archivos inspeccionados: 28, incluidos contexto de consumo, manifiestos y docs;
  inventario con hashes en `target/issue-620/qr/scope.json`.
- Hallazgos totales: 7, todos vinculados al contrato/validación de #620.
- Archivos de código de producción/tests modificados por el QR: 0.
- Documentos de seguimiento escritos: este informe y TASKS.md.
- Fuentes del fix: hashes comprobados sin cambios respecto de la validación.
- Evidencia: `target/issue-620/qr/kripteia-{rust,python}.log`, logs de tests y
  lint, `reproductions.json`, `path-collision.log`, `mutation-summary-contract.log`
  y `empty-vectors.log`. Las mutaciones se hicieron únicamente en copias.
