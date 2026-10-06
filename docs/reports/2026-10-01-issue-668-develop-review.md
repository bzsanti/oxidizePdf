# Revisión #668 sobre las contribuciones integradas

## Resumen ejecutivo

Rama `fix/issue-668-simple-encodings` del repositorio actual, base develop
`4a2c38a8c9b4e0629a9027b3ba6e89c1d85b3b3a`. Cambio independiente de #667/#669.
Cinco tablas PDF centralizadas: Standard, WinAnsi, MacExpert, Symbol y
ZapfDingbats. La función MacRoman de Omer es idéntica byte a byte a HEAD;
el algoritmo de tracking y los codecs de sistema operativo no cambian.
La revisión del corpus detectó y corrigió una ampliación indebida de la política
de recuperación. No quedan hallazgos confirmados sin resolver en este diff.

## Contratos y evidencia

- Oráculos independientes congelados: seis tablas de 256 posiciones (MacRoman
  como regresión). Las cinco corregidas suman 915 asignaciones y 365 posiciones
  indefinidas. Reemplazo U+FFFD, vecinos intactos, PUA/secuencias y política de
  NBSP/soft-hyphen explícita; no normalización de expectativas al producto.
- RED ejecutado: 13 PASS/14 FAIL en 27 contratos; GREEN inicial: 27/27 más 46 regresiones
  de #523/#648/#649/#662/#663. Doce Standard14 latinas en ambos modos; las 165
  entradas MacExpert usan un Type1C real y redistribuible, no solo un nombre.
- Selección incorporada: solo Type1 apropiada, sin programa incrustado ni
  codificación explícita; prefijos de subconjunto válidos. Controls de nombres,
  subtipos, Differences y ToUnicode. La exclusión de programas incrustados se
  comprobó por inspección; no acredita su codificación intrínseca. Las 188 reasignaciones
  de nombres Zapf se verifican por API pública.
- Métricas: se conservan códigos de origen y anchuras AFM. La prueba de origen
  del glifo siguiente detecta la pérdida de anchura por marcadores incorporados.
- Mutaciones ejecutadas en esta rama: selección incorporada desactivada,
  Differences desconocido devuelve el código base y marcador AFM Symbol ignorado.
  Las tres fallan por aserción; restauración verificada y 28 contratos/regresiones
  pasan después. No se presentan mutaciones históricas como nuevas ejecuciones.
- Biblioteca: 6776 PASS, cero fallos, tres omitidas previas. Configuración mínima:
  29/29 tras la corrección; unstable-spi + semantic: 29 contratos + 14 tests SPI. Clippy focalizado,
  formato completo y diff-check pasan.

## Hallazgos

1. **Recuperación personalizada demasiado amplia (corregido)**: el reemplazo
   incondicional de Differences desconocido eliminaba texto Type3 y de fuentes
   con codificación intrínseca desconocida. MuPDF 1.26.10 confirmó el texto
   recuperado antes del cambio. Dos pruebas públicas fallaron por aserción antes
   de acotar U+FFFD a codificaciones conocidas fuera de Type3; pasan las 29
   finales y 46 regresiones. La mutación del fallback conocido vuelve a fallar
   por aserción. Se preserva recuperación histórica, sin declararla normativa.

Se verificaron selección,
precedencias, pérdida de glifo por Differences desconocido y consumo de encoding
por las métricas. Los problemas corregidos en el candidato histórico tienen aquí
RED/GREEN o mutaciones discriminantes. La cobertura intrínseca completa de fuentes
incrustadas y las demás familias de #666 siguen fuera de este cierre parcial.

## Calidad de Tests (Kripteia)

97/100, 92 tests, seis archivos detectados por el analizador. Se proporcionaron
cuatro archivos de producto y tres de tests; el módulo de datos no cuenta como
archivo con tests. Los helpers ensamblan PDFs y llaman las APIs públicas reales.
Las constantes y los unwrap en tests fueron inspeccionados: son oráculos externos
y fallos explícitos, no sustitutos del código de producción.

## Análisis de Seguridad (Kripteia Security)

Resultado real: `No security issues found.` Sin dependencias, unsafe ni FFI
nuevos. Indexación por u8 en arrays de longitud 256; resolución Zapf sobre datos
estáticos y salida Unicode acotada por la entrada. Las tablas evitan Strings
temporales por byte; Differences conserva su resolución existente.

## Métricas

- Archivos principales inspeccionados: 7.
- Hallazgos confirmados: 1, corregido y verificado.
- Archivos de producto modificados por la corrección posterior: 1.
- Pendientes: CI remota e integración.
- Evidencia y hashes: `2026-10-01-issue-668-develop-evidence/`.

Skill: [quality-review](/home/santi/.codex/skills/quality-review/SKILL.md).

## Diagnóstico del corpus y validación final

La primera ejecución aprobó los umbrales históricos pero perdió cuatro PDFs
comparables (1058 → 1054). Se investigó antes de publicar: pdfium_type3,
poppler_type3 y los dos pdf-signature-sample-2sigs usan nombres personalizados
(`/mya`, `/achar`, `/a65`). La misma política afectaba mapas de dos documentos
grandes. Las expectativas normativas de las cinco tablas permanecen intactas.

Tras acotar la política pasan 72/72 pruebas (34 de fusiones y 38 de orden,
estando estas últimas instrumentadas con una línea diagnóstica por documento).
Se recuperan los cuatro PDFs y los mapas; 1058 comparados, 744 omitidos y
594837 palabras de referencia en ambos modos. Orden plano: 110427/594837
(0.185642), frente a 110433/594837 de la base; reading-order: 94887/594837
(0.159518), frente a 94893. Transposiciones idénticas: 97716/82176. Fusiones:
278/211815, 1695 comparados y 107 omitidos, igual que la base. Sin recalibración.

Los cambios residuales incluyen letras griegas de Symbol, confirmadas por MuPDF,
y fragmentos de PDFs corruptos (`tjet`, `tdatene`); los avisos de MuPDF se
conservan y no se presenta esa recuperación como un oráculo exacto. La cobertura
alfabética de fusiones pasa de 0.9732 a 0.9731; no se oculta esa diferencia.
`corpus-diagnosis.json` registra los 16 documentos afectados inicialmente y
sus cambios finales. Tres líneas diagnósticas se intercalaron con stderr y se
excluyeron del análisis JSON, sin afectar métricas ni los 16 casos investigados.

Código final: hashes en `source.json`; MacRoman idéntico a develop. Biblioteca
6776/0/3, configuración mínima 29/29, SPI 43/43, regresiones 75/75 y Clippy
pasan tras la corrección. MuPDF es una dependencia externa de validación fijada,
instalada bajo target; no añade dependencias C/FFI al producto.
