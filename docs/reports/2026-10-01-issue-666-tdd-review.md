# #666 — primer incremento TDD de contratos de texto

Issue: #666 — test(text): build normative encoding and glyph-spacing contract batteries — https://github.com/bzsanti/oxidizePdf/issues/666

Estado: implementación parcial, fase RED deliberada. La issue permanece abierta.
Plan completo: [matriz y secuencia TDD](../plans/2026-10-01-issue-666-text-contracts-tdd.md).
Evidencia: [directorio](2026-10-01-issue-666-evidence/).

## Alcance implementado

21 tests públicos (7 codificación, 14 espaciado), helper de PDFs, generador offline,
seis tablas externas de 256 posiciones con procedencia, licencias y SHA256,
y manifiesto de cobertura de 46 filas. Las tablas no se generan desde el decoder.
Standard/MacRoman/WinAnsi contrastadas con 229 filas del anexo D.2 de PDF 32000-1;
581 códigos asignados ejercitados mediante extracción real, sin trim ni normalización.
MacExpert/Symbol/Zapf tienen referencias preparadas, no conformidad validada.
Las posiciones indefinidas constan en las tablas; su contrato de extracción queda pendiente.

Espaciado: 216 combinaciones (9 escenarios × 12 fuentes Standard14 latinas × 2 modos),
controles Type0 dañados y tres escalas horizontales. Precedencia Differences/ToUnicode,
secuencias Unicode, pares sustitutos, combinantes, strings literales/hexadecimales.
No se atribuye cobertura de Symbol/Zapf a las doce fuentes latinas.

## Comparación reproducible

Mismos archivos nuevos aplicados sobre tres clones aislados, sin cambios de producción:

| Revisión | SHA | Tests que pasan | Tests que fallan |
| --- | --- | ---: | ---: |
| develop | 48d8b8f8bbf08b2976fd739e6c2462c9552a3cc6 | 16 | 5 |
| PR #664 | 70f28714e2ef7115c0f0898d47a3af6cae3de953 | 16 | 5 |
| PR #665 | c7031f7645d947de03264485c8e38c4452feebe3 | 15 | 6 |

```sh
cargo test --locked --offline -p oxidize-pdf \
  --test text_encoding_contract_test --test text_spacing_contract_test \
  --no-fail-fast -- --nocapture
```

Los tres comandos salen con 101 por aserciones de contrato; cero tests ignorados.
No son errores de compilación ni fixtures que el parser no pueda abrir.

| Contrato | develop | #664 | #665 |
| --- | ---: | ---: | ---: |
| StandardEncoding, discrepancias / 149 | 47 | 47 | 47 |
| WinAnsiEncoding, discrepancias / 224 | 8 | 8 | 8 |
| MacRomanEncoding, discrepancias / 208 | 76 | 1 | 76 |
| Recuperación lenient Type0 dañada | falla | falla | pasa |
| Preservar espacios con fuentes simples sin Widths | pasa | pasa | falla en 24 casos |
| No activar recuperación desconocida en strict | pasa | pasa | falla |
| Geometría de TJ con Tz | falla | falla | falla |

#664 conserva currency 0xDB interpretado como Euro. WinAnsi pierde bullet en 0x7F,
reemplaza cinco posiciones bullet por U+FFFD y cambia comillas curvas 0x93/94 a rectas.
La geometría espera B.x=104 y 116 para Tz=50 y 200; obtiene 105.5 y 113.
El caso Tz=100 pasa. El cambio de Tr evita que la fusión legítima de fragments
oculte el origen de B y no modifica su posición.

## Discriminación por mutación

En una copia de #665 se desactivó únicamente el guard de espacio explícito
(`!has_explicit_space` → `false && !has_explicit_space`). El nuevo test sin espacio
alcanza el fallback con tres glifos y dos ajustes y falla: espera `1 0 1`, obtiene
`101`. El test original del PR `corrupt_descendant_fonts_without_explicit_spaces`
sigue pasando con esa misma mutación. El nuevo test pasa sobre #665 sin mutar.
Esto prueba una diferencia de sensibilidad concreta; no equivale a mutation score global.

Mutación independiente de infraestructura: cambiar 0xDB de currency a Euro en el
TSV sin cambiar su hash hace fallar el control de integridad; restaurarlo lo hace
pasar. Este control detecta cambios del oráculo, no sustituye la comparación semántica.
Logs RED/GREEN y del test original conservados en evidencia.

## Validación de infraestructura

- Regeneración offline: seis TSVs y provenance.json idénticos byte a byte.
- `.gitattributes` local fija LF en TSV para evitar hashes distintos por autocrlf;
  `git check-attr eol` confirma `lf`. No se ha ejecutado CI en Windows.
- Seis PDFs representativos pasan `qpdf --check`, exit 0. Esto valida sintaxis,
  no corrección semántica de una fuente: el descendiente de tipo incorrecto es intencional.
- Clippy focalizado con `--locked --offline` y `-D warnings`: exit 0.
- rustfmt de los tres archivos Rust: exit 0. No se han cambiado dependencias.
- `git diff --exit-code -- oxidize-pdf-core/src Cargo.toml Cargo.lock` en el clon base:
  exit 0; las ejecuciones no contienen correcciones de producto.

## Calidad de tests (Kripteia)

Salida real: Overall Score 90, Tests 21, Files 2; codificaciones 91, espaciado 90.
Peor puntuado: `declared_zero_widths_allow_uniform_advances`, 80 por proporción
setup/assert y llamada indirecta. El aviso de ausencia de llamada de producción
no identifica correctamente el helper: este invoca PdfReader y TextExtractor reales.
Los valores constantes son expectativas independientes deliberadas. Se revisaron
manualmente las aserciones y los guards; la mutación anterior aporta evidencia
que la puntuación estática por sí sola no proporciona.

## Seguridad (Kripteia Security)

Salida real: `No security issues found.` Alcance: los tres archivos Rust y generador
Python nuevos en el directorio de revisión. No constituye auditoría del parser completo.
El generador trabaja con archivos locales, no descarga ni ejecuta código de fuentes.
Las URLs, hashes y licencias de las referencias externas quedan registrados.
No se identificaron defectos adicionales confirmados en el harness de este incremento.

## Pendientes y restricciones

La matriz completa no está implementada: fuentes incrustadas, Type3, CMaps CJK,
vertical, scripts completos, cadenas de documento, resto de geometría/fronteras,
mutaciones adicionales, corpus diferencial y CI siguen pendientes. No se ha corrido
workspace completo ni recalibrado métricas; las suites nuevas aún están en RED.
MacRoman y recuperación se corrigen bajo #662/#663. Las correcciones de StandardEncoding,
WinAnsi y escala TJ quedan bloqueadas hasta crear/vincular issues abiertas apropiadas;
responsable bzsanti/Codex. #666 autoriza su caracterización, no sustituye esas issues.

Los archivos nuevos se han copiado al workspace original para revisión; las pruebas
se ejecutaron en clones de revisiones exactas, no sobre su WIP preexistente.
Sin commits, push, fusiones ni publicación de reviews. Siguiente paso: continuar
T1 con contratos de posiciones indefinidas y controles de precedencia pendientes,
y T2 con fuentes simples restantes antes de declarar cubierta la matriz.
