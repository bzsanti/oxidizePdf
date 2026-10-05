# Plan general y TDD: codificaciones, fuentes y espaciado PDF

Issue: #666 — test(text): build normative encoding and glyph-spacing contract batteries — https://github.com/bzsanti/oxidizePdf/issues/666

Responsable: Codex / mantenimiento. Prioridad P1. Estado: implementación, QR y CI
completados; integración pendiente en PR #681. Autorización del usuario: 2026-10-01. Base de
caracterización: develop `48d8b8f8bbf08b2976fd739e6c2462c9552a3cc6`.
PR #664: `70f28714e2ef7115c0f0898d47a3af6cae3de953`;
PR #665: `c7031f7645d947de03264485c8e38c4452feebe3`.

## Estado vigente — 2026-10-05

Las 46/46 filas de fuentes, codificaciones, CMaps, repertorios, cadenas y
espaciado/salidas están validadas, en secuencia y con QR resuelto. T4 y T5
completadas; no quedan fases de implementación ni hallazgos pendientes.

Código validado: `a2562e2aea63417276c68897c949f7b4f1a4b3a1`. CI: 29 checks
aprobados y dos jobs programados omitidos en PR; matriz nativa 12/12 aprobada
(Linux/Windows/macOS, default/minimal/SPI, MSRV 1.88). Suite local completa:
10.367 PASS/0 FAIL/71 ignorados preexistentes; contratos rápidos sin ignorados.
Corpus manual 37382367497 aprobado, incluido el job nocturno T2/T3 y contenido.
Evidencia y aceptación: `../reports/2026-10-05-t5-review.md` y su directorio.

#666 sigue abierta para integración del [PR #681](https://github.com/bzsanti/oxidizePdf/pull/681).
Merge y release están excluidos de esta tarea. TASKS.md es el seguimiento diario.
Las entradas inferiores son históricas; no sustituyen este estado final.
La instrucción vigente permite corregir regresiones sobre la marcha bajo #666.

## Base vigente al cierre — 2026-10-01

Continuar desde develop `d33e7f94b6053e26f29b0523a52b87b08013babb`.
Los PRs #664/#665 de Omer fueron integrados conservando autoría e historia;
#669 (#667 documental) y #670 (#668 codificaciones simples) también están
fusionados, con 17 checks aprobados y dos jobs programados omitidos por PR.
#667/#668 están cerradas; #666 permanece abierta. El relevo vigente y los límites
de la validación están al principio de TASKS.md.

No reintroducir el candidato alternativo ni reutilizar clones/rutas históricas.
Las entradas de incrementos siguientes documentan historia, no estados actuales.
Recaracterizar las baterías locales pendientes sobre la base integrada antes de
atribuir fallos actuales a TJ/Tz, vertical o CJK. Cada corrección exige una issue
abierta aplicable; no cambiar oráculos ni umbrales. No usar /tmp ni worktrees.

## Objetivo y separación de contratos

Actualización 2026-10-03: caracterización actual de 16 targets y 149 contratos:
131 PASS/18 FAIL/0 omitidos. El incremento Type1 añade diez contratos (8/2) y
16 PDFs con programas PFB/Type1C completos/subconjuntos; codificación intrínseca
y Differences sin BaseEncoding quedan RED con contraste externo completo.
Informe: `../reports/2026-10-03-issue-666-type1-review.md`.
Las 46 filas de coverage.json declaran oráculos, estrategia, cierre y pendientes;
`tools/text_contracts/validate_coverage.py` y ocho tests adversariales comprueban
su trazabilidad, sin certificar soporte. Las cifras anteriores son históricas.
Siguiente bloque de baterías: TrueType simbólica/cmap alternativos y CMap usecmap;
corrección intrínseca/vertical/TJ-Tz requiere crear/vincular issues específicas.
#662/#663 las cerrará oshtivi; no corresponden a Codex.

Baterías permanentes definidas antes de mejorar los PRs. Verificar separadamente:
código de fuente → glifo/CID/GID; código → Unicode; desplazamientos geométricos;
síntesis de separadores; orden de lectura; cadenas del documento. Decodificar
Unicode no demuestra shaping ni orden bidireccional. Un round trip no es oráculo.
No interpretar códigos Identity como Unicode sin evidencia. Una anchura ausente
no equivale a cero. La recuperación debe quedar diferenciada de la interpretación
normativa y su incertidumbre debe tener política explícita.

## Matriz completa de fuentes

| ID | Familia / fuentes exactas | Variantes que deben cubrirse |
|---|---|---|
| F01 | Helvetica, Helvetica-Bold, Helvetica-Oblique, Helvetica-BoldOblique | Standard14 sin incrustar; métricas implícitas y explícitas |
| F02 | Times-Roman, Times-Bold, Times-Italic, Times-BoldItalic | Igual; anchuras proporcionales y cambios de estilo |
| F03 | Courier, Courier-Bold, Courier-Oblique, Courier-BoldOblique | Igual; monoespaciado |
| F04 | Symbol | Codificación incorporada, símbolos y griego; sin asumir Latin-1 |
| F05 | ZapfDingbats | Codificación incorporada y nombres aNNN |
| F06 | Type1 incrustada / Type1C CFF simple | Completa/subconjunto; codificación propia; nombres reasignados |
| F07 | TrueType simple | Completa/subconjunto; simbólica/no simbólica; cmap de fuente |
| F08 | Type3 | CharProcs reales, FontMatrix no trivial y Widths controladas |
| F09 | Type0 + CIDFontType0 | CFF CID, colecciones y escritura H/V |
| F10 | Type0 + CIDFontType2 | CIDToGIDMap identidad/stream, completa/subconjunto, H/V |

Las Standard14 se enumeran individualmente en el manifiesto ejecutable. Para
fuentes incrustadas fijar archivos, versión, licencia y checksum; usar candidatos
redistribuibles como Liberation y Noto solo tras verificar esos datos. El nombre
comercial no determina el subtipo PDF: verificar el diccionario y programa real.
No afirmar que un fixture con BaseFont inventada y sin programa valida incrustación.

## Matriz completa de codificaciones y selección

| ID | Ruta | Cobertura y oráculo |
|---|---|---|
| E01 | StandardEncoding | 256 posiciones; asignadas/no definidas; Annex D |
| E02 | WinAnsiEncoding | 256 posiciones; bullet en huecos, NBSP y soft hyphen |
| E03 | MacRomanEncoding PDF | 256 posiciones; currency 0xDB; separar Mac OS Roman |
| E04 | MacExpertEncoding | 256 posiciones; nombres expert y política Unicode/PUA |
| E05 | Symbol incorporada | 256 posiciones; nombres y Unicode de símbolos |
| E06 | ZapfDingbats incorporada | 256 posiciones; tabla Adobe de nombres aNNN |
| E07 | Codificación propia sin Encoding | Casos legalmente válidos por subtipo; programa incrustado |
| E08 | BaseEncoding + Differences | Contadores/reasignaciones, nombres compuestos, límites y precedencia |
| E09 | ToUnicode | bfchar/bfrange, destinos múltiples, UTF-16BE/surrogates, parcial y precedencia |

No cruzar combinaciones inválidas como si fueran contratos normativos. Seis
tablas de 256 posiciones son 1536 entradas de inventario, no 1536 pruebas de
producto aprobadas. Mantener nombres originales y secuencias Unicode. Para
posiciones no definidas, la tabla marca ausencia; definir por ruta si se requiere
error, reemplazo o diagnóstico antes de exigir una salida inventada. No imponer
por accidente la tolerancia actual. Los casos pendientes no cuentan como PASS.

## Matriz CMap / CID

| ID | Colección | CMaps / variaciones |
|---|---|---|
| C01 | Identidad | Identity-H, Identity-V; CIDs no Unicode; con/sin ToUnicode |
| C02 | Adobe-GB1 | GB-EUC-H/V, GBK-EUC-H/V, GBKp-EUC-H, UniGB-UCS2-H/V, UniGB-UTF16-H/V |
| C03 | Adobe-CNS1 | B5pc-H/V, ETen-B5-H/V, UniCNS-UCS2-H/V, UniCNS-UTF16-H/V |
| C04 | Adobe-Japan1 | 90ms-RKSJ-H/V, 90pv-RKSJ-H/V, UniJIS-UCS2-H/V, UniJIS-UTF16-H/V |
| C05 | Adobe-Korea1 | KSC-EUC-H/V, KSCms-UHC-H/V, UniKS-UCS2-H/V, UniKS-UTF16-H/V |
| C06 | Adobe-KR | Colección distinta; resolver CMaps de una revisión oficial fijada; no alias Korea1 |
| C07 | CMap incrustado | 1–4 bytes, codespacerange, cidchar/cidrange, usecmap, WMode, notdef |
| C08 | Colección propia | Registry/Ordering/Supplement, con/sin ToUnicode y sin Unicode recuperable |

Validar compatibilidad Registry/Ordering y límites del Supplement. Identity-H/V
no garantiza contenido Unicode. UCS-2 y UTF-16 necesitan casos distintos fuera
del BMP. Referencia externa: adobe-type-tools/cmap-resources, revisión fijada
antes de construir cada fixture; no generar expectativas desde tablas vendidas
por oxidize-pdf. Contrastar nombres desconocidos, mapas cíclicos/incompletos y
colas truncadas; establecer política explícita por modo.

## Repertorios y cadenas del documento

| ID | Repertorio | Ejemplos / propiedad |
|---|---|---|
| U01 | Latino occidental | Español, portugués, francés, alemán; ordinales, monedas y puntuación |
| U02 | Latino ampliado | Polaco, checo, turco, vietnamita |
| U03 | Griego / cirílico | Acentos y mezcla de scripts y números |
| U04 | Árabe / hebreo | Marcas y cifras mixtas; decodificación separada de bidi/orden |
| U05 | Devanagari / tailandés | Secuencias de marcas; sin atribuir shaping a la decodificación |
| U06 | CJK | Las cuatro familias; horizontal y vertical |
| U07 | Unicode adicional | No-BMP, combinantes, ligaduras multicaracter; sin normalizar implícitamente |
| D01 | PDFDocEncoding | 256 posiciones con tabla D.2; metadatos, marcadores, formularios |
| D02 | UTF-16BE con BOM | Vacío, no-BMP, truncado y secuencias inválidas según contrato |
| D03 | UTF-8 PDF 2.0 | BOM/versión y tipos de cadena aplicables; no aplicar a Tj/TJ por defecto |

## Matriz de espaciado y geometría

| ID | Dimensión | Casos positivos y negativos emparejados |
|---|---|---|
| S01 | Métricas | Cero explícito / no cero / ausente / inválido; Widths y W/DW |
| S02 | TJ | Uniforme/no uniforme; avance menor/igual/mayor al umbral; signo; outlier |
| S03 | Evidencia de separación | Espacio literal, espacio ToUnicode, solo ajuste, palabra sin espacios |
| S04 | Elegibilidad | Un solo ajuste frente a dos/mayoría; kern inicial/final; uno/varios glifos por string |
| S05 | Estado | Tfs, Tz, Tc, Tw; defaults y cambios durante texto |
| S06 | Transformaciones | Tm, CTM, rotación, escala, cizallamiento y composición; unidades verificadas |
| S07 | Fronteras | Tj/TJ consecutivos, cambio de fuente, BT/ET, q/Q, Forms anidados |
| S08 | Política | Strict/lenient, descendiente resoluble/dañado, controles simples y verticales |
| S09 | Salidas | Texto plano, fragments, preserve_layout y consumidores que prometan el mismo contrato |

Cada caso declara texto literal esperado y geometría calculada externamente
cuando la prueba haga una afirmación de posición. Evitar assertions de cantidad
como sustituto del contenido. Primer bloque: doce fuentes latinas Standard14,
Width ausente/cero/no cero, ajustes 0.3/0.6/1 em, espacio/no espacio, strict/lenient,
Tc/Tw y outliers; control separado de geometría con anchuras declaradas.
Symbol y ZapfDingbats conservan sus códigos propios, no reutilizan letras latinas
como oráculo. Vertical y política de recuperación no se dan por implementados
por incluirlos en el inventario.

## Oráculos, fixtures y estados

- Norma: PDF 32000-1:2008 capítulo 9 y Annex D; para D03, PDF 2.0 y errata.
  Copia oficial localizada en adobe/dc-acrobat-sdk-docs/docs/pdfstandards;
  conservar SHA-256 del PDF. No redistribuir el documento normativo completo.
- Nombres → Unicode: Adobe AGL/aglfn revisión
  `4036a9ca80a62f64f9de4f7321a9a045ad0ecfd6`, incluidas tablas ZapfDingbats.
- Referencia independiente adicional: PDFBox 3.0.5 commit
  `804cc824f1a19bcce85a3d0d60a8f10e98188e52`; conservar atribución/licencia.
  Verificar tablas latinas contra Annex D, incluidas sus notas; registrar
  explícitamente la política Unicode de espacios duplicados y soft hyphen.
- PDFs sintéticos: longitudes/xref correctos, hex strings para probar todos los
  bytes sin escapado accidental. Cada anomalía deliberada debe estar aislada.
  qpdf valida sintaxis/estructura, no sustituye el oráculo semántico de fuente.
- Estado por fila: planned / oracle-ready / executable / validated / failing /
  unsupported / policy-pending. Registrar revisión, configuración, caso y comando.
  Un manifiesto válido no acredita el soporte de producto de sus filas.

## Secuencia TDD y entregables

### T0 — Infraestructura y referencias

RED: hacer fallar el verificador al borrar una fila, duplicar código o alterar el
hash de una fuente. GREEN: generador offline, TSV inmutables de 256 filas por
tabla, procedencia/licencias y matriz trazable. REFACTOR: un lector de fixtures
de test y un ensamblador compartido, sin dependencia de conversores de producto.
La prueba de integridad solo valida infraestructura, no el decodificador.

### T1 — Primer bloque ejecutable, antes de corregir producto

RED: recorrer por API pública los códigos definidos Standard/WinAnsi/MacRoman;
agregar diagnósticos encoding/code/name/expected/actual sin detenerse en el primer
byte. Probar ToUnicode sobre Differences sobre BaseEncoding, secuencias múltiples
y no-BMP. Añadir matriz Standard14/anchuras/TJ y geometría independiente.
Ejecutar sin cambios de producto contra base y cada PR por separado.

GREEN del harness: los controles y el lector de oráculos funcionan; los fallos
del producto permanecen rojos. No ignorarlos ni cambiar expectativas para poner
verde la suite. El RED inicial no es un artefacto listo para integrar en main.
Mantenerlo en el clon de trabajo con parche durable y comandos reproducibles.

REFACTOR: solo utilidades de test, conservando casos y resultados. Publicar
informe local del primer bloque y siguiente acción exacta; #666 permanece abierta.

### T2 — Cobertura restante de fuentes simples y cadenas

RED: Symbol/Zapf incorporadas y MacExpert con fuente apropiada; incrustación
Type1/TrueType/CFF, Type3 válido; PDFDocEncoding y UTF-16BE. No usar un fixture
inválido para declarar un fallo de decodificación. GREEN: completar soporte bajo
issues de defecto relacionadas, después de congelar baterías/oráculos. REFACTOR:
centralizar tablas normativas compartidas sin mezclar Mac OS/PDF.

### T3 — CMaps, scripts y geometría ampliada

RED por cada C/U/S pendiente; pares con solo una variable cambiada. Probar límites
de código, mapas con usecmap, CID≠GID≠Unicode, vertical y transformación compuesta.
GREEN bajo issues apropiadas conservando restricciones no-C; REFACTOR sin perder
precedencias, límites ni identidad de fixtures. D03 separado por versión.

### T4 — Discriminación y corpus

Mutaciones mínimas sobre copias: eliminar guard de métricas/espacio/modo, forzar
MacRoman 0xDB=Euro, saltarse Differences/ToUnicode, omitir escala o invertir signo.
Cada mutación debe fallar por la aserción esperada, no compilación o setup.
Los tests negativos de tracking deben alcanzar el guard (>=3 glifos y 2 ajustes).

Ejecutar corpus diferencial existente más métricas separadas de sustitución y
contenido perdido con población y políticas fijadas. Conservar diagnóstico por
PDF/página/caso, no solo promedios; cualquier métrica nueva tiene identidad propia.
No recalibrar umbrales históricos ni normalizar diferencias fuera del contrato.

### T5 — Integración y mantenimiento

Suite determinista rápida en CI normal, corpus pesado con ejecución explícita y
programada. Los tests nuevos no llevarán ignore ni dependencias de fuentes del
host. Verificar default, configuración mínima soportada y SPI si afecta la ruta;
Linux/Windows/macOS y MSRV. QR de calidad/seguridad y mutaciones antes de integrar.
La issue cierra solo con matriz completada o exclusiones acordadas explícitamente,
resultados reproducibles y código integrado. Plan/inventario o primer bloque no
son criterios suficientes de cierre.

## Seguimiento y límites de esta activación

Primera entrega autorizada: plan completo y comienzo real de T0/T1, con RED de
base/#664/#665 y evidencia. No se fusionan PRs ni se inicia release. Los arreglos
de MacRoman se vinculan a #662; los de recuperación a #663. Otros defectos que
aparezcan se registran bloqueados para corrección hasta crear/vincular issue
aplicable; propietario bzsanti/Codex, desbloqueo issue OPEN confirmada.

Las tablas pueden revelar defectos heredados además del caso Euro. No ampliar
silenciosamente una issue estrecha ni declarar las 14 fuentes validadas por
probar únicamente Helvetica. Conservar todos los cambios locales originales.


## Segundo incremento — 2026-10-01

Implementadas nueve pruebas adicionales: tablas completas de Symbol/Zapf con
codificación incorporada, precedencia ToUnicode sobre ambas, reinicio de contador
Differences, tres contratos Type3 con CharProcs reales y dos variantes bfrange.
Anexos D.5/D.6 contrastados íntegramente por el generador (189/188 asignaciones).
Total: 30 tests, 958 códigos asignados y las 216 combinaciones previas de espaciado.
Las nuevas pruebas Type3/precedencia/rangos pasan; Symbol/Zapf permanecen en RED.
Detalles en `docs/reports/2026-10-01-issue-666-step2.md`.

Siguiente incremento: referencias/licencias y programas redistribuibles para
Type1/CFF/TrueType y MacExpert; cadenas PDFDocEncoding/UTF-16BE. La política de
posiciones indefinidas sigue pendiente: no se ha impuesto como normativa la
recuperación actual ni una salida arbitraria. La matriz completa sigue abierta.


## Tercer incremento — 2026-10-01

Diez tests nuevos: tabla PDFDocEncoding completa por dos rutas públicas (232
posiciones definidas; 24 indefinidas inventariadas), UTF-16BE válido y vacíos,
y cuatro pruebas con SourceSans3 3.052 CFF/OpenType incrustada completa. Licencia
OFL y hash oficial conservados. Total 40 tests. Metadatos y ActualText revelan
38 discrepancias PDFDocEncoding por ruta; fuente incrustada y UTF-16BE pasan.
Informe: `docs/reports/2026-10-01-issue-666-step3.md`.

Siguiente bloque: TrueType y subconjuntos, Type1/MacExpert, rutas marcadores y
formularios, política de UTF-16 malformado/posiciones indefinidas. CJK, vertical,
mutaciones ampliadas y corpus siguen pendientes. No cerrar #666 por este hito.


## Cuarto incremento — 2026-10-01

Nueve pruebas nuevas: TrueType simple completa/subconjunto, selección WinAnsi y
precedencia ToUnicode, espacio/avance; PDFDocEncoding y UTF-16BE en marcadores y
valores V de AcroForm. Total 49 tests. Subconjunto reproducible con siete glifos,
renombrado conforme a OFL; comparación raster externa idéntica para AéB.
Informe: `docs/reports/2026-10-01-issue-666-step4.md`.

F07 sigue parcial: no se atribuye cobertura de fuentes TrueType simbólicas ni
cmap alternativos a estas pruebas. Formularios cubren lectura del valor guardado,
no relleno/apariencias. Próximo bloque: Type1/MacExpert, CMaps/CID y políticas
pendientes, seguido de mutaciones ampliadas y corpus.


## Quinto incremento — 2026-10-01

Nueve tests nuevos (58 totales): CIDFontType2 incrustada, CIDToGIDMap stream,
Identity-H y CMaps horizontales cidchar/cidrange con códigos de 1–4 bytes;
Type1C sintética con nombres MacExpert y ToUnicode para 165 códigos.
MacExpert contrastada completamente con Annex D.4. Fallback sin ToUnicode
para /fi produce W: RED adicional. Política de PUA inferida aún pendiente.

Comparación raster externa: cidchar/cidrange/Identity-H iguales, GIDs
intercambiados distintos. Poppler avisa/omite texto con ToUnicode de códigos
largos: verificación externa de ese texto incompleta, sin ocultar el límite.
Informe: `docs/reports/2026-10-01-issue-666-step5.md`.
Siguiente bloque: colecciones CJK y CMaps oficiales fijados, CIDFontType0,
vertical y controles de códigos malformados; Type1 PFB sigue pendiente.


Actualización posterior — discrepancia Poppler resuelta: los límites numéricos de
Poppler 24.02.0 explican avisos/pérdida de texto. MuPDF 1.26.10 confirma los
fixtures originales; nueve casos mínimos verifican los umbrales. Se conservan
expectativas y fixtures. Informe: docs/reports/2026-10-01-issue-666-poppler-discrepancy.md.


## Flujo de lectores independientes — 2026-10-01

Incorporado tools/text_contracts: MuPDF/PyMuPDF fijados para contraste principal
de extracción; Poppler secundario con excepciones exactas por caso/versión;
qpdf estructural. Manifest inicial de 14 PDFs con hashes y expectativas externas,
sin trim ni normalización. Ocho pruebas adversariales validan el flujo. Lectores
ausentes, fixtures alterados o desacuerdos inesperados producen exit 1.
El informe aceptado conserva los límites Poppler, no los convierte en acuerdos.

No sustituye los 58 tests Rust ni certifica la matriz completa. Nuevos casos CJK,
CIDFontType0/vertical deberán incorporar referencia/expectativa independiente;
para páginas múltiples/layout se ampliará explícitamente el schema. Integración
CI y corpus quedan para T5. Informe: docs/reports/2026-10-01-issue-666-reader-validation.md.


## Sexto incremento — escritura vertical — 2026-10-01

Nueve tests nuevos, once escenarios: Identity-V/WMode 1, DW2, W2 array/rango,
fallback DW2, TJ ±300, Tz 50/200 y Tc. Total 67 tests Rust. Los nueve quedan RED
en base y ambos PRs por avance horizontal; TJ negativo además sintetiza espacio.

El validador externo incorpora los once PDFs (25 totales) y origen de glifo
opcional calculado de métricas, comparado con get_texttrace de MuPDF en nueve.
No se confunde el origen del pen de los tests Rust con el origen0 del glifo ni
con el origen de layout de rawdict. Los dos Tz no afirman validación externa de
sus offsets de glifo. Diez pruebas del validador pasan, incluida mutación de
geometría con texto correcto. Informe: docs/reports/2026-10-01-issue-666-step6.md.

La corrección vertical requiere issue abierta propia antes de modificar producto.
Siguiente bloque: colecciones/CMaps CJK oficiales y CIDFontType0; quedan Type1 PFB,
políticas malformadas, mutaciones ampliadas, corpus y CI pendientes.


## Séptimo incremento — CIDFontType0 — 2026-10-01

Seis tests nuevos, total 73: programa CFF CID real con ROS/FDArray/FDSelect,
Identity-H, Encoding incrustado que remapea CID, ToUnicode y anchuras por CID;
control vertical W2. Cinco pasan y el vertical reproduce el RED ya conocido.
CIDs 17/29 distintos de GIDs 1/2 y contornos distintos permiten contraste raster.

Cuatro fixtures externos nuevos: texto coincide en MuPDF/Poppler y geometría en MuPDF;
mapas equivalentes producen igual raster, orden invertido lo cambia. Validador
29 PDFs, 13 geometrías; sin nuevas excepciones. Fuente original reproducible,
sin dependencia de fuentes del host. Colecciones CJK oficiales, múltiples FD y
Type1 PFB siguen pendientes. Informe: docs/reports/2026-10-01-issue-666-step7.md.

## Octavo incremento — CMaps CJK oficiales — 2026-10-01

35 CMaps / 117 muestras trazables a revisiones fijadas de cmap-resources y
mapping-resources-pdf. Siete tests nuevos; 80 totales. Cinco colecciones RED,
incluida Adobe-KR separada de Korea1; precedencia ToUnicode explícita pasa.
Cinco tests del generador y regeneración reproducible pasan. qpdf valida 117
PDFs; 86 coincidencias exactas en cada lector. Los 31 desacuerdos MuPDF se
conservan pendientes de clasificación, sin alterar expectativas ni admitirlos
como excepciones del gate. No se afirma cobertura exhaustiva ni tipografía CJK.
Informe: docs/reports/2026-10-01-issue-666-step8.md.

## Noveno incremento — transformaciones y fronteras — 2026-10-01

Nueve contratos de geometría pasan en las tres revisiones: Tm/CTM, rotación,
cizallamiento, composición, BT/ET, q/Q, fuentes y Forms anidados. MuPDF confirma
22 orígenes en nueve PDFs estructuralmente válidos. Total inventariado 89 tests.
Informe: docs/reports/2026-10-01-issue-666-step9.md. Discrepancias CJK explicadas
por tablas históricas, recursos ausentes y códigos nuevos; oráculos conservados.

## Décimo incremento — scripts y PDF 2.0 — 2026-10-01

Ocho tests de repertorios Unicode pasan; cinco contratos UTF-8 documentales dan
3 PASS/2 RED. Total 102 tests inventariados, nuevas pruebas ejecutadas en las
tres revisiones. Seis metadata UTF-8 confirmadas con MuPDF/qpdf. Correcciones
PDFDocEncoding/UTF-8 vinculadas a #667 OPEN. Política malformada y rutas restantes
pendientes. Informe: docs/reports/2026-10-01-issue-666-step10.md.

## Primer GREEN documental — #667 — 2026-10-01

PDFDocEncoding completo y recuperación UTF-16 sin pérdida silenciosa de cola,
con política U+FFFD explícita. Dos pruebas adicionales, 104 totales inventariadas.
Suite documental RED 6/6, GREEN 12/0; 58 objetos y 22 consumidores pasan.
UTF-8 versionado, mutaciones, revisión/corpus/CI pendientes. Producto solo en
clon aislado; parche durable. Informe: docs/reports/2026-10-01-issue-667-document-strings.md.

## UTF-8 con contexto de versión — #667 — 2026-10-01

Once contratos PDF 2.0 pasan; 110 tests inventariados. Versiones de cabecera y
catálogo, controles pre-2.0 y entradas inválidas cubiertos. Metadata, ActualText,
marcadores y valor AcroForm por objeto público. Biblioteca final 6776 PASS,
3 omitidos preexistentes; Clippy focalizado/formato pasan. Mutaciones, QR,
corpus, CI y rutas restantes pendientes. Informe: docs/reports/2026-10-01-issue-667-utf8.md.

## ActualText estructural y mutaciones — #667 — 2026-10-01

Tres contratos nuevos, 113 inventariados. Página/Form con ParentTree y override
inline pasan; cuatro PDFs válidos para qpdf. Seis mutaciones documentales
producen el fallo de aserción previsto; restauración 26/26, target aislado.
Informe: docs/reports/2026-10-01-issue-667-structured-mutations.md. Mutaciones
de otras familias, QR/corpus/CI y resto de matriz siguen pendientes.

## Correcciones de la revisión documental — #667 — 2026-10-01

Version indirecta acotada, nombres AcroForm y metadata de firmas con contexto
de versión. Cuatro tests adicionales, 117 inventariados. Treinta contratos
documentales y dos consumidores pasan; biblioteca 6776 PASS/3 omitidos.
Kripteia 94/340/10 y Security sin alertas; reinspección sin nuevos hallazgos
confirmados. Corpus/configuraciones/CI y resto de la matriz siguen pendientes.
Informe: docs/reports/2026-10-01-issue-667-review-fixes.md.

### Avance #662: MacRoman PDF

Tabla PDF corregida en candidato aislado con RED/GREEN: 208 asignados y
48 indefinidos, precedencias y mutación currency/euro. Dos contratos nuevos;
119 tests inventariados, sin afirmar que la matriz completa pase. Informe
`../reports/2026-10-01-issue-662-macroman.md`. Revisión/corpus/CI pendientes.

## GREEN de codificaciones simples — #668

Seis tablas PDF centralizadas: 1123 asignados y 413 indefinidos con recuperación
U+FFFD. MacExpert conserva AGL PUA; ToUnicode puede aportar texto semántico.
Doce Standard14 latinas × tres encodings × ambos modos; selección Symbol/Zapf,
Differences (188 nombres Zapf), controles negativos y métricas AFM/origen.
Diez contratos adicionales: 129 totales. Suite completa 112 PASS/17 RED/0 omitidos.
QR corrigió tres hallazgos, cinco mutaciones discriminan contratos. Biblioteca
6776/0/3; Clippy/formato focalizados pasan. Corpus/configuraciones/CI pendientes.
Informe: `../reports/2026-10-01-issue-668-quality-review.md`.

## Recuperación Type0 — #663

Ocho contratos nuevos (137 inventariados): modo, descendiente, límites em,
espacios ToUnicode, secuencias, códigos múltiples/truncados y exclusión vertical.
RED 6/2, GREEN 8/0; el RED original lenient pasa. Cuatro mutaciones detectadas,
biblioteca 6776/0/3. La batería de espaciado solo mantiene el fallo TJ/Tz.
No se reparan métricas desconocidas: la política limita la síntesis de espacios.
Corpus/configuraciones/CI pendientes. Informe `../reports/2026-10-01-issue-663-recovery.md`.

## F07/C07 — incremento 2026-10-03

F07 incorpora ocho contratos GREEN con ocho glyf completos/subconjuntos y cmap
0/4/12 simbólica/no simbólica, 12 PDFs confirmados por ambos lectores. C07 añade
diez contratos: cuatro GREEN/seis RED de herencia y anchuras; dos probes notdef
siguen exploratorios por discrepancia externa. Regeneración de 35 archivos idéntica,
tres mutaciones de entrada detectadas y tres controles GREEN. Total reejecutado:
167 tests, 143 PASS/24 FAIL/0 omitidas, 18 targets. Cobertura por fila actualizada;
no se declara terminada la matriz. Informe y límites:
`../reports/2026-10-03-issue-666-symbolic-usecmap-review.md`.
Siguiente: resolver el oráculo notdef y políticas de ciclos/truncamiento; vincular
issues específicas antes de corregir producto. #662/#663 corresponden a oshtivi.


## Continuación CJK y consumidor — 2026-10-04

Bajo #676 OPEN se corrigen las tres regresiones del relevo: KR es una colección
independiente, CID0 conserva omisión explícita en el fallback y el consumidor
ResolvedFontResource resuelve code→CID→Unicode antes de devolver texto. El fixture
UniJIS de #513 ahora declara Japan1 y W para CID194; el código00E1 no es CID225.
El RED corregido del fixture revela Unicode ausente, evitando ocultarlo al cambiar
solo la aserción del CID. Las117 muestras Adobe existentes verifican también el
consumidor con/sin ToUnicode, origen del código, CID, GID y anchura.

Política CID0: no aporta texto en fallback, pero conserva su avance y admite
ToUnicode explícito. Prueba adicional RED detectó un espacio espurio cuando todos
los códigos son .notdef; se preserva la salida vacía en ambos niveles de fallback.
Códigos truncados del renderer producen error; los no mapeados conservan su código,
carecen de CID/Unicode y usan DW. Registro privado no activa tablas Adobe;
CIDSystemInfo y sus cadenas pueden ser indirectos.

La regeneración offline y hashes de procedencia coinciden. Evidencia y comandos
actuales en docs/reports/2026-10-04-issue-666-evidence/. La validación final se
registra en TASKS.md al terminar; no transferir resultados intermedios al árbol
final. Matriz completa, políticas restantes, revisión/CI e integración siguen
pendientes; no cerrar #666 ni #676 por este incremento.
