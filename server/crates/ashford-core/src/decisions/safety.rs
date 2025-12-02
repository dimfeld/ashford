//! Safety enforcement for LLM decisions.
//!
//! The SafetyEnforcer validates LLM decisions against policy constraints
//! and determines whether human approval is required before execution.

use crate::config::PolicyConfig;
use crate::llm::decision::{ActionType, DecisionOutput};

use super::policy::{SafetyOverride, SafetyResult};

/// Enforces safety policies on LLM decisions.
///
/// The enforcer checks multiple conditions and uses OR logic:
/// if any condition triggers an override, approval is required.
#[derive(Debug, Clone)]
pub struct SafetyEnforcer {
    policy: PolicyConfig,
}

impl SafetyEnforcer {
    /// Create a new SafetyEnforcer with the given policy configuration.
    pub fn new(policy: PolicyConfig) -> Self {
        Self { policy }
    }

    /// Enforce safety policies on a decision.
    ///
    /// Checks all policy conditions and returns a SafetyResult indicating
    /// whether approval is required and why.
    ///
    /// The following conditions are checked (OR logic - any triggers approval):
    /// 1. Action is classified as Dangerous
    /// 2. Confidence is below the configured threshold
    /// 3. Action type is in the approval_always list
    /// 4. LLM explicitly requested approval (needs_approval = true)
    pub fn enforce(&self, decision: &DecisionOutput) -> SafetyResult {
        let mut overrides = Vec::new();

        // Check each condition and collect all applicable overrides
        if let Some(override_reason) = self.check_danger_level(decision.decision.action) {
            overrides.push(override_reason);
        }

        if let Some(override_reason) = self.check_confidence(decision.decision.confidence) {
            overrides.push(override_reason);
        }

        if let Some(override_reason) = self.check_approval_always(decision.decision.action) {
            overrides.push(override_reason);
        }

        if let Some(override_reason) = self.check_llm_advisory(decision.decision.needs_approval) {
            overrides.push(override_reason);
        }

        SafetyResult::new(overrides)
    }

    /// Check if the action is classified as dangerous.
    fn check_danger_level(&self, action: ActionType) -> Option<SafetyOverride> {
        if action.danger_level().requires_approval() {
            Some(SafetyOverride::DangerousAction)
        } else {
            None
        }
    }

    /// Check if confidence is below the configured threshold.
    fn check_confidence(&self, confidence: f64) -> Option<SafetyOverride> {
        let threshold = self.policy.confidence_default;
        // Convert f32 threshold to f64 for comparison
        if confidence < threshold as f64 {
            Some(SafetyOverride::LowConfidence {
                confidence,
                threshold,
            })
        } else {
            None
        }
    }

    /// Check if the action type is in the approval_always list.
    fn check_approval_always(&self, action: ActionType) -> Option<SafetyOverride> {
        let action_str = action.as_str();
        if self.policy.approval_always.iter().any(|a| a == action_str) {
            Some(SafetyOverride::InApprovalAlwaysList)
        } else {
            None
        }
    }

    /// Check if the LLM explicitly requested approval.
    fn check_llm_advisory(&self, needs_approval: bool) -> Option<SafetyOverride> {
        if needs_approval {
            Some(SafetyOverride::LlmRequestedApproval)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::decision::{
        DecisionDetails, Explanations, MessageRef, TelemetryPlaceholder, UndoHint,
    };
    use serde_json::json;

    fn default_policy() -> PolicyConfig {
        PolicyConfig {
            approval_always: vec![],
            confidence_default: 0.7,
        }
    }

    fn policy_with_approval_always(actions: Vec<&str>) -> PolicyConfig {
        PolicyConfig {
            approval_always: actions.into_iter().map(String::from).collect(),
            confidence_default: 0.7,
        }
    }

    fn sample_decision_output(
        action: ActionType,
        confidence: f64,
        needs_approval: bool,
    ) -> DecisionOutput {
        DecisionOutput {
            message_ref: MessageRef {
                provider: "gmail".into(),
                account_id: "acc_1".into(),
                thread_id: "thr_1".into(),
                message_id: "msg_1".into(),
            },
            decision: DecisionDetails {
                action,
                parameters: json!({}),
                confidence,
                needs_approval,
                rationale: "Test rationale".into(),
            },
            explanations: Explanations {
                salient_features: vec![],
                matched_directions: vec![],
                considered_alternatives: vec![],
            },
            undo_hint: UndoHint {
                inverse_action: ActionType::None,
                inverse_parameters: json!({}),
            },
            telemetry: TelemetryPlaceholder::default(),
        }
    }

    // ===================
    // Danger level tests
    // ===================

    #[test]
    fn dangerous_action_requires_approval() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::Delete, 0.9, false);

        let result = enforcer.enforce(&decision);

        assert!(result.requires_approval);
        assert!(
            result
                .overrides_applied
                .contains(&SafetyOverride::DangerousAction)
        );
    }

    #[test]
    fn safe_action_does_not_require_approval_alone() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::Archive, 0.9, false);

        let result = enforcer.enforce(&decision);

        assert!(!result.requires_approval);
        assert!(result.overrides_applied.is_empty());
    }

    #[test]
    fn reversible_action_does_not_require_approval_alone() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::Star, 0.9, false);

        let result = enforcer.enforce(&decision);

        assert!(!result.requires_approval);
        assert!(result.overrides_applied.is_empty());
    }

    #[test]
    fn all_dangerous_actions_require_approval() {
        let enforcer = SafetyEnforcer::new(default_policy());

        for action in [
            ActionType::Delete,
            ActionType::Forward,
            ActionType::AutoReply,
            ActionType::Escalate,
        ] {
            let decision = sample_decision_output(action, 0.9, false);
            let result = enforcer.enforce(&decision);
            assert!(
                result.requires_approval,
                "{:?} should require approval",
                action
            );
            assert!(
                result
                    .overrides_applied
                    .contains(&SafetyOverride::DangerousAction),
                "{:?} should have DangerousAction override",
                action
            );
        }
    }

    // =========================
    // Confidence threshold tests
    // =========================

    #[test]
    fn confidence_below_threshold_requires_approval() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::Archive, 0.5, false);

        let result = enforcer.enforce(&decision);

        assert!(result.requires_approval);
        assert!(result.overrides_applied.iter().any(|o| matches!(
            o,
            SafetyOverride::LowConfidence {
                confidence: c,
                threshold: t
            } if (*c - 0.5).abs() < f64::EPSILON && (*t - 0.7).abs() < f32::EPSILON
        )));
    }

    #[test]
    fn confidence_at_threshold_does_not_require_approval() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::Archive, 0.7, false);

        let result = enforcer.enforce(&decision);

        assert!(!result.requires_approval);
        assert!(
            !result
                .overrides_applied
                .iter()
                .any(|o| matches!(o, SafetyOverride::LowConfidence { .. }))
        );
    }

    #[test]
    fn confidence_above_threshold_does_not_require_approval() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::Archive, 0.9, false);

        let result = enforcer.enforce(&decision);

        assert!(!result.requires_approval);
    }

    #[test]
    fn confidence_threshold_respects_policy_value() {
        let mut policy = default_policy();
        policy.confidence_default = 0.9;
        let enforcer = SafetyEnforcer::new(policy);

        // 0.85 is below 0.9 threshold
        let decision = sample_decision_output(ActionType::Archive, 0.85, false);
        let result = enforcer.enforce(&decision);

        assert!(result.requires_approval);
        assert!(result.overrides_applied.iter().any(|o| matches!(
            o,
            SafetyOverride::LowConfidence { threshold: t, .. } if (*t - 0.9).abs() < f32::EPSILON
        )));
    }

    // ==========================
    // approval_always list tests
    // ==========================

    #[test]
    fn action_in_approval_always_requires_approval() {
        let policy = policy_with_approval_always(vec!["archive"]);
        let enforcer = SafetyEnforcer::new(policy);
        let decision = sample_decision_output(ActionType::Archive, 0.9, false);

        let result = enforcer.enforce(&decision);

        assert!(result.requires_approval);
        assert!(
            result
                .overrides_applied
                .contains(&SafetyOverride::InApprovalAlwaysList)
        );
    }

    #[test]
    fn action_not_in_approval_always_follows_normal_rules() {
        let policy = policy_with_approval_always(vec!["delete"]);
        let enforcer = SafetyEnforcer::new(policy);
        let decision = sample_decision_output(ActionType::Archive, 0.9, false);

        let result = enforcer.enforce(&decision);

        assert!(!result.requires_approval);
        assert!(
            !result
                .overrides_applied
                .contains(&SafetyOverride::InApprovalAlwaysList)
        );
    }

    #[test]
    fn approval_always_matches_exact_action_string() {
        let policy = policy_with_approval_always(vec!["apply_label", "mark_read"]);
        let enforcer = SafetyEnforcer::new(policy);

        let decision1 = sample_decision_output(ActionType::ApplyLabel, 0.9, false);
        let result1 = enforcer.enforce(&decision1);
        assert!(result1.requires_approval);

        let decision2 = sample_decision_output(ActionType::MarkRead, 0.9, false);
        let result2 = enforcer.enforce(&decision2);
        assert!(result2.requires_approval);

        // Archive is not in the list
        let decision3 = sample_decision_output(ActionType::Archive, 0.9, false);
        let result3 = enforcer.enforce(&decision3);
        assert!(!result3.requires_approval);
    }

    // ========================
    // LLM advisory flag tests
    // ========================

    #[test]
    fn llm_needs_approval_true_is_honored() {
        let enforcer = SafetyEnforcer::new(default_policy());
        // Safe action with high confidence, but LLM requests approval
        let decision = sample_decision_output(ActionType::Archive, 0.9, true);

        let result = enforcer.enforce(&decision);

        assert!(result.requires_approval);
        assert!(
            result
                .overrides_applied
                .contains(&SafetyOverride::LlmRequestedApproval)
        );
    }

    #[test]
    fn llm_needs_approval_false_does_not_override_policy() {
        let enforcer = SafetyEnforcer::new(default_policy());
        // Dangerous action - LLM says no approval needed, but policy overrides
        let decision = sample_decision_output(ActionType::Delete, 0.9, false);

        let result = enforcer.enforce(&decision);

        assert!(result.requires_approval);
        assert!(
            result
                .overrides_applied
                .contains(&SafetyOverride::DangerousAction)
        );
        assert!(
            !result
                .overrides_applied
                .contains(&SafetyOverride::LlmRequestedApproval)
        );
    }

    // ==========================
    // Combined scenario tests
    // ==========================

    #[test]
    fn multiple_overrides_collected() {
        let policy = policy_with_approval_always(vec!["delete"]);
        let enforcer = SafetyEnforcer::new(policy);
        // Delete (dangerous + in approval_always) with low confidence and LLM approval flag
        let decision = sample_decision_output(ActionType::Delete, 0.5, true);

        let result = enforcer.enforce(&decision);

        assert!(result.requires_approval);
        assert_eq!(result.overrides_applied.len(), 4);
        assert!(
            result
                .overrides_applied
                .contains(&SafetyOverride::DangerousAction)
        );
        assert!(
            result
                .overrides_applied
                .iter()
                .any(|o| matches!(o, SafetyOverride::LowConfidence { .. }))
        );
        assert!(
            result
                .overrides_applied
                .contains(&SafetyOverride::InApprovalAlwaysList)
        );
        assert!(
            result
                .overrides_applied
                .contains(&SafetyOverride::LlmRequestedApproval)
        );
    }

    #[test]
    fn safe_action_high_confidence_no_approval_passes_through() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::Archive, 0.95, false);

        let result = enforcer.enforce(&decision);

        assert!(!result.requires_approval);
        assert!(result.overrides_applied.is_empty());
    }

    #[test]
    fn dangerous_plus_low_confidence() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::Forward, 0.5, false);

        let result = enforcer.enforce(&decision);

        assert!(result.requires_approval);
        assert_eq!(result.overrides_applied.len(), 2);
        assert!(
            result
                .overrides_applied
                .contains(&SafetyOverride::DangerousAction)
        );
        assert!(
            result
                .overrides_applied
                .iter()
                .any(|o| matches!(o, SafetyOverride::LowConfidence { .. }))
        );
    }

    #[test]
    fn edge_case_zero_confidence() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::Archive, 0.0, false);

        let result = enforcer.enforce(&decision);

        assert!(result.requires_approval);
        assert!(result.overrides_applied.iter().any(|o| matches!(
            o,
            SafetyOverride::LowConfidence {
                confidence: c,
                ..
            } if *c == 0.0
        )));
    }

    #[test]
    fn edge_case_max_confidence() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::Archive, 1.0, false);

        let result = enforcer.enforce(&decision);

        assert!(!result.requires_approval);
    }

    #[test]
    fn safety_result_telemetry_json_structure() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::Delete, 0.5, true);

        let result = enforcer.enforce(&decision);
        let telemetry = result.to_telemetry_json();

        // Verify structure
        assert!(telemetry["requires_approval"].as_bool().unwrap());
        assert!(telemetry["safety_overrides"].is_array());
        assert!(telemetry["override_details"].is_array());

        // Verify override strings are human-readable
        let overrides = telemetry["safety_overrides"].as_array().unwrap();
        assert!(!overrides.is_empty());
        assert!(
            overrides
                .iter()
                .any(|o| o.as_str().unwrap().contains("dangerous"))
        );
    }

    // ==========================
    // Additional edge case tests
    // ==========================

    #[test]
    fn all_safe_actions_do_not_require_approval_alone() {
        let enforcer = SafetyEnforcer::new(default_policy());

        for action in [
            ActionType::ApplyLabel,
            ActionType::MarkRead,
            ActionType::MarkUnread,
            ActionType::Archive,
            ActionType::Move,
            ActionType::None,
        ] {
            let decision = sample_decision_output(action, 0.9, false);
            let result = enforcer.enforce(&decision);
            assert!(
                !result.requires_approval,
                "{:?} should not require approval",
                action
            );
            assert!(
                result.overrides_applied.is_empty(),
                "{:?} should have no overrides",
                action
            );
        }
    }

    #[test]
    fn all_reversible_actions_do_not_require_approval_alone() {
        let enforcer = SafetyEnforcer::new(default_policy());

        for action in [
            ActionType::Star,
            ActionType::Unstar,
            ActionType::Snooze,
            ActionType::AddNote,
            ActionType::CreateTask,
        ] {
            let decision = sample_decision_output(action, 0.9, false);
            let result = enforcer.enforce(&decision);
            assert!(
                !result.requires_approval,
                "{:?} should not require approval",
                action
            );
            assert!(
                result.overrides_applied.is_empty(),
                "{:?} should have no overrides",
                action
            );
        }
    }

    #[test]
    fn approval_always_is_case_sensitive() {
        // approval_always should match exactly what ActionType::as_str() returns
        // This tests that "Archive" (wrong case) doesn't match "archive"
        let policy = policy_with_approval_always(vec!["Archive"]); // Wrong case
        let enforcer = SafetyEnforcer::new(policy);

        let decision = sample_decision_output(ActionType::Archive, 0.9, false);
        let result = enforcer.enforce(&decision);

        // Should NOT match because ActionType::Archive.as_str() returns "archive" (lowercase)
        assert!(
            !result
                .overrides_applied
                .contains(&SafetyOverride::InApprovalAlwaysList),
            "approval_always should be case-sensitive"
        );
    }

    #[test]
    fn zero_confidence_threshold_allows_all_confidence_values() {
        let mut policy = default_policy();
        policy.confidence_default = 0.0;
        let enforcer = SafetyEnforcer::new(policy);

        // Even zero confidence should pass when threshold is 0
        let decision = sample_decision_output(ActionType::Archive, 0.0, false);
        let result = enforcer.enforce(&decision);

        assert!(
            !result
                .overrides_applied
                .iter()
                .any(|o| matches!(o, SafetyOverride::LowConfidence { .. })),
            "confidence at threshold 0.0 should not trigger LowConfidence"
        );
    }

    #[test]
    fn max_confidence_threshold_requires_approval_for_anything_below_1() {
        let mut policy = default_policy();
        policy.confidence_default = 1.0;
        let enforcer = SafetyEnforcer::new(policy);

        let decision = sample_decision_output(ActionType::Archive, 0.99, false);
        let result = enforcer.enforce(&decision);

        assert!(result.requires_approval);
        assert!(
            result
                .overrides_applied
                .iter()
                .any(|o| matches!(o, SafetyOverride::LowConfidence { .. })),
            "confidence below 1.0 threshold should trigger LowConfidence"
        );
    }

    #[test]
    fn telemetry_json_empty_when_no_overrides() {
        let result = SafetyResult::approved();
        let telemetry = result.to_telemetry_json();

        assert!(!telemetry["requires_approval"].as_bool().unwrap());
        assert!(telemetry["safety_overrides"].as_array().unwrap().is_empty());
        assert!(telemetry["override_details"].as_array().unwrap().is_empty());
    }

    #[test]
    fn telemetry_json_contains_all_override_types() {
        let result = SafetyResult::new(vec![
            SafetyOverride::DangerousAction,
            SafetyOverride::LowConfidence {
                confidence: 0.5,
                threshold: 0.7,
            },
            SafetyOverride::InApprovalAlwaysList,
            SafetyOverride::LlmRequestedApproval,
        ]);

        let telemetry = result.to_telemetry_json();
        let overrides = telemetry["safety_overrides"].as_array().unwrap();

        assert_eq!(overrides.len(), 4);
        assert!(
            overrides
                .iter()
                .any(|o| o.as_str().unwrap().contains("dangerous"))
        );
        assert!(
            overrides
                .iter()
                .any(|o| o.as_str().unwrap().contains("confidence"))
        );
        assert!(
            overrides
                .iter()
                .any(|o| o.as_str().unwrap().contains("approval_always"))
        );
        assert!(
            overrides
                .iter()
                .any(|o| o.as_str().unwrap().contains("LLM"))
        );

        // Verify override_details contains structured data
        let details = telemetry["override_details"].as_array().unwrap();
        assert_eq!(details.len(), 4);
    }

    #[test]
    fn enforcer_handles_action_none() {
        let enforcer = SafetyEnforcer::new(default_policy());
        let decision = sample_decision_output(ActionType::None, 0.9, false);

        let result = enforcer.enforce(&decision);

        // ActionType::None is classified as Safe
        assert!(!result.requires_approval);
        assert!(result.overrides_applied.is_empty());
    }
}
