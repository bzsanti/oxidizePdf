# #666 — conciliación de pendientes y cierre F01–F06

## Resumen ejecutivo

#666 confirmada OPEN el2026-10-05. Se cierra la revisión local de F01–F06 y del contrato compartido E07. No se rehacen implementaciones existentes ni se atribuyen nuevos resultados a informes históricos. Se corrige un hueco de las pruebas Standard14; ningún archivo de producto cambia en este incremento. La matriz pasa de18 a25 filas validadas localmente, sobre46. Quedan21 filas por cerrar; son contratos de la matriz, no21 fases nuevas ni21 implementaciones pendientes.

## Contratos y evidencia

F01–F05: `text_standard14_metrics_contract_test` cubre224 PDFs,14 fuentes con anchos implícitos/explícitos y196 cambios de fuente con retorno; `text_spacing_contract_test` cubre ausencia/cero/no cero y recuperación; `text_encoding_contract_test` cubre las tablas completas, incluidas Symbol y Zapf. Fuentes externas AFM y pruebas de lectura/mutación: `2026-10-03-issue-666-standard14-output-review.md` y evidencia contigua. Las geometrías Zapf con ToUnicode explícito no sustituyen el contrato de codificación incorporada, comprobado aparte. Fuentes, PDFs y oráculos permanecen idénticos.

F06/E07: `text_type1_intrinsic_contract_test` conserva16 PDFs de PFB/CFF completos/subconjuntos, nombres reasignados, cuatro precedencias y avances; las pruebas embedded/expert complementan las variantes. Las dos pruebas inicialmente rojas del informe Type1 fueron corregidas bajo #674. Los hallazgos posteriores de procedimiento inerte y consumidor resuelto se corrigieron y se validaron en `2026-10-04-issue-666-qr-fixes.md`; `text_qr_regression_contract_test` prueba los16 casos por consumidor resuelto y el procedimiento inerte. El lector sigue siendo estático/acotado: no ejecuta PostScript calculado. Esa limitación preexistente no se presenta como soporte de un intérprete completo. Las políticas TrueType ya cerradas en F07 no bloquean F06.

Validación actual: **79 PASS/0 FAIL/0 ignoradas**, siete targets. Clippy con warnings denegados y formato pasan. No se repite corpus: el producto es idéntico al validado por F10 (7248/0/3, corpus incluido). Evidencia de este incremento en `2026-10-05-666-reconciliation-evidence/`; hashes y comparación con la instantánea F10 incluidos. Los controles externos y seis mutaciones de entrada de Standard14/Type1 son evidencia previa, no ejecuciones nuevas.

## Hallazgos

1. **Standard14 aceptaba coordenadas NaN:** `oxidize-pdf-core/tests/text_standard14_metrics_contract_test.rs:55`. El predicado `abs()>tolerancia` no detecta NaN. Se exige ahora finitud de x/y antes de tolerancia. Sonda sobre copias de la prueba real: ambos controles pasan; NaN pasaba antes y falla después con diagnóstico de coordenadas. Es un defecto de la guarda del test; no se observó NaN producido por el extractor.
2. **Estados históricos presentados como pendientes vigentes:** `oxidize-pdf-core/tests/fixtures/text_contracts/coverage.json`. F01–F06/E07 todavía incluían RED o trabajos ya corregidos y mezclaban cierre local con CI global. Se actualizan sólo las siete filas contrastadas, manteniendo informes históricos y pendientes de integración explícitos.

## Pendientes, en orden de cierre

La siguiente acción es E08/E09. Las filas siguientes permanecen abiertas hasta su propia conciliación y validación; esta lista distingue trabajo por revisar de implementación demostrablemente ausente. No se promete que haya que añadir código en cada fila.

| Filas | Evidencia existente | Acción verificable para cerrar |
|---|---|---|
| E08, E09 | Differences y precedencia; wrap corregido; recuperación de CMap | Revisar nombres inválidos y destinos Unicode malformados, incluidos surrogates; contrastar ambos consumidores y fijar pruebas/política donde falten. |
| C01 | F09/F10 H/V con/sin ToUnicode; códigos no Unicode y colas | Conciliar estas pruebas con la fila Identity y cerrar su revisión específica; el pendiente de ausencia de ToUnicode ya tiene evidencia. |
| C02–C06 |117 muestras Adobe, cinco colecciones, herencia KR y consumidores corregidos | Contrastar por familia H/V, revisión/supplement y límites; completar únicamente huecos demostrados y QR de estas filas. |
| C07 | UseCMap, notdef,1–4 bytes y recuperación #678 | Conciliar límites, ciclos, prioridad y errores strict/lenient por consumidor; conservar las discrepancias MuPDF documentadas sin contarlas como coincidencias. |
| C08 | F09/F10 recuperación privada; Registry indirecto corregido | Resolver compatibilidad Registry/Ordering/Supplement entre CMap y fuente y probar sus límites; no basta con la recuperación Unicode ya cerrada. |
| U06 | Muestras CJK y políticas de recuperación | Vincular el cierre a C02–C06 y verificar repertorio H/V; no atribuir shaping a decodificación. |
| U07 | No-BMP, ligaduras y combinantes válidos | Conciliar casos inválidos de E09, preservar secuencias sin normalización y cerrar QR del contrato. |
| S01 | Standard14/Type3/CID, cero/no cero/ausente | Contrastar métricas inválidas por subtipo y consumidor contra las correcciones existentes; añadir sólo combinaciones sin prueba discriminante. |
| S02, S05 | TJ/Tz y vertical #672/#673; cambios de estado | Revisar cobertura de signos, escalas, outlier y transiciones del estado de texto con valores geométricos observables. |
| S03, S04 | Tracking y recuperación con controles de espacios y umbrales | Contrastar espacios literales/ToUnicode/implícitos, ajustes iniciales/finales y elegibilidad; demostrar acceso a cada guarda pendiente. |
| S06 | Rotación, cizalla, escala y Forms; guarda NaN corregida | Conciliar combinaciones de fuente/transformación y validación externa del avance; cerrar revisión sin asumir producto cartesiano ilimitado. |
| S07 |196 cambios Standard14, BT/ET, q/Q y Forms anidados | Contrastar aislamiento/restauración de recursos y estado entre operadores/Form; completar casos concretos que falten. |
| S08 | Límites de programas/mapas F07–F10 y recuperación | Consolidar tabla de política por modo/API, sin confundir error del renderer con extracción de metadata válida. |
| S09 | APIs por página/documento y subtipos de fuente | Completar equivalencia de salida con Forms mixtos y recursos sombreados; comparar contenido y posiciones, no sólo cantidades. |

Después del cierre local de estas filas: revisión transversal y controles discriminantes; corpus final con métricas de sustituciones y contenido perdido según el plan, además de fusiones/orden; verificar el alcance real de esas métricas antes de darlas por hechas. Sólo después se inicia la CI, incluyendo configuraciones soportadas, MSRV y plataformas. Integración posterior requiere su alcance/autorización; no se hace merge ni release en este trabajo. No hay F11 ni se introduce nueva numeración.

## Calidad de Tests (Kripteia)

Ambos análisis repetidos en Standard14, Type1 y lector intrínseco. Standard14:100/100,7 tests; Type1:88/100,10 tests; lector intrínseco:94/100,7 tests. Resultados exactos en los tres logs; los helpers llegan al extractor real y las constantes son oráculos deliberados. La sonda NaN aporta discriminación que el scanner no garantiza. Las evidencias externas y mutaciones históricas conservan su fecha y alcance.

## Análisis de Seguridad (Kripteia Security)

Sin alertas automáticas en los tres alcances. Se conserva el límite de512KiB del lector Type1, parsing sin ejecución de PostScript y controles de límites CFF existentes. Ningún cambio de producto ni dependencia; copias de sonda bajo target, sin alterar programas ni fuentes congelados.

## Métricas

Siete filas conciliadas;79 pruebas pasan. Dos hallazgos corregidos (guarda de prueba y seguimiento). Revisión de tres targets principales y lector intrínseco, consumidores comunes y cinco informes históricos. CI/integración siguen pendientes globales. E08/E09 no iniciadas en este incremento.
