# Issue #667: document strings on integrated develop

## Resumen ejecutivo

Cambio sobre develop `4a2c38a8c9b4e0629a9027b3ba6e89c1d85b3b3a`, en la rama
`fix/issue-667-document-strings` del repositorio actual. Siete archivos de
producto; sin cambios de dependencias ni de las soluciones MacRoman/tracking
de Omer. PDFDocEncoding normativo, UTF-16 malformado con reemplazo visible y
UTF-8 con BOM limitado a la versión efectiva PDF 2.0. La revisión manual y
los dos análisis Kripteia están completados; CI remota e integración pendientes.

## Contratos y evidencia

- PDFDocEncoding: tabla independiente de 256 posiciones (232 asignadas,
  24 indefinidas), congelada antes de la implementación. Metadata, ActualText,
  marcadores y valores almacenados de formularios se comprueban por API pública.
  RED en esta base: seis pruebas pasan y seis fallan; GREEN: 12/12.
- PDF 2.0: 18 pruebas pasan; versión cabecera/catálogo, referencias indirectas,
  límite 64, ciclos, BOM ausente/interno, no-BMP, combinantes y secuencias
  malformadas. RED en esta ejecución: APIs ausentes impiden compilar la suite;
  no se presenta ese resultado como 18 fallos funcionales. RED funcional previo
  conservado en los informes históricos de #667.
- Consumidores: propiedades ActualText inline/nombradas/estructurales en páginas
  y Forms, nombres AcroForm y metadata de firmas reciben el contexto de versión.
  El test de firmas prueba texto, no validez criptográfica. Fechas y DA mantienen
  sus contratos propios; strings binarios y códigos de fuente no se reinterpretan.
- Regresiones: 50 pruebas adicionales de #459/#648/#649/#662/#663 pasan.
  La expectativa WinAnsi antigua de #459 se corrige explícitamente a PDFDocEncoding
  (0x97 = Š); los bytes originales de la fixture permanecen intactos.
- Biblioteca: 6776 PASS, cero fallos, tres omitidas preexistentes. Configuración
  mínima compression: 30/30; unstable-spi + semantic: 30 contratos + 14 tests SPI.
  Clippy completo, cargo fmt --all -- --check y diff-check pasan.

## Hallazgos

No hay nuevos hallazgos confirmados en el diff adaptado. Se verificaron los tres
hallazgos corregidos previamente: Version indirecta, identidad de nombres de
formulario y contexto de firmas. La resolución está acotada y propaga errores;
no se altera el flujo GlyphZeroWidthStatus ni la tabla MacRoman del autor.
No se certifica con esta revisión el resto de la matriz de #666.

## Calidad de Tests (Kripteia)

Ejecución sobre los siete archivos de producto y tres archivos de tests:
94/100, 340 tests, 10 archivos. Las advertencias sobre constantes se revisaron:
las expectativas literales provienen de contratos independientes, no de tablas
del producto. RED/GREEN ejecutado en esta base; las mutaciones históricas no se
presentan como repetidas en esta rama.

## Análisis de Seguridad (Kripteia Security)

Resultado real: `No security issues found.` Sin unsafe, FFI, dependencias ni
cambios criptográficos nuevos. Recorrido Version iterativo, detección de ciclos
y máximo 64 referencias; índices de tablas limitados por byte. Ningún resultado
de recuperación de Unicode certifica validez del PDF o de una firma.

## Métricas

- Archivos principales revisados: 10.
- Nuevos hallazgos confirmados: 0.
- Archivos de producto modificados por la fase de revisión: 0.
- Corpus final: 72/72 pruebas. Fusiones 278/211815, 1695 PDFs comparados y 107 omitidos; orden plano 0.185653 y reading-order 0.159528, 1058 comparados y 744 omitidos. Métricas idénticas a la base integrada; umbrales intactos.
- Pendientes: CI remota e integración.
- Evidencia: `2026-10-01-issue-667-develop-evidence/`, con hashes de fuentes.

Revisión realizada con [quality-review](/home/santi/.codex/skills/quality-review/SKILL.md).
