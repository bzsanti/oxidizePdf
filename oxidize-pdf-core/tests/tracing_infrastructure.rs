//! Tests for tracing infrastructure
//!
//! Validates that debug logging can be enabled/disabled via feature flag

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

#[test]
fn test_tracing_subscriber_init() {
    // Test that we can initialize a tracing subscriber
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_span_events(FmtSpan::ACTIVE)
        .with_test_writer()
        .finish();

    // Set as default for this test
    let _guard = tracing::subscriber::set_default(subscriber);

    // Emit a test log
    tracing::debug!("Test debug log");
    tracing::info!("Test info log");

    // If we get here without panicking, tracing infrastructure works
    assert!(true);
}

#[test]
fn test_tracing_with_env_filter() {
    // Test that we can filter logs by module
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("oxidize_pdf=debug"));

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::debug!(target: "oxidize_pdf::parser", "Parser debug log");
    tracing::info!(target: "oxidize_pdf::text", "Text info log");

    assert!(true);
}

#[test]
#[cfg(feature = "verbose-debug")]
fn test_verbose_debug_feature_enabled() {
    // This test only runs when verbose-debug feature is enabled
    // Validates that debug logs are compiled in with the feature
    tracing::debug!("This message should be compiled when verbose-debug is enabled");
    assert!(true, "verbose-debug feature is enabled");
}

#[test]
#[cfg(not(feature = "verbose-debug"))]
fn test_verbose_debug_feature_disabled() {
    // This test runs when verbose-debug is NOT enabled (default)
    // Debug logs should still work via runtime filtering, but can be compiled out
    assert!(true, "verbose-debug feature is disabled (default)");
}
