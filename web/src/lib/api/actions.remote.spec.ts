/**
 * Tests for the Actions API logic.
 *
 * Since remote functions (*.remote.ts) cannot be directly imported in tests
 * due to SvelteKit's internal validation, we test the underlying logic
 * by recreating the API call patterns used in the remote functions.
 *
 * These tests verify:
 * - Query string building with proper parameter formatting
 * - Array-to-comma-separated-string conversion for query params
 * - Correct API endpoint paths
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { get, post, buildQueryString } from './client';
import type {
	ActionListItem,
	ActionDetail,
	PaginatedResponse,
	UndoActionResponse
} from '$lib/types/generated';

// Mock the $env/dynamic/private module
vi.mock('$env/dynamic/private', () => ({
	env: {
		BACKEND_URL: 'http://test-backend:8080'
	}
}));

describe('Actions API Logic', () => {
	let originalFetch: typeof fetch;

	afterEach(() => {
		if (originalFetch) {
			globalThis.fetch = originalFetch;
		}
		vi.clearAllMocks();
	});

	/**
	 * This function replicates the query building logic from actions.remote.ts
	 * for the listActions remote function.
	 */
	function buildListActionsQuery(input: {
		timeWindow?: string;
		accountId?: string;
		sender?: string;
		actionTypes?: string | string[];
		statuses?: string | string[];
		minConfidence?: number;
		maxConfidence?: number;
		limit?: number;
		offset?: number;
	}): string {
		// Convert array values to comma-separated strings for query params
		const actionTypesStr = Array.isArray(input.actionTypes)
			? input.actionTypes.join(',')
			: input.actionTypes;
		const statusesStr = Array.isArray(input.statuses) ? input.statuses.join(',') : input.statuses;

		return buildQueryString({
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
	}

	describe('listActions query building', () => {
		it('should build query with default pagination', () => {
			const query = buildListActionsQuery({});
			expect(query).toContain('limit=20');
			expect(query).toContain('offset=0');
		});

		it('should include time_window filter', () => {
			const query = buildListActionsQuery({ timeWindow: '24h' });
			expect(query).toContain('time_window=24h');
		});

		it('should include account_id filter', () => {
			const query = buildListActionsQuery({ accountId: 'acc123' });
			expect(query).toContain('account_id=acc123');
		});

		it('should include sender filter', () => {
			const query = buildListActionsQuery({ sender: 'user@example.com' });
			expect(query).toContain('sender=user%40example.com');
		});

		it('should convert actionTypes array to comma-separated string', () => {
			const query = buildListActionsQuery({ actionTypes: ['apply_label', 'archive'] });
			expect(query).toContain('action_type=apply_label%2Carchive');
		});

		it('should pass actionTypes string as-is', () => {
			const query = buildListActionsQuery({ actionTypes: 'apply_label,archive' });
			expect(query).toContain('action_type=apply_label%2Carchive');
		});

		it('should convert statuses array to comma-separated string', () => {
			const query = buildListActionsQuery({ statuses: ['completed', 'failed'] });
			expect(query).toContain('status=completed%2Cfailed');
		});

		it('should include confidence filters as provided (0-100)', () => {
			const query = buildListActionsQuery({ minConfidence: 50, maxConfidence: 90 });
			expect(query).toContain('min_confidence=50');
			expect(query).toContain('max_confidence=90');
		});

		it('should handle edge case confidence values (0 and 100)', () => {
			const query = buildListActionsQuery({ minConfidence: 0, maxConfidence: 100 });
			expect(query).toContain('min_confidence=0');
			expect(query).toContain('max_confidence=100');
		});

		it('should include custom pagination params', () => {
			const query = buildListActionsQuery({ limit: 50, offset: 100 });
			expect(query).toContain('limit=50');
			expect(query).toContain('offset=100');
		});

		it('should not include undefined filter values in query', () => {
			const query = buildListActionsQuery({
				timeWindow: undefined,
				accountId: undefined,
				sender: undefined
			});
			expect(query).not.toContain('time_window');
			expect(query).not.toContain('account_id');
			expect(query).not.toContain('sender');
		});
	});

	describe('API calls', () => {
		it('should call GET /api/actions with query string', async () => {
			const mockResponse: PaginatedResponse<ActionListItem> = {
				items: [],
				total: 0,
				limit: 20,
				offset: 0,
				has_more: false
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockResponse)
			});

			const queryString = buildListActionsQuery({ timeWindow: '24h' });
			const result = await get<PaginatedResponse<ActionListItem>>(`/api/actions${queryString}`);

			expect(result).toEqual(mockResponse);
			expect(fetch).toHaveBeenCalledWith(
				expect.stringContaining('/api/actions'),
				expect.any(Object)
			);
		});

		it('should call GET /api/actions/{id} for action detail', async () => {
			const mockAction: ActionDetail = {
				id: 'action-123',
				org_id: 1,
				user_id: 1,
				account_id: 'acc-1',
				message_id: 'msg-1',
				decision_id: 'dec-1',
				action_type: 'archive',
				parameters_json: {},
				status: 'completed',
				error_message: null,
				executed_at: '2024-01-01T00:00:00Z',
				undo_hint_json: {},
				trace_id: null,
				created_at: '2024-01-01T00:00:00Z',
				updated_at: '2024-01-01T00:00:00Z',
				decision: null,
				message_subject: 'Test Email',
				message_from_email: 'sender@example.com',
				message_from_name: 'Sender',
				message_snippet: 'Preview text...',
				provider_message_id: null,
				account_email: null,
				can_undo: false,
				gmail_link: null,
				has_been_undone: false,
				undo_action_id: null
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockAction)
			});

			const result = await get<ActionDetail>('/api/actions/action-123');

			expect(result).toEqual(mockAction);
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/actions/action-123',
				expect.any(Object)
			);
		});

		it('should call POST /api/actions/{id}/undo for undo action', async () => {
			const mockResponse: UndoActionResponse = {
				undo_action_id: 'undo-action-456',
				status: 'queued',
				message: 'Undo action queued successfully'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockResponse)
			});

			const result = await post<UndoActionResponse>('/api/actions/action-123/undo');

			expect(result).toEqual(mockResponse);
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/actions/action-123/undo',
				expect.objectContaining({
					method: 'POST'
				})
			);
		});
	});
});
