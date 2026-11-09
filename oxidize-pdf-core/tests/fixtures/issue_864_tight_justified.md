# Issue 864: tightly justified text

Source: https://github.com/bzsanti/oxidizePdf/issues/864, supplied by TensorQuay-Dev.
Original base64-decoded XeLaTeX PDF, unmodified. The issue includes the LaTeX source: one paragraph in Noto Serif, generated with XeTeX/xdvipdfmx (TeX Live 2026).
SHA256: c486b9c53eba36d4adb20c392d601a2d015a3913a201155d3466068e78ea207c.

Its embedded font subset has no space glyph. The first line uses TJ advances of 179/180 milli-em between words; subsequent lines use wider gaps. Poppler 24.02.0 separates every word, while the previous default TJ threshold of 200 milli-em joined the first line. This fixture tests actual embedded-font decoding and geometry rather than only a synthetic Helvetica font.
