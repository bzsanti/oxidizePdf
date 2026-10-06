Vínculo de seguimiento con #677 (recuperación de tagged split sin /StructParents, relacionada con #621).

La reconstrucción de estructura etiquetada, parent mappings, referencias y publicación atómica corresponde a #677. La intersección con #666 es una regresión compartida: comprobar que dividir el PDF conserva el texto extraído y ActualText cuando exista, con resultados por página y orden verificables.

Tras disponer del fixture/corrección de #677, reutilizarlo desde los contratos de #666 para esa conservación. Esto no amplía #666 a toda la reparación estructural ni hace de #677 un bloqueo de su cierre, salvo que aparezca un fallo en los contratos de extracción. Mantener ambas issues vinculadas al implementar esa prueba.
