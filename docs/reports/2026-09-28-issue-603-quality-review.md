# Revisión de #603 / #641 — 2026-09-28

## Resumen ejecutivo

Base main/v5.1.5 fb4042fd; alcance: diff documental, rustdoc, guard y artefactos
sintéticos de evidencia. Inspección de contratos en firmas, certificados,
cifrado del writer, filtros, OCR y preservación. Sin cambios de implementación,
API, dependencias, algoritmos, datos de producción ni baselines.
La corrección documental se propone directamente a main para corregir la página
pública de 5.1.5 sin promover los fixes de develop ni publicar una nueva versión.
Es una excepción documental a la ruta develop de la plantilla, no una release.

## Contratos y evidencia

- README/casos/inventario/rustdoc: capacidades básicas e interfaces de extensión PAdES
  coherentes; dos guards pasan. Mutación que restaura «2x faster than PDFSharp»
  hace fallar el nuevo guard. Enlaces relativos de documentos comprobados.
- Firmas: 35 pruebas de consumidor pasan (18 #526, 10 #540, 7 #606); tres
  interoperabilidades externas omitidas por su configuración habitual. Cinco
  doctests de firmas pasan (incluido el nuevo ejemplo ejecutado), dos ejemplos
  históricos ignorados. No se atribuye ejecución a las pruebas omitidas.
- AES-256: DocumentEncryption::handler selecciona aes_256_r5; no se afirma R6
  writer. Finalize: validate_cms_container solo comprueba la envoltura DER;
  se documenta expresamente que no valida SignedData, firma ni confianza.
- Certificados: los resultados exigen evidencia de revocación/trust apropiada;
  `Ok` no equivale a firma válida. Se conserva la implementación criptográfica.
- Monitorización: evidencia histórica sintética de 2026-09-27, identificada
  como tal; no presentada como nueva prueba ni como adopción en producción.
- Probe archivado: entradas artificiales, sin secretos/certificados reales ni
  contenido privado. Registro original conservado, no nueva ejecución atribuida.

## Hallazgos

No hallazgos pendientes en el cambio final. Durante la preparación se corrigió
la sobreafirmación de escritura R6 y se acotó el check DER de finalización;
no se modifican esas implementaciones. Los defectos conocidos #637–#640 se
mantienen explícitos y separados del cierre documental.

## Calidad de Tests (Kripteia)

Ejecución real con el script de la skill; log adjunto:
[2026-09-28-issue-603-kripteia.log](2026-09-28-issue-603-kripteia.log).
86/100 en 13 tests de dos archivos; guard documental 100/100 (dos tests).
Los avisos de bajo ratio y vecindario similar afectan a pruebas preexistentes
con fixtures explícitos. La afirmación automática «does not call any production
code» es incorrecta: invocan is_valid y otros métodos del resultado. No se
extrapola la puntuación a toda la biblioteca. El doctest no está contado por
el analizador y se ejecutó con cargo.

## Análisis de Seguridad (Kripteia Security)

Salida real: «No security issues found». Alcance: módulo de firmas y guard
copiados para análisis; no auditoría criptográfica completa. Inspección manual:
ningún cambio ejecutable de producto, secreto, FFI, autorización o persistencia.

## Métricas

- Archivos revisados: 14 (documentos, rustdoc, guard y evidencia).
- Hallazgos pendientes del cambio: 0.
- Archivos de producto modificados durante la revisión: 0.
- Verificación remota pendiente: CI del PR e integración; validación local
  descrita arriba no equivale a publicación ni cierre.
