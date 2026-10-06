Vínculo de seguimiento con #666 (baterías de interpretación de fuentes, Unicode, texto y espaciado).

Al implementar la recuperación de #677, conservar un fixture sintético reutilizable que permita comprobar en #666 que el split mantiene el texto extraído y ActualText cuando exista, por página y en el orden solicitado. Incluir ActualText explícitamente en el fixture compartido si el caso mínimo actual no lo contiene.

#677 sigue siendo responsable de reconstrucción estructural segura, parent mappings, referencias, rechazo de ambigüedad y ausencia de salidas parciales. #666 cubre la conservación observable del texto/ActualText; no absorbe la reparación estructural ni queda bloqueada por #677 salvo que se detecte un fallo de extracción. Relación principal de este defecto: #621.
