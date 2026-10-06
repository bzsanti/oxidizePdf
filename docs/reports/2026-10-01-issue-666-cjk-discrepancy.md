# #666 — clasificación de los desacuerdos CJK con MuPDF

Los 31 desacuerdos del octavo incremento se explican mediante los recursos
oficiales de MuPDF 1.26.10, conservando el oráculo Adobe fijado:

- 21 resultados coinciden exactamente con su tabla CID → Unicode histórica.
  Incluyen omisión de selectores de variante, caracteres de compatibilidad y
  secuencias antiguas como `0.` frente a U+1F100.
- Ocho casos usan recursos ausentes: cuatro de 90pv-RKSJ-V y cuatro de
  UniAKR-UTF16-H. Los avisos del lector identifican el nombre ausente.
- Dos casos UniJIS-UTF16-H/V usan D82CDD32 (U+1B132). No aparece en el mapa
  histórico horizontal ni en su padre UniJIS-X; MuPDF devuelve texto vacío.
  Adobe actual asigna los CIDs 12269/12270, cuya interpretación Unicode también
  cambió respecto de las tablas históricas consultadas.

Evidencia por caso, URL y SHA-256 de cinco fuentes oficiales:
`2026-10-01-issue-666-cjk-discrepancy-evidence/classification.json`.
Los archivos completos de MuPDF no se redistribuyen. No se atribuye esta
clasificación fuente por fuente a Poppler; su salida permanece registrada.

La prueba exige la revisión Adobe declarada, no una verdad independiente de
versión. Estas diferencias no invalidan por sí solas las tablas antiguas como
implementación de una revisión anterior. El trabajo correctivo debe distinguir
actualización de cobertura y fallo de decodificación; no cambiar expectativas
para ajustarlas al lector ni presentar sus resultados como equivalentes exactos.
No se añaden excepciones automáticas al gate de lectores en esta investigación.
