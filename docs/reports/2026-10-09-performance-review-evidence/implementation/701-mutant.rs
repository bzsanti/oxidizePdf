//! Lossless font usage with a cheap duplicate check for repeated ASCII text.
use std::collections::HashSet;

#[derive(Clone, Default)]
pub(crate) struct UsedCharacters {
    ascii: u128,
    all: HashSet<char>,
}
impl UsedCharacters {
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
            if ch.is_ascii() { self.all.insert(ch); }
        }
    }
}
impl FromIterator<char> for UsedCharacters {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut result = Self {
            ascii: 0,
            all: HashSet::with_capacity(iter.size_hint().0),
        };
        result.extend(iter);
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_ascii_and_unicode_union_remains_lossless_after_clone() {
        let initial: String = (0..=127).filter_map(char::from_u32).collect();
        let mut usage: UsedCharacters = initial.chars().collect();
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
