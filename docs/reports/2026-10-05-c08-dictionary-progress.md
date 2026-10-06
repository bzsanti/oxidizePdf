# #666 — C08, avance de diccionario y herencia

C08 sigue activa; no es un cierre de fase. Se resuelve la lectura y validación de CIDSystemInfo del diccionario del stream. El programa PostScript aún no aporta Registry/Ordering/Supplement: es el siguiente trabajo obligatorio, antes del QR final, agregado/corpus y U06.

## Contratos y evidencia

Reproducción pública: Encoding stream declarado Adobe/Japan1 con fuente Adobe/GB1 producía啊 para CID940. RED3/1 en red-dictionary.log. Ahora se exige que los metadatos presentes del stream tengan Registry/Ordering string y Supplement entero no negativo, resolviendo referencias indirectas; las colecciones conocidas incompatibles y los valores malformados no autorizan fallback Unicode. Se mantiene CID940 y avance600; ToUnicode explícitoZ siempre gana. La metadata ausente conserva la recuperación histórica.

EncodingCMap distingue invalid_collection de collection ausente. La herencia propaga invalidez y detecta conflicto de colección entre hijo/padre; no se pierde el estado inválido al heredar una colección conocida. Named parent GB1 con suplemento99 sigue válido: suplementos acumulativos no requieren igualdad. Un padre Japan1 y un hijo GB1 son incompatibles aunque una entrada del hijo remapee el CID. No se alteran códigos, glifos o métricas.

Tres tests nuevos: metadata directa/indirecta (incluido null, tipos/signos inválidos y campos ausentes), herencia con padre nombrado compatible/incompatible, y CID0 incompatible que debe conservar U+FFFD. Este último pasa desde su primera ejecución; cid0-control.log no es RED ni demuestra un defecto nuevo.

El consumidor de recuperación tenía un fixture incoherente: declaraba colección privada y esperaba UnicodeJapan1. Se cambió únicamente su Registry/Ordering en diccionario/programa a Adobe/Japan1 para mantener el control positivo de truncación y sus expectativasA�/�/A. Se conserva consumers.log con el fallo anterior; no se relaja el contrato ni se recalibra un oráculo externo.

## Revisión parcial

Inspección manual de parse_encoding_stream, declare_collection/inherit y compatible_cid_ordering por los dos consumidores. Sin nuevo flujo de ejecución de programas; sólo lectura de objetos resueltos. No se cambia la profundidad16 ni el límite8MiB. Todavía pendiente el parsing estático de metadatos dentro del programa, evitando procedimientos no ejecutados, strings/comentarios engañosos y conflictos con el diccionario.

## Calidad de Tests (Kripteia)

Encoding93/12, extraction91/10, contrato90/6. Wrappers revisados: check_embedded invoca PdfReader, TextExtractor y ResolvedFontResource con expectativas literales y ambos ParseOptions. No se interpreta el score como cierre del contrato completo.

## Análisis de Seguridad (Kripteia Security)

Sin alertas en tres alcances. No hay ejecución PostScript, nuevas dependencias ni asignaciones según Supplement; referencias indirectas pasan por document.resolve. Validación proporcional al incremento:69/69 en seis targets y Clippy -D warnings. QR de fase y agregado/corpus aún pendientes hasta terminar parsing de programa.
