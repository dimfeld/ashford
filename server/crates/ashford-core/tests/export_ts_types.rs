//! Test that exports all TypeScript types when run.
//!
//! Run with: cargo test --test export_ts_types -- --ignored
//!
//! This test is ignored by default so it doesn't run during normal CI.
//! The export_to path is configured in Cargo.toml under [package.metadata.ts-rs].

use std::fs;
use std::path::Path;
use ts_rs::TS;

// Re-export test to trigger ts-rs type generation
#[test]
#[ignore = "Run manually to generate TypeScript types: cargo test --test export_ts_types -- --ignored"]
fn export_typescript_types() {
    // Decision types
    ashford_core::DecisionSource::export_all().expect("DecisionSource");
    ashford_core::ActionStatus::export_all().expect("ActionStatus");
    ashford_core::ActionLinkRelationType::export_all().expect("ActionLinkRelationType");
    ashford_core::Decision::export_all().expect("Decision");
    ashford_core::Action::export_all().expect("Action");
    ashford_core::ActionLink::export_all().expect("ActionLink");

    // Rule types
    ashford_core::RuleScope::export_all().expect("RuleScope");
    ashford_core::SafeMode::export_all().expect("SafeMode");
    ashford_core::DeterministicRule::export_all().expect("DeterministicRule");
    ashford_core::LlmRule::export_all().expect("LlmRule");

    // Condition types (from rules module)
    ashford_core::rules::LogicalOperator::export_all().expect("LogicalOperator");
    ashford_core::rules::LogicalCondition::export_all().expect("LogicalCondition");
    ashford_core::rules::LeafCondition::export_all().expect("LeafCondition");

    // Account types
    ashford_core::SyncStatus::export_all().expect("SyncStatus");
    ashford_core::AccountState::export_all().expect("AccountState");

    // Message types
    ashford_core::Mailbox::export_all().expect("Mailbox");
    ashford_core::gmail::types::Header::export_all().expect("Header");

    // API types
    ashford_core::AccountSummary::export_all().expect("AccountSummary");
    ashford_core::LabelColors::export_all().expect("LabelColors");
    ashford_core::LabelSummary::export_all().expect("LabelSummary");
    ashford_core::MessageSummary::export_all().expect("MessageSummary");
    ashford_core::PaginatedResponse::<String>::export_all().expect("PaginatedResponse");

    // Post-process generated files to fix missing imports
    // ts-rs doesn't add imports for types referenced in #[ts(type = "...")] annotations
    post_process_generated_types();

    println!("TypeScript types exported successfully!");
}

/// Post-process generated TypeScript files to fix known issues with ts-rs output.
///
/// Specifically, this adds missing imports for types referenced in `#[ts(type = "...")]`
/// annotations, which ts-rs doesn't handle automatically.
fn post_process_generated_types() {
    let export_dir = std::env::var("TS_RS_EXPORT_DIR")
        .unwrap_or_else(|_| "../web/src/lib/types/generated".into());
    let export_path = Path::new(&export_dir);

    // Fix LogicalCondition.ts - add missing LeafCondition import
    let logical_condition_path = export_path.join("LogicalCondition.ts");
    if logical_condition_path.exists() {
        fix_logical_condition_import(&logical_condition_path);
    }
}

/// Fix the LogicalCondition.ts file to include the missing LeafCondition import.
///
/// The generated file uses `LeafCondition` in the type annotation (from `#[ts(type = "...")]`)
/// but ts-rs doesn't add the import automatically.
fn fix_logical_condition_import(path: &Path) {
    let content = fs::read_to_string(path).expect("Failed to read LogicalCondition.ts");

    // Check if the fix is already applied (import already exists)
    if content.contains("import type { LeafCondition }") {
        return;
    }

    // Check if LeafCondition is referenced in the file
    if !content.contains("LeafCondition") {
        return;
    }

    // Add the LeafCondition import after the LogicalOperator import
    let fixed_content = content.replace(
        "import type { LogicalOperator } from \"./LogicalOperator\";",
        "import type { LeafCondition } from \"./LeafCondition\";\nimport type { LogicalOperator } from \"./LogicalOperator\";",
    );

    fs::write(path, fixed_content).expect("Failed to write fixed LogicalCondition.ts");
    println!("Fixed missing LeafCondition import in LogicalCondition.ts");
}
