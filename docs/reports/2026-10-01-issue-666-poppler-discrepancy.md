# #666 — discrepancia ToUnicode con Poppler resuelta

Issue: #666 — test(text): build normative encoding and glyph-spacing contract batteries — https://github.com/bzsanti/oxidizePdf/issues/666

Conclusión: los avisos y la pérdida de texto observados se explican por dos
límites numéricos de Poppler 24.02.0. No requieren corregir los fixtures CID ni
cambiar su expectativa. La extracción queda contrastada con MuPDF 1.26.10,
accedido mediante PyMuPDF 1.26.5. No se amplió la matriz ni se modificó producción.

## Causa concreta

El código oficial de la versión instalada configura la lectura de ToUnicode de
fuentes CID con 16 bits en [GfxFont.cc, línea 1786](https://gitlab.freedesktop.org/poppler/poppler/-/blob/poppler-24.02.0/poppler/GfxFont.cc#L1786).
El parser calcula por tanto un máximo 0xFFFF y emite el aviso al excederlo,
pero continúa intentando almacenar el mapeo. Independientemente, el almacenamiento
descarta códigos mayores que 0xFFFFFF. Ambas condiciones están en
[CharCodeToUnicode.cc, líneas 298, 334 y 472](https://gitlab.freedesktop.org/poppler/poppler/-/blob/poppler-24.02.0/poppler/CharCodeToUnicode.cc#L298).

En el fixture original, C00001 excede el primer límite pero no el segundo:
avisa y conserva é. E0000001 excede ambos: avisa y descarta la secuencia fi.
Resultado de Poppler: ABé en lugar de ABéfi. El mapa de glifos sí funciona, lo
que explica que los rasters anteriores coincidieran a pesar del texto perdido.
No es un problema de Unicode fuera del BMP: el destino perdido aquí es `fi`.

## Contraste normativo

PDF 32000-1:2008 §9.10.3 exige codespacerange consistente con Encoding y destinos
UTF-16BE; el límite de un byte citado allí corresponde a fuentes simples. Estos
fixtures son Type0 con CMaps propios. La especificación de CMaps admite códigos
multibyte y describe rangos de tres y cuatro bytes; no justifica los límites
numéricos anteriores. Referencias: [PDF 32000-1](https://developer.adobe.com/document-services/docs/assets/35e4369068f86065372c18787171a17e/PDF_ISO_32000-1.pdf)
y [Adobe Technical Note 5014](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5014.CIDFont_Spec.pdf),
§7 y apéndice A (página impresa 90 para rangos multibyte).

Los códigos fuente se mantienen dentro de sus codespaces, no solapados; las
colecciones del Encoding y descendiente coinciden; CID/GID/ToUnicode conservan
las asociaciones declaradas. No se encontró un error del fixture que explique
los avisos. Esto resuelve la discrepancia concreta, no certifica todos los
contratos PDF de la matriz.

## Experimento mínimo

Nueve PDFs derivados del mismo fixture, cada uno con un solo código, CID 17,
GID 2 y destino ToUnicode `fi`. Se actualizan exclusivamente Encoding,
ToUnicode y contenido de página; el lector independiente reescribe xref/longitudes.
Todos pasan qpdf --check. Las salidas completas, incluidos saltos de línea y
form-feed añadidos por las herramientas CLI, se conservan en JSON.

| Código fuente | Bytes | Poppler 24.02.0 | MuPDF 1.26.10 | oxidize base |
| --- | ---: | --- | --- | --- |
| 41 | 1 | fi, sin aviso | fi | fi |
| 0041 | 2 | fi, sin aviso | fi | fi |
| 000041 | 3 | fi, sin aviso | fi | fi |
| 00000041 | 4 | fi, sin aviso | fi | fi |
| FFFF | 2 | fi, sin aviso | fi | fi |
| 010000 | 3 | fi, con aviso | fi | fi |
| FFFFFF | 3 | fi, con aviso | fi | fi |
| 01000000 | 4 | sin texto, con aviso | fi | fi |
| E0000001 | 4 | sin texto, con aviso | fi | fi |

Esto separa longitud de valor numérico y prueba ambos umbrales. Poppler devuelve
exit 0 incluso cuando pierde texto: su código de salida no basta para validar.
El probe Rust exige igualdad exacta con `fi` en los nueve PDFs y termina exit 0,
sobre base 48d8b8f8bbf08b2976fd739e6c2462c9552a3cc6 sin cambios de producto.

Además, MuPDF lee los cinco PDFs originales del incremento 5 sin avisos:
cidchar/cidrange/Identity-H/GIDs intercambiados → ABéfi; MacExpert → ,.ﬁ.
Esto confirma externamente tanto los códigos largos como U+FB01 sin la
normalización de ligaduras observada en pdftotext.

## Decisión y evidencia

Conservar los tests y expectativas. Para estos casos, Poppler se mantiene como
contraste raster y como lector con un límite conocido; MuPDF aporta el contraste
independiente de extracción. No generalizar la limitación a versiones distintas
sin comprobarlas. No hay cambio de producto oxidize-pdf que realizar por esta
discrepancia ni se crea una issue correctiva de producto por ella.

Evidencia: [directorio](2026-10-01-issue-666-poppler-evidence/), con versiones,
resultados originales, nueve PDFs mínimos, generador, probe público Rust y
hashes/URLs de las fuentes oficiales inspeccionadas. PyMuPDF se instaló solo en
/tmp para esta investigación; no se añade dependencia al producto ni a cargo test.
La suite permanente sigue en 58 tests, con sus RED previos. TASKS, plan y
manifiesto quedan actualizados; el trabajo general continúa pendiente.
