# Corrección de revisión #651 / #649

## Resumen ejecutivo

Revisión del autor sobre 857c780 más el rediseño del detector. El descuento de
tracking exige anchos cero verificables por código fuente (Type1/TrueType) o
CID (Type0). No lo activa la presencia de un espacio Unicode. Se conservan el
pipeline público y la comparación del exceso con tj_space_threshold. Fuente
exacta identificada en el manifiesto contiguo del directorio de evidencia.

## Contratos y evidencia

RED sobre el detector anterior: 6 fallos / 7 controles; defectos de métricas,
Tc, ausencia de espacio literal, hueco mayor y CIDs diferentes de códigos y
Unicode. GREEN final: 16 regresiones #649 y 16 #458 pasan. Las tres positivas
originales ahora declaran Widths cero; comprueban el contenido exacto. Los
controles usan Helvetica normal o Widths positivos y conservan I a I.
Type0 usa tanto Identity-H como CMap code->CID con W indexado por CID, distinto
de Unicode. Test sin espacio literal devuelve Test; hueco mayor AB CD E.
Biblioteca: 6780 pasan/3 omitidos. Corpus: 34 fusión y 38 orden pasan; no cambia
baselines. Clippy focalizado pasa. No se repitió workspace completo.

El detector requiere strings de un solo glifo alternando con avances positivos,
al menos dos ajustes repetidos y mayoría estricta dentro de 0.05 em del mínimo.
Un hueco mayor no invalida esa mayoría: solo su exceso genera un espacio.
Tc/Tw distintos de cero, métricas ausentes/positivas, un único ajuste, avances
ambiguos, strings multiglifo y kerns iniciales/finales conservan la regla
ordinaria. Estas son limitaciones deliberadas documentadas, no una garantía
universal de reconstrucción lingüística. La inferencia de tracking sigue siendo
una heurística, ahora condicionada por métricas reales.

## Hallazgos

1. Fusión de palabras normales corregida: se verifica el ancho de todos los
   glifos antes de inferir tracking. Incluye control con Widths positivos.
2. Palabra sin espacio literal y hueco real con outlier corregidos; ya no se
   exige espacio Unicode ni un cluster del 90% de todos los ajustes.
3. Fixtures engañosos corregidos con Widths/W reales y CMap no identidad;
   opciones públicas documentan la excepción. TASKS y CHANGELOG actualizados.
4. La reproducción ejecutable de la issue se vincula a estos tests y al
   comando real; la propuesta histórica queda identificada como reemplazada.

No quedan defectos confirmados pendientes dentro de este alcance. No se amplía
el soporte a escritura vertical ni se infieren avances de fuentes desconocidas.

## Calidad de Tests (Kripteia)

extraction.rs: 89/100, 92 tests detectados. Regresión #649: 96/100, 16 tests.
Los avisos de happy path no sustituyen los controles de métricas ni el RED/GREEN;
los literales de salida son oráculos independientes. No se modificó producto
para elevar la puntuación. Tres aserciones se reforzaron a igualdad exacta al
final; se repitieron las 32 regresiones sobre esas aserciones.

## Análisis de Seguridad (Kripteia Security)

Ambos análisis: No security issues found. Sin nuevos unsafe, FFI, red o secretos.
Descuento limitado a métricas explícitas; fallback ante incertidumbre. Se
eliminan el vector de ajustes, la ordenación y la decodificación Unicode extra.
Dos recorridos lineales; Type0 reutiliza la resolución de códigos/CIDs existente.
Los límites de salida permanecen en append_bounded; no se afirma un presupuesto
global de memoria ni una validación general del PDF.

## Métricas

- Archivos directos de código revisados: 2.
- Validación: 32 focalizadas + 6780 biblioteca + 72 diferenciales; cero fallos.
- Validación final de pruebas más estrictas: 32/32; producto idéntico al corpus.
- Pendiente: CI del nuevo commit; integración no ejecutada.
