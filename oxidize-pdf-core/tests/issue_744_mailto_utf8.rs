//! RFC 6068 sections 5 and 6.3: UTF-8 bytes and %20, never form-style spaces.
use oxidize_pdf::{actions::UriAction, objects::Object};

#[test]
fn subjects_preserve_utf8_and_component_delimiters() {
    for (subject, expected) in [
        ("", ""),
        ("AZaz09-_.~", "AZaz09-_.~"),
        ("Café €", "Caf%C3%A9%20%E2%82%AC"),
        ("😀 +%&?#", "%F0%9F%98%80%20%2B%25%26%3F%23"),
    ] {
        let action = UriAction::email_with_subject("a@b.c", subject);
        assert_eq!(action.uri, format!("mailto:a@b.c?subject={expected}"));
        assert_eq!(
            action.to_dict().get("URI"),
            Some(&Object::String(action.uri.clone()))
        );
        // Decode hexadecimal octets independently of the production encoder.
        let mut decoded = Vec::new();
        let mut rest = action.uri.strip_prefix("mailto:a@b.c?subject=").unwrap();
        while !rest.is_empty() {
            if rest.starts_with('%') {
                decoded.push(u8::from_str_radix(&rest[1..3], 16).unwrap());
                rest = &rest[3..];
            } else {
                decoded.push(rest.as_bytes()[0]);
                rest = &rest[1..];
            }
        }
        assert_eq!(String::from_utf8(decoded).unwrap(), subject);
    }
}
