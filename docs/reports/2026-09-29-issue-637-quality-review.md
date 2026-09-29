# #637 — revisión de recuperación explícita

## Resumen ejecutivo

Revisión del autor, sin delegación, sobre develop
`0fb6444adb05d0a57945198bc0976917f2201df3` y el WIP de
`fix/issue-637-explicit-recovery`. Los archivos exactos se identifican en
`2026-09-29-issue-637-validation-evidence/source.json`.

Se mantiene el rechazo de Flate inválido en las APIs de bytes. Las nuevas
APIs de recuperación tienen resultados propios con diagnósticos y distinguen
contenido sin integridad verificada, incompleto y omitido. No convierten límites
ni errores de predictores en recuperación. La recuperación fue autorizada por
el mantenedor después de confirmar daño en los PDFs de entrada con qpdf/zlib.

Los tres gates que bloqueaban la implementación estricta pasan bajo el protocolo
explícito autorizado, con umbrales y baselines numéricos intactos. No se afirma
que mejore la integridad de los PDFs dañados ni que recuperación equivalga a
éxito estricto. Revisión manual y análisis automáticos completados. Workspace aislado: 9890
pruebas aprobadas, cero fallos y 72 omitidas (exit 0). Matriz focalizada de
features: 52 aprobadas. Clippy del arnés final también pasa. No hay commit, PR, integración ni release.

## Contratos y evidencia

| Contrato / origen | Productor y consumidor | Contraejemplo y evidencia | Estado |
| --- | --- | --- | --- |
| No éxito vacío fabricado; aceptación #637 | inflate_bounded → APIs estrictas → documento/extracción/Form | 18 regresiones públicas: bytes corruptos, cada prefijo truncado, checksum, vacío legítimo y PDF completo. Pasan; RED anterior conservado en informe del 28. | Demostrado |
| Recuperación observable; decisión del mantenedor | decode_stream_with_recovery → RecoveredStream → RecoveredText | Checksum erróneo conserva bytes con Unverified; bloque DEFLATE almacenado truncado produce exactamente ABCD con Incomplete. RED 10 fallos/1 control sobre API sin recuperación; GREEN 16 pruebas finales. | Demostrado |
| No omitir límites ni predictores | inflate_bounded / apply_declared_predictor → StreamRecoveryErrorKind → extracción | Límite de 16 KiB con salida de 20.000 bytes; expansión de 65 MiB que activa ratio; predictor TIFF no soportado con checksum dañado. Propagan error. El cap absoluto usa el mismo helper ejercitado por la regresión de 256 MiB. | Demostrado |
| Conservar contenido sano sin ocultar omisiones | Contents y Form Do → extracción por página | Flate agotado/Form vacío seguido de KEEP; identidad de Contents o nombre y referencia del Form en diagnóstico. Reutilizar extractor vuelve a fallar en API estricta. | Demostrado |
| No inventar operadores al omitir un stream | recuperación de Contents → grupos de parser | Operando GHOST antes del stream omitido no se une a Tj posterior. Los operandos de dos streams verificados sí conservan KEEP. | Demostrado |
| Límites y diagnósticos son observables fuera del crate | API pública → consumidor sin dev-dependencies, compression-only | Consumidor externo acepta ABCD incompleto explícito, rechaza límite 3 y Flate vacío; ruta byte-only rechaza incluso con ParseOptions::tolerant(). | Demostrado |
| Gates comparan recuperación sin afirmar integridad | T3 y diferencial de orden → JSON/logs | 1613/1761 (91,6%), 54 documentos recuperados; 136 diagnósticos: 105 unverified, 8 incomplete, 23 omitted. Orden 0,202859 / 0,178390, 1058 PDFs comparados en ambas rutas. | Demostrado |

La suite focalizada total comprende 72 pruebas (#637 estricto/recuperación,
#638, #617 y #458), todas aprobadas. Los 54 documentos con diagnóstico y éxito
son exactamente los 54 rechazos identificados en el diagnóstico; ningún PDF se
excluyó para alcanzar el umbral. La extracción estricta permanece en torno a
1560/1761, con variación de un archivo por el hallazgo previo descrito abajo.

Mutación en copia desechable: retirar diagnósticos y convertir los errores
fatales de extracción en omisiones produce siete fallos funcionales (9/16
controles pasan), incluyendo pérdida del diagnóstico y falso éxito ante límite.
El árbol revisado no se modificó para esa prueba; no se atribuye la mutación a
un cambio real del producto.

Incidencia de validación: la primera ejecución workspace reutilizó el binario
de esa prueba mutada en el target compartido y repitió sus siete fallos. Sus
resultados globales se descartan. Las fuentes conservan todos sus hashes;
la ejecución final usa `/tmp/issue-637-final-validation-target`, exclusivo del
candidato. La suite focalizada inicial, el corpus focalizado y el consumidor
externo se ejecutaron con los diagnósticos correctos, antes de la mutación o
con artefactos independientes. No se presenta la primera ejecución como verde.

### Revisión manual

- El decoder exige StreamEnd y checksum para éxito estricto. Comprueba bytes
  producidos y ratio antes de aceptar incluso el último bloque que señala error.
  Solo errores de formato pueden llegar a la recuperación; los límites no
  desencadenan fallback ni retorno de prefijo.
- Dos intentos como máximo: zlib y DEFLATE bruto. La cabecera debe ser válida
  para saltar sus dos bytes; no se prueban offsets arbitrarios. Diccionarios
  predefinidos no se interpretan como payload. No se mantienen simultáneamente
  los buffers expandidos de ambos intentos.
- El pipeline mantiene el límite de cada etapa y reutiliza el postprocesado
  estricto de #638. Las referencias DecodeParms se resuelven en el documento
  antes de entrar al decoder. No hay nuevos paquetes ni cambios de lockfile.
- El contexto de recuperación vive solo durante la llamada explícita por
  página y se retira tanto al devolver texto como al propagar un error. La
  API estricta lo desactiva. Las llamadas anidadas de Forms conservan el mismo
  contexto y restauran la caché de fuentes al propagar errores.
- Las omisiones están limitadas al error tipado InvalidFlate. Otros errores
  no se reconocen por substring ni se convierten en éxito. Las fronteras de
  stream recuperado/omitido impiden empalmar operandos; el estado gráfico
  posterior puede seguir siendo incierto y el resultado lo lleva diagnosticado.
- El cap es por contenido/Form y por etapa, no un límite global de memoria.
  No se afirma que reparar Flate valide fuentes, estructura, operadores ni
  otros filtros. PlainTextExtractor y las APIs previas siguen siendo estrictas.
- T3 guarda éxito estricto, éxito con recuperación, errores y diagnósticos por
  documento. El control del 90% mide disponibilidad bajo recuperación explícita.
  Orden usa la misma política y registra errores/omisiones; el baseline previo
  ya contenía recuperación implícita. No se alteran sus valores ni el ratchet.

## Hallazgos

1. **La creación del directorio de evidencia podía fallar silenciosamente**:
   `oxidize-pdf-core/tests/t3_stress.rs` — el nuevo gate usaba `if let Ok` al
   crear el directorio y podía aprobar sin guardar el diagnóstico por PDF.
   Corregido: tanto crear el directorio como escribir el JSON son requisitos
   explícitos antes de aceptar el gate. Es un cambio del arnés de prueba,
   no del producto. La rama de error se razona a partir de `expect`; no se
   afirma una inyección de fallo de filesystem. La compilación/Clippy posterior revisa el arnés final; la ejecución amplia
   comprueba el camino exitoso y deja el JSON verificable. El cambio solo
   convierte en fallo la imposibilidad de crear ese directorio.
2. **Resultado estructural previo no determinista en qpdf_issue-99.pdf**:
   `oxidize-pdf-core/src/parser/reader.rs:1072` — la API Pages observada en la
   sonda alterna diccionario vacío y árbol con una página; no se ha aislado aún
   la causa interna exacta. La extracción alterna éxito vacío y error en ambas
   versiones (30 procesos por variante: base 17/13, candidato 14/16). Puede
   mover la cobertura un documento; no es mejora ni defecto introducido por
   #637. Crear/vincular una issue específica, aislar la recuperación estructural
   y añadir regresión determinista antes de corregirlo. Registrado bloqueado en
   TASKS, responsable mantenimiento/bzsanti. No se rebajan controles por ello.

No quedan defectos nuevos pendientes en el alcance implementado.
Los límites anteriores son parte explícita del contrato, no garantías de
validación universal del PDF. Las precisiones de rustdoc y la guía de migración
se completaron durante el trabajo; no se modificó comportamiento para elevar
puntuaciones de herramientas.

## Calidad de Tests (Kripteia)

Ejecutado el script de la skill quality-review sobre los 11 archivos Rust
modificados/nuevos. Resultados reales:

| Archivo | Tests detectados | Puntuación |
| --- | ---: | ---: |
| parser/filters.rs | 73 | 91 |
| parser/filters/recovery.rs | 0 | 100 (sin tests inline) |
| parser/document.rs | 47 | 98 |
| parser/mod.rs | 17 | 91 |
| text/extraction.rs | 92 | 89 |
| text/extraction/recovery.rs | 0 | 100 (sin tests inline) |
| text/mod.rs | 37 | 99 |
| issue_637_flate_errors_test.rs | 18 | 97 |
| issue_637_recovery_test.rs | 16 | 98 |
| t3_stress.rs | 4 | 81 |
| differential_order_test.rs | 7 | 93 |

Los dos módulos sin tests inline se ejercitan por las regresiones públicas;
100/0 no acredita cobertura. Los recuentos del scanner no incluyen módulos
importados y no coinciden con los tests ejecutados. Los avisos nuevos de
happy-path en checksums/raw no reconocen el helper que corrompe la entrada;
los oráculos de bytes constantes son independientes y adecuados. Los avisos
de aserciones múltiples/unwrap de fixtures no invalidan los fallos funcionales
observados. T3 conserva tests históricos de tendencia sin aserciones fuertes;
no se retocan para elevar la puntuación. La nueva prueba de cobertura sí
comprueba ratio, ausencia de panics y ausencia de timeouts.

## Análisis de Seguridad (Kripteia Security)

Los 11 análisis Security terminaron con `No security issues found.`. Se
inspeccionaron manualmente las fronteras de datos no confiables, límites,
clasificación de errores, omisiones y restauración del estado. No hay nuevo
unsafe, FFI, red, secretos ni escritura de PDFs de entrada. Los inputs del
corpus se leen; la recuperación no reescribe ni corrige su integridad original.

La ausencia de avisos automáticos no acredita seguridad universal. No se
amplía aquí el soporte de cifrado ni se auditan primitivas criptográficas.

## Métricas

- Archivos de alcance directo revisados: 13 (11 Rust, guía y changelog).
- Hallazgos confirmados: 2; uno corregido en el arnés de evidencia y uno previo
  bloqueado por issue separada.
- Archivos de producto modificados durante la revisión: 0 (sin contar rustdoc).
- Arneses de prueba corregidos durante la revisión: 1, persistencia de evidencia T3.
- Precisiones documentales: contrato del límite por contenido/Form y alcance
  de ResourceLimit para Flate; guía de recuperación y protocolo del corpus.
- Validación final: workspace 9890/0/72, features 52/0 y Clippy del arnés final
  aprobados. El único cambio posterior a compilar workspace es exigir que T3
  guarde evidencia; se verificó con Clippy, sin repetir el corpus completo.
  `workspace-source.json` y `source.json` identifican ambas versiones; el código
  de producto es idéntico. No se inyectó un fallo de filesystem.
