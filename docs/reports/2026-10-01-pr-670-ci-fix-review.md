# #670 — fallo de integración en la política de CR

## Resumen ejecutivo

Ubuntu falló en tres pruebas de issue_476_carriage_return_policy_test.rs.
La validación focalizada anterior omitió esta suite. El fixture asignaba
semántica de CR/LF a 0x0D/0x0A de WinAnsi, que el oráculo independiente ya
registraba como indefinidos. La corrección normativa de #668 recupera esos
códigos como U+FFFD; la política de CR actúa después de decodificar Unicode.

Se corrige únicamente el fixture: declara ASCII y CR/LF mediante ToUnicode.
Las tres aserciones originales se conservan. Dos controles nuevos prueban
NormalizeLineEnding y el caso sin ToUnicode, donde todas las políticas deben
conservar U+FFFD. No cambian tablas, expectativas normativas ni producto.

## Contratos y evidencia

RED local: 1 PASS/3 FAIL, mismas aserciones y salidas que Ubuntu. GREEN:
6/6 pruebas de CR y 29/29 contratos de codificación. El control sin ToUnicode
activa el mismo PDF sin asociar el CMap; detectaría que se volvieran a tratar
los bytes indefinidos como controles. Las salidas de Remove, ReplaceWithSpace
y NormalizeLineEnding son distintas y comprobadas exactamente.

La batería ampliada selecciona todos los targets de integración versionados
con internal-testing, además de la biblioteca. Excluye los tests no versionados
de otros bloques pendientes de #666. El helper local preexistente solo añade
una función no utilizada por estos targets; no se mezcla en el commit.
Resultado de integración final: 375 targets y biblioteca, 9766 PASS,
cero fallos, 47 omitidas; exit 0. Doctests: 220 PASS/24 omitidos.
Clippy y formato pasan. La corrección está publicada como a8b595e; la nueva CI
remota sigue en curso, sin fallos a última consulta.

## Hallazgos

1. El fixture antiguo mezclaba decodificación de glifos con normalización de
   texto Unicode. Corregido mediante ToUnicode explícito, manteniendo las
   aserciones de política y añadiendo el control negativo de bytes indefinidos.
2. La validación anterior del PR omitió la suite #476. Se amplía a todos los
   targets de integración versionados para registrar por separado su resultado completo.

## Calidad de Tests (Kripteia)

93/100, seis tests, un archivo. Se inspeccionaron las alertas de constantes
esperadas y de ausencia de llamadas a producto: las constantes son los
resultados del contrato y el helper llama PdfReader/PdfDocument/TextExtractor
reales. No se sustituyen las expectativas por cálculos del producto.

## Análisis de Seguridad (Kripteia Security)

No security issues found. Cambio de fixture y tests; sin código de producto,
FFI, unsafe o dependencias nuevas.

## Métricas

Un archivo Rust revisado; dos hallazgos, fixture corregido y ampliación de
validación completa con exit 0. Evidencia en 2026-10-01-pr-670-ci-fix-evidence/.
Pendiente: nueva CI remota. Hook y publicación completados.

Skill: [quality-review](/home/santi/.codex/skills/quality-review/SKILL.md).
