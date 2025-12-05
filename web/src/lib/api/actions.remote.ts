/**
 * Remote functions for Actions API.
 * These execute on the server but can be called transparently from client code.
 */

import { query, command } from '$app/server';
import * as v from 'valibot';
import { get, post, buildQueryString } from './client';
import type {
	ActionListItem,
	ActionDetail,
	PaginatedResponse,
	UndoActionResponse
} from '$lib/types/generated';
import { ACTION_STATUSES, TIME_WINDOWS } from './actions.constants';

// ============================================================================
// Schema Definitions
// ============================================================================

/**
 * Schema for listing actions with filters.
 */
const listActionsInputSchema = v.object({
	/** Time window filter (24h, 7d, 30d) */
	timeWindow: v.optional(v.picklist(TIME_WINDOWS)),
	/** Filter by account ID */
	accountId: v.optional(v.string()),
	/** Filter by sender email or domain */
	sender: v.optional(v.string()),
	/** Filter by action types (comma-separated or array) */
	actionTypes: v.optional(v.union([v.string(), v.array(v.string())])),
	/** Filter by statuses (comma-separated or array) */
	statuses: v.optional(v.union([v.string(), v.array(v.picklist(ACTION_STATUSES))])),
	/** Minimum confidence (0-100) */
	minConfidence: v.optional(v.pipe(v.number(), v.minValue(0), v.maxValue(100))),
	/** Maximum confidence (0-100) */
	maxConfidence: v.optional(v.pipe(v.number(), v.minValue(0), v.maxValue(100))),
	/** Number of items per page */
	limit: v.optional(v.pipe(v.number(), v.integer(), v.minValue(1), v.maxValue(100))),
	/** Page offset (0-based) */
	offset: v.optional(v.pipe(v.number(), v.integer(), v.minValue(0)))
});

type ListActionsInput = v.InferOutput<typeof listActionsInputSchema>;

/**
 * Schema for getting a single action by ID.
 */
const getActionInputSchema = v.object({
	id: v.pipe(v.string(), v.minLength(1))
});

type GetActionInput = v.InferOutput<typeof getActionInputSchema>;

/**
 * Schema for undoing an action.
 */
const undoActionInputSchema = v.object({
	actionId: v.pipe(v.string(), v.minLength(1))
});

type UndoActionInput = v.InferOutput<typeof undoActionInputSchema>;

// ============================================================================
// Query Functions (Read Operations)
// ============================================================================

/**
 * Lists actions with optional filtering and pagination.
 *
 * @example
 * ```svelte
 * <script lang="ts">
 *   import { listActions } from '$lib/api/actions.remote';
 *
 *   let actions = await listActions({
 *     timeWindow: '24h',
 *     statuses: ['completed', 'failed'],
 *     limit: 20
 *   });
 * </script>
 * ```
 */
export const listActions = query(
	listActionsInputSchema,
	async (input: ListActionsInput): Promise<PaginatedResponse<ActionListItem>> => {
		// Convert array values to comma-separated strings for query params
		const actionTypesStr = Array.isArray(input.actionTypes)
			? input.actionTypes.join(',')
			: input.actionTypes;
		const statusesStr = Array.isArray(input.statuses) ? input.statuses.join(',') : input.statuses;

		const queryString = buildQueryString({
			time_window: input.timeWindow,
			account_id: input.accountId,
			sender: input.sender,
			action_type: actionTypesStr,
			status: statusesStr,
			min_confidence: input.minConfidence,
			max_confidence: input.maxConfidence,
			limit: input.limit ?? 20,
			offset: input.offset ?? 0
		});

		return get<PaginatedResponse<ActionListItem>>(`/api/actions${queryString}`);
	}
);

/**
 * Gets detailed information for a single action.
 *
 * @example
 * ```svelte
 * <script lang="ts">
 *   import { getAction } from '$lib/api/actions.remote';
 *
 *   let { id } = $props();
 *   let action = await getAction({ id });
 * </script>
 * ```
 */
export const getAction = query(
	getActionInputSchema,
	async (input: GetActionInput): Promise<ActionDetail> => {
		return get<ActionDetail>(`/api/actions/${input.id}`);
	}
);

// ============================================================================
// Command Functions (Write Operations)
// ============================================================================

/**
 * Requests an undo for a completed action.
 *
 * @example
 * ```svelte
 * <script lang="ts">
 *   import { undoAction } from '$lib/api/actions.remote';
 *
 *   async function handleUndo(actionId: string) {
 *     const result = await undoAction({ actionId });
 *     console.log('Undo queued:', result.undo_action_id);
 *   }
 * </script>
 * ```
 */
export const undoAction = command(
	undoActionInputSchema,
	async (input: UndoActionInput): Promise<UndoActionResponse> => {
		return post<UndoActionResponse>(`/api/actions/${input.actionId}/undo`);
	}
);
