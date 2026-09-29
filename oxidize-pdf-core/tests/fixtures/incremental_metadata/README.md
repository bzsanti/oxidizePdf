# Incremental metadata fixtures

`python3 generate.py` writes independent, hand-assembled PDFs with two pages,
Helvetica text, inherited resources/MediaBox, catalog language, XMP and custom
Info. `nested.pdf` adds an intermediate Pages node; `generation.pdf` uses Info
object 9 with generation 7. `rich.pdf` adds nonzero page origins, inherited
CropBox/Rotate, binary custom values, an indirect annotations array and a page
destination. `maxsize.pdf` exhausts the object allocator for a negative test.

Derived fixtures (qpdf 11.9.0; encryption uses a random salt/IV):

```sh
qpdf --object-streams=generate flat.pdf compressed.pdf
qpdf --encrypt '' owner 256 -- flat.pdf encrypted.pdf
```

These are generated test data, not customer documents.
