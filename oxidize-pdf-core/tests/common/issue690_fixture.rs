use std::collections::BTreeMap;

fn encode_tagged_fixture_with_absent_references(
    objects: BTreeMap<u32, Vec<u8>>,
    missing_indexes: bool,
    bad_metadata: bool,
    additional_absent: &[u32],
) -> Vec<u8> {
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
            || (bad_metadata && id == 9)
            || additional_absent.contains(&id);
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

pub fn fixture(flat_tree: bool, language: bool, descriptions: bool) -> Vec<u8> {
    fixture_with(flat_tree, language, descriptions, |_| {})
}

pub fn fixture_with(
    flat_tree: bool,
    language: bool,
    descriptions: bool,
    customize: impl FnOnce(&mut BTreeMap<u32, Vec<u8>>),
) -> Vec<u8> {
    let shape: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/issue690-shape.json")).unwrap();
    let mut objects = BTreeMap::new();
    objects.insert(1, format!("<< /Type /Catalog /Version /1.4 /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 3 0 R {} >>", if language { "/Lang (en-US)" } else { "" }).into_bytes());
    let leaves = (10..22).map(|id| format!("{id} 0 R")).collect::<Vec<_>>();
    objects.insert(
        2,
        format!(
            "<< /Type /Pages /Count 12 /Kids [{}] >>",
            if flat_tree {
                leaves.join(" ")
            } else {
                "22 0 R 23 0 R".to_string()
            }
        )
        .into_bytes(),
    );
    if !flat_tree {
        objects.insert(
            22,
            format!(
                "<< /Type /Pages /Parent 2 0 R /Count 8 /Kids [{}] >>",
                leaves[..8].join(" ")
            )
            .into_bytes(),
        );
        objects.insert(
            23,
            format!(
                "<< /Type /Pages /Parent 2 0 R /Count 4 /Kids [{}] >>",
                leaves[8..].join(" ")
            )
            .into_bytes(),
        );
    }
    objects.insert(
        3,
        b"<< /Type /StructTreeRoot /K 4 0 R /ParentTree 6 0 R /IDTree 7 0 R >>".to_vec(),
    );
    objects.insert(
        8,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    );
    let mut structure_children = Vec::new();
    for (page_index, items) in shape["pages"].as_array().unwrap().iter().enumerate() {
        let page_id = 10 + page_index;
        let parent = if flat_tree {
            2
        } else if page_index < 8 {
            22
        } else {
            23
        };
        let mut content = String::new();
        for (row, item) in items.as_array().unwrap().iter().enumerate() {
            let mcid = item[0].as_u64().unwrap();
            let role = item[1].as_str().unwrap();
            let has_description = item[2].as_bool().unwrap();
            let structure_id = 1000 + mcid as u32;
            structure_children.push(format!("{structure_id} 0 R"));
            objects.insert(
                structure_id,
                format!(
                    "<< /Type /StructElem /S /{role} /P 4 0 R /Pg {page_id} 0 R /K {mcid} {} >>",
                    if role == "Figure" && (has_description || descriptions) {
                        "/Alt (Synthetic description)"
                    } else {
                        ""
                    }
                )
                .into_bytes(),
            );
            content.push_str(&format!("/{role} << /MCID {mcid} >> BDC BT /F1 2 Tf 10 {} Td (synthetic item {mcid}) Tj ET EMC\n", 290.0 - row as f64 * 2.6));
        }
        let stream_id = 30 + page_index;
        objects.insert(page_id as u32, format!("<< /Type /Page /Parent {parent} 0 R /MediaBox [0 0 300 300] /Resources << /Font << /F1 8 0 R >> >> /Contents {stream_id} 0 R >>").into_bytes());
        objects.insert(
            stream_id as u32,
            format!(
                "<< /Length {} >>\nstream\n{content}endstream",
                content.len()
            )
            .into_bytes(),
        );
    }
    objects.insert(
        4,
        format!(
            "<< /Type /StructElem /S /Document /P 3 0 R /K [{}] >>",
            structure_children.join(" ")
        )
        .into_bytes(),
    );
    customize(&mut objects);
    let mut bytes =
        encode_tagged_fixture_with_absent_references(objects, true, false, &[9, 26, 27, 28]);
    bytes[..8].copy_from_slice(b"%PDF-1.3");
    bytes
}

pub fn matrix(
    depth: usize,
    indexes: bool,
    keys: bool,
    language: bool,
    descriptions: bool,
) -> Vec<u8> {
    fixture_with(depth == 0, language, descriptions, |objects| {
        if depth == 2 {
            objects.insert(
                2,
                b"<< /Type /Pages /Count 12 /Kids [24 0 R 25 0 R] >>".to_vec(),
            );
            for (node, parent, count) in [(22, 24, 8), (23, 25, 4)] {
                let value = String::from_utf8(objects[&node].clone())
                    .unwrap()
                    .replace("/Parent 2 0 R", &format!("/Parent {parent} 0 R"));
                objects.insert(node, value.into_bytes());
                objects.insert(
                    parent,
                    format!("<< /Type /Pages /Parent 2 0 R /Count {count} /Kids [{node} 0 R] >>")
                        .into_bytes(),
                );
            }
        }
        let shape: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/issue690-shape.json")).unwrap();
        let mut nums = String::new();
        for (page, items) in shape["pages"].as_array().unwrap().iter().enumerate() {
            if keys {
                let id = 10 + page as u32;
                let value = String::from_utf8(objects[&id].clone()).unwrap().replacen(
                    "<<",
                    &format!("<< /StructParents {page}"),
                    1,
                );
                objects.insert(id, value.into_bytes());
            }
            let items = items.as_array().unwrap();
            let last = items.last().unwrap()[0].as_u64().unwrap() as usize;
            let mut owners = vec!["null".to_string(); last + 1];
            for item in items {
                let id = item[0].as_u64().unwrap() as usize;
                owners[id] = format!("{} 0 R", 1000 + id);
            }
            nums.push_str(&format!("{page} [{}] ", owners.join(" ")));
        }
        if indexes {
            objects.insert(6, format!("<< /Nums [{nums}] >>").into_bytes());
            objects.insert(7, b"<< /Names [] >>".to_vec());
        }
        // Inline ActualText is observable even while the original index is absent.
        let object = String::from_utf8(objects[&30].clone()).unwrap();
        let content = object
            .split_once("stream\n")
            .unwrap()
            .1
            .strip_suffix("endstream")
            .unwrap();
        let content = content.replacen(
            "/MCID 1 >>",
            "/MCID 1 /ActualText (source replacement) >>",
            1,
        );
        objects.insert(
            30,
            format!(
                "<< /Length {} >>\nstream\n{content}endstream",
                content.len()
            )
            .into_bytes(),
        );
    })
}
