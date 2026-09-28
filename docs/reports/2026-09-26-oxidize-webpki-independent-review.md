# Revisión independiente de oxidize-webpki — 2026-09-26

Issue: #627 — https://github.com/bzsanti/oxidizePdf/issues/627
Base: b8e8f173c59e02a12fe1db0da051fe1d20950c20.
Revisor: agente independiente review_webpki, autorizado por el usuario, solo lectura.

## Resumen ejecutivo

Aprobación técnica de la extracción local. Sin defectos nuevos ni pérdida de
algoritmos, vectores o validaciones. Lógica criptográfica idéntica; consumidor
usa la misma lista explícita para cadenas y revocación. Paquete con manifiesto
propio, licencia, documentación y fixtures. Verificados 24 archivos de fixtures
contra base y correspondencia de código, README, licencia y vectores con el
artefacto empaquetado. Manifiesto normalizado sin dependencias de path, PDF,
TLS ni WebPKI.

## Hallazgos

Sin hallazgos nuevos verificables. Los dos defectos del informe del autor
(cfg desplazado y conversión CRLF) están corregidos. Publicación, integración
y ejecución nativa remota Windows/macOS pendientes, no acreditadas por esta
revisión. Límites previos de Python, configuración sin compression y spin
retirado no son regresiones de la extracción.

## Calidad de Tests (Kripteia)

Ejecutado independientemente: Overall Score: 98 | Tests: 5 | Files: 1.
rsa_key_size_bounds_preserve_8192_bit_support: 90; otros cuatro: 100.
Unwrap corresponde a construcción/lectura de fixtures y falla ante errores;
el ratio de aserciones no refleja seis casos RSA.

Tests cubren 11 combinaciones primitivas y tres identificadores RSA adicionales.
Aceptar mensajes manipulados, ignorar firmas, admitir claves equivocadas o
eliminar identificadores rompe las aserciones; sin nueva campaña de mutaciones.
Repetidos: cinco tests + doctest con toolchain actual y 1.88, cinco tests +
doctest desde paquete extraído, 27 tests de consumidor, formato y Clippy del
proveedor con warnings denegados. Todo pasa.

## Análisis de Seguridad (Kripteia Security)

Salida independiente: No security issues found.
Sin unsafe/FFI propio, secretos ni operaciones con claves privadas. Conserva
límites RSA antes de convertir enteros, DER, rechazos, Ed25519 estricto y ausencia
de fallback exitoso. No es una auditoría criptográfica de las dependencias.

## Métricas

- Archivos revisados: 40 entradas, incluidos 24 archivos de fixtures.
- Hallazgos nuevos: 0.
- Archivos modificados por el revisor: 0.
