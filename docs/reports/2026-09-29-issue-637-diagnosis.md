# #637 — diagnóstico de cobertura y recuperación

Issue: [#637](https://github.com/bzsanti/oxidizePdf/issues/637), confirmada OPEN
el 2026-09-29. Diagnóstico solicitado por el mantenedor antes de retomar #639.
No cambios de producto, umbrales, baselines, PR ni publicación.

## Resultado

Los rechazos de Flate están justificados como validación de integridad, pero
el rechazo de toda la página/documento elimina también texto recuperable.
No hay evidencia que justifique deshacer la verificación de checksum o aceptar
recuperación silenciosa. Tampoco procede bajar automáticamente los gates.

Recomendación: conservar la API estricta y añadir una vía **explícita de
recuperación con resultado y diagnóstico estructurados**. El llamador debe
poder distinguir íntegro, recuperado sin integridad verificada e incompleto;
un stream agotado o vacío inválido sigue siendo un error, aunque otra parte
del documento pueda recuperarse. El mantenedor autorizó esta vía el 2026-09-29 tras confirmar que la corrupción
está en los PDFs de entrada, no introducida por oxidize-pdf. La implementación
continúa en un clon sobre develop actualizado; este informe conserva el
diagnóstico previo.

## Comparación controlada

- Base: `b76f7382186a704f74c1beb7c48037523ebee715`.
- Candidato: esa misma base con los tres archivos de producto del WIP #637.
  Sus SHA-256 coinciden con el informe del 28 de septiembre.
- Dos binarios separados, compilados offline desde una copia en `/tmp`,
  misma configuración default. Sonda por API pública; cada PDF se ejecuta en
  un proceso con timeout de 60 s. Cuatro procesos concurrentes por pasada.
- Se examinan los mismos 1.802 PDFs T3. Ambos parsean 1.761; ninguna sonda
  termina con error de proceso o timeout en la primera comparación.
- Primera pasada: base 1.613 extracciones, candidato 1.560, reproduciendo el
  resultado anterior. **54 documentos pasan de éxito a error** y uno cambia
  de error a éxito vacío por un comportamiento no determinista previo.
- Los documentos que extraen en ambas versiones conservan la longitud de
  texto. Esto no es una nueva comprobación de igualdad de bytes ni de orden.
- Seis de los 54 éxitos anteriores ya tenían texto vacío. Los otros 48
  tenían texto en alguna parte del documento.

Una segunda pasada completa da base 1.614 y candidato 1.559; el único archivo
que cambia entre repeticiones es `qpdf_issue-99.pdf`. Los 54 rechazos se
reproducen sin cambios en sus resultados.

La cifra neta de cobertura no debe ocultar el caso no determinista descrito
abajo. Los resultados completos por pasada y por documento están en
`2026-09-29-issue-637-diagnosis-evidence/`.

## Qué contenido se pierde

Se inspeccionan todas las páginas fallidas de esos 54 documentos:

- 129 páginas pasan de éxito a error. La base emitía 138.865 bytes UTF-8,
  10.437 palabras de al menos cuatro letras; 117 páginas tenían texto.
- Poppler termina con código 0 para las 129 páginas. Hay **8.430 ocurrencias
  de palabras compartidas** con la base, distribuidas en 93 páginas de 26
  documentos. La comparación usa min(multiplicidades), minúsculas y palabras
  alfabéticas de cuatro o más caracteres. No mide corrección visual ni orden.
- Las páginas afectadas de 45 documentos tenían texto en la base; en nueve
  documentos eran vacías, aunque otras páginas podían tener contenido.
- En `preserve_027613.pdf`, las once páginas rechazadas contenían 658 palabras
  comparables, todas presentes en Poppler. El gran deterioro de la métrica
  de orden a nivel de documento no equivale a miles de palabras físicamente
  transpuestas: perder ocurrencias tempranas cambia el emparejamiento k-ésimo.
- En 21 documentos, los Contents de la página se decodifican correctamente,
  pero un Form con `/FlateDecode` y longitud cero provoca el error. qpdf
  confirma un Form vacío en los 21. Por ejemplo, la página del fixture
  `pdfium_FRC_10_8.2.2__T8.3_original_file.pdf` invoca `/F Do` (Form vacío)
  antes de `/FFT0 Do` (Form válido con otros Forms). La base extrae 406 bytes;
  el candidato rechaza la página. Poppler da texto vacío en este fixture:
  no se presenta como corroboración independiente de esas palabras.
- `qpdf_overlay-no-resources.pdf` pierde `Potato` por un Form vacío previo.

## Integridad de los streams

Los 115 streams fallidos de Contents capturados en esta pasada se clasifican
con Python zlib, independiente del decoder Rust:

| Categoría | Streams |
| --- | ---: |
| Checksum incorrecto | 83 |
| Cabecera o DEFLATE inválido | 27 |
| Incompleto | 3 |
| Cero bytes con filtro Flate | 2 |
| Zlib completo válido | 0 |

Estos 115 no incluyen los Forms vacíos; no son un inventario de todos los
streams del corpus ni de los documentos que ya fallaban en la base.

También se reexaminan los 107 streams del diagnóstico diferencial anterior:

- 80 terminan al decodificar DEFLATE sin la cabecera zlib, pero ninguno tiene
  checksum correcto. Los bytes recuperados son exactamente los que devolvía
  la base, contrastados por SHA-256. No se pueden certificar como íntegros.
- 24 fallan incluso con DEFLATE sin cabecera y tres quedan incompletos.
- La base devolvía éxito con bytes en 105 y éxito vacío en dos.
- qpdf devuelve los mismos bytes en 53 y un prefijo más corto en los otros
  54 (recuperación de longitudes erróneas). **Ninguno de los 107 streams
  extraídos por qpdf es zlib completo válido**. Por tanto, las diferencias
  de delimitación observadas no explican estos rechazos como falsos positivos.

No se ensaya ni recomienda suprimir checksums en modo estricto. Alcanzar
el final de DEFLATE no garantiza integridad cuando el checksum discrepa.

## Incidencia previa: cobertura no determinista

`qpdf_issue-99.pdf` alterna entre `Ok([])` con cero páginas y error de Contents
con una página. Reproducción en 30 procesos independientes por versión:

- Base: 17 éxitos vacíos y 13 errores.
- Candidato: 14 éxitos vacíos y 16 errores.

qpdf tampoco reconstruye su Root. Las sondas del reader muestran que la
recuperación puede devolver un diccionario Pages vacío; introducir lecturas
adicionales cambia su resultado. No se ha aislado la causa interna exacta.
La existencia en ambas versiones descarta atribuir este caso a #637.
La diferencia inicial de +1 éxito no es una mejora del candidato.

Seguimiento bloqueado por falta de issue específica para esa recuperación
estructural no determinista. Responsable de crear/vincular: mantenimiento /
bzsanti. Desbloqueo: issue aplicable confirmada OPEN. No se modifica el
parser estructural ni se recalibran métricas bajo ese hallazgo.

## Opciones concretas para desbloquear #637

1. **Recuperación explícita (recomendada).** Las APIs que solo devuelven bytes
   conservan su contrato estricto. Una nueva API devuelve contenido y estado,
   con errores por objeto/página. La extracción tolerante puede conservar
   streams sanos y contenido recuperado, sin ocultar omisiones ni integridad
   fallida. Mantener límites absolutos, ratio y límites del llamador; errores
   de recursos nunca habilitan fallback. No usar heurísticas de predictores
   como reparación de compresión. Medir por separado extracción íntegra y
   recuperación en T3; comparar orden solo dentro del protocolo declarado.
2. **Rechazo total.** Mantener el candidato actual y aceptar explícitamente
   la pérdida de recuperación. Requiere aprobar un cambio de protocolo/gates
   respaldado por la clasificación anterior. No bastaría sustituir 90% por
   88%: hay que conservar controles de contenido, fallos y población evaluada,
   y resolver el ruido de la incidencia estructural por separado.

Para la primera opción, el cierre verificable exige RED/GREEN de corrupto,
truncado, checksum, vacío legítimo, límite/ratio y Forms mixtos; diagnóstico
observable en el consumidor; ningún éxito completo cuando hubo recuperación;
corpus bajo ambos contratos, QR completo y validación contra develop actual
(que ya incluye #638) antes del PR. Este diagnóstico no implementa esa API
ni da por aprobada una recalibración.

## Evidencia y reproducción

El directorio contiguo contiene resultados por PDF, páginas, streams, Forms,
verificación qpdf/zlib, repeticiones del caso inestable y fuentes de sondas.
`probe.rs` se compila como ejemplo `diagnose637` con
`cargo build --locked --offline -p oxidize-pdf --example diagnose637` en una
copia aislada. Guardar el binario de cada variante antes de recompilar;
actualizar los mtimes al copiar fuentes o usar targets separados. Los
scripts registran las rutas locales exactas usadas en esta sesión.

`coverage_runner.py BINARIO SALIDA.jsonl` reproduce la pasada de cobertura.
La sonda acepta `PDF DIRECTORIO failed` para capturar páginas fallidas y
`PDF DIRECTORIO 0,1,2` para contrastar esas páginas en la base.
Los bytes crudos/textos auxiliares permanecen en
`/tmp/issue-637-details-20260929`; sus hashes y resultados son durables.

No se ha repetido la suite completa de producto ni los gates de orden:
el código del repositorio no cambió. Los números de orden del informe del
28 siguen siendo evidencia anterior, no una nueva ejecución de hoy.
