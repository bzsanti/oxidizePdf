# #666 — Standard14 y equivalencia de APIs

Siete contratos Standard14 GREEN y seis de salidas GREEN. 224 PDFs:
14 métricas implícitas,14 overrides explícitos y196 pares de cambio/retorno.
Fuentes de verdad: AFM originales de Adobe, sin modificar, con licencia,
14 hashes y revisión del espejo0675784d24b28a55c607cad6b74596ce19ce333c.
El enlace original Adobe devolvió404; procedencia consta en provenance.json.

MuPDF1.26.10/Poppler24.02.0 confirman448 textos; MuPDF224 geometrías, qpdf224 PDFs,
sin excepciones ni avisos. Zapf usa ToUnicode explícito para aislar métricas de
la interpretación distinta aNN→Unicode de MuPDF; el probe previo se conserva.
La batería previa de codificaciones sigue cubriendo el mapeo Zapf implícito.
El primer probe usó Tr2, que produce dos trazas de dibujo (relleno y trazo);
los fixtures definitivos alternan0/1 sin alterar posiciones ni expectativas.

APIs de página/documento/extractor, por defecto y configuradas, con/sin layout:
Standard14, Type1/PFB/CFF, TrueType simbólica/no simbólica, CID-CFF y Type3;
texto exacto, concatenación de fragmentos, geometría finita y aislamiento de
/F1 entre páginas. CIDFontType2 y casos Form adicionales aún pendientes.

Regeneración226 archivos Standard14 y10 TJ/Tz idéntica. Tres mutaciones de
entrada detectadas (código mostrado, ancho explícito y selección de fuente),
tres controles pasan; originals/fixtures/producto intactos. Script reproducible
standard14-mutations.py y resultados contiguos. No es mutación de producto.

Kripteia Standard14:100/7; APIs98/6; seguridad sin alertas. Python scanner0tests:
la regeneración/comparación y revisión manual aportan evidencia independiente.
AFM se preserva como binary en .gitattributes para conservar hashes/CRLF.
Sin dependencias de producto nuevas. No se marca #666 completa ni se afirma CI.
