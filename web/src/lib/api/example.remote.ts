/**
 * Example remote functions demonstrating query and command patterns.
 *
 * This file serves as a reference implementation for other feature plans.
 * Remote functions execute on the server but can be called from client code.
 *
 * See docs/svelte_remote_functions.md for full documentation.
 */

import { query, command } from '$app/server';
import * as v from 'valibot';
import { get, post, buildQueryString } from './client';
import type { Action, PaginatedResponse, ActionStatus } from '$lib/types/generated';

// ============================================================================
// Schema Definitions
// ============================================================================

/**
 * Schema for listing actions with optional filters.
 * Demonstrates how to define input validation with Valibot.
 */
const listActionsInputSchema = v.object({
	/** Filter by action status */
	status: v.optional(
		v.picklist([
			'queued',
			'executing',
			'completed',
			'failed',
			'canceled',
			'rejected',
			'approved_pending'
		] as const satisfies readonly ActionStatus[])
	),
	/** Filter by account ID */
	accountId: v.optional(v.string()),
	/** Maximum number of results to return */
	limit: v.optional(v.pipe(v.number(), v.integer(), v.minValue(1), v.maxValue(100))),
	/** Offset for pagination */
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
 * Schema for requesting an action undo.
 */
const undoActionInputSchema = v.object({
	actionId: v.pipe(v.string(), v.minLength(1)),
	reason: v.optional(v.string())
});

type UndoActionInput = v.InferOutput<typeof undoActionInputSchema>;

// ============================================================================
// Query Functions (Read Operations)
// ============================================================================

/**
 * Lists actions with optional filtering and pagination.
 *
 * Example usage in a Svelte component:
 * ```svelte
 * <script lang="ts">
 *   import { listActions } from '$lib/api/example.remote';
 *
 *   let actions = await listActions({ status: 'completed', limit: 10 });
 * </script>
 * ```
 *
 * Query functions support automatic caching and can be refreshed:
 * ```svelte
 * <script lang="ts">
 *   import { listActions } from '$lib/api/example.remote';
 *
 *   let actionsQuery = listActions({ limit: 10 });
 *
 *   // Later, to refresh:
 *   actionsQuery.refresh();
 * </script>
 * ```
 */
export const listActions = query(
	listActionsInputSchema,
	async (input: ListActionsInput): Promise<PaginatedResponse<Action>> => {
		const queryString = buildQueryString({
			status: input.status,
			account_id: input.accountId,
			limit: input.limit ?? 20,
			offset: input.offset ?? 0
		});

		return get<PaginatedResponse<Action>>(`/api/actions${queryString}`);
	}
);

/**
 * Gets a single action by ID.
 *
 * Example usage:
 * ```svelte
 * <script lang="ts">
 *   import { getAction } from '$lib/api/example.remote';
 *
 *   let { id } = $props();
 *   let action = await getAction({ id });
 * </script>
 * ```
 */
export const getAction = query(
	getActionInputSchema,
	async (input: GetActionInput): Promise<Action> => {
		return get<Action>(`/api/actions/${input.id}`);
	}
);

// ============================================================================
// Command Functions (Write Operations)
// ============================================================================

/**
 * Response type for undo operation.
 */
interface UndoActionResponse {
	/** ID of the newly created undo action */
	undoActionId: string;
	/** Status of the undo request */
	status: 'queued' | 'rejected';
	/** Message explaining the result */
	message: string;
}

/**
 * Requests an undo for a completed action.
 *
 * Commands are used for write operations that don't need form binding.
 * They can be called from event handlers or effects.
 *
 * Example usage:
 * ```svelte
 * <script lang="ts">
 *   import { undoAction } from '$lib/api/example.remote';
 *
 *   async function handleUndo(actionId: string) {
 *     try {
 *       const result = await undoAction({ actionId, reason: 'User requested undo' });
 *       console.log('Undo queued:', result.undoActionId);
 *     } catch (error) {
 *       console.error('Undo failed:', error);
 *     }
 *   }
 * </script>
 *
 * <button onclick={() => handleUndo('action-123')}>Undo</button>
 * ```
 */
export const undoAction = command(
	undoActionInputSchema,
	async (input: UndoActionInput): Promise<UndoActionResponse> => {
		return post<UndoActionResponse>(`/api/actions/${input.actionId}/undo`, {
			reason: input.reason
		});
	}
);

// ============================================================================
// Health Check (No Input Required)
// ============================================================================

/**
 * Response from the health check endpoint.
 */
interface HealthResponse {
	status: 'ok' | 'degraded' | 'error';
	version: string;
}

/**
 * Checks the backend API health status.
 *
 * Demonstrates a query with no input parameters.
 * Pass an empty object {} when calling.
 *
 * Example usage:
 * ```svelte
 * <script lang="ts">
 *   import { checkHealth } from '$lib/api/example.remote';
 *
 *   let health = await checkHealth({});
 * </script>
 * ```
 */
export const checkHealth = query(v.object({}), async (): Promise<HealthResponse> => {
	return get<HealthResponse>('/healthz');
});
