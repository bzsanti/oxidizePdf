//! Project a tagged structure onto retained pages, rebuilding its derived indexes.
//!
//! Page and structure object identities stay stable in the incremental revision.
//! /K is the authority for content ownership; /ParentTree and /IDTree are rebuilt
//! from the retained associations. Only unreadable, exact-generation index-root
//! references at offset zero are recoverable, and only on those two root edges.
//! Other uses of the same malformed reference still fail the source graph walk.
//! Missing page /StructParents keys are assigned after ownership traversal,
//! avoiding existing page and OBJR keys. Present but invalid keys are rejected.
//! The materialized projection must still pass tagged-content validation.
//! No content stream or accessibility metadata on retained elements is discarded.
//! External-stream MCRs and deletion combined with page cloning/import require
//! additional ownership remapping and are explicitly rejected, not flattened.
use crate::error::PdfError;
use crate::parser::{
    objects::{PdfArray, PdfDictionary, PdfObject, PdfString},
    PdfReader,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Read, Seek};
type Id = (u32, u16);
const LIMIT: usize = 100_000;
const MAX_DEPTH: usize = 256;

#[derive(Default)]
pub(super) struct Projection {
    pub replacements: HashMap<Id, PdfObject>,
    pub recovered: HashMap<Id, PdfObject>,
    pub recovered_ids: HashSet<Id>,
}
fn error(message: impl Into<String>) -> PdfError {
    PdfError::InvalidStructure(message.into())
}
fn reference(id: Id) -> PdfObject {
    PdfObject::Reference(id.0, id.1)
}
fn array(items: Vec<PdfObject>) -> PdfObject {
    PdfObject::Array(PdfArray(items))
}
fn dictionary(object: PdfObject, context: &str) -> Result<PdfDictionary, PdfError> {
    match object {
        PdfObject::Dictionary(d) => Ok(d),
        _ => Err(error(format!("{context}: expected dictionary"))),
    }
}

pub(super) fn read_object<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    id: Id,
    overrides: &HashMap<Id, PdfObject>,
) -> Result<PdfObject, PdfError> {
    if let Some(object) = overrides.get(&id) {
        return Ok(object.clone());
    }
    reader
        .get_object(id.0, id.1)
        .cloned()
        .map_err(|e| error(format!("reference {} {} R: {e}", id.0, id.1)))
}
struct Projector<'a, R: Read + Seek> {
    reader: &'a mut PdfReader<R>,
    pages: HashMap<Id, PdfDictionary>,
    retained: &'a HashSet<Id>,
    preserve_shape: bool,
    projection: Projection,
    visited: HashSet<Id>,
    entries: usize,
    parents: BTreeMap<Id, BTreeMap<usize, Id>>,
    reserved_keys: HashSet<i64>,
    object_parents: BTreeMap<i64, Id>,
    key_pages: HashMap<i64, Id>,
    ids: BTreeMap<Vec<u8>, Id>,
}
impl<R: Read + Seek> Projector<'_, R> {
    fn load(&mut self, id: Id) -> Result<PdfObject, PdfError> {
        read_object(self.reader, id, &HashMap::new())
    }
    fn budget(&mut self, depth: usize) -> Result<(), PdfError> {
        self.entries += 1;
        if depth > MAX_DEPTH || self.entries > LIMIT {
            return Err(error("tagged split exceeds structure traversal limits"));
        }
        Ok(())
    }
    // Only index roots with an explicit zero-offset entry may be reconstructed.
    // They are derived data: authoritative associations come from /K and pages.
    fn inspect_index(
        &mut self,
        value: &PdfObject,
        kind: &str,
        depth: usize,
        seen: &mut HashSet<Id>,
    ) -> Result<(), PdfError> {
        self.budget(depth)?;
        let object = match value {
            PdfObject::Reference(n, g) => {
                if !seen.insert((*n, *g)) {
                    return Err(error(format!(
                        "{kind}: cyclic or shared index {} {} R",
                        n, g
                    )));
                }
                if depth == 0
                    && self.reader.object_storage_offset(*n) == Some(0)
                    && self.reader.object_references().contains(&(*n, *g))
                {
                    // Confirm this is genuinely unreadable, not a synthetic cached object.
                    if self.reader.get_object(*n, *g).is_err() {
                        self.projection.recovered_ids.insert((*n, *g));
                        return Ok(());
                    }
                }
                self.load((*n, *g))?
            }
            other => other.clone(),
        };
        let d = dictionary(object, kind)?;
        if let Some(kids) = d.get("Kids") {
            let PdfObject::Array(kids) = kids else {
                return Err(error(format!("{kind}: /Kids must be an array")));
            };
            for child in &kids.0 {
                self.inspect_index(child, kind, depth + 1, seen)?;
            }
        } else if let Some(items) = d.get(if kind == "ParentTree" {
            "Nums"
        } else {
            "Names"
        }) {
            let PdfObject::Array(items) = items else {
                return Err(error(format!("{kind}: index entries must be an array")));
            };
            if items.0.len() % 2 != 0 {
                return Err(error(format!("{kind}: odd index entry count")));
            }
        } else {
            return Err(error(format!(
                "{kind}: index has neither entries nor children"
            )));
        }
        Ok(())
    }
    fn page(&self, d: &PdfDictionary, inherited: Option<Id>) -> Result<Option<Id>, PdfError> {
        match d.get("Pg") {
            Some(PdfObject::Reference(n, g)) if self.pages.contains_key(&(*n, *g)) => {
                Ok(Some((*n, *g)))
            }
            Some(_) => Err(error("tagged split /Pg does not reference a source page")),
            None => Ok(inherited),
        }
    }
    fn associate_mcid(&mut self, page: Option<Id>, mcid: i64, owner: Id) -> Result<bool, PdfError> {
        let page = page.ok_or_else(|| error("tagged split MCID has no effective /Pg"))?;
        if !(0..LIMIT as i64).contains(&mcid) {
            return Err(error("tagged split MCID exceeds supported range"));
        }
        if let Some(value) = self.pages[&page].get("StructParents") {
            let key = value.as_integer().filter(|k| *k >= 0).ok_or_else(|| {
                error(format!(
                    "tagged page {} {} R lacks valid /StructParents",
                    page.0, page.1
                ))
            })?;
            if let Some(previous) = self.key_pages.insert(key, page) {
                if previous != page {
                    return Err(error("tagged pages share /StructParents key"));
                }
            }
            if self.object_parents.contains_key(&key) {
                return Err(error("page and object share ParentTree key"));
            }
        }
        // Check ownership even for removed pages; filtering must not hide an
        // ambiguous source association merely because another part keeps it.
        if self
            .parents
            .entry(page)
            .or_default()
            .insert(mcid as usize, owner)
            .is_some()
        {
            return Err(error("MCID has multiple structure owners"));
        }
        Ok(self.retained.contains(&page))
    }
    fn kid(
        &mut self,
        value: &PdfObject,
        owner: Id,
        inherited: Option<Id>,
        depth: usize,
    ) -> Result<Option<PdfObject>, PdfError> {
        self.budget(depth)?;
        match value {
            PdfObject::Null => Ok(self.preserve_shape.then(|| value.clone())),
            PdfObject::Integer(mcid) => Ok(self
                .associate_mcid(inherited, *mcid, owner)?
                .then(|| value.clone())),
            PdfObject::Array(items) => {
                let mut kept = Vec::new();
                for item in &items.0 {
                    if let Some(item) = self.kid(item, owner, inherited, depth + 1)? {
                        kept.push(item);
                    }
                }
                Ok((self.preserve_shape || !kept.is_empty()).then(|| array(kept)))
            }
            PdfObject::Reference(n, g) => {
                let id = (*n, *g);
                if !self.visited.insert(id) {
                    return Err(error(format!(
                        "tagged split cyclic/shared /K reference {n} {g} R"
                    )));
                }
                let original = self.load(id)?;
                let projected = match &original {
                    PdfObject::Dictionary(d)
                        if d.get_type() == Some("StructElem") || d.contains_key("S") =>
                    {
                        self.element(id, d, owner, inherited, depth + 1)?
                    }
                    _ => self.kid(&original, owner, inherited, depth + 1)?,
                };
                if let Some(projected) = projected {
                    if projected != original {
                        self.projection.replacements.insert(id, projected);
                    }
                    Ok(Some(reference(id)))
                } else {
                    Ok(None)
                }
            }
            PdfObject::Dictionary(d) => {
                let page = self.page(d, inherited)?;
                match d.get_type() {
                    Some("MCR") | None if d.contains_key("MCID") => {
                        if d.contains_key("Stm") {
                            return Err(error(
                                "tagged split of external-stream MCR is unsupported",
                            ));
                        }
                        let mcid = d
                            .get("MCID")
                            .and_then(PdfObject::as_integer)
                            .ok_or_else(|| error("invalid MCR /MCID"))?;
                        Ok(self
                            .associate_mcid(page, mcid, owner)?
                            .then(|| value.clone()))
                    }
                    Some("OBJR") => {
                        let page = page.ok_or_else(|| error("tagged OBJR has no effective /Pg"))?;
                        let object = d
                            .get("Obj")
                            .and_then(PdfObject::as_reference)
                            .ok_or_else(|| error("tagged OBJR lacks indirect /Obj"))?;
                        let target = dictionary(self.load(object)?, "OBJR target")?;
                        let key = target
                            .get("StructParent")
                            .and_then(PdfObject::as_integer)
                            .filter(|k| *k >= 0)
                            .ok_or_else(|| error("OBJR target lacks valid /StructParent"))?;
                        self.reserved_keys.insert(key);
                        if !self.retained.contains(&page) {
                            return Ok(None);
                        }
                        if self.key_pages.contains_key(&key)
                            || self.object_parents.insert(key, owner).is_some()
                        {
                            return Err(error("duplicate OBJR ParentTree key"));
                        }
                        Ok(Some(value.clone()))
                    }
                    _ => Err(error("unsupported direct structure child in tagged split")),
                }
            }
            _ => Err(error("invalid tagged structure child")),
        }
    }
    fn element(
        &mut self,
        id: Id,
        d: &PdfDictionary,
        parent: Id,
        inherited: Option<Id>,
        depth: usize,
    ) -> Result<Option<PdfObject>, PdfError> {
        if d.get("P").and_then(PdfObject::as_reference) != Some(parent) {
            return Err(error(format!(
                "structure element {} {} R has inconsistent /P",
                id.0, id.1
            )));
        }
        let page = self.page(d, inherited)?;
        let kids = match d.get("K") {
            Some(k) => self.kid(k, id, page, depth + 1)?,
            None => None,
        };
        if kids.is_none()
            && (d.contains_key("K") || page.is_some_and(|p| !self.retained.contains(&p)))
        {
            return Ok(None);
        }
        let mut result = d.clone();
        if let Some(kids) = kids {
            result.insert("K".to_string(), kids);
        } else {
            result.0.retain(|k, _| k.0 != "K");
        }
        if page.is_some_and(|p| !self.retained.contains(&p)) {
            result.0.retain(|k, _| k.0 != "Pg");
        }
        if let Some(id_value) = d.get("ID") {
            let id_value = id_value
                .as_string()
                .ok_or_else(|| error("structure /ID must be a string"))?
                .as_bytes()
                .to_vec();
            if self.ids.insert(id_value, id).is_some() {
                return Err(error("duplicate retained structure /ID"));
            }
        }
        Ok(Some(PdfObject::Dictionary(result)))
    }
    fn index(&mut self, root: &mut PdfDictionary, key: &str, entries: Vec<PdfObject>) {
        let mut tree = PdfDictionary::new();
        tree.insert(
            if key == "ParentTree" { "Nums" } else { "Names" }.to_string(),
            array(entries),
        );
        if let Some(id) = root.get(key).and_then(PdfObject::as_reference) {
            self.projection
                .replacements
                .insert(id, PdfObject::Dictionary(tree));
        } else {
            root.insert(key.to_string(), PdfObject::Dictionary(tree));
        }
    }
}

pub(super) fn project<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    catalog: &PdfDictionary,
    pages: HashMap<Id, PdfDictionary>,
    retained: &HashSet<Id>,
) -> Result<Projection, PdfError> {
    project_impl(reader, catalog, pages, retained, false)
}

// Recovery keeps every source page and authoritative /K shape, including empty
// groups. Pruning belongs only to a projection that deletes pages.
pub(super) fn recover<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    catalog: &PdfDictionary,
    pages: HashMap<Id, PdfDictionary>,
) -> Result<Projection, PdfError> {
    let retained = pages.keys().copied().collect();
    project_impl(reader, catalog, pages, &retained, true)
}

fn project_impl<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    catalog: &PdfDictionary,
    pages: HashMap<Id, PdfDictionary>,
    retained: &HashSet<Id>,
    preserve_shape: bool,
) -> Result<Projection, PdfError> {
    let Some(root_value) = catalog.get("StructTreeRoot").cloned() else {
        return Ok(Projection::default());
    };
    let root_id = root_value
        .as_reference()
        .ok_or_else(|| error("tagged split requires indirect /StructTreeRoot"))?;
    let mut p = Projector {
        reader,
        reserved_keys: pages
            .values()
            .filter_map(|page| {
                page.get("StructParents")
                    .and_then(PdfObject::as_integer)
                    .filter(|key| *key >= 0)
            })
            .collect(),
        pages,
        retained,
        preserve_shape,
        projection: Projection::default(),
        visited: HashSet::from([root_id]),
        entries: 0,
        parents: BTreeMap::new(),
        object_parents: BTreeMap::new(),
        key_pages: HashMap::new(),
        ids: BTreeMap::new(),
    };
    let mut root = dictionary(p.load(root_id)?, "StructTreeRoot")?;
    if root.get_type() != Some("StructTreeRoot") {
        return Err(error("invalid /StructTreeRoot type"));
    }
    let original_root = root.clone();
    if root
        .get("ParentTree")
        .and_then(PdfObject::as_reference)
        .is_some()
        && root.get("ParentTree").and_then(PdfObject::as_reference)
            == root.get("IDTree").and_then(PdfObject::as_reference)
    {
        return Err(error("ParentTree and IDTree share an index object"));
    }
    for key in ["ParentTree", "IDTree"] {
        if let Some(value) = root.get(key) {
            p.inspect_index(value, key, 0, &mut HashSet::new())
                .map_err(|e| {
                    let location = value
                        .as_reference()
                        .map(|(n, g)| format!("{n} {g} R"))
                        .unwrap_or_else(|| "direct dictionary".to_string());
                    error(format!("{key} at {location}: {e}"))
                })?;
        }
    }
    let kids = match root.get("K") {
        Some(k) => p.kid(k, root_id, None, 0)?,
        None => None,
    };
    if let Some(kids) = kids {
        root.insert("K".to_string(), kids);
    } else if !preserve_shape {
        root.insert("K".to_string(), array(Vec::new()));
    }
    let mut nums = BTreeMap::new();
    let mut slots = 0usize;
    let mut fresh_key = 0i64;
    // Page identities order assignment deterministically for both planning and
    // materialization, independent of HashMap iteration and /K encounter order.
    for (&page, owners) in &p.parents {
        if !p.retained.contains(&page) {
            continue;
        }
        let key = match p.pages[&page].get("StructParents") {
            Some(value) => value
                .as_integer()
                .ok_or_else(|| error("invalid page parent key"))?,
            None => {
                while p.reserved_keys.contains(&fresh_key) {
                    fresh_key = fresh_key
                        .checked_add(1)
                        .ok_or_else(|| error("ParentTree key overflow"))?;
                }
                let key = fresh_key;
                p.reserved_keys.insert(key);
                let mut dictionary = p.pages[&page].clone();
                dictionary.insert("StructParents".to_string(), PdfObject::Integer(key));
                p.projection
                    .replacements
                    .insert(page, PdfObject::Dictionary(dictionary));
                key
            }
        };
        slots = slots
            .checked_add(owners.keys().next_back().copied().unwrap_or(0) + 1)
            .ok_or_else(|| error("ParentTree allocation exceeds limit"))?;
        if slots > 1_000_000 {
            return Err(error("ParentTree allocation exceeds limit"));
        }
        let mut values = vec![PdfObject::Null; owners.keys().next_back().copied().unwrap_or(0) + 1];
        for (&mcid, &owner) in owners {
            values[mcid] = reference(owner);
        }
        nums.insert(key, array(values));
    }
    for (&key, &owner) in &p.object_parents {
        if nums.insert(key, reference(owner)).is_some() {
            return Err(error("conflicting ParentTree keys"));
        }
    }
    let next = nums
        .keys()
        .next_back()
        .copied()
        .unwrap_or(-1)
        .checked_add(1)
        .ok_or_else(|| error("ParentTree key overflow"))?;
    p.index(
        &mut root,
        "ParentTree",
        nums.into_iter()
            .flat_map(|(k, v)| [PdfObject::Integer(k), v])
            .collect(),
    );
    root.insert("ParentTreeNextKey".to_string(), PdfObject::Integer(next));
    if root.contains_key("IDTree") || !p.ids.is_empty() {
        let names = p
            .ids
            .iter()
            .flat_map(|(k, &v)| [PdfObject::String(PdfString::new(k.clone())), reference(v)])
            .collect();
        p.index(&mut root, "IDTree", names);
    }
    if !p.projection.recovered_ids.is_empty() {
        let mut source_root = original_root;
        for key in ["ParentTree", "IDTree"] {
            if source_root
                .get(key)
                .and_then(PdfObject::as_reference)
                .is_some_and(|id| p.projection.recovered_ids.contains(&id))
            {
                source_root.insert(key.to_string(), PdfObject::Null);
            }
        }
        // Exempt only the known derived-index edges, never aliases elsewhere.
        p.projection
            .recovered
            .insert(root_id, PdfObject::Dictionary(source_root));
    }
    p.projection
        .replacements
        .insert(root_id, PdfObject::Dictionary(root));
    Ok(p.projection)
}
