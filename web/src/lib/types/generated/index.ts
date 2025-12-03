// Auto-generated index file for ts-rs types
// This file re-exports all generated types for convenient imports

export type { AccountState } from './AccountState';
export type { AccountSummary } from './AccountSummary';
export type { Action } from './Action';
export type { ActionLink } from './ActionLink';
export type { ActionLinkRelationType } from './ActionLinkRelationType';
export type { ActionStatus } from './ActionStatus';
export type { Decision } from './Decision';
export type { DecisionSource } from './DecisionSource';
export type { DeterministicRule } from './DeterministicRule';
export type { Header } from './Header';
export type { LabelColors } from './LabelColors';
export type { LabelSummary } from './LabelSummary';
export type { LeafCondition } from './LeafCondition';
export type { LlmRule } from './LlmRule';
export type { LogicalCondition } from './LogicalCondition';
export type { LogicalOperator } from './LogicalOperator';
export type { Mailbox } from './Mailbox';
export type { MessageSummary } from './MessageSummary';
export type { PaginatedResponse } from './PaginatedResponse';
export type { RuleScope } from './RuleScope';
export type { SafeMode } from './SafeMode';
export type { SyncStatus } from './SyncStatus';

// Condition is an untagged enum in Rust that ts-rs doesn't handle well,
// so we manually define the union type here
import type { LogicalCondition } from './LogicalCondition';
import type { LeafCondition } from './LeafCondition';
export type Condition = LogicalCondition | LeafCondition;
