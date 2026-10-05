"""Minimal deterministic PDF assembly for independent binary reference fixtures."""


def stream(data, dictionary=""):
    return f"<< {dictionary} /Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream"


def assemble(objects):
    data = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n"
    offsets = [0]
    for index, obj in enumerate(objects, 1):
        offsets.append(len(data))
        data += f"{index} 0 obj\n".encode() + obj + b"\nendobj\n"
    start = len(data)
    data += f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode()
    data += b"".join(f"{offset:010} 00000 n \n".encode() for offset in offsets[1:])
    data += f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n".encode()
    return data


def cmap(kind, name, body, ros="/Registry (Contract) /Ordering (Synthetic) /Supplement 0"):
    return (f"/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n"
            f"/CIDSystemInfo << {ros} >> def /CMapName /{name} def /CMapType {kind} def\n"
            f"{body}\nendcmap CMapName currentdict /CMap defineresource pop end end").encode()
