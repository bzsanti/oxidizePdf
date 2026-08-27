//! Bounded, read-only parsing of document outlines and destinations.

use super::objects::{PdfArray, PdfDictionary, PdfObject};
use super::{ParseError, ParseResult, PdfReader};
use crate::geometry::{Point, Rectangle};
use crate::graphics::Color;
use crate::structure::{
    Destination, DestinationType, OutlineFlags, OutlineItem, OutlineTree, PageDestination,
};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};

type ObjectRef = (u32, u16);

/// Resource limits for parsing untrusted outline and destination trees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineReadOptions {
    /// Maximum outline items visited.
    pub max_items: usize,
    /// Maximum outline and name-tree nesting depth.
    pub max_depth: usize,
    /// Maximum named destinations collected.
    pub max_named_destinations: usize,
    /// Maximum name-tree nodes visited, including empty intermediate nodes.
    pub max_name_tree_nodes: usize,
}

impl Default for OutlineReadOptions {
    fn default() -> Self {
        Self {
            max_items: 100_000,
            max_depth: 256,
            max_named_destinations: 100_000,
            max_name_tree_nodes: 100_000,
        }
    }
}

pub(crate) fn read_outline<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    pages: &HashMap<ObjectRef, u32>,
    options: &OutlineReadOptions,
) -> ParseResult<Option<OutlineTree>> {
    let catalog = reader.catalog()?.clone();
    let Some(outlines_value) = catalog.get("Outlines").cloned() else {
        return Ok(None);
    };
    let (root, root_ref) = resolve_dictionary(reader, &outlines_value, "/Catalog/Outlines")?;
    let root_ref = root_ref.ok_or_else(|| malformed("/Outlines must be an indirect object"))?;
    let Some(first) = root.get("First") else {
        if root.contains_key("Last") {
            return Err(malformed("/Outlines has Last without First"));
        }
        return Ok(Some(OutlineTree::new()));
    };
    let first = require_reference(first, "/Outlines/First")?;
    let named = read_named_destinations(reader, &catalog, options)?;
    let last = root
        .get("Last")
        .ok_or_else(|| malformed("/Outlines has First without Last"))
        .and_then(|value| require_reference(value, "/Outlines/Last"))?;
    let mut parser = OutlineParser {
        reader,
        pages,
        named,
        options,
        visited: HashSet::new(),
        active_destinations: HashSet::new(),
        active_names: HashSet::new(),
        item_count: 0,
    };
    let items = parser.read_siblings(first, Some(root_ref), Some(last), 0, "/Outlines")?;
    Ok(Some(OutlineTree { items }))
}

struct OutlineParser<'a, R: Read + Seek> {
    reader: &'a mut PdfReader<R>,
    pages: &'a HashMap<ObjectRef, u32>,
    named: HashMap<Vec<u8>, PdfObject>,
    options: &'a OutlineReadOptions,
    visited: HashSet<ObjectRef>,
    active_destinations: HashSet<ObjectRef>,
    active_names: HashSet<Vec<u8>>,
    item_count: usize,
}

impl<R: Read + Seek> OutlineParser<'_, R> {
    fn read_siblings(
        &mut self,
        first: ObjectRef,
        parent: Option<ObjectRef>,
        expected_last: Option<ObjectRef>,
        depth: usize,
        path: &str,
    ) -> ParseResult<Vec<OutlineItem>> {
        if depth > self.options.max_depth {
            return Err(malformed("outline nesting exceeds configured limit"));
        }
        let mut result = Vec::new();
        let mut current = Some(first);
        let mut previous = None;
        let mut final_ref = None;
        while let Some(reference) = current {
            self.item_count = self
                .item_count
                .checked_add(1)
                .ok_or_else(|| malformed("outline item count overflow"))?;
            if self.item_count > self.options.max_items {
                return Err(malformed("outline item count exceeds configured limit"));
            }
            if !self.visited.insert(reference) {
                return Err(malformed(
                    "outline hierarchy contains a cycle or duplicate item",
                ));
            }
            let value = PdfObject::Reference(reference.0, reference.1);
            let (dictionary, _) = resolve_dictionary(self.reader, &value, path)?;
            if dictionary.get("Parent").and_then(PdfObject::as_reference) != parent {
                return Err(malformed(
                    "outline item Parent does not match its containing list",
                ));
            }
            if dictionary.get("Prev").and_then(PdfObject::as_reference) != previous {
                return Err(malformed("outline item Prev link is inconsistent"));
            }
            let title = self
                .resolve_optional(dictionary.get("Title"))?
                .as_ref()
                .and_then(PdfObject::as_string)
                .map(|title| title.to_text())
                .ok_or_else(|| malformed("outline item has no string Title"))?;
            let destination = self.read_item_destination(&dictionary, path)?;
            let flags = self
                .resolve_optional(dictionary.get("F"))?
                .as_ref()
                .and_then(PdfObject::as_integer)
                .unwrap_or(0);
            if flags < 0 || flags & !3 != 0 {
                return Err(malformed("outline item F contains unsupported flag bits"));
            }
            let color = read_color(self.resolve_optional(dictionary.get("C"))?.as_ref())?;
            let first_child = dictionary
                .get("First")
                .map(|value| require_reference(value, "outline First"))
                .transpose()?;
            let last_child = dictionary
                .get("Last")
                .map(|value| require_reference(value, "outline Last"))
                .transpose()?;
            if first_child.is_some() != last_child.is_some() {
                return Err(malformed(
                    "outline item must contain both First and Last child links",
                ));
            }
            let children = match first_child {
                Some(first_child) => {
                    self.read_siblings(first_child, Some(reference), last_child, depth + 1, path)?
                }
                None => Vec::new(),
            };
            let open = self
                .resolve_optional(dictionary.get("Count"))?
                .as_ref()
                .and_then(PdfObject::as_integer)
                .map_or(true, |count| count >= 0);
            result.push(OutlineItem {
                title,
                destination,
                children,
                color,
                flags: OutlineFlags {
                    italic: flags & 1 != 0,
                    bold: flags & 2 != 0,
                },
                open,
            });
            previous = Some(reference);
            final_ref = Some(reference);
            current = dictionary
                .get("Next")
                .map(|value| require_reference(value, "outline Next"))
                .transpose()?;
        }
        if expected_last.is_some() && final_ref != expected_last {
            return Err(malformed(
                "outline Last link does not identify the final sibling",
            ));
        }
        Ok(result)
    }

    fn read_item_destination(
        &mut self,
        dictionary: &PdfDictionary,
        path: &str,
    ) -> ParseResult<Option<Destination>> {
        if dictionary.contains_key("Dest") && dictionary.contains_key("A") {
            return Err(malformed("outline item contains both Dest and A"));
        }
        if let Some(value) = dictionary.get("Dest") {
            return self.resolve_destination(value, path).map(Some);
        }
        let Some(action_value) = dictionary.get("A") else {
            return Ok(None);
        };
        let (action, _) = resolve_dictionary(self.reader, action_value, "outline action")?;
        if action
            .get("S")
            .and_then(PdfObject::as_name)
            .map(|name| name.as_str())
            != Some("GoTo")
        {
            return Ok(None);
        }
        let value = action
            .get("D")
            .ok_or_else(|| malformed("GoTo action has no D destination"))?;
        self.resolve_destination(value, path).map(Some)
    }

    fn resolve_optional(&mut self, value: Option<&PdfObject>) -> ParseResult<Option<PdfObject>> {
        value
            .map(|value| resolve_object(self.reader, value))
            .transpose()
    }

    fn resolve_destination(&mut self, value: &PdfObject, path: &str) -> ParseResult<Destination> {
        self.resolve_destination_at(value, path, 0)
    }

    fn resolve_destination_at(
        &mut self,
        value: &PdfObject,
        path: &str,
        depth: usize,
    ) -> ParseResult<Destination> {
        if depth > self.options.max_depth {
            return Err(malformed("destination resolution exceeds configured depth"));
        }
        let reference = value.as_reference();
        if reference.is_some_and(|reference| !self.active_destinations.insert(reference)) {
            return Err(malformed("destination objects contain a cycle"));
        }
        let value = resolve_object(self.reader, value)?;
        let result = match value {
            PdfObject::Array(array) => parse_destination_array(&array, self.pages),
            PdfObject::Name(name) => self.resolve_named(name.as_str().as_bytes(), path),
            PdfObject::String(name) => self.resolve_named(name.as_bytes(), path),
            PdfObject::Dictionary(dictionary) => {
                let value = dictionary
                    .get("D")
                    .ok_or_else(|| malformed("destination dictionary has no D entry"))?;
                self.resolve_destination_at(value, path, depth + 1)
            }
            _ => Err(malformed(format!("malformed destination at {path}"))),
        };
        if let Some(reference) = reference {
            self.active_destinations.remove(&reference);
        }
        result
    }

    fn resolve_named(&mut self, name: &[u8], path: &str) -> ParseResult<Destination> {
        if !self.active_names.insert(name.to_vec()) {
            return Err(malformed("named destinations contain a cycle"));
        }
        let value = self
            .named
            .get(name)
            .cloned()
            .ok_or_else(|| malformed(format!("unknown named destination at {path}")))?;
        let result = self.resolve_destination_at(&value, path, self.active_names.len());
        self.active_names.remove(name);
        result
    }
}

fn parse_destination_array(
    array: &PdfArray,
    pages: &HashMap<ObjectRef, u32>,
) -> ParseResult<Destination> {
    if array.0.len() < 2 {
        return Err(malformed("destination array has fewer than two entries"));
    }
    let page = match &array.0[0] {
        PdfObject::Reference(number, generation) => pages
            .get(&(*number, *generation))
            .copied()
            .ok_or_else(|| malformed("destination page reference is not in the page tree"))?,
        PdfObject::Integer(index) if *index >= 0 => {
            let index = u32::try_from(*index)
                .map_err(|_| malformed("destination page index is out of range"))?;
            if index as usize >= pages.len() {
                return Err(malformed("destination page index is outside the page tree"));
            }
            index
        }
        _ => {
            return Err(malformed(
                "destination target is not a page reference or non-negative index",
            ))
        }
    };
    let kind = array.0[1]
        .as_name()
        .ok_or_else(|| malformed("destination view type is not a name"))?
        .as_str();
    let param = |index: usize| -> ParseResult<Option<f64>> {
        match array.0.get(index) {
            Some(PdfObject::Null) => Ok(None),
            Some(value) => value
                .as_real()
                .filter(|value| value.is_finite())
                .map(Some)
                .ok_or_else(|| malformed("destination parameter is not a finite number or null")),
            None => Err(malformed("destination is missing a required parameter")),
        }
    };
    let required =
        |index: usize| param(index)?.ok_or_else(|| malformed("FitR parameters cannot be null"));
    let dest_type = match kind {
        "XYZ" => {
            let zoom = param(4)?;
            if zoom.is_some_and(|zoom| zoom < 0.0) {
                return Err(malformed("XYZ zoom cannot be negative"));
            }
            DestinationType::XYZ {
                left: param(2)?,
                top: param(3)?,
                zoom,
            }
        }
        "Fit" => DestinationType::Fit,
        "FitH" => DestinationType::FitH { top: param(2)? },
        "FitV" => DestinationType::FitV { left: param(2)? },
        "FitR" => DestinationType::FitR {
            rect: Rectangle::new(
                Point::new(required(2)?, required(3)?),
                Point::new(required(4)?, required(5)?),
            ),
        },
        "FitB" => DestinationType::FitB,
        "FitBH" => DestinationType::FitBH { top: param(2)? },
        "FitBV" => DestinationType::FitBV { left: param(2)? },
        _ => return Err(malformed(format!("unknown destination view type {kind}"))),
    };
    Ok(Destination {
        page: PageDestination::PageNumber(page),
        dest_type,
    })
}

fn read_named_destinations<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    catalog: &PdfDictionary,
    options: &OutlineReadOptions,
) -> ParseResult<HashMap<Vec<u8>, PdfObject>> {
    let mut result = HashMap::new();
    if let Some(legacy) = catalog.get("Dests") {
        let (dictionary, _) = resolve_dictionary(reader, legacy, "/Catalog/Dests")?;
        for (name, value) in dictionary.0 {
            insert_named(&mut result, name.0.into_bytes(), value, options)?;
        }
    }
    if let Some(names) = catalog.get("Names") {
        let (names, _) = resolve_dictionary(reader, names, "/Catalog/Names")?;
        if let Some(destinations) = names.get("Dests") {
            let mut active = HashSet::new();
            let mut nodes = 0usize;
            read_name_tree(
                reader,
                destinations,
                0,
                &mut active,
                &mut nodes,
                &mut result,
                options,
            )?;
        }
    }
    Ok(result)
}

fn read_name_tree<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    value: &PdfObject,
    depth: usize,
    active: &mut HashSet<ObjectRef>,
    nodes: &mut usize,
    result: &mut HashMap<Vec<u8>, PdfObject>,
    options: &OutlineReadOptions,
) -> ParseResult<Option<(Vec<u8>, Vec<u8>)>> {
    if depth > options.max_depth {
        return Err(malformed("destination name tree exceeds configured depth"));
    }
    *nodes = nodes
        .checked_add(1)
        .ok_or_else(|| malformed("destination name-tree node count overflow"))?;
    if *nodes > options.max_name_tree_nodes {
        return Err(malformed(
            "destination name-tree nodes exceed configured limit",
        ));
    }
    let reference = value.as_reference();
    if reference.is_some_and(|reference| !active.insert(reference)) {
        return Err(malformed("destination name tree contains a cycle"));
    }
    let (dictionary, _) = resolve_dictionary(reader, value, "destination name tree")?;
    if dictionary.contains_key("Names") && dictionary.contains_key("Kids") {
        return Err(malformed("name-tree node contains both Names and Kids"));
    }
    let limits = dictionary.get("Limits").map(read_name_limits).transpose()?;
    let actual_range = if let Some(names) = dictionary.get("Names") {
        let names = resolve_object(reader, names)?;
        let names = names
            .as_array()
            .ok_or_else(|| malformed("name-tree Names is not an array"))?;
        if names.0.len() % 2 != 0 {
            return Err(malformed("name-tree Names array has odd length"));
        }
        let mut previous: Option<Vec<u8>> = None;
        let mut first_key = None;
        let mut last_key = None;
        for pair in names.0.chunks_exact(2) {
            let key = pair[0]
                .as_string()
                .ok_or_else(|| malformed("name-tree key is not a string"))?
                .as_bytes()
                .to_vec();
            if previous.as_ref().is_some_and(|previous| previous >= &key) {
                return Err(malformed("name-tree keys are not strictly increasing"));
            }
            first_key.get_or_insert_with(|| key.clone());
            last_key = Some(key.clone());
            previous = Some(key.clone());
            insert_named(result, key, pair[1].clone(), options)?;
        }
        first_key.zip(last_key)
    } else if let Some(kids) = dictionary.get("Kids") {
        let kids = resolve_object(reader, kids)?;
        let kids = kids
            .as_array()
            .ok_or_else(|| malformed("name-tree Kids is not an array"))?;
        let mut first_key = None;
        let mut last_key: Option<Vec<u8>> = None;
        for child in &kids.0 {
            let child_range =
                read_name_tree(reader, child, depth + 1, active, nodes, result, options)?;
            let Some((lower, upper)) = child_range else {
                continue;
            };
            if last_key.as_ref().is_some_and(|previous| previous >= &lower) {
                return Err(malformed(
                    "name-tree child ranges overlap or are not strictly increasing",
                ));
            }
            first_key.get_or_insert_with(|| lower.clone());
            last_key = Some(upper);
        }
        first_key.zip(last_key)
    } else {
        None
    };
    match (limits, actual_range.as_ref()) {
        (Some((lower, upper)), Some((actual_lower, actual_upper)))
            if lower != *actual_lower || upper != *actual_upper =>
        {
            return Err(malformed("name-tree Limits do not match subtree keys"));
        }
        (Some(_), None) => {
            return Err(malformed("empty name-tree node has non-empty Limits"));
        }
        _ => {}
    }
    if let Some(reference) = reference {
        active.remove(&reference);
    }
    Ok(actual_range)
}

fn read_name_limits(value: &PdfObject) -> ParseResult<(Vec<u8>, Vec<u8>)> {
    let limits = value
        .as_array()
        .ok_or_else(|| malformed("name-tree Limits is not an array"))?;
    if limits.0.len() != 2 {
        return Err(malformed("name-tree Limits must contain two strings"));
    }
    let lower = limits.0[0]
        .as_string()
        .ok_or_else(|| malformed("name-tree lower limit is not a string"))?
        .as_bytes()
        .to_vec();
    let upper = limits.0[1]
        .as_string()
        .ok_or_else(|| malformed("name-tree upper limit is not a string"))?
        .as_bytes()
        .to_vec();
    if lower > upper {
        return Err(malformed("name-tree Limits are reversed"));
    }
    Ok((lower, upper))
}

fn insert_named(
    result: &mut HashMap<Vec<u8>, PdfObject>,
    key: Vec<u8>,
    value: PdfObject,
    options: &OutlineReadOptions,
) -> ParseResult<()> {
    if result.contains_key(&key) {
        return Err(malformed("duplicate named destination"));
    }
    if result.len() >= options.max_named_destinations {
        return Err(malformed("named destinations exceed configured limit"));
    }
    result.insert(key, value);
    Ok(())
}

fn read_color(value: Option<&PdfObject>) -> ParseResult<Option<Color>> {
    let Some(value) = value else { return Ok(None) };
    let array = value
        .as_array()
        .ok_or_else(|| malformed("outline C is not an RGB array"))?;
    if array.0.len() != 3 {
        return Err(malformed("outline C must have exactly three components"));
    }
    let mut values = [0.0; 3];
    for (index, value) in array.0.iter().enumerate() {
        values[index] = value
            .as_real()
            .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
            .ok_or_else(|| malformed("outline color component is outside 0..=1"))?;
    }
    Ok(Some(Color::Rgb(values[0], values[1], values[2])))
}

fn resolve_object<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    value: &PdfObject,
) -> ParseResult<PdfObject> {
    match value.as_reference() {
        Some((number, generation)) => reader.get_object(number, generation).cloned(),
        None => Ok(value.clone()),
    }
}

fn resolve_dictionary<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    value: &PdfObject,
    path: &str,
) -> ParseResult<(PdfDictionary, Option<ObjectRef>)> {
    let reference = value.as_reference();
    let value = resolve_object(reader, value)?;
    value
        .as_dict()
        .cloned()
        .map(|dictionary| (dictionary, reference))
        .ok_or_else(|| malformed(format!("{path} is not a dictionary")))
}

fn require_reference(value: &PdfObject, path: &str) -> ParseResult<ObjectRef> {
    value
        .as_reference()
        .ok_or_else(|| malformed(format!("{path} is not an indirect reference")))
}

fn malformed(message: impl Into<String>) -> ParseError {
    ParseError::SyntaxError {
        position: 0,
        message: format!("outline: {}", message.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::objects::PdfName;

    fn destination(kind: &str, parameters: Vec<PdfObject>) -> DestinationType {
        let mut values = vec![
            PdfObject::Reference(10, 0),
            PdfObject::Name(PdfName(kind.to_string())),
        ];
        values.extend(parameters);
        let pages = HashMap::from([((10, 0), 3)]);
        parse_destination_array(&PdfArray(values), &pages)
            .expect("valid destination")
            .dest_type
    }

    #[test]
    fn exposes_every_standard_destination_view() {
        assert!(matches!(destination("Fit", vec![]), DestinationType::Fit));
        assert!(matches!(
            destination("FitH", vec![PdfObject::Integer(20)]),
            DestinationType::FitH { top: Some(20.0) }
        ));
        assert!(matches!(
            destination("FitV", vec![PdfObject::Null]),
            DestinationType::FitV { left: None }
        ));
        assert!(matches!(
            destination(
                "FitR",
                vec![
                    PdfObject::Integer(0),
                    PdfObject::Integer(1),
                    PdfObject::Integer(2),
                    PdfObject::Integer(3),
                ],
            ),
            DestinationType::FitR { .. }
        ));
        assert!(matches!(destination("FitB", vec![]), DestinationType::FitB));
        assert!(matches!(
            destination("FitBH", vec![PdfObject::Real(4.0)]),
            DestinationType::FitBH { top: Some(4.0) }
        ));
        assert!(matches!(
            destination("FitBV", vec![PdfObject::Null]),
            DestinationType::FitBV { left: None }
        ));
    }
}
