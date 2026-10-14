//! Parse the single-file XML report emitted by veraPDF 1.24+ (`--format xml`).
use crate::error::{PdfError, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::BTreeMap;

type Attributes = BTreeMap<String, String>;

fn malformed(reason: impl std::fmt::Display) -> PdfError {
    PdfError::ExternalValidationError(format!("veraPDF report error: {reason}"))
}

fn attributes(element: &BytesStart<'_>, reader: &Reader<&[u8]>) -> Result<Attributes> {
    element
        .attributes()
        .map(|attribute| {
            let attribute = attribute.map_err(malformed)?;
            let key = std::str::from_utf8(attribute.key.as_ref()).map_err(malformed)?;
            let value = attribute
                .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
                .map_err(malformed)?;
            Ok((key.to_owned(), value.into_owned()))
        })
        .collect()
}

fn number(attrs: &Attributes, key: &str) -> Result<u64> {
    attrs
        .get(key)
        .ok_or_else(|| malformed(format!("missing {key}")))?
        .parse()
        .map_err(|_| malformed(format!("invalid {key}")))
}

#[derive(Default)]
struct Report {
    jobs: usize,
    compliance: Option<bool>,
    details: bool,
    summary: bool,
    totals: Option<(u64, u64)>,
}

impl Report {
    fn element(&mut self, path: &[String], attrs: &Attributes) -> Result<()> {
        let path: Vec<&str> = path.iter().map(String::as_str).collect();
        match path.as_slice() {
            ["report", "jobs", "job"] => {
                self.jobs += 1;
                if self.jobs != 1 {
                    return Err(malformed("expected one validation job"));
                }
            }
            ["report", "jobs", "job", "validationReport"] => {
                if self.compliance.is_some() {
                    return Err(malformed("duplicate validation report"));
                }
                if attrs.get("jobEndStatus").map(String::as_str) != Some("normal") {
                    return Err(malformed("validation job did not finish normally"));
                }
                if attrs.get("profileName").map(String::as_str)
                    != Some("PDF/A-1b validation profile")
                {
                    return Err(malformed("unexpected or absent PDF/A-1b profile"));
                }
                self.compliance = Some(match attrs.get("isCompliant").map(String::as_str) {
                    Some("true") => true,
                    Some("false") => false,
                    _ => return Err(malformed("missing or invalid isCompliant")),
                });
            }
            ["report", "jobs", "job", "validationReport", "details"] => {
                if self.details {
                    return Err(malformed("duplicate validation details"));
                }
                let failed_rules = number(attrs, "failedRules")?;
                let failed_checks = number(attrs, "failedChecks")?;
                let passed_rules = number(attrs, "passedRules")?;
                let passed_checks = number(attrs, "passedChecks")?;
                let compliant = self
                    .compliance
                    .ok_or_else(|| malformed("details without validation report"))?;
                if compliant != (failed_rules == 0 && failed_checks == 0)
                    || (failed_rules == 0) != (failed_checks == 0)
                    || (compliant && (passed_rules == 0 || passed_checks == 0))
                {
                    return Err(malformed("compliance and rule/check counts disagree"));
                }
                self.details = true;
            }
            ["report", "batchSummary"] => {
                if self.summary || number(attrs, "totalJobs")? != 1 {
                    return Err(malformed("expected one batch summary and one job"));
                }
                for key in [
                    "failedToParse",
                    "encrypted",
                    "outOfMemory",
                    "veraExceptions",
                ] {
                    if number(attrs, key)? != 0 {
                        return Err(malformed(format!("batch did not complete: {key}")));
                    }
                }
                self.summary = true;
            }
            ["report", "batchSummary", "validationReports"] => {
                if self.totals.is_some() || number(attrs, "failedJobs")? != 0 {
                    return Err(malformed("duplicate totals or failed validation job"));
                }
                self.totals = Some((number(attrs, "compliant")?, number(attrs, "nonCompliant")?));
            }
            _ => {}
        }
        Ok(())
    }

    fn finish(self) -> Result<bool> {
        let compliant = self
            .compliance
            .ok_or_else(|| malformed("no validation report"))?;
        let expected = if compliant { (1, 0) } else { (0, 1) };
        if self.jobs != 1 || !self.details || !self.summary || self.totals != Some(expected) {
            return Err(malformed("incomplete or inconsistent single-file report"));
        }
        Ok(compliant)
    }
}

pub(super) fn parse_report(xml: &[u8]) -> Result<bool> {
    if xml.len() > 8 * 1024 * 1024 {
        return Err(malformed("report exceeds 8 MiB parsing limit"));
    }
    // Reject invalid UTF-8 rather than accepting a lossy rendition of status fields.
    std::str::from_utf8(xml).map_err(malformed)?;
    let mut reader = Reader::from_reader(xml);
    let mut path = Vec::new();
    let mut root_seen = false;
    let mut report = Report::default();
    loop {
        match reader.read_event().map_err(malformed)? {
            event @ (Event::Start(_) | Event::Empty(_)) => {
                let empty = matches!(event, Event::Empty(_));
                let element = match event {
                    Event::Start(e) | Event::Empty(e) => e,
                    _ => unreachable!(),
                };
                let name = std::str::from_utf8(element.name().as_ref())
                    .map_err(malformed)?
                    .to_owned();
                if path.is_empty() {
                    if root_seen || name != "report" {
                        return Err(malformed("expected one report root"));
                    }
                    root_seen = true;
                }
                if path.len() >= 32 {
                    return Err(malformed("XML nesting limit exceeded"));
                }
                path.push(name);
                report.element(&path, &attributes(&element, &reader)?)?;
                if empty {
                    path.pop();
                }
            }
            Event::End(_) => {
                path.pop().ok_or_else(|| malformed("unexpected end tag"))?;
            }
            Event::DocType(_) => return Err(malformed("DTD is not supported")),
            Event::GeneralRef(reference) => {
                let predefined =
                    matches!(&reference[..], b"amp" | b"lt" | b"gt" | b"quot" | b"apos");
                if path.is_empty()
                    || (!predefined && reference.resolve_char_ref().map_err(malformed)?.is_none())
                {
                    return Err(malformed("undefined entity or reference outside root"));
                }
            }
            Event::Text(text)
                if path.is_empty() && text.iter().any(|b| !b.is_ascii_whitespace()) =>
            {
                return Err(malformed("text outside report root"));
            }
            Event::CData(_) => return Err(malformed("unexpected CDATA")),
            Event::Eof => break,
            _ => {}
        }
    }
    if !path.is_empty() {
        return Err(malformed("truncated XML report"));
    }
    report.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    const VALID: &[u8] = include_bytes!("../../tests/fixtures/issue_729/valid.xml");
    const INVALID: &[u8] = include_bytes!("../../tests/fixtures/issue_729/invalid.xml");

    #[test]
    fn captured_success_with_zero_failures_passes() {
        assert!(parse_report(VALID).unwrap());
    }
    #[test]
    fn escaped_names_and_zero_failed_checks_are_not_failure_words() {
        let valid = std::str::from_utf8(VALID).unwrap();
        let changed = valid.replace(
            "/tmp/oxidize-design-audit/pdfa.pdf",
            "failed &amp; &#65; &lt;Checks&gt;.pdf",
        );
        assert!(parse_report(changed.as_bytes()).unwrap());
    }

    #[test]
    fn captured_noncompliance_is_false_not_execution_error() {
        assert!(!parse_report(INVALID).unwrap());
    }
    #[test]
    fn malformed_or_unrelated_reports_cannot_pass() {
        for xml in [
            b"ValidationProfile".as_slice(),
            b"",
            b"<report/>",
            &VALID[..VALID.len() / 2],
        ] {
            assert!(parse_report(xml).is_err(), "{xml:?}");
        }
    }
    #[test]
    fn duplicate_attributes_entities_and_mismatched_tags_are_errors() {
        let valid = std::str::from_utf8(VALID).unwrap();
        for changed in [
            valid.replace(
                "isCompliant=\"true\"",
                "isCompliant=\"true\" isCompliant=\"false\"",
            ),
            valid.replace("</report>", "</other>"),
            valid.replace(
                "<report>",
                "<!DOCTYPE report [<!ENTITY fake SYSTEM \"file:///etc/passwd\">]><report>",
            ),
            valid.replace("/tmp/oxidize-design-audit/pdfa.pdf", "&undefined;"),
        ] {
            assert!(parse_report(changed.as_bytes()).is_err());
        }
    }

    #[test]
    fn inconsistent_status_counters_or_multiple_reports_are_errors() {
        let valid = std::str::from_utf8(VALID).unwrap();
        for changed in [
            valid.replace("isCompliant=\"true\"", "isCompliant=\"maybe\""),
            valid.replace("failedChecks=\"0\"", "failedChecks=\"1\""),
            valid.replace("jobEndStatus=\"normal\"", "jobEndStatus=\"failed\""),
            valid.replace("totalJobs=\"1\"", "totalJobs=\"2\""),
            valid.replace("outOfMemory=\"0\"", "outOfMemory=\"1\""),
            valid.replace(
                "profileName=\"PDF/A-1b validation profile\"",
                "profileName=\"PDF/A-2b validation profile\"",
            ),
            format!("{valid}{valid}"),
        ] {
            assert!(parse_report(changed.as_bytes()).is_err(), "{changed}");
        }
    }
}
