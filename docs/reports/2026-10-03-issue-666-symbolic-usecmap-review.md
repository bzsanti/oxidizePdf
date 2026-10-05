# #666 — TrueType simbólica y herencia UseCMap

Issue: https://github.com/bzsanti/oxidizePdf/issues/666 (OPEN). Incremento de baterías; la issue no está terminada. Producto sin cambios, idéntico a develop d33e7f94; checkout a9c47dee. Evidencia en `2026-10-03-issue-666-evidence/`.

## Resultado

18 targets, 167 pruebas: **143 PASS / 24 FAIL / 0 omitidas**, exit 101 esperado de caracterización. Comando y resultados exactos: `expanded-contracts-command.json`, `expanded-contracts-summary.json`, `expanded-contracts.log`. Los 18 fallos anteriores se conservan; se reproducen seis nuevos de UseCMap.

- F07: ocho contratos nuevos GREEN. Ocho programas glyf originales, completos/subconjuntos, con cmap Windows Symbol (3,0,4), Macintosh (1,0,0), Unicode BMP (3,1,4) y UCS4 (3,10,12). Verifican GID, métricas, códigos privados, mapeo no BMP, WinAnsi no simbólica, ToUnicode multiescalar y avance por glifo fuente. Doce PDFs: 24 textos coincidentes MuPDF/Poppler, 12 estructuras qpdf válidas. No se infiere Unicode de códigos simbólicos sin ToUnicode.
- C07: diez contratos nuevos, cuatro PASS y seis FAIL. Nueve casos de extracción/geometría y uno de integridad de once PDFs con CID-keyed CFF fijada por hash. Los casos prueban herencia de Encoding y ToUnicode, dos niveles, sustitución en el hijo, Identity-H por operador/diccionario y precedencia explícita sobre notdef. Los contratos GREEN recorren strict/lenient; los RED se detienen en la primera aserción strict, por lo que esta ejecución no certifica el resultado lenient de esos casos.
- Dos PDFs adicionales notdef-char/notdef-range son exploratorios: solo están incluidos en integridad y comparación externa, sin aserción Rust normativa de extracción hasta resolver la discrepancia.

## Hallazgos de producto

1. P1 — `oxidize-pdf-core/src/text/extraction_cmap.rs:218`: el stream Encoding se analiza sin resolver su entrada UseCMap. `encoding_cmap.rs:76` conserva el nombre del operador, pero no hereda el mapa. `encoding-parent`, `named-operator` y `named-dictionary` sitúan el segundo glifo en x=105 en lugar de x=104, pese a W[17]=400 a 10 pt.
2. P1 — `oxidize-pdf-core/src/text/extraction_cmap.rs:343`: `parse_tounicode_stream` ignora el documento y analiza solo los bytes del hijo. `unicode-parent` y `both-chain` pierden todo el texto; `unicode-shadow` pierde el carácter heredado. Control plano y sustitución CID local pasan.

Corrección bloqueada hasta crear/vincular issue específica OPEN; responsable bzsanti/Codex. #666 autoriza las baterías; no se modifica producto bajo una entrada sin issue aplicable. No atribuir estos defectos a #662/#663.

## Referencias y límites de lectores

Semántica de herencia: ISO 32000-1, 9.7.5, tabla 120 (UseCMap), y [Adobe Technical Note 5014](https://pdfa.org/norm-refs/5014.CIDFont_Spec.pdf). Expectativas de texto declaradas explícitamente en ToUnicode; posiciones calculadas de W y tamaño 10 pt, sin consultar tablas del producto.

`usecmap-readers.json` permanece **rejected**: 15 coincidencias, siete discrepancias, 11 estructuras qpdf válidas, cero excepciones añadidas. MuPDF confirma texto y geometría de los nueve casos afirmados por Rust. Poppler discrepa en tres herencias ToUnicode (texto vacío/parcial, advertencia de colección desconocida en dos), y añade espacio en dos casos Identity-H. MuPDF omite el glifo y avance en los dos probes notdef, con advertencia; Poppler coincide con su expectativa exploratoria. No se ajusta el validador para ocultar estos resultados.

El primer probe encoding-shadow compartía un CID entre dos códigos con distinto Unicode, introduciendo una discrepancia adicional de MuPDF. Antes de congelar contratos se separó ese problema: el hijo intercambia ambos CIDs, conserva ToUnicode inequívoco y verifica x=107. La salida inicial se conserva en `usecmap-readers-initial.json`; no constituye validación de los bytes finales.

## Revisión y controles

- Regeneración independiente en target: 22 archivos symbolic y 13 usecmap idénticos byte a byte (`symbolic-usecmap-reproducibility.json`). Enumeración explícita de archivos generados, sin incorporar archivos ajenos a provenance.
- Tres mutaciones de entrada detectadas por las aserciones previstas y tres controles GREEN: fi→fj, ancho simbólico 700→900, ancho CID17 400→900. Consumidor bajo target, sin tocar fuentes/producto/fixtures originales. Script y resultado: `symbolic-usecmap-mutations.py/.json`. Esto verifica discriminación de entrada, no cobertura de mutaciones del producto.
- Clippy focalizado y formato pasan; 13 unittest Python pasan; inventario de 46 filas válido.
- Kripteia: simbólica 96/8 pruebas; UseCMap 91/10. Alertas «no production code» son falsos positivos por helpers: verify_platform llama TrueTypeFont, check llama PdfReader/TextExtractor. Literales son oráculos deliberados. Security sin alertas.
- Kripteia Python no reconoce pruebas en los generadores/helper (0 tests/0 files); no equivale a cobertura. Inspección manual: sin descargas, ejecución de comandos ni parsing de entradas externas en generadores; salidas deterministas elegidas por el operador, versión FontTools fijada, font CFF con hash verificado. El script de mutación ejecuta cargo solo como herramienta local de validación.

Pendientes: selección entre cmap competidores y política sin ToUnicode; resolver notdef, ciclos y códigos truncados; issues de corrección, resto de matriz, integración/CI. Ni F07/C07 ni #666 se marcan completos.
