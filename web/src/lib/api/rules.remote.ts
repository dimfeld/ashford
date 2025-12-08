/**
 * Remote functions for Rules API.
 * These execute on the server but can be called transparently from client code.
 */

import { query, command } from '$app/server';
import * as v from 'valibot';
import { get, post, patch, del } from './client';
import type { DeterministicRule, LlmRule, LabelSummary } from '$lib/types/generated';

// ============================================================================
// Schema Definitions
// ============================================================================

/**
 * Schema for empty input (used for list queries with no params).
 */
const emptyInputSchema = v.object({});

/**
 * Schema for getting a single rule by ID.
 */
const getRuleByIdSchema = v.object({
	id: v.pipe(v.string(), v.minLength(1))
});

/**
 * Schema for creating a deterministic rule.
 * Note: scope defaults to 'global' on the backend if not provided.
 */
const createDeterministicRuleSchema = v.object({
	name: v.pipe(v.string(), v.minLength(1, 'Name is required')),
	description: v.optional(v.nullable(v.string())),
	scope: v.optional(v.picklist(['global', 'account', 'sender', 'domain'] as const)),
	scope_ref: v.optional(v.nullable(v.string())),
	priority: v.optional(v.number()),
	enabled: v.optional(v.boolean()),
	conditions_json: v.record(v.string(), v.unknown()),
	action_type: v.pipe(v.string(), v.minLength(1, 'Action type is required')),
	action_parameters_json: v.optional(v.record(v.string(), v.unknown())),
	safe_mode: v.optional(v.picklist(['default', 'always_safe', 'dangerous_override'] as const))
});

export type CreateDeterministicRuleInput = v.InferOutput<typeof createDeterministicRuleSchema>;

/**
 * Schema for updating a deterministic rule.
 */
const updateDeterministicRuleSchema = v.object({
	id: v.pipe(v.string(), v.minLength(1)),
	name: v.optional(v.string()),
	description: v.optional(v.nullable(v.string())),
	scope: v.optional(v.picklist(['global', 'account', 'sender', 'domain'] as const)),
	scope_ref: v.optional(v.nullable(v.string())),
	priority: v.optional(v.number()),
	enabled: v.optional(v.boolean()),
	conditions_json: v.optional(v.record(v.string(), v.unknown())),
	action_type: v.optional(v.string()),
	action_parameters_json: v.optional(v.record(v.string(), v.unknown())),
	safe_mode: v.optional(v.picklist(['default', 'always_safe', 'dangerous_override'] as const))
});

export type UpdateDeterministicRuleInput = v.InferOutput<typeof updateDeterministicRuleSchema>;

/**
 * Schema for deleting a rule.
 */
const deleteRuleSchema = v.object({
	id: v.pipe(v.string(), v.minLength(1))
});

/**
 * Schema for swapping priorities between two deterministic rules.
 */
const swapPrioritiesSchema = v.object({
	rule_a_id: v.pipe(v.string(), v.minLength(1, 'Rule A ID is required')),
	rule_b_id: v.pipe(v.string(), v.minLength(1, 'Rule B ID is required'))
});

export type SwapPrioritiesInput = v.InferOutput<typeof swapPrioritiesSchema>;

/**
 * Schema for creating an LLM rule.
 * Note: scope defaults to 'global' on the backend if not provided.
 */
const createLlmRuleSchema = v.object({
	name: v.pipe(v.string(), v.minLength(1, 'Name is required')),
	description: v.optional(v.nullable(v.string())),
	scope: v.optional(v.picklist(['global', 'account', 'sender', 'domain'] as const)),
	scope_ref: v.optional(v.nullable(v.string())),
	rule_text: v.pipe(v.string(), v.minLength(1, 'Rule text is required')),
	enabled: v.optional(v.boolean()),
	metadata_json: v.optional(v.record(v.string(), v.unknown()))
});

export type CreateLlmRuleInput = v.InferOutput<typeof createLlmRuleSchema>;

/**
 * Schema for updating an LLM rule.
 */
const updateLlmRuleSchema = v.object({
	id: v.pipe(v.string(), v.minLength(1)),
	name: v.optional(v.string()),
	description: v.optional(v.nullable(v.string())),
	scope: v.optional(v.picklist(['global', 'account', 'sender', 'domain'] as const)),
	scope_ref: v.optional(v.nullable(v.string())),
	rule_text: v.optional(v.string()),
	enabled: v.optional(v.boolean()),
	metadata_json: v.optional(v.record(v.string(), v.unknown()))
});

export type UpdateLlmRuleInput = v.InferOutput<typeof updateLlmRuleSchema>;

// ============================================================================
// Query Functions (Read Operations)
// ============================================================================

/**
 * Lists all deterministic rules, sorted by priority ascending.
 */
export const getDeterministicRules = query(
	emptyInputSchema,
	async (): Promise<DeterministicRule[]> => {
		return get<DeterministicRule[]>('/api/rules/deterministic');
	}
);

/**
 * Gets a single deterministic rule by ID.
 */
export const getDeterministicRule = query(
	getRuleByIdSchema,
	async (input: { id: string }): Promise<DeterministicRule> => {
		return get<DeterministicRule>(`/api/rules/deterministic/${input.id}`);
	}
);

/**
 * Lists all LLM rules.
 */
export const getLlmRules = query(emptyInputSchema, async (): Promise<LlmRule[]> => {
	return get<LlmRule[]>('/api/rules/llm');
});

/**
 * Gets a single LLM rule by ID.
 */
export const getLlmRule = query(
	getRuleByIdSchema,
	async (input: { id: string }): Promise<LlmRule> => {
		return get<LlmRule>(`/api/rules/llm/${input.id}`);
	}
);

/**
 * Lists all labels (for condition builder).
 */
export const getLabels = query(emptyInputSchema, async (): Promise<LabelSummary[]> => {
	return get<LabelSummary[]>('/api/labels');
});

// ============================================================================
// Command Functions (Write Operations)
// ============================================================================

/**
 * Creates a new deterministic rule.
 */
export const createDeterministicRule = command(
	createDeterministicRuleSchema,
	async (input: CreateDeterministicRuleInput): Promise<DeterministicRule> => {
		return post<DeterministicRule>('/api/rules/deterministic', input);
	}
);

/**
 * Updates an existing deterministic rule.
 */
export const updateDeterministicRule = command(
	updateDeterministicRuleSchema,
	async (input: UpdateDeterministicRuleInput): Promise<DeterministicRule> => {
		const { id, ...body } = input;
		return patch<DeterministicRule>(`/api/rules/deterministic/${id}`, body);
	}
);

/**
 * Deletes a deterministic rule.
 */
export const deleteDeterministicRule = command(
	deleteRuleSchema,
	async (input: { id: string }): Promise<void> => {
		await del<void>(`/api/rules/deterministic/${input.id}`);
	}
);

/**
 * Creates a new LLM rule.
 */
export const createLlmRule = command(
	createLlmRuleSchema,
	async (input: CreateLlmRuleInput): Promise<LlmRule> => {
		return post<LlmRule>('/api/rules/llm', input);
	}
);

/**
 * Updates an existing LLM rule.
 */
export const updateLlmRule = command(
	updateLlmRuleSchema,
	async (input: UpdateLlmRuleInput): Promise<LlmRule> => {
		const { id, ...body } = input;
		return patch<LlmRule>(`/api/rules/llm/${id}`, body);
	}
);

/**
 * Deletes an LLM rule.
 */
export const deleteLlmRule = command(
	deleteRuleSchema,
	async (input: { id: string }): Promise<void> => {
		await del<void>(`/api/rules/llm/${input.id}`);
	}
);

/**
 * Atomically swaps priorities between two deterministic rules.
 * This ensures both updates succeed or neither does, preventing inconsistent state.
 */
export const swapDeterministicRulePriorities = command(
	swapPrioritiesSchema,
	async (input: SwapPrioritiesInput): Promise<{ success: boolean }> => {
		return post<{ success: boolean }>('/api/rules/deterministic/swap-priorities', input);
	}
);
