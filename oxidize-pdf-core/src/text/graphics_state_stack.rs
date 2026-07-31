//! Bounded save-state stack for the `q`/`Q` operators (issue #455).
//!
//! Both text extractors snapshot graphics state on `q` and restore it on `Q`.
//! Nothing in the content stream limits how deep that nesting goes, so a stream
//! of one million `q` operators — about 2 MB — used to push one million
//! snapshots. Since #452 each snapshot also carries the text state, including a
//! heap allocation for the font name whenever a font is set, so the per-entry
//! cost of that flood roughly doubled.
//!
//! Annex C of ISO 32000-1 gives 28 as the historical implementation limit for
//! graphics state nesting, so [`MAX_DEPTH`] is generous by a wide margin and no
//! real document reaches it.
//!
//! # Why the dropped pushes are counted
//!
//! The subtle part is not the cap, it is the pairing. A stack that silently
//! dropped pushes but honoured every `Q` would keep restoring — from the wrong
//! entry. Every `Q` past the cap would hand back the state of a level further
//! out than the one it closes, and every later `Q` would stay shifted by the
//! same amount. With the text state inside each snapshot (#452), that
//! mispairing changes the font in force, so it corrupts decoding rather than
//! just the CTM.
//!
//! So [`GraphicsStateStack`] counts what it drops and answers exactly that many
//! pops with `None`. Levels deeper than the cap stop restoring — a documented
//! loss, only reachable by documents no viewer renders — while every level
//! within the cap keeps restoring exactly.
//!
//! # Why the counter lives here
//!
//! The count is part of the stack, not a field beside it, because both are
//! saved and restored together at a Form XObject boundary: `Do` gives the form
//! its own stack so a stray `Q` inside it cannot pop the page's snapshots. A
//! counter left outside that swap would let the form consume the page's dropped
//! pushes, which is the very mispairing this type exists to prevent. Keeping
//! them in one value makes `std::mem::take` at that boundary correct by
//! construction.

/// Maximum number of `q` snapshots either extractor keeps.
///
/// Annex C of ISO 32000-1 lists 28 as the historical nesting limit for the
/// graphics state stack; this is two orders of magnitude above that.
pub(crate) const MAX_DEPTH: usize = 1024;

/// A `q`/`Q` save-state stack that stops growing at [`MAX_DEPTH`] without ever
/// letting a `Q` restore the wrong entry.
#[derive(Debug, Clone)]
pub(crate) struct GraphicsStateStack<T> {
    entries: Vec<T>,
    /// Pushes refused because `entries` was already at [`MAX_DEPTH`]. The next
    /// `dropped` pops restore nothing, so each `Q` still pairs with its own `q`.
    dropped: usize,
}

impl<T> Default for GraphicsStateStack<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            dropped: 0,
        }
    }
}

impl<T> GraphicsStateStack<T> {
    /// Push a snapshot for `q`, unless the stack is full.
    ///
    /// The snapshot is built lazily because building it is what costs: a
    /// snapshot clones the font name, so a flood must not pay for entries the
    /// stack is about to refuse.
    pub(crate) fn push_with(&mut self, capture: impl FnOnce() -> T) {
        if self.entries.len() < MAX_DEPTH {
            self.entries.push(capture());
        } else {
            // Saturating because a wrap would turn a refused push into a
            // restore from the wrong entry. Unreachable in practice — each `q`
            // costs at least one byte of content stream — but the failure mode
            // is bad enough not to leave it to arithmetic.
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    /// Pop the snapshot for a `Q`.
    ///
    /// Returns `None` when this `Q` closes a level whose push was refused, and
    /// also when the stack is empty: an unbalanced `Q` is ignored rather than
    /// fatal, so malformed documents still extract.
    pub(crate) fn pop(&mut self) -> Option<T> {
        if self.dropped > 0 {
            self.dropped -= 1;
            return None;
        }
        self.entries.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un desenrollado equilibrado devuelve cada nivel dentro del tope, en
    /// orden LIFO, y los de más allá del tope no devuelven nada.
    #[test]
    fn a_balanced_unwind_past_the_cap_pairs_every_push_with_its_own_pop() {
        let overflow = 7;
        let mut stack = GraphicsStateStack::default();
        for level in 0..MAX_DEPTH + overflow {
            stack.push_with(|| level);
        }

        // Los `overflow` primeros `Q` cierran niveles cuyo empuje se descartó.
        for _ in 0..overflow {
            assert_eq!(
                stack.pop(),
                None,
                "a pop closing a dropped push must restore nothing"
            );
        }
        // Y a partir de ahí, cada nivel guardado sale exactamente una vez y en
        // orden inverso: el tope recorta los niveles internos, no desplaza los
        // que sí caben.
        for level in (0..MAX_DEPTH).rev() {
            assert_eq!(stack.pop(), Some(level));
        }
        assert_eq!(stack.pop(), None, "the stack is empty after a full unwind");
    }

    /// Los `pop` de más no dejan la cuenta en negativo ni resucitan entradas.
    #[test]
    fn extra_pops_are_ignored_and_do_not_corrupt_the_count() {
        let mut stack = GraphicsStateStack::default();
        stack.push_with(|| 1);
        for _ in 0..MAX_DEPTH + 3 {
            let _ = stack.pop();
        }
        stack.push_with(|| 2);
        assert_eq!(
            stack.pop(),
            Some(2),
            "after over-popping, a fresh push must still be the next thing restored"
        );
    }

    /// El tope no puede pagar la instantánea que va a rechazar.
    #[test]
    fn a_refused_push_never_builds_its_snapshot() {
        let mut captures = 0usize;
        let mut stack = GraphicsStateStack::default();
        for _ in 0..MAX_DEPTH + 500 {
            stack.push_with(|| {
                captures += 1;
            });
        }
        assert_eq!(
            captures, MAX_DEPTH,
            "only the pushes that fit may build a snapshot"
        );
    }
}
