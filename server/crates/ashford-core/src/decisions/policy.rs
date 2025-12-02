//! Safety policy definitions for action danger levels and override tracking.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Classification of how dangerous an action is.
/// Used to determine approval requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionDangerLevel {
    /// Safe actions that are unlikely to cause harm.
    /// Examples: archive, apply_label, mark_read
    Safe,
    /// Actions that can be easily undone.
    /// Examples: star, snooze
    Reversible,
    /// Actions that are difficult or impossible to undo.
    /// Examples: delete, forward, auto_reply
    Dangerous,
}

impl ActionDangerLevel {
    /// Returns whether this danger level requires approval by default.
    pub fn requires_approval(&self) -> bool {
        matches!(self, ActionDangerLevel::Dangerous)
    }
}

impl fmt::Display for ActionDangerLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionDangerLevel::Safe => write!(f, "safe"),
            ActionDangerLevel::Reversible => write!(f, "reversible"),
            ActionDangerLevel::Dangerous => write!(f, "dangerous"),
        }
    }
}

/// Reasons why safety enforcement required approval for an action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SafetyOverride {
    /// Action is classified as dangerous (delete, forward, auto_reply, escalate).
    DangerousAction,
    /// Confidence score is below the configured threshold.
    LowConfidence {
        /// The actual confidence from the LLM decision.
        confidence: f64,
        /// The configured threshold that was not met.
        threshold: f32,
    },
    /// Action type is in the approval_always configuration list.
    InApprovalAlwaysList,
    /// The LLM explicitly requested approval via needs_approval=true.
    LlmRequestedApproval,
}

impl fmt::Display for SafetyOverride {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SafetyOverride::DangerousAction => {
                write!(f, "action is classified as dangerous")
            }
            SafetyOverride::LowConfidence {
                confidence,
                threshold,
            } => {
                write!(
                    f,
                    "confidence {:.2} is below threshold {:.2}",
                    confidence, threshold
                )
            }
            SafetyOverride::InApprovalAlwaysList => {
                write!(f, "action type is in approval_always list")
            }
            SafetyOverride::LlmRequestedApproval => {
                write!(f, "LLM explicitly requested approval")
            }
        }
    }
}

/// Result of safety enforcement on a decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyResult {
    /// All safety overrides that were applied.
    /// Empty if the decision passed all checks.
    pub overrides_applied: Vec<SafetyOverride>,
    /// Final determination of whether approval is required.
    /// True if any override was applied (OR logic).
    pub requires_approval: bool,
}

impl SafetyResult {
    /// Create a new SafetyResult from a list of overrides.
    /// `requires_approval` is automatically set based on whether any overrides were applied.
    pub fn new(overrides: Vec<SafetyOverride>) -> Self {
        let requires_approval = !overrides.is_empty();
        Self {
            overrides_applied: overrides,
            requires_approval,
        }
    }

    /// Create a SafetyResult indicating no approval is required.
    pub fn approved() -> Self {
        Self {
            overrides_applied: Vec::new(),
            requires_approval: false,
        }
    }

    /// Convert the safety result to a JSON value for telemetry storage.
    pub fn to_telemetry_json(&self) -> serde_json::Value {
        serde_json::json!({
            "safety_overrides": self.overrides_applied.iter()
                .map(|o| o.to_string())
                .collect::<Vec<_>>(),
            "requires_approval": self.requires_approval,
            "override_details": self.overrides_applied,
        })
    }
}

impl Default for SafetyResult {
    fn default() -> Self {
        Self::approved()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn danger_level_requires_approval() {
        assert!(!ActionDangerLevel::Safe.requires_approval());
        assert!(!ActionDangerLevel::Reversible.requires_approval());
        assert!(ActionDangerLevel::Dangerous.requires_approval());
    }

    #[test]
    fn danger_level_display() {
        assert_eq!(ActionDangerLevel::Safe.to_string(), "safe");
        assert_eq!(ActionDangerLevel::Reversible.to_string(), "reversible");
        assert_eq!(ActionDangerLevel::Dangerous.to_string(), "dangerous");
    }

    #[test]
    fn safety_override_display() {
        assert_eq!(
            SafetyOverride::DangerousAction.to_string(),
            "action is classified as dangerous"
        );

        assert_eq!(
            SafetyOverride::LowConfidence {
                confidence: 0.45,
                threshold: 0.7
            }
            .to_string(),
            "confidence 0.45 is below threshold 0.70"
        );

        assert_eq!(
            SafetyOverride::InApprovalAlwaysList.to_string(),
            "action type is in approval_always list"
        );

        assert_eq!(
            SafetyOverride::LlmRequestedApproval.to_string(),
            "LLM explicitly requested approval"
        );
    }

    #[test]
    fn safety_result_new_sets_requires_approval() {
        let empty = SafetyResult::new(vec![]);
        assert!(!empty.requires_approval);
        assert!(empty.overrides_applied.is_empty());

        let with_override = SafetyResult::new(vec![SafetyOverride::DangerousAction]);
        assert!(with_override.requires_approval);
        assert_eq!(with_override.overrides_applied.len(), 1);
    }

    #[test]
    fn safety_result_approved() {
        let result = SafetyResult::approved();
        assert!(!result.requires_approval);
        assert!(result.overrides_applied.is_empty());
    }

    #[test]
    fn safety_result_to_telemetry_json() {
        let result = SafetyResult::new(vec![
            SafetyOverride::DangerousAction,
            SafetyOverride::LowConfidence {
                confidence: 0.5,
                threshold: 0.7,
            },
        ]);

        let json = result.to_telemetry_json();

        assert_eq!(json["requires_approval"], true);

        let overrides = json["safety_overrides"].as_array().unwrap();
        assert_eq!(overrides.len(), 2);
        assert_eq!(overrides[0], "action is classified as dangerous");
        assert_eq!(overrides[1], "confidence 0.50 is below threshold 0.70");

        // Check detailed override info is also present
        let details = json["override_details"].as_array().unwrap();
        assert_eq!(details.len(), 2);
    }

    #[test]
    fn safety_override_serialization() {
        // Test serialization with tag
        let override_json = serde_json::to_value(&SafetyOverride::DangerousAction).unwrap();
        assert_eq!(override_json["type"], "dangerous_action");

        let low_conf = SafetyOverride::LowConfidence {
            confidence: 0.5,
            threshold: 0.7,
        };
        let low_conf_json = serde_json::to_value(&low_conf).unwrap();
        assert_eq!(low_conf_json["type"], "low_confidence");
        assert_eq!(low_conf_json["confidence"], 0.5);
        // f32 threshold may have precision differences when serialized
        let threshold = low_conf_json["threshold"].as_f64().unwrap();
        assert!((threshold - 0.7).abs() < 0.001, "threshold should be ~0.7");
    }
}
