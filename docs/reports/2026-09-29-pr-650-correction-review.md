# Corrección de revisión #650 / #648

## Resumen ejecutivo

Revisión del autor sobre fa4f944 más la corrección perpendicular. Se conserva
el enfoque del PR: reducir el umbral TJ únicamente tras un desplazamiento
perpendicular significativo. El guard usa ahora abs(det(M))/hypot(M.a,M.b),
en el mismo marco que pen_delta; el componente de cizallamiento paralelo no
lo infla. Fuentes exactas: correction-evidence/source.json contiguo.

## Contratos y evidencia

RED previo: misma separación horizontal 4 pt y vertical 1.05 pt producía
alpha beta sin shear y alphabeta con CTM [1 0 0.5 1 0 0]. GREEN: ambos devuelven
alpha beta. Regresiones públicas: 11 de #648 y 16 de #458 pasan. Rotación,
escala, jitter, superíndice adyacente, cambio de fuente y salto de línea siguen
cubiertos. Biblioteca: 6780 pasan, 3 omitidos. Corpus: 34 fusión y 38 orden
pasan, con 1695/1058 PDFs comparados. Umbrales/baselines sin cambios.
Clippy focalizado y formato pasan. No se repitió workspace completo.

## Hallazgos

1. Discrepancia de proyección en extraction.rs corregida y cubierta por el
   contraejemplo público. Matrices no finitas/degeneradas conservan el fallback
   existente; no se amplía aquí el contrato de matrices inválidas.
2. TASKS y CHANGELOG actualizados. La reproducción ejecutable de la issue se
   vincula a los tests públicos; Td se mide desde el origen de línea. Publicación
   y CI se verifican por separado; este informe acredita validación local.

No quedan fallos confirmados pendientes dentro del alcance de esta corrección.

## Calidad de Tests (Kripteia)

extraction.rs: 89/100, 92 tests detectados. Regresión #648: 100/100, 11 tests.
La puntuación no prueba geometría: la evidencia discriminante es el RED/GREEN
con cizallamiento. Las cadenas literales son oráculos independientes apropiados.

## Análisis de Seguridad (Kripteia Security)

Ambos análisis: No security issues found. Sin nuevo unsafe, FFI, red ni acceso
externo; cálculo constante sobre el estado de texto. No cambia los límites ni
el tratamiento de errores. División no finita cae en el fallback explícito.

## Métricas

- Archivos directos de código revisados: 2.
- Corrección de producto: proyección del guard vertical; sin otras rutas nuevas.
- Validación: 27 focalizadas + 6780 biblioteca + 72 diferenciales; cero fallos.
- Pendiente: CI del nuevo commit; integración no ejecutada.
