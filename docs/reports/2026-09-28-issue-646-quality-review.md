# Signature preparation — revisión del autor (#646)

## Resumen ejecutivo

Integración de `feature/signature-preparation` (12359b7) con develop 93e6796.
Rust 2021/MSRV 1.88, biblioteca con compresión y feature opcional signatures.
Alcance: preparación incremental de campos por participantes, apariencias,
políticas DocMDP/FieldMDP, interfaz con firma CMS y documentación de release.
Los cambios del merge de develop no son funcionalidad nueva de esta rama.
QR del autor cerrado antes de publicar PR: revisión manual, Kripteia,
regresiones y workspace/corpus pasan. CI remota e integración aún pendientes.

## Contratos y evidencia

| Contrato | Productor/consumidor y comprobación | Estado |
| --- | --- | --- |
| Las revisiones conservan los bytes originales | create/draw/remove → parser; prefijo exacto y reapertura | Demostrado |
| Completar manuscrito no equivale a firma CMS | `/AP`, `/OxidizeCompleted`, `/Ff` y ausencia de `/V`; verify_signatures vacío | Demostrado |
| Preparación combinada conserva la apariencia | draw(false) → SignaturePreparationOptions::existing → finalize | Demostrado estructuralmente; CMS sintético no acredita identidad ni validez criptográfica |
| Listar/eliminar no afecta al otro participante | Dos slots, eliminar Alice, Bob conserva nombre/metadata | Demostrado |
| Geometría válida y límites coherentes | Rectángulo fuera de página, rotación 45°, 101.º campo | RED tres fallos/6 controles; GREEN diez pruebas, incluido control de flags |
| Dibujo rotado y etiqueta escapada | Cuatro rotaciones; contenido real de AP/N; paréntesis y barra inversa escapados | Demostrado |
| Respetar DocMDP/FieldMDP | Crear/eliminar son FormStructure; completar es FormFill; P=1/2/3 y bloqueo Include | P=1/2/3 pasan; retirar DocMDP o FieldMDP por separado hace fallar la aserción de rechazo correspondiente |
| API de consumidor real | Dependencia compression-only, sin dev-dependencies | Ejecutable compression-only pasa; qpdf acepta prepared/drawn/removed; Poppler renderiza trazo y etiqueta |

No se usan mocks para sustituir writer/parser/permisos. Los fixtures geométricos
se ensamblan de forma independiente. Las transformaciones esperadas son constantes
PDF escritas en el test, no el resultado de llamar al algoritmo productivo.
Las pruebas de CMS reales existentes complementan el control estructural del
nuevo flujo; la apariencia manuscrita no se presenta como validación de identidad.

## Hallazgos

1. **Creación excedía el límite de lectura (corregido)**:
   `oxidize-pdf-core/src/signatures/preparation.rs:60` — crear el campo 101
   producía un documento que list_signature_slots rechazaba. Se comprueba el
   límite de 100 antes de crear. Fixture de 100 campos: listado válido y nueva
   creación rechazada. RED/GREEN documentado.
2. **Rotación aceptada al crear e ilegible después (corregido)**:
   `oxidize-pdf-core/src/signatures/preparation.rs:69` — una página a 45° permitía
   preparar un campo que read_signature_slot rechazaba. Se valida la rotación
   soportada antes de escribir. RED específico y GREEN.
3. **Lectura omitía el límite de página (corregido)**:
   `oxidize-pdf-core/src/signatures/preparation.rs:175` — un campo externo con
   rectángulo finito pero fuera de página pasaba lectura y podía dibujarse.
   Se aplica el mismo límite crop/media que en create. Fixture independiente
   con borde derecho 9000 en una página de ancho 600 reproduce el defecto.

4. **Completar borraba otros flags del campo (corregido)**:
   `oxidize-pdf-core/src/signatures/preparation.rs:480` — asignar Ff=1 eliminaba
   NoExport y otros bits preexistentes. Se añade ReadOnly con OR, conservando
   los flags originales. RED devuelve 1 frente a 5; GREEN comprueba NoExport
   más ReadOnly en el diccionario reabierto.

Se mantiene la semántica pública de digitally_signed como presencia de /V,
con advertencia documental explícita: no verifica CMS. No hay promesas nuevas
sobre PAdES, identidad, consentimiento o gestión de claves. Las dos correcciones
de ejemplos que ya estaban en la rama se conservan y revisan: import de trait y
aserciones sobre valores reales en lugar de formato Debug.

## Calidad de Tests (Kripteia)

Fuente signatures: **91/100**, 122 tests, nueve archivos detectados.
Nuevas regresiones: **99/100**, diez tests. Integración de firma/permisos:
**96/100**, 14 tests (tres de interoperabilidad omitidos por defecto).
El fichero preparation.rs no contiene unitarias internas; sus tests viven en
la integración pública. El primer escaneo aislado de ese fichero detectó cero
pruebas; no se utiliza ese 100/100 como evidencia de cobertura.
Avisos sobre constantes esperadas y unwrap de tests no invalidan estos oráculos:
son resultados independientes y un unwrap fallido hace fallar la prueba.
Las puntuaciones inferiores de tests CMS/error existentes no son hallazgos
nuevos de esta feature y no se corrigen durante esta integración.

## Análisis de Seguridad (Kripteia Security)

Los tres escaneos (módulo de firmas y ambos ficheros de integración) informan
**No security issues found**. Inspección manual adicional: límites de metadata,
rectángulos finitos, puntos normalizados, tamaño de strokes, escaping WinAnsi,
campos read-only/completed, FieldMDP y DocMDP antes de escribir revisiones.
No se añade unsafe, FFI, dependencia, red ni operación con claves privadas.
Las mutaciones de guard se realizan en una copia desechable, no en el árbol
revisado. Un escaneo limpio no sustituye esos controles contractuales.

## Métricas

- Archivos del cambio revisados: 11, más consumidores/shared helpers de firmas,
  IncrementalUpdate y parser de campos consultados.
- Hallazgos: cuatro; cuatro corregidos bajo la tarea de integración #646.
- Archivos de producto corregidos tras revisión: uno (preparation.rs).
- Workspace final con signatures y corpus exigido: 9895 pasan, cero fallos,
  72 omitidos. Gates de fusión (34) y orden (38) pasan; corpus T0–T6
  disponible y ejecutado, sin recalibración de líneas base.
- Focalizadas: 46 pasan, tres interop omitidas por defecto. Ejecutadas después
  explícitamente: 3/3 pasan con qpdf/OpenSSL/Poppler. Default features: 10/10.
  Consumidor externo compression-only pasa sobre fuente final.
- Formato, diff check y Clippy workspace/all-targets con signatures pasan.
- Mutaciones DocMDP/FieldMDP: un fallo esperado cada una; tests rechazan crear
  bajo certificación y completar un campo bloqueado, respectivamente. Ejecutadas
  antes del ajuste de flags; controles de autorización sin cambios, GREEN final pasa.
- Pendiente: CI remota Windows/macOS, integración y futura release. No se afirma
  validación nueva en Studio ni en bindings; esta revisión cubre la API Rust.
