//! OmniDocBench serialization contract v1. Keep the stats copy byte-identical.
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Serialization {
    #[default]
    Normalized,
    Preserved,
}

impl Serialization {
    pub fn parse(id: &str) -> Result<Self, String> {
        match id {
            "rust-split-whitespace-v1" => Ok(Self::Normalized),
            "preserve-text-v1" => Ok(Self::Preserved),
            _ => Err(format!("unknown serialization contract: {id}")),
        }
    }

    pub fn configuration(self) -> Value {
        let id = match self {
            Self::Normalized => "rust-split-whitespace-v1",
            Self::Preserved => "preserve-text-v1",
        };
        json!({"id": id, "encoding": "UTF-8", "appended_newline": false})
    }

    pub fn serialize(self, text: &str) -> String {
        match self {
            Self::Preserved => text.to_owned(),
            Self::Normalized => text
                .split(contract_whitespace)
                .filter(|word| !word.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

// Unicode White_Space as used by the historical Rust split_whitespace run.
// Explicitly pinned: a toolchain's Unicode updates cannot silently change v1.
fn contract_whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{0085}' |
        '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' |
        '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_vectors_match_exact_bytes_and_historical_rust() {
        let vectors: Vec<Value> =
            serde_json::from_str(include_str!("omnidocbench-serialization-v1.json")).unwrap();
        let mut required: std::collections::BTreeSet<String> = [
            "ascii",
            "lines",
            "unicode",
            "unicode-lines",
            "empty",
            "whitespace",
            "non-whitespace",
            "composition",
            "python-controls",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        for code in [
            9, 10, 11, 12, 13, 32, 133, 160, 5760, 8192, 8193, 8194, 8195, 8196, 8197, 8198, 8199,
            8200, 8201, 8202, 8232, 8233, 8239, 8287, 12288,
        ] {
            let name = format!("U+{code:04X}");
            required.insert(name.clone());
            let vector = vectors
                .iter()
                .find(|v| v["name"] == name)
                .expect("required whitespace vector");
            assert_eq!(
                vector["input"],
                format!("A{}B", char::from_u32(code).unwrap())
            );
            assert_eq!(vector["normalized"], "A B");
        }
        let names: std::collections::BTreeSet<_> = vectors
            .iter()
            .map(|v| v["name"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(names, required, "conformance vector inventory changed");
        assert_eq!(names.len(), vectors.len(), "duplicate vector names");
        for (name, input) in [
            ("non-whitespace", "A\u{200b}B\u{feff}C"),
            ("python-controls", "A\u{001c}B\u{001d}C\u{001e}D\u{001f}E"),
            ("composition", "árbol, 中文 e\u{0301}"),
        ] {
            let vector = vectors.iter().find(|v| v["name"] == name).unwrap();
            assert_eq!(vector["input"], input);
            assert_eq!(vector["normalized"], input);
        }
        for vector in vectors {
            let input = vector["input"].as_str().unwrap();
            let expected = vector["normalized"].as_str().unwrap();
            let actual = Serialization::Normalized.serialize(input);
            assert_eq!(actual.as_bytes(), expected.as_bytes(), "{}", vector["name"]);
            assert_eq!(
                input.split_whitespace().collect::<Vec<_>>().join(" "),
                expected
            );
            assert_eq!(
                Serialization::Preserved.serialize(input).as_bytes(),
                input.as_bytes()
            );
            assert_eq!(Serialization::Normalized.serialize(&actual), actual);
        }
    }

    #[test]
    fn unknown_contract_is_rejected() {
        assert!(Serialization::parse("future-v2").is_err());
        assert_eq!(
            Serialization::parse("preserve-text-v1").unwrap(),
            Serialization::Preserved
        );
        assert_eq!(
            Serialization::parse("rust-split-whitespace-v1").unwrap(),
            Serialization::default()
        );
    }
}
