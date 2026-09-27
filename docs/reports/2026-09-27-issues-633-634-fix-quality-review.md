# Quality review — correcciones #633 y #634

Revisión del autor mediante skill quality-review. Dos pasadas: contrato/diseño
antes de comprobar implementación y pruebas. Base develop ff263dba; diff en
writer/pdf_writer/mod.rs y operations/semantic_preservation.rs. No API pública
nueva ni cambios de dependencias. Sin revisión delegada.

## Contratos y evidencia

#633: los enlaces de hermanos y Parent deben representar el árbol construido
por el usuario. Los IDs se reservan ahora por lista antes de recorrer hijos;
First/Last/Prev/Next proceden de esa misma lista. Complejidad O(n) de asignación;
se elimina el recorrido previo de recuento y los vectores acumulados de IDs.
Se conserva visible_count y la conversión de cada dictionary/destino/estilo.
Cinco tests: hermanos raíz tras descendientes, tres ramas anidadas con nietos,
lista plana, rama única cerrada/negrita y árbol vacío. Comparación del árbol
completo; parser valida Parent/Prev/Last, ciclos y referencias. RED: ambos árboles
con descendientes fallan con cycle or duplicate; control plano pasa. GREEN: cinco.

#634: informe y salida deben aplicar la misma política. Un helper privado crea
PageExtractor con preserve_metadata derivado de ReconstructMetadataPolicy.
Ambas operaciones lo usan sobre el snapshot ya validado; persistencia, alias,
permisos, orden de preparación de partes y rollback permanecen intactos.
Legacy PageExtractor::new conserva su comportamiento para consumidores existentes.
Cuatro tests comparan título/autor/asunto/keywords e informe, dos partes en split,
conteo de páginas y bytes de fuente intactos. RED: ambos Discard fallan por título
presente, ambos controles FirstInputWins pasan. GREEN: cuatro.

Regresiones existentes #539/#548/#621: 37 pasan. No sustitutos ni mocks; PDFs
sintéticos creados, escritos y leídos con APIs reales. Los nuevos tests han
mostrado discriminación con el código defectuoso original; no se requirió una
mutación adicional que reproduzca esos mismos fallos.

## Hallazgos

No quedan hallazgos de corrección en el alcance del diff. Las dos causas
confirmadas en la revisión previa quedan corregidas y verificadas con RED/GREEN.

## Calidad de Tests (Kripteia)

Script oficial de la skill sobre copia de los dos archivos cambiados,
page_extraction, tests del writer, #539 y las dos regresiones nuevas:
98/100, 141 tests en 6 archivos. Nuevos tests: #633 5/100, #634 4/85.
La herramienta marca wrappers de #634 como sin llamada productiva: revisión
manual confirma que llaman check_metadata, que ejecuta extract/split reales y
aserciones de cuatro campos y report. Avisos genéricos sobre unwrap/constantes
no son defectos en estos tests. Alcance parcial, no métrica del core entero.

## Análisis de Seguridad (Kripteia Security)

Resultado real: No security issues found. No unsafe/FFI añadido ni nuevas
entradas de red o rutas públicas. La retención de metadatos contraria a Discard
queda cubierta por las dos regresiones negativas. El scan no sustituye esta
comprobación de comportamiento.

## Validación final

Biblioteca: 6791 pasan, 3 omitidos. Clippy all-targets con -D warnings,
cargo fmt --all -- --check y git diff --check pasan. QR completado antes del PR.
CI remota posterior separada de la revisión local. No publicación de paquetes.

## Métricas

Dos archivos de producto cambiados, dos archivos de regresión nuevos, nueve tests
nuevos y cuatro fallos RED relevantes. Cero hallazgos pendientes del diff.
