# #666 — MuPDF incorporado al flujo de validación

Issue: #666 — test(text): build normative encoding and glyph-spacing contract batteries — https://github.com/bzsanti/oxidizePdf/issues/666

Implementado un validador reproducible en tools/text_contracts/validate_readers.py,
manifest de casos y requisitos fijados. Sin dependencia nueva de producto ni
cambio de expectativas o correcciones Rust. #666 sigue abierta.

## Contrato

MuPDF contrasta texto exacto; Poppler aporta una segunda lectura con excepciones
acotadas y qpdf verifica estructura. Las expectativas proceden de ToUnicode
explícito o del par normativo MacExpert/AGL, no de los lectores. Se fijan
PyMuPDF 1.26.5 y MuPDF 1.26.10; se registran versiones Poppler/qpdf.

Cada PDF tiene SHA256 y fuente de expectativa. El manifest inicial reutiliza
14 PDFs durables de la investigación: nueve umbrales, cuatro mapas CID y uno
MacExpert. No duplica sus programas de fuente ni depende de archivos /tmp.
El contrato actual es una página/una línea: MuPDF añade LF y pdftotext LF+FF.
Se comparan salidas completas, sin trim ni normalización Unicode.

Los límites conocidos de Poppler exigen coincidencia exacta de versión, salida,
aviso y exit code por caso. No basta reconocer el mensaje de aviso. Nunca
permiten omitir o aprobar una discrepancia de MuPDF. Un lector ausente, versión
MuPDF no revisada, fixture cambiado o fallo inesperado da exit 1 y un informe.
No hay recalibración automática ni escritura de expectativas desde resultados.

## Validación TDD

Primero, las pruebas del validador fallaron porque faltaba su implementación.
Después, ocho pruebas pasan:

- Control real: pérdida conocida Poppler y salida exacta MuPDF.
- Expectativa fi→fl: falla aunque Poppler coincida con su limitación conocida.
- Hash de fixture alterado: falla antes de invocar lectores para ese caso.
- Mismo aviso con texto inesperado: no queda cubierto por la excepción.
- Diferente versión Poppler: no hereda la excepción histórica.
- Fixture ausente: falla sin resultados de lectores para ese caso.
- Herramienta requerida ausente: error, sin skip silencioso.
- MuPDF de versión no revisada: error.

Ejecución completa con lectores reales: exit 0, 14 comparaciones MuPDF exactas,
6 comparaciones Poppler exactas, 8 límites Poppler explícitos, 14 comprobaciones
qpdf correctas. Cero discrepancias inesperadas. Esto no convierte las ocho
limitaciones en acuerdos entre lectores ni valida toda la matriz del producto.

La CLI también se probó con expectativa mutada: exit 1; informe conserva
mismatch de MuPDF y known_limitation de Poppler. Ejecutada sin la instalación
MuPDF de /tmp: exit 1 e informe de dependencia ausente. No requiere ocultar
warnings SWIG de Python; los avisos internos de MuPDF se registran por separado.

## Reproducción

```sh
PYTHONPATH=/tmp/issue666-pymupdf python3 tools/text_contracts/validate_readers.py \
  --output /tmp/issue666-reader-gate-report.json
PYTHONPATH=/tmp/issue666-pymupdf python3 -m unittest discover \
  -s tools/text_contracts/tests -v
```

La instalación limpia y los comandos con entorno independiente se documentan
en tools/text_contracts/README.md. La herramienta no instala ni descarga nada.
Resultados y hashes en [evidencia](2026-10-01-issue-666-reader-validation-evidence/).

## Alcance pendiente

La suite Rust permanece en 58 tests con los RED ya documentados. Estas ocho
pruebas Python validan el nuevo flujo, no se suman como contratos PDF de producto.
No se ha ejecutado de nuevo Rust porque no cambió, ni se ha integrado este flujo
en CI mientras la matriz general sigue en construcción. Tampoco se declara
completo el corpus, la comparación geométrica o la revisión de calidad final.
Siguiente paso: usar este validador al añadir CMaps CJK, CIDFontType0 y vertical,
ampliando explícitamente el schema cuando cambie el contrato de página/layout.
