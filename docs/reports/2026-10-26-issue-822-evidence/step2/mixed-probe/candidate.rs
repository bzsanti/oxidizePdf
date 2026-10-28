//! Lossless font usage with a cheap duplicate check for repeated ASCII text.
use std::collections::HashSet;

#[derive(Clone, Default)]
pub(crate) struct UsedCharacters {
    ascii: u128,
    all: HashSet<char>,
}
impl UsedCharacters {
    pub(crate) fn from_text(text: &str) -> Self {
        let chars = text.chars();
        let initial_capacity = chars.size_hint().0;
        // Only ASCII has a fixed repertoire of 128 characters. Preserve the
        // Unicode reservation to avoid extra rehashing for high-cardinality text.
        let initial_capacity = if text.is_ascii() {
            initial_capacity.min(128)
        } else {
            initial_capacity
        };
        let mut result = Self {
            ascii: 0,
            all: HashSet::with_capacity(initial_capacity),
        };
        result.extend(chars);
        result
    }

    pub(crate) fn as_set(&self) -> &HashSet<char> {
        &self.all
    }
}
impl std::ops::Deref for UsedCharacters {
    type Target = HashSet<char>;
    fn deref(&self) -> &Self::Target {
        &self.all
    }
}
impl Extend<char> for UsedCharacters {
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        for ch in iter {
            if ch.is_ascii() {
                let bit = 1u128 << (ch as u32);
                if self.ascii & bit != 0 {
                    continue;
                }
                self.ascii |= bit;
            }
            self.all.insert(ch);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_initial_reservation_keeps_all_distinct_unicode_characters() {
        let text: String = (0x4e00..0x5e00).filter_map(char::from_u32).collect();
        let usage = UsedCharacters::from_text(&text);
        assert_eq!(usage.as_set(), &text.chars().collect::<HashSet<_>>());
        let mut copy = usage.clone();
        copy.extend("Aé😀".chars());
        assert_eq!(usage.len(), 4096);
        assert_eq!(copy.len(), 4099);
        for ch in "Aé😀".chars() {
            assert!(copy.contains(&ch));
            assert!(!usage.contains(&ch));
        }
        let mut ascii_first = UsedCharacters::from_text(&"A".repeat(16_384));
        ascii_first.extend(text.chars());
        assert_eq!(ascii_first.len(), 4097);
        for ch in text.chars() {
            assert!(ascii_first.contains(&ch));
        }
    }
    #[test]
    fn repeated_ascii_and_unicode_union_remains_lossless_after_clone() {
        let initial: String = (0..=127).filter_map(char::from_u32).collect();
        let mut usage = UsedCharacters::from_text(&initial);
        for _ in 0..100 {
            usage.extend(initial.chars());
        }
        assert_eq!(usage.as_set(), &initial.chars().collect::<HashSet<_>>());
        let mut copy = usage.clone();
        for text in ["\u{80}é中文😀\u{10ffff}", "é😀\0\u{7f}", "新"] {
            copy.extend(text.chars());
        }
        let expected: HashSet<_> = initial
            .chars()
            .chain("\u{80}é中文😀\u{10ffff}新".chars())
            .collect();
        assert_eq!(copy.as_set(), &expected);
        assert_eq!(usage.len(), 128, "clone must not share mutable usage");
    }
}
