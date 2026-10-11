# Rendimiento: diagnóstico y candidato para #700–#703

## Implementación de #700–#703: revisión del candidato

Base `512d7c54309cce7b744a9d747bf849ba4de4da52` (develop, árbol de 5.4.1), rama `perf/issues-700-703`, checkout existente `target/issue690-review`. El usuario autorizó continuar y delegó el orden: métricas, codificación, tracking y números. Se conservaron los cambios de la raíz y los controles binarios; no se crearon clones/worktrees. Los apartados de diagnóstico al final son históricos, anteriores a implementar las mejoras.

### Contratos y evidencia del candidato

| Issue / contrato | Cambio y comprobación | Estado |
| --- | --- | --- |
| #700: métricas, precedencia y sustitución | Referencias a tablas estándar y Arc a snapshots custom; cada consulta vuelve a resolver, sin locks durante la medición. API global deprecated sigue devolviendo una copia independiente. Test con contador local al hilo: RED 100 asignaciones, GREEN cero por 100 consultas calentadas. Anchuras, dos stores, sustitución, fallback global y snapshot viejo comprobados. | Demostrado localmente |
| #702: WinAnsi, escapes y UTF-16 | Emisión directa en buffer final, sin vector WinAnsi/UTF-16 intermedio. Oráculo histórico con BMP completo, controles, escapes y suplementarios; qpdf, Poppler bbox y raster por página en ASCII/acentos/CJK/flow. Reserva custom exacta con multiplicación comprobada. | Demostrado localmente |
| #701: caracteres por fuente/contexto/página | Bitmap privado de 128 bits evita repetir hashing ASCII; HashSet completo sigue siendo la autoridad exportada al writer. Unicode fuera de ASCII conserva inserciones. Clone no comparte estado mutable. Nueve tests de tracking y siete contratos writer, incluidos registro tardío y colisión Helvetica. Mutación que descarta no ASCII falla en la igualdad de conjuntos. | Demostrado localmente |
| #703: precisión, signo, orden y geometría | Solo enteros finitos de magnitud ≤2^53 usan decimal por divisiones enteras; fracciones/extremos usan el formateador anterior. Mantiene cero negativo y saneamiento a cero. Oráculo de bytes para todos los operadores afectados, fronteras y 4.096 patrones f64; mutación que pierde el signo de -0 falla. | Demostrado localmente; coste residual declarado |
| Regresión integrada e integración | Formato y Clippy all-targets/internal-testing pasan; calidad/seguridad manual y automática en rutas afectadas. No cambian manifests, lock ni dependencias de producto; no se añade C/FFI. | Suite completa aprobada; CI/integración pendientes |

### Resultados finales

Las cifras son reducción del tiempo (un valor negativo indica regresión). No se suman entre fases.

| Cambio/carga | Reducción de la razón de medianas | Comparación |
| --- | ---: | --- |
| #700, layout estándar | 52,2% | Control 5.4.1 frente a #700 |
| #700, tabla custom sintética de 10.000 caracteres | 96,9% | Caso sintético; no representa una fuente media |
| #700, generación con flow comprimido | 25,1% | Control 5.4.1 frente a #700 |
| #702, acentos/CJK/flow | 1,7–9,2% | Incremental sobre #700 |
| #701, acentos | 17,9% comprimido / 26,4% raw | Incremental sobre #700+#702 |
| #701, flow | 11,1% | Incremental; CJK raw no concluyente |
| #703, gráficos con enteros | 44,8% comprimido / 72,5% raw | Incremental sobre las tres mejoras de texto |
| #703, gráficos fraccionarios | −0,6% comprimido / −3,6% raw | Comprimido no concluyente; regresión raw observada |

El intervalo pareado 95% de #703 raw fraccionario es [−5,4%, −1,0%]: **no se afirma ausencia de regresión**. El candidato conserva este pequeño coste de detectar la ruta rápida a cambio del ahorro en enteros; queda visible para la revisión del PR, conforme al criterio de la issue de medir y declarar el coste del fallback. El intento original perdía 7,8–9,1% y fue descartado. No se promete que cualquier documento sea más rápido.

Las cargas de gráficos tienen 8.000 operandos afectados por documento: 100% enteros en la carga entera, 0% en la fraccionaria. La factura de diez páginas tiene 660 operandos afectados, 100% enteros. Tasas derivadas de entradas conocidas y conteo de sus operadores, no extrapoladas a PDFs reales arbitrarios. Se preservan qpdf sin avisos, bbox/texto y píxeles en los cuatro pares de gráficos y seis pares de facturas finales (todos sus folios). En las páginas exclusivamente gráficas Poppler emite el aviso esperado `no word list`; el verificador solo lo acepta si no hay palabras y su número coincide con las páginas rasterizadas.

Conjunto completo frente a 5.4.1, µs por documento:

| Caso | Control | Candidato | Menos tiempo | Intervalo pareado 95% |
| --- | ---: | ---: | ---: | --- |
| default-1 | 240.6 | 210.0 | 12.7% | [8.5%, 16.7%] |
| default-10 | 1078.5 | 756.6 | 29.8% | [17.3%, 37.4%] |
| default-100 | 8877.4 | 5649.2 | 36.4% | [34.1%, 37.9%] |
| raw-1 | 89.6 | 59.6 | 33.5% | [13.5%, 37.8%] |
| raw-10 | 591.7 | 288.6 | 51.2% | [46.7%, 56.1%] |
| raw-100 | 5662.9 | 2443.8 | 56.8% | [55.3%, 60.4%] |


### Protocolo y límites

Consumidor compression-only con lock independiente y perfil release opt3/debug1, mismo compilador Rust 1.96 y fuentes de carga para control/candidato. Cada comparación usa doce bloques con orden aleatorizado (seed 700), tres calentamientos por muestra y 10.000 remuestreos de la mediana pareada para intervalo bootstrap 95%. Incluye construcción, serialización, conversión/union de tracking y destrucción; excluye escritura y validación. Las mediciones no se solapan con compilación, perfiles o verificaciones iniciados aquí. Host compartido: el intervalo describe estas muestras, no garantiza el comportamiento en otros equipos.

No se suman porcentajes entre fases. Se conserva el intento inicial de #702 con reserva insuficiente, descartado por coste CJK; la reserva exacta y muestras más largas sustituyen aquella evidencia para aceptar el candidato. El log `702-unit.log` corresponde a una invocación accidental del checkout raíz, interrumpida; solo `702-unit-candidate.log` valida el candidato (868 PASS). El primer candidato #703 mostró una regresión del 7,8–9,1% en coordenadas fraccionarias: se restauró el formateo conjunto en ese fallback. Su primera suite completa se interrumpió al invalidar el candidato, antes de contar resultados; el log se conserva como `703-initial-workspace-interrupted.log`. Los fallos iniciales de inferencia de tipos de #701 y de permisos de un binario de control son fallos de montaje, no regresiones funcionales. Los logs definitivos se distinguen por sus nombres y comandos.

Asignaciones medidas separadamente con Valgrind, sin usar sus tiempos como benchmark: 104 documentos por proceso (3 calentamientos + 100 medidos + 1 salida), incluyendo inicialización/runtime. Control: 258.051 asignaciones, 113.583.724 bytes solicitados acumulados. Candidato conjunto: 168.611 asignaciones, 108.261.014 bytes en la medición final (34,7% menos llamadas, 4,7% menos bytes). No es RSS ni pico vivo. Ambos conservan el mismo bloque de runtime de 544 bytes al salir, con cero errores Memcheck. #701 añade un bitmap por conjunto y padding de almacenamiento; no se presenta como reducción de memoria por sí sola.

### Calidad de Tests (Kripteia)

Scans sobre las fuentes finales: métricas 90/55 tests; texto 99/38; encoding 85/35; flow 96/52; gráficos 99/158; página 96/82; ops 91/16; nuevo conjunto 100/1; nuevo test de asignaciones 90/1. Los números son tests reconocidos por el scanner, no ejecuciones. Los avisos sobre constantes esperadas corresponden a oráculos de bytes/anchuras; el aviso de baja proporción asserts/setup del contador no reconoce sus llamadas al helper con dos aserciones por caso. Los avisos de «no llama a producto» en ops omiten el helper de prueba que llama a `serialize_ops`; se trazó esa llamada. Un `unwrap` en una expectativa del test hace fallar ante ausencia de métricas, no descarta silenciosamente el error como sugiere el aviso. No se usa el score para aceptar contratos. RED/GREEN conservado para asignaciones y mutaciones discriminantes de Unicode/cero negativo.

### Análisis de Seguridad (Kripteia Security)

Sin hallazgos en los ocho archivos de producto escaneados. El scanner enumera cuatro hooks unsafe del allocator de test: todos delegan punteros, layouts y tamaños a System, sin acceso al bloque y con contadores TLS de Cell que no asignan; revisión manual completada. El nuevo producto es Rust seguro, no introduce FFI ni dependencias. Se revisaron desplazamiento ASCII acotado a 0–127, índices hex 0–15, buffer decimal de 16 dígitos acotado a 2^53, overflow de capacidad y liberación del lock antes de medir. Parser/OCR/criptografía/split quedan fuera: sus algoritmos y fronteras no cambian.

### Hallazgos de revisión

1. **Reserva custom insuficiente corregida**: `src/text/mod.rs:158` — se corrigió la reserva inicial de UTF-16, que generaba realocaciones en CJK; la medición final y el oráculo exacto reemplazan la aprobación del primer intento.
2. **Fallback fraccionario agrupado**: `src/graphics/ops.rs:228` — se corrigió el formateo individual de operandos del primer #703, que añadía coste al fallback fraccionario; la medición final recoge el coste residual y el beneficio de enteros.
3. **Documentación coherente con el contrato**: `src/text/mod.rs:196`, `src/text/encoding.rs:516` — se corrigieron comentarios heredados que afirmaban omitir tracking builtin y uso productivo del helper histórico de escapes; ambos contradecían los contratos conservados.

No quedan defectos confirmados de producto dentro del alcance revisado; la aceptación local está demostrada, con CI/revisión/integración pendientes. Revisión y corrección se ejecutaron en pasos separados. No se modificó código ajeno al alcance. #690 sigue abierta: el usuario no puede confirmar ahora la versión que falla y pidió diferir esa investigación.

### Métricas de la revisión del candidato

Diez archivos de producto/tests revisados, tres hallazgos corregidos y un tradeoff de rendimiento declarado; ningún defecto funcional confirmado pendiente. La revisión no modificó código durante su inspección; las correcciones se hicieron después como parte de la implementación autorizada. Fuentes exactas en `accepted-source-hashes.json`; binarios de las comparaciones identificados en `binary-hashes.json`. Suite completa: 10.434 PASS / 0 FAIL / 71 ignorados existentes, incluidas 221 pruebas de documentación. Exit 0; fuentes de producto idénticas a las medidas. Formato y Clippy all-targets/internal-testing exit 0. Pendientes: CI y revisión/integración del PR.

---

## Diagnóstico inicial de 5.4.1 (histórico)

## Resumen ejecutivo

Sí hay margen adicional. La oportunidad más concreta es eliminar las copias completas de métricas en cada consulta de anchura. En generación siguen pesando el registro de caracteres, la preparación del texto y el formateo de operadores. Son oportunidades posteriores a #661, cuyas optimizaciones ya están integradas; no se propone repetir aquel trabajo.

Revisión de solo lectura sobre `target/issue690-review`, HEAD `8a92dce443e743b385aad27bebf85ed03762b09c`, árbol idéntico a `v5.4.1`. El checkout raíz anterior y sus cambios locales se conservan. Las rutas de código siguientes son relativas a `target/issue690-review/oxidize-pdf-core`. Se inspeccionaron once archivos de producto/tests en el alcance de generación y layout; no se auditó exhaustivamente parser, OCR, imágenes, criptografía o split.

No se ha implementado una optimización ni demostrado un porcentaje de mejora. La consulta de GitHub devuelve siete issues abiertas (#294, #581, #583, #642, #662, #663, #690), ninguna aplicable a estos cuatro hallazgos. Su implementación queda registrada como bloqueada por falta de issue propia, conforme a AGENTS.md.

## Contratos y evidencia

| Contrato | Evidencia y criterio para una futura corrección | Estado |
| --- | --- | --- |
| Anchuras y resolución de fuentes | `measure_text_with` usa `lookup`; `flow.rs:224,249` lo llama por palabra y línea, `text_block.rs:57` por palabra. Conservar precedencia del documento, fallback global, sustitución del mismo nombre, anchuras y aislamiento entre documentos. | Copia y asignaciones demostradas; corrección pendiente |
| Registro completo de caracteres | `TextContext::write` registra todos los caracteres; `Document::add_page` agrega por nombre. La inscripción custom puede ser posterior y coincidir con Helvetica. La regresión `late_custom_registration_keeps_usage_from_both_contexts_and_pages` cubre inscripción tardía desde texto/gráficos. | Ruta inspeccionada y perfilada; representación alternativa pendiente |
| Bytes y geometría de los operadores | Mantener codificación, escapes, hexadecimal UTF-16, orden, decimales, redondeo y tratamiento de no finitos. Las pruebas existentes de escapes usan bytes esperados independientes. | Inspección y seis salidas públicas verificadas; candidato y mutaciones pendientes |
| Validez de documentos medidos | Seis PDFs, 1/10/100 páginas con/sin compresión: qpdf exit 0, conteos exactos y todas las líneas Poppler idénticas al generador esperado. | Demostrado; no incluye comparación visual |

Consumidor independiente: Rust 1.96.0, release opt-level 3/debug 1, sin LTO del workspace, `default-features=false`, `compression`. Lock propio conservado: flate2 1.1.10/miniz_oxide 0.9.1. No es una medición del binario construido con el lock del workspace. Host compartido sin control de frecuencia/planificación.

Doce bloques completos, seis casos aleatorizados por bloque (seed 1009): 72 muestras y 8.880 documentos cronometrados. Incluye construcción, serialización y destrucción; tres calentamientos por muestra. Escritura del PDF y validación quedan fuera. No se solaparon estas mediciones con los builds o perfiles iniciados en esta revisión.

| Páginas | Con compresión, µs/documento | Sin compresión, µs/documento |
| --- | ---: | ---: |
| 1 | 241,0 | 106,6 |
| 10 | 1.092,1 | 657,5 |
| 100 | 8.156,9 | 5.342,0 |

Medianas del mismo binario, no comparación antes/después. En diez páginas la salida ocupa 9.483 bytes comprimida y 27.315 sin comprimir: desactivar compresión implica un intercambio CPU/tamaño, no una mejora gratuita. Estas cifras no sustituyen ni recalibran series históricas.

Callgrind ejecutado por separado, 100 documentos de diez páginas más calentamiento y salida representativa. Mide instrucciones, no tiempo de CPU. Los costes inclusivos se solapan y no se suman: `TextContext::write` 35,38% comprimido / 51,87% raw; `build_show_text_op` 9,37% / 13,76%; `serialize_ops` 18,73% / 27,56%. La compresión reutilizada representa 31,59% inclusivo del perfil comprimido, pero su política ya fue optimizada en #661; no se propone modificar sus defaults sin nuevas curvas tiempo/tamaño.

## Hallazgos

1. **Copias completas de métricas por consulta**: `src/text/metrics.rs:286–308` devuelve `FontMetrics` por valor, clona el contenido del `Arc` custom y clona la tabla estándar. Cada palabra procesada por layout vuelve a pagar la copia. La sonda pública, tras calentamiento y excluyendo preparación/registro, mide 1.000 asignaciones y 4.624.000 bytes solicitados en 1.000 consultas de Helvetica; con una tabla custom sintética de 10.000 caracteres mide 1.000 asignaciones y 147.472.000 bytes. Solución: prestar las tablas estándar y conservar el `Arc<FontMetrics>` custom, resolviendo una vez por operación de layout cuando el contrato de sustitución lo permita. No retener bloqueos globales durante el layout ni introducir una caché que mezcle documentos. Los bytes son acumulados, no RSS/pico; los tiempos del contador instrumentado no se usan como benchmark.

2. **Hash por cada carácter, incluidos los repetidos**: `src/text/mod.rs:262–269` extiende un `HashSet<char>` con todo el texto de cada llamada. El perfil confirma coste relevante de `TextContext::write` y sus inserciones; ese coste inclusivo también contiene codificación y construcción de operadores, por lo que no equivale al ahorro posible de tracking. Solución candidata: conjunto compacto para ASCII con fallback Unicode, manteniendo unión por nombre y materialización cuando el consumidor la necesite. No suprimir el registro estándar: rompería los contratos de registro tardío/nombres custom coincidentes. Medir texto repetitivo y CJK, con pruebas de unión entre páginas y ambos contextos.

3. **Buffers intermedios y capacidad excesiva al preparar texto**: `src/text/mod.rs:150–174`, `src/text/encoding.rs:48` y `:523` construyen primero el vector WinAnsi y después otro vector escapado con capacidad `4 * len`; el primero crece desde `Vec::new`. La ruta custom materializa además un `Vec<u16>` y formatea hexadecimal por unidad. `build_show_text_op` pesa 9,37% inclusivo en el perfil comprimido. Solución: emitir codificación y escapes en un único buffer, reservar según una política medida e iterar UTF-16 directamente con tabla hexadecimal. Mantener sustituciones, bytes de control, caracteres no BMP y escapes exactos; comprobar capacidad/overflow ante entradas grandes. El ahorro temporal y de memoria viva todavía no está medido.

4. **Formateo general de números en cada operador**: `src/graphics/ops.rs:197–226,317–345` sigue usando `write!/writeln!` para coordenadas y otros valores. #661 optimizó números del writer de objetos, no esta ruta. `serialize_ops` representa 18,73% inclusivo comprimido y 27,56% raw, incluyendo otras tareas de serialización. Solución candidata: una ruta rápida para valores exactamente representables a la precisión requerida y fallback al formateador actual, o un formateador equivalente validado. Conservar redondeo, valores extremos, negativos/cero, no finitos y precisión por operador; no sustituir sin más por representación shortest. Exigir comparación exacta de bytes y geometría antes de atribuir mejora.

## Calidad de Tests (Kripteia)

Se ejecutó el script obligatorio de quality-review en cuatro archivos: `text/mod.rs` 99/100, 37 tests; `text/encoding.rs` 85/100, 35 tests; `text/metrics.rs` 90/100, 55 tests; `graphics/ops.rs` 90/100, 15 tests. Logs íntegros en el directorio de evidencia: 142 tests reconocidos por el scanner, no 142 tests ejecutados.

Entre los avisos figuran constantes esperadas y supuesta ausencia de llamadas de producto. La inspección de las pruebas de escapes/operadores confirma llamadas reales y constantes útiles como oráculo de bytes; no se convierten esos avisos en defectos. Los tests funcionales actuales no demuestran ausencia de asignaciones: una futura regresión de copias debe detectarse con un contador aislado y población fijada. No se mutó producto ni se repitió la suite completa/Clippy del core durante esta revisión de rendimiento. La validación nueva ejecutada es el build del consumidor, sus sondas públicas y los seis controles externos; no se presenta como aprobación de un candidato futuro.

## Análisis de Seguridad (Kripteia Security)

Los cuatro análisis finalizaron con “No security issues found”. Inspección manual: preservar límites de capacidad, Unicode completo, escapes y saneamiento numérico; evitar invalidar referencias al reemplazar métricas o sostener locks durante trabajo prolongado. El allocator de la sonda delega punteros/layouts a `System`, solo incrementa contadores atómicos y no asigna dentro de los hooks. No hay cambios de producto, dependencias, políticas de recuperación ni garantías de seguridad. El escaneo no certifica seguridad del repositorio completo ni de las soluciones aún no implementadas.

## Métricas

- Archivos de producto/tests inspeccionados en el alcance: 11; hallazgos: 4.
- Archivos de producto modificados: 0; checkout de release limpio al terminar.
- Evidencia: [2026-10-09-performance-review-evidence](2026-10-09-performance-review-evidence/), con muestras, locks, fuentes, perfiles comprimidos, logs y hashes.
- Reproducción: adaptar solo el path del manifest al checkout identificado, situar `probe.rs` en `src/main.rs`, compilar `--release --locked --offline --bin writer-perf-probe` y ejecutar `run.py` en un directorio nuevo. Para asignaciones: `metrics.rs` en `src/bin/metrics.rs`, compilar con `--features alloc-count --bin metrics`. Comandos Callgrind y cantidades constan en los perfiles/logs archivados.
- Pendiente para implementación: issues propias abiertas; candidato/control pareados; contratos y regresiones de cada cambio; comparación visual cuando afecte emisión/layout; revisión final y CI. Siguiente paso recomendado: métricas prestadas/Arc primero, después preparación de texto y tracking; formateo tras demostrar equivalencia numérica.
