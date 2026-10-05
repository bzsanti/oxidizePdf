# #666 — Type1 intrínseca y trazabilidad de la matriz

## Resumen ejecutivo

Incremento de baterías sobre producto idéntico a develop
`d33e7f94b6053e26f29b0523a52b87b08013babb`; checkout
`a9c47deec7f805e73533032b9d26c7f7f4c95708`, con WIP previo preservado.
No hay cambios en código de producto ni dependencias Cargo. #662/#663 quedan
abiertas para que oshtivi ejecute sus cierres, por instrucción del usuario.

Se añaden diez contratos Rust y 16 PDFs con programas originales Type1/PFB y
Type1C, completos y subconjuntos. Los programas tienen .notdef, space, A/B y,
solo en la versión completa, C. A y B tienen contornos diferentes y anchuras
400/700. La codificación intrínseca asigna 65→B y 66→A; los códigos ASCII no
pueden acertar accidentalmente. La versión subconjunto conserva los contornos
y elimina el glifo C. Las expectativas no proceden del decodificador.

Resultado del incremento: **8 PASS / 2 FAIL**, ambos fallos por ignorar la
codificación intrínseca. Matriz ejecutada completa disponible: **16 targets,
149 tests, 131 PASS / 18 FAIL / 0 omitidos**. No es la matriz de alcance completa
ni una aprobación para integrar. #666 sigue abierta. F07 simbólica, C07/usecmap,
políticas y combinaciones restantes siguen enumeradas en coverage.json.

## Contratos y evidencia

| Contrato / fuente | Entrada y consumidor | Evidencia actual |
|---|---|---|
| Type1/PFB y Type1C reales | FontTools 4.60.1 genera programas; lectura independiente verifica Encoding, charstrings, contornos y anchuras; PDF incrusta payload PFB sin cabeceras de registro | 24 archivos regenerados idénticos; hashes/PFB validados por Rust |
| Encoding intrínseca, ISO 32000-1 §9.6.6.1 | PdfReader → TextExtractor, strict/lenient; cuatro fuentes | Esperado BA, observado AB en las ocho combinaciones; RED demostrado |
| Differences sin BaseEncoding | Un solo código reemplazado; el otro debe heredar del programa | Esperado AA, observado AB; RED demostrado |
| Encoding explícita / ToUnicode | WinAnsi produce AB; ToUnicode produce XY sobre Differences | Ambos contratos GREEN en los dos modos y cuatro fuentes |
| Avances independientes del Unicode | Widths por código, Tfs=10; segunda posición x=107 o 104, y=700 | Cuatro contratos geométricos GREEN; MuPDF confirma 32 orígenes en los 16 PDFs |
| Fixtures externos | MuPDF 1.26.10/PyMuPDF 1.26.5, Poppler 24.02.0, qpdf | 32 coincidencias exactas de texto, 16 geometrías y 16 checks estructurales; sin avisos ni excepciones |
| Contorno completo/subconjunto | Raster MuPDF de cada modo; control intrínseco frente a WinAnsi | Iguales al reducir glifos; distintos al cambiar selección; ToUnicode conserva raster de Differences |
| Discriminación de aserciones | Consumidor desechable bajo target, copia de las aserciones; producto y fixtures canónicos inmutables | Tres controles pasan; cambiar código mostrado, destino Unicode o Widths produce el fallo de contenido/posición previsto |
| Trazabilidad T0 | 46 IDs del plan contrastados con manifest; oráculo, estrategia, criterio de cierre, tests y pendientes | Ocho pruebas adversariales pasan; 13 Python totales con las cinco previas de CJK |

Las mutaciones son **de entrada**, no mutaciones del producto ni prueba completa
del T4 general. Cada una ejecuta las aserciones originales contra un PDF alterado
con igual longitud para conservar xref. Los logs verifican BB frente a AB, ZY
frente a XY y x=109 frente a x=104; no se contabilizan errores de compilación.

La geometría usa `preserve_layout=true`. El primer ensayo del arnés omitía esa
opción y no exponía fragments; se corrigió el setup antes del resultado final,
sin modificar producto ni expectativas. Los programas iniciales sin /space
producían avisos FreeType en MuPDF; se sustituyeron por programas con /space
real antes de congelar hashes. No se relajó el validador para admitir avisos.

Comandos y resultados completos: `2026-10-03-issue-666-evidence/`.
`all-contracts-command.json` conserva los 16 targets exactos;
`all-contracts-summary.json` registra exit 101 y población. `baseline.log`
reproduce los 16 RED previos sobre código integrado: diez verticales, cinco
colecciones CJK y TJ/Tz. No se confunden discrepancias CJK con una clasificación
normativa nueva; sigue aplicando el diagnóstico independiente de 2026-10-01.

## Hallazgos

1. **La codificación del programa no llega a la extracción**:
   `oxidize-pdf-core/src/text/extraction_cmap.rs:1034` — el fallback sin Encoding
   usa el valor del byte. Los fixtures PFB/CFF correctos producen AB donde los
   dos lectores y la codificación del programa determinan BA. Differences sin
   BaseEncoding también pierde la herencia y produce AB en vez de AA. Solución:
   extraer la codificación intrínseca del programa con límites explícitos y
   usarla como base, preservando Differences y ToUnicode. Corrección bloqueada
   hasta vincular issue específica OPEN; responsable bzsanti/Codex. No se
   modifica producto bajo #666 ni se atribuye este alcance a #662/#663.
2. **El generador incluía archivos ajenos en provenance**:
   `tools/generate_text_type1_contracts.py:183` — la primera versión recorría
   todo OUTPUT y podía incorporar PDFs de otra ejecución. Corregido en este
   incremento enumerando únicamente los nombres generados. Probe con
   unrelated.pdf: archivo preservado y excluido; los 24 archivos canónicos
   permanecen idénticos. Revisión del cambio y scanner repetidos.

## Calidad de Tests (Kripteia)

Rust: **88/100, 10 tests, 1 archivo**. Los avisos de “no production code” no
siguen los helpers: ambos llaman al lector y extractor públicos. Las tres
pruebas geométricas señaladas como vecinas similares seleccionan diferentes
diccionarios PDF; las mutaciones de texto/anchura acreditan que se ejecutan
datos reales. No se reemplazaron oráculos constantes por lógica de producto.
La prueba de PFB es integridad de fixtures, no cobertura de parser de entradas
hostiles; el alcance está separado de los ocho contratos funcionales.

Python: el scanner reporta **100/100, 0 tests, 0 archivos** en generador,
validador y unittest. Esa salida no acredita cobertura Python. Se inspeccionaron
manualmente y se ejecutaron los ocho tests nuevos, la regeneración y los
controles externos. Logs: `type1-kripteia-{rust,python}.log`,
`coverage-kripteia.log`, `coverage-tests-kripteia.log`.

## Análisis de Seguridad (Kripteia Security)

Los cuatro análisis ejecutados reportan “No security issues found”. No hay
unsafe/FFI nuevo, credenciales ni peticiones remotas en las pruebas. FontTools
y PyMuPDF son herramientas de validación instaladas bajo target, no dependencias
del producto. Los programas PostScript originales se inspeccionan con el
lector acotado de FontTools; no se invoca un intérprete PostScript del sistema.
La CLI de trazabilidad rechaza rutas de plan fuera del repositorio y nombres
de targets que escapen del directorio. No certifica el contenido normativo
de los oráculos ni la seguridad de las dependencias de validación.

## Métricas

- Código nuevo revisado: cuatro archivos (suite Rust, generador, validador y
  unittest); además manifest, atributos Git y los 24 artefactos generados.
- Hallazgos totales: 2; uno de producto pendiente, uno del generador corregido.
- Archivos de producto modificados: 0.
- Clippy focalizado con warnings denegados, rustfmt y diff-check: pasan.
- Verificaciones pendientes: corrección intrínseca bajo issue propia, resto
  de matriz/mutaciones/corpus/configuraciones, revisión final y CI/integración.
- No commits, PR, merge ni cierre remoto en este incremento.
