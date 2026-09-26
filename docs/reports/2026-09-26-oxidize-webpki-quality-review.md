# Revisión de extracción oxidize-webpki — 2026-09-26

Issue: #627 — https://github.com/bzsanti/oxidizePdf/issues/627
Base: b8e8f173c59e02a12fe1db0da051fe1d20950c20; cambio local sin publicar.

## Resumen ejecutivo

Adaptador extraído a un crate con versión, licencia MIT, MSRV 1.88,
documentación y vectores propios. No depende de PDF, TLS ni WebPKI: implementa
el trait público de rustls-pki-types mediante primitivas RustCrypto existentes.
El consumidor pasa la misma lista explícita a WebPKI para cadenas y CRLs.
Se preservaron byte por byte los 24 archivos de fixtures. No cambió la lógica
criptográfica ni se añadieron versiones de dependencias al lock.

Revisión manual del autor y análisis automatizado completados. La revisión
independiente solicitada por el relevo fue completada posteriormente y aprobó
la extracción sin hallazgos nuevos; véase el informe independent-review.md.

## Hallazgos

1. **Atributo cfg desplazado durante la extracción, corregido**:
   `oxidize-pdf-core/src/signatures/mod.rs:31` — al retirar el módulo se había
   conservado su atributo cfg, que pasaba a ocultar detection sin signatures.
   Se retiró también el atributo. El diff final elimina ambas líneas y
   `cargo check --locked --offline -p oxidize-pdf` pasa con features por defecto.
2. **Conversión de bytes firmados en Windows, corregida**:
   `.gitattributes:24` — cms_content.bin se convertía LF→CRLF. Un checkout con
   core.autocrlf=true y la misma mutación en el fixture reprodujeron exactamente
   los dos fallos de hash_valid de CI (16 pasan, 2 fallan). Los atributos binary
   preservan ahora el mensaje y encodings criptográficos. Checkout con CRLF
   verificado idéntico; las 18 pruebas de #526 vuelven a pasar. No se afirma una
   ejecución nativa Windows posterior al cambio.

## Calidad de Tests (Kripteia)

Salida real: Overall Score: 98 | Tests: 5 | Files: 1.
Peor resultado: rsa_key_size_bounds_preserve_8192_bit_support, 90/100; ratio
assert/setup 18%. Los casos comprueban explícitamente aceptación/rechazo para
1024, 2047, 2048, 4096, 8192 y 8193 bits. Los unwrap son construcción/lectura
de fixtures y fallan el test; no silencian errores del producto. Resto: 100/100.

Los 11 algoritmos primitivos verifican firmas OpenSSL y rechazan mensaje,
firma o clave incorrectos; se comprueban los tres identificadores RSA sin
parámetros, clave Ed25519 débil y política de sal PSS. Eliminar verificación o
admitir cualquier mensaje haría fallar las aserciones negativas; no se realizó
una nueva campaña general de mutaciones. El cambio de bytes Windows sí tuvo
RED/GREEN ejecutado. Logs íntegros en el JSON de validación.

## Análisis de Seguridad (Kripteia Security)

Salida real: No security issues found.

Inspección manual: fuente nueva prohíbe unsafe; no FFI ni operaciones con claves
privadas. DER y tamaños se validan antes de convertir enteros RSA, errores
retornan InvalidSignature, sin fallback exitoso. Las dependencias conservan
sus límites y avisos previamente registrados (incluido rsa para operaciones
privadas; no se presenta como libre de avisos). El empaquetado advierte spin
0.9.8 retirado, hallazgo previo bloqueado en TASKS.md, sin actualizarlo aquí.

## Validación

- Proveedor: cinco pruebas y doctest pasan, también Rust 1.88.
- Paquete autónomo: cargo package verifica compilación; tests desde el paquete
  extraído pasan y no requieren archivos del consumidor.
- Consumidor: 27 tests de firmas/certificados; consumidor externo acepta cadena
  confiable y rechaza revocada sin dev-dependencies ni compiladores C/C++.
- Gate: 11 pruebas de política; grafos proveedor y producto en cinco targets;
  builds Linux de proveedor y producto máximo con CC/CXX=false.
- Formato y Clippy del crate y consumidor pasan.
- Python: wheel directo y desde sdist compilados con CC/CXX=false; cuatro
  smoke tests por wheel. El sdist incluye ambos crates. Solo snapshot con path;
  no acredita versiones publicadas ni corrige el booleano preexistente.

## Métricas

- Archivos fuente revisados: 5 (lib/tests del crate, certificate/mod del core,
  manifiesto del crate), además de manifiestos, lock, CI, atributos y documentos.
- Hallazgos totales: 2, ambos corregidos antes del informe.
- Cambios durante el análisis Kripteia: 0.
