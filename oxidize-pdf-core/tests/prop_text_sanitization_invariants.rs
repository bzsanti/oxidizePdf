//! Property invariants for carriage-return sanitization (issue #476).
//!
//! The examples in `text_sanitization_test` pin representative strings. These
//! properties range over arbitrary surrounding text and CR run lengths so the
//! policy contract is guarded independently of any one reported document.

use oxidize_pdf::text::{sanitize_extracted_text_with_policy, CarriageReturnHandling};
use proptest::prelude::*;

const POLICIES: [CarriageReturnHandling; 3] = [
    CarriageReturnHandling::Remove,
    CarriageReturnHandling::ReplaceWithSpace,
    CarriageReturnHandling::NormalizeLineEnding,
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    /// Sanitization must remove the ambiguous representation completely and
    /// reaching the fixed point a second time must not change the output.
    #[test]
    fn sanitized_text_contains_no_cr_and_is_idempotent(
        input in proptest::collection::vec(any::<char>(), 0..256)
            .prop_map(|chars| chars.into_iter().collect::<String>()),
        policy in prop::sample::select(POLICIES.to_vec()),
    ) {
        let once = sanitize_extracted_text_with_policy(&input, policy);
        let twice = sanitize_extracted_text_with_policy(&once, policy);

        prop_assert!(!once.contains('\r'), "CR leaked for {policy:?}: {once:?}");
        prop_assert_eq!(once, twice, "sanitization is not idempotent for {:?}", policy);
    }

    /// A standalone CR between printable, non-whitespace fragments is the only
    /// ambiguous case and each public policy gives it one exact interpretation.
    #[test]
    fn standalone_cr_obeys_the_selected_policy(
        left in "[A-Za-z0-9]{1,24}",
        right in "[A-Za-z0-9]{1,24}",
    ) {
        let input = format!("{left}\r{right}");

        prop_assert_eq!(
            sanitize_extracted_text_with_policy(&input, CarriageReturnHandling::Remove),
            format!("{left}{right}")
        );
        prop_assert_eq!(
            sanitize_extracted_text_with_policy(&input, CarriageReturnHandling::ReplaceWithSpace),
            format!("{left} {right}")
        );
        prop_assert_eq!(
            sanitize_extracted_text_with_policy(&input, CarriageReturnHandling::NormalizeLineEnding),
            format!("{left}\n{right}")
        );
    }

    /// CRLF is not ambiguous: it is one line ending under every policy.
    #[test]
    fn crlf_is_one_line_feed_under_every_policy(
        left in "[A-Za-z0-9]{1,24}",
        right in "[A-Za-z0-9]{1,24}",
    ) {
        let input = format!("{left}\r\n{right}");
        let expected = format!("{left}\n{right}");

        for policy in POLICIES {
            prop_assert_eq!(
                sanitize_extracted_text_with_policy(&input, policy),
                expected.as_str(),
                "CRLF contract changed for {:?}",
                policy
            );
        }
    }

    /// Runs of standalone CR exercise deduplication and make accidental
    /// pair-consumption visible: only the space policy collapses the run.
    #[test]
    fn standalone_cr_runs_follow_policy_cardinality(
        left in "[A-Za-z0-9]{1,24}",
        right in "[A-Za-z0-9]{1,24}",
        count in 1usize..64,
    ) {
        let input = format!("{left}{}{right}", "\r".repeat(count));

        prop_assert_eq!(
            sanitize_extracted_text_with_policy(&input, CarriageReturnHandling::Remove),
            format!("{left}{right}")
        );
        prop_assert_eq!(
            sanitize_extracted_text_with_policy(&input, CarriageReturnHandling::ReplaceWithSpace),
            format!("{left} {right}")
        );
        prop_assert_eq!(
            sanitize_extracted_text_with_policy(&input, CarriageReturnHandling::NormalizeLineEnding),
            format!("{left}{}{right}", "\n".repeat(count))
        );
    }
}
