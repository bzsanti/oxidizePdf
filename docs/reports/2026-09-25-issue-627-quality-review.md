# Quality review — issue #627

## Resumen ejecutivo

Revisión final del cambio sobre develop `410358d8`. WebPKI conserva cadenas,
anclas, vigencia, restricciones, uso de clave y revocación; un adaptador privado
sustituye ring por primitivas RustCrypto/Dalek. No hay nuevas APIs públicas ni
criptografía aritmética propia. El conjunto efectivo de algoritmos se conserva,
con RSA de 2048–8192 bits y PSS de salt/digest acoplados; Ed25519 usa verificación
estricta. Los dos puntos de invocación de WebPKI usan la lista explícita del
adaptador, incluso si dependencias de desarrollo activan webpki/ring por separado.

QR del core completado; #627 sigue abierta para integración/publicación y pin
final del binding. El wheel Linux y el sdist reconstruido prueban el candidato
con un override de path, no los paquetes publicados. La CI multiplataforma
está preparada pero no se ha ejecutado remotamente en esta revisión.

## Hallazgos

1. **Vectores criptográficos susceptibles a conversión de EOL — corregido**:
   `oxidize-pdf-core/tests/fixtures/signatures/provider_627/.gitattributes:2`.
   Algunas firmas aleatorias carecen de NUL y Git las detectaba como texto;
   una conversión CRLF podía invalidarlas en Windows. Se marcan `.sig`, `.spki`
   y `message.bin` como binarios. El índice coincide byte a byte con los 24
   archivos de fixtures y el checkout con `core.autocrlf=true` conserva todos
   los vectores criptográficos.
2. **El wrapper Python confunde ejecución correcta con firma válida — externo,
   pendiente de issue**: `oxidize-python/src/parser.rs:1913`, commit `81a74e6b`.
   `.is_ok()` se asigna a `valid`, ignorando los booleanos del resultado.
   `signed_rsa_altered.pdf` devuelve `valid=True` tanto en el entorno previo
   como con el wheel candidato. Solución: definir el contrato del resultado
   público y propagar integridad, firma y certificados, con controles válidos,
   alterados y no confiables. Las únicas issues abiertas del binding consultadas
   son #198 y #150, ajenas al defecto; se registra bloqueado en TASKS.md,
   responsable bzsanti. No se modifica el wrapper bajo esta corrección del core.

3. **La configuración sin compression no compila como dependencia real —
   preexistente, pendiente de issue**: `oxidize-pdf-core/src/compression.rs:7`.
   `flate2` se declara opcional pero existen imports incondicionales y una
   llamada sin cfg a `try_standard_zlib_decode`. El consumidor aislado con
   solo signatures falla con 19 errores; los tests internos pueden ocultar
   el problema al habilitar flate2 como dev-dependency. La matriz mínima
   compilable mantiene `compression,signatures`. Corregir el contrato de la
   feature y probarlo desde consumidor externo requiere una issue específica;
   está registrado bloqueado en TASKS.md. #627 no modifica esos imports.

## Calidad de Tests (Kripteia)

Salida real, módulo de firmas:

```text
Overall Score: 91 | Tests: 127 | Files: 10
crypto_provider_tests.rs: Tests 5, Score 98
```

Salida real, alcance focalizado (proveedor, certificado y nueva integración):

```text
Overall Score: 93 | Tests: 22 | Files: 3
certificate.rs: Tests 13, Score 89
crypto_provider_tests.rs: Tests 5, Score 98
issue_627_certificate_provider_test.rs: Tests 4, Score 100
```

Peores resultados: tests preexistentes de agregación de estado de certificados
(expirado/no confiable), 70. El aviso de «no production code» es falso positivo:
llaman al método contractual `is_valid`. No se usan esos tests como prueba de
criptografía. La cobertura nueva usa once vectores OpenSSL, claves incorrectas
válidas, mensajes/firmas alterados, DER malformado, parámetros RSA ausentes,
límites 2047/2048 y 8192/8193, salt PSS incorrecto e identidad Ed25519.
Las cuatro regresiones de integración ejercitan la API pública con CRL válida,
CRL alterada, certificado alterado, revocación y entradas malformadas.

Kripteia Python informa `Tests: 0 | Files: 0` para unittest; su score 100 no se
interpreta como cobertura. Se ejecutaron realmente 11 tests del gate y 3 del
binding instalado, estos últimos también tras reconstruir el sdist. Una
mutación que devuelve lista vacía desde el gate provoca cinco fallos y cuatro
errores en los diez tests existentes al ejecutar la mutación; no sobrevive.

Validación: 6.821 tests de biblioteca pasan (3 ignorados); 27 de cadenas/firmas,
3 de interoperabilidad externa y Clippy all-targets pasan. Formato y diff check
pasan. El gate pasó en 20 combinaciones de features/targets; la compilación
máxima del producto con CC/CXX=false pasa en Linux. Un consumidor aislado, sin dev-dependencies y con CC/CXX=false, acepta
la cadena válida y rechaza la revocada; CI incorpora esa prueba. Es inventario de grafo para
los demás targets, no ejecución local de binarios Windows/macOS/aarch64.

## Análisis de Seguridad (Kripteia Security)

Salida real en Rust y Python, tanto alcance completo como focalizado:

```text
=== Kripteia Security Analysis ===
No security issues found.
```

Revisión manual: sin unsafe/FFI nuevos en el adaptador; DER se decodifica por
bibliotecas mantenidas, tamaño RSA acotado antes de BigUint, longitud de firma
acotada, PSS con parámetros exactos y Ed25519 estricto. Todos los errores de
verificación se propagan como InvalidSignature; no hay fallback de éxito.
El gate rechaza también crates desconocidos por fuentes/archivos nativos y
`links`, no solo nombres ring/cc. Las exclusiones de datos C son por hash;
los marcadores Rayon/PyO3 son por versión y valor exactos. Se revisaron las
interfaces de plataforma y los import archives de Windows por separado.

La base RustSec consultada mantiene RUSTSEC-2023-0071 para `rsa` 0.9.10 (fuga
por tiempos de operaciones privadas). Este adaptador solo verifica con datos
públicos; no contiene operación con clave privada. No se presenta la dependencia
como libre de avisos. Ed25519-Dalek 2.2.0 y Curve25519-Dalek 4.1.3 incluyen las
correcciones de RUSTSEC-2022-0093 y RUSTSEC-2024-0344. Fuentes primarias y límites
en `docs/architecture/no-native-dependencies.md`.

El resultado automático no detectó el defecto externo del wrapper; prevalece
la reproducción manual del hallazgo 2. No se afirma una auditoría formal de
las primitivas ni cumplimiento de los wheels ya publicados.

## Métricas

- Archivos revisados del cambio: 42, incluidos 24 archivos de fixtures.
- Hallazgos totales: 3 (uno corregido, dos preexistentes bloqueados).
- Archivos modificados durante la pasada final de revisión: 0.
- Sin cambios de corpus, benchmarks o baselines.
- Evidencia y hashes: `2026-09-25-issue-627-validation.json`.
