# Revisión de #640 — contención del cifrado por destinatarios

## Resumen ejecutivo

Base `089a92776dc52c6091767edd03caf3668f9a4cd6`, rama
`fix/issue-640-public-key-containment`, cambios locales de #640. Revisión del
autor, sin delegación. Rust 2021, MSRV 1.88; sin dependencias ni features nuevas.
Se elimina la simulación y se conservan las firmas públicas. La disponibilidad
cambia intencionalmente: todas las operaciones criptográficas del handler
rechazan con `PdfError::EncryptionError`; permisos siempre denegados.

La revisión y la validación local están completadas: workspace 9829 pasan /
73 omitidos (incluidos 217 doctests aprobados), Clippy all-targets -D warnings,
formato y build pasan. Con signatures pasan 41 pruebas focalizadas. Este informe
no acredita integración ni publicación. Hashes y resultados ejecutados están
en el JSON de validación homónimo.

## Contratos y evidencia

| Contrato y fuente | Productores/consumidores, contraejemplo y evidencia | Estado |
|---|---|---|
| #640: no crear datos que aparenten proteger una semilla | `add_recipient`, campos públicos y helpers de diccionarios. Certificado falso de 100 bytes devolvía `Ok(())`; RED reproduce el fallo. GREEN rechaza certificados inválidos/reales, conserva estado previo y no genera semilla. | Demostrado; las 16 regresiones finales pasan con y sin signatures. |
| #640: no liberar semillas con claves ajenas | `decrypt_seed`: RED devuelve veinte bytes `0xAA` con clave `a`; GREEN rechaza claves `a`, `b` y vacía, receptores truncados y antiguos. | Demostrado. |
| Ninguna entrada criptográfica alternativa puede retornar éxito | Las ocho entradas `SecurityHandler` mediante `Box<dyn SecurityHandler>`, métodos V2/AESV2/AESV3/None y despacho de `CryptFilterManager`. RED falla en las ocho. Los casos AES explícitos usan ciphertext CBC válido bajo el antiguo IV, por lo que no confunden rechazo de padding con contención. GREEN pasa. | Demostrado. |
| Metadatos no equivalen a autorización | Destinatario inyectado manualmente con todos los permisos: `verify_permission` devuelve true en RED y false en GREEN. Índices ausentes también denegados. | Demostrado. |
| Compatibilidad y límites de extensión | Constructores, campos, conversiones SubFilter y helpers de diccionarios mantienen firmas. Se documenta que copian metadatos sin validar ni cifrar. No existe método de derivación de clave de archivo en este tipo/trait; se retira la derivación local de claves RC4 y todos los IV deterministas. | Inspeccionado; regresiones de metadatos pasan. |
| Contraseña/firmas independientes | `StandardSecurityHandler`, `DocumentEncryption`, parser y exports inspeccionados: no consumen `PublicKeySecurityHandler`. El parser rechaza filtros distintos de Standard. Control RC4 por trait pasa antes y después. Identity es explícitamente sin cifrado y no invoca handlers. | Control y suites de contraseña pasan. Con signatures: 18 regresiones CMS/certificados y siete de apariencia pasan. |

Se revisaron los campos mutables y una configuración con `seed_length = usize::MAX`:
ninguna operación criptográfica lee la longitud ni reserva memoria a partir de ella.
No existe fallback criptográfico, generación de semilla, reloj, RNG, IV, primitiva
ni acceso al certificado/clave en los métodos contenidos. Los campos y helpers
restantes son metadatos sin autenticación; la documentación de migración prohíbe
interpretarlos como protección o silenciar el error escribiendo un PDF sin cifrar.

Los antiguos tests que esperaban éxito simulado se sustituyeron por regresiones
de rechazo por API pública. Las tres pruebas de conversiones/configuración se
conservan. No se alteró el handler estándar, la verificación de firmas, corpus,
baselines ni PAdES. La implementación interoperable corresponde a #642.

## Hallazgos

Sin hallazgos confirmados pendientes dentro del alcance de contención revisado.
La conservación de serializadores crudos es una decisión de compatibilidad
documentada, no una afirmación de interoperabilidad del antiguo formato.
La revisión no audita ni certifica otras primitivas criptográficas.

## Calidad de Tests (Kripteia)

Script de la skill ejecutado sobre `public_key.rs`: 100/100, tres tests, un
archivo. Sobre `issue_640_public_key_containment_test.rs`: 96/100, 16 tests, un
archivo. Los avisos de proporción aserción/preparación corresponden a matrices
y al helper `assert_unsupported`, que comprueba variante y mensaje y falla ante
`Ok`. RED acredita discriminación real. Los valores constantes son oráculos
independientes de contrato; `unwrap()` en preparación falla mostrando el error,
no lo descarta silenciosamente como sugiere el texto heurístico de la herramienta.

Salida íntegra: JSON de validación y logs `/tmp/issue-640-kripteia-{handler,tests}.log`.

## Análisis de Seguridad (Kripteia Security)

Ambas ejecuciones completadas: `No security issues found.` Sin secretos,
unsafe/FFI ni funciones peligrosas señalados en estos dos archivos. Revisión
manual: límites de confianza, configuración mutable, permisos, errores,
mutaciones parciales, entrada por trait/filtros y ausencia de semillas/IV.
Un análisis estático limpio no demuestra seguridad general del producto.

## Métricas

- Archivos revisados: 15 (handler, mod, test nuevo, trait/filtros, handler estándar,
  object_encryption, parser/encryption_handler, error, manifiesto del core,
  README, CLAIMS, migración, CHANGELOG y test de claims).
- Hallazgos totales pendientes: 0 en el alcance revisado.
- Archivos de producto modificados durante la fase de revisión: 0; la implementación
  y la aclaración de rustdoc preceden al cierre de esta revisión.
- Verificaciones pendientes: ninguna local en este alcance. CI multiplataforma e integración remota son pasos posteriores; no se acreditan aquí.
