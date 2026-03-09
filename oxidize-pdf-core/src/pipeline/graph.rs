use crate::pipeline::Element;
use std::collections::HashMap;

/// Index-based graph of relationships between document elements.
///
/// Built from a `&[Element]` slice, the graph stores parent/child and next/prev
/// relationships as indices into the original slice.  No ownership is taken and
/// no lifetimes are introduced in the type — the graph is a standalone value that
/// can be stored, cloned, or sent across threads independently of the elements.
///
/// # Building
///
/// ```rust,ignore
/// let elements = doc.partition()?;
/// let graph = ElementGraph::build(&elements);
/// ```
///
/// # Parent / child semantics
///
/// An element at index `i` is a child of the nearest preceding `Title` element
/// whose text matches `elements[i].metadata().parent_heading`.  A `Title` element
/// whose own `parent_heading` equals its own text is treated as a root (no parent)
/// unless a *different* preceding title has the same text.
pub struct ElementGraph {
    parent: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
    next: Vec<Option<usize>>,
    prev: Vec<Option<usize>>,
    /// Which indices are Title elements (needed for `top_level_sections`).
    is_title: Vec<bool>,
}

impl ElementGraph {
    /// Build a graph from a slice of elements.
    pub fn build(elements: &[Element]) -> Self {
        let n = elements.len();

        let mut parent: Vec<Option<usize>> = vec![None; n];
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut next: Vec<Option<usize>> = vec![None; n];
        let mut prev: Vec<Option<usize>> = vec![None; n];
        let mut is_title: Vec<bool> = vec![false; n];

        // ── next / prev ──────────────────────────────────────────────────────
        for i in 0..n {
            if i + 1 < n {
                next[i] = Some(i + 1);
            }
            if i > 0 {
                prev[i] = Some(i - 1);
            }
        }

        // ── parent / child ───────────────────────────────────────────────────
        // Map heading text → most recent Title index with that text.
        // We update this map as we scan forward so that "most recent" is always correct.
        let mut latest_title_for_heading: HashMap<String, usize> = HashMap::new();

        for i in 0..n {
            if matches!(elements[i], Element::Title(_)) {
                is_title[i] = true;
                let title_text = elements[i].text().to_string();
                latest_title_for_heading.insert(title_text, i);
            }
        }

        // Second pass: assign parent / child.
        // We need to process in order so we can handle the "most recent title"
        // requirement by rebuilding the map incrementally.
        let mut active_title_for_heading: HashMap<String, usize> = HashMap::new();

        for i in 0..n {
            if matches!(elements[i], Element::Title(_)) {
                let title_text = elements[i].text().to_string();
                active_title_for_heading.insert(title_text, i);
                // A title is a root unless a *different* preceding title has an
                // explicit `parent_heading` pointing to yet another title.
                // Per the spec: "A Title element's own parent_heading equals its
                // own text — it should NOT be its own parent."
                // So titles are always roots in the current model.
                // (parent stays None for titles)
            } else {
                // Non-title element: look up its parent_heading.
                if let Some(heading_text) = elements[i].metadata().parent_heading.as_deref() {
                    if let Some(&title_idx) = active_title_for_heading.get(heading_text) {
                        parent[i] = Some(title_idx);
                        children[title_idx].push(i);
                    }
                }
            }
        }

        Self {
            parent,
            children,
            next,
            prev,
            is_title,
        }
    }

    /// Number of elements in the graph (equals the length of the source slice).
    pub fn len(&self) -> usize {
        self.parent.len()
    }

    /// Returns `true` when the graph was built from an empty slice.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the index of the parent element of `idx`, or `None` if it is a root.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= self.len()`.
    pub fn parent_of(&self, idx: usize) -> Option<usize> {
        self.parent[idx]
    }

    /// Returns the indices of the children of element `idx`.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= self.len()`.
    pub fn children_of(&self, idx: usize) -> &[usize] {
        &self.children[idx]
    }

    /// Returns the index of the element immediately after `idx`, or `None` if `idx`
    /// is the last element.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= self.len()`.
    pub fn next_of(&self, idx: usize) -> Option<usize> {
        self.next[idx]
    }

    /// Returns the index of the element immediately before `idx`, or `None` if `idx`
    /// is the first element.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= self.len()`.
    pub fn prev_of(&self, idx: usize) -> Option<usize> {
        self.prev[idx]
    }

    /// Returns the child indices of a Title element (i.e., the elements belonging
    /// to its section).
    ///
    /// This is an alias for [`children_of`](Self::children_of) with a semantically
    /// clearer name for the common case of iterating a document section.
    pub fn elements_in_section(&self, title_idx: usize) -> Vec<usize> {
        self.children_of(title_idx).to_vec()
    }

    /// Returns the indices of all Title elements that have no parent (top-level sections).
    pub fn top_level_sections(&self) -> Vec<usize> {
        (0..self.len())
            .filter(|&i| self.is_title[i] && self.parent[i].is_none())
            .collect()
    }
}
