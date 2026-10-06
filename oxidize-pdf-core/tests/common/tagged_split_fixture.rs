use std::collections::BTreeMap;

pub fn fixture_custom(
    missing_indexes: bool,
    bad_metadata: bool,
    corrupt_index: bool,
    customize: impl FnOnce(&mut BTreeMap<u32, Vec<u8>>),
) -> Vec<u8> {
    let mut objects: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
    objects.insert(1, format!("<< /Type /Catalog /Pages 2 0 R /Lang (en-US) /MarkInfo << /Marked true >> /StructTreeRoot 3 0 R {} >>", if bad_metadata {"/Metadata 9 0 R"} else {""}).into_bytes());
    objects.insert(
        2,
        format!(
            "<< /Type /Pages /Count 12 /Kids [{}] >>",
            (10..22)
                .map(|n| format!("{n} 0 R"))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .into_bytes(),
    );
    objects.insert(3, b"<< /Type /StructTreeRoot /K 4 0 R /ParentTree 6 0 R /IDTree 7 0 R /ParentTreeNextKey 12 >>".to_vec());
    objects.insert(
        4,
        b"<< /Type /StructElem /S /Document /P 3 0 R /K 5 0 R >>".to_vec(),
    );
    objects.insert(
        5,
        format!(
            "<< /Type /StructElem /S /Sect /P 4 0 R /Pg 10 0 R /K [{}] >>",
            (50..62)
                .map(|n| format!("{n} 0 R"))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .into_bytes(),
    );
    if !missing_indexes {
        objects.insert(
            6,
            if corrupt_index {
                b"not-a-pdf-object".to_vec()
            } else {
                format!(
                    "<< /Nums [{}] >>",
                    (0..12)
                        .map(|n| format!("{n} [{} 0 R]", 50 + n))
                        .collect::<Vec<_>>()
                        .join(" ")
                )
                .into_bytes()
            },
        );
        objects.insert(
            7,
            format!(
                "<< /Names [{}] >>",
                (0..12)
                    .map(|n| format!("(p{n:02}) {} 0 R", 50 + n))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .into_bytes(),
        );
    }
    objects.insert(
        8,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    );
    for n in 0..12 {
        objects.insert(10+n,format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Resources << /Font << /F1 8 0 R >> >> /Contents {} 0 R /StructParents {n} >>",30+n).into_bytes());
        let content = format!(
            "/P << /MCID 0 >> BDC BT /F1 12 Tf 20 200 Td (page {}) Tj ET EMC\n",
            n + 1
        );
        objects.insert(
            30 + n,
            format!(
                "<< /Length {} >>\nstream\n{content}endstream",
                content.len()
            )
            .into_bytes(),
        );
        objects.insert(
            50 + n,
            format!(
                "<< /Type /StructElem /S /P /P 5 0 R /Pg {} 0 R /K 0 /ID (p{n:02}) >>",
                10 + n
            )
            .into_bytes(),
        );
    }
    customize(&mut objects);
    let size = objects.keys().max().copied().unwrap() + 1;
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = BTreeMap::new();
    for (id, object) in objects {
        offsets.insert(id, bytes.len());
        bytes.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        bytes.extend(object);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for id in 1..size {
        let in_use = offsets.contains_key(&id)
            || (missing_indexes && (id == 6 || id == 7))
            || (bad_metadata && id == 9);
        bytes.extend_from_slice(
            format!(
                "{:010} 00000 {} \n",
                offsets.get(&id).copied().unwrap_or(0),
                if in_use { 'n' } else { 'f' }
            )
            .as_bytes(),
        );
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    bytes
}
