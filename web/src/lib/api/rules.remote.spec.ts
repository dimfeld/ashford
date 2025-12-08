/**
 * Tests for the Rules API logic.
 *
 * Since remote functions (*.remote.ts) cannot be directly imported in tests
 * due to SvelteKit's internal validation, we test the underlying logic
 * by recreating the API call patterns used in the remote functions.
 *
 * These tests verify:
 * - Correct API endpoint paths for CRUD operations
 * - Request body structure for create/update operations
 * - DELETE operations return void
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { get, post, patch, del } from './client';
import type { DeterministicRule, LlmRule, LabelSummary } from '$lib/types/generated';

// Mock the $env/dynamic/private module
vi.mock('$env/dynamic/private', () => ({
	env: {
		BACKEND_URL: 'http://test-backend:8080'
	}
}));

describe('Rules API Logic', () => {
	let originalFetch: typeof fetch;

	afterEach(() => {
		if (originalFetch) {
			globalThis.fetch = originalFetch;
		}
		vi.clearAllMocks();
	});

	describe('Deterministic Rules', () => {
		const mockDeterministicRule: DeterministicRule = {
			id: 'rule-123',
			org_id: 1,
			user_id: 1,
			name: 'Test Rule',
			description: 'A test rule',
			scope: 'global',
			scope_ref: null,
			priority: 100,
			enabled: true,
			disabled_reason: null,
			conditions_json: { type: 'sender_domain', value: 'test.com' },
			action_type: 'archive',
			action_parameters_json: {},
			safe_mode: 'default',
			created_at: '2024-01-01T00:00:00Z',
			updated_at: '2024-01-01T00:00:00Z'
		};

		it('should call GET /api/rules/deterministic for list', async () => {
			const mockRules: DeterministicRule[] = [mockDeterministicRule];

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockRules)
			});

			const result = await get<DeterministicRule[]>('/api/rules/deterministic');

			expect(result).toEqual(mockRules);
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/deterministic',
				expect.any(Object)
			);
		});

		it('should call GET /api/rules/deterministic/{id} for single rule', async () => {
			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockDeterministicRule)
			});

			const result = await get<DeterministicRule>('/api/rules/deterministic/rule-123');

			expect(result).toEqual(mockDeterministicRule);
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/deterministic/rule-123',
				expect.any(Object)
			);
		});

		it('should call POST /api/rules/deterministic for create', async () => {
			const createInput = {
				name: 'New Rule',
				scope: 'global' as const,
				conditions_json: { type: 'sender_domain', value: 'example.com' },
				action_type: 'archive'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 201,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve({ ...mockDeterministicRule, ...createInput })
			});

			const result = await post<DeterministicRule>('/api/rules/deterministic', createInput);

			expect(result.name).toBe('New Rule');
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/deterministic',
				expect.objectContaining({
					method: 'POST',
					body: JSON.stringify(createInput)
				})
			);
		});

		it('should call PATCH /api/rules/deterministic/{id} for update', async () => {
			const updateInput = {
				enabled: false,
				name: 'Updated Rule'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve({ ...mockDeterministicRule, ...updateInput })
			});

			const result = await patch<DeterministicRule>(
				'/api/rules/deterministic/rule-123',
				updateInput
			);

			expect(result.enabled).toBe(false);
			expect(result.name).toBe('Updated Rule');
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/deterministic/rule-123',
				expect.objectContaining({
					method: 'PATCH',
					body: JSON.stringify(updateInput)
				})
			);
		});

		it('should call DELETE /api/rules/deterministic/{id} for delete', async () => {
			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 204,
				headers: new Headers({ 'content-length': '0' }),
				json: () => Promise.reject(new Error('No content'))
			});

			const result = await del<void>('/api/rules/deterministic/rule-123');

			expect(result).toBeUndefined();
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/deterministic/rule-123',
				expect.objectContaining({
					method: 'DELETE'
				})
			);
		});

		it('should include priority in update for reordering', async () => {
			const updateInput = { priority: 50 };

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve({ ...mockDeterministicRule, priority: 50 })
			});

			const result = await patch<DeterministicRule>(
				'/api/rules/deterministic/rule-123',
				updateInput
			);

			expect(result.priority).toBe(50);
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/deterministic/rule-123',
				expect.objectContaining({
					method: 'PATCH',
					body: JSON.stringify({ priority: 50 })
				})
			);
		});

		it('should call POST /api/rules/deterministic/swap-priorities for atomic swap', async () => {
			const swapInput = {
				rule_a_id: 'rule-123',
				rule_b_id: 'rule-456'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '50' }),
				json: () => Promise.resolve({ success: true })
			});

			const result = await post<{ success: boolean }>(
				'/api/rules/deterministic/swap-priorities',
				swapInput
			);

			expect(result).toEqual({ success: true });
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/deterministic/swap-priorities',
				expect.objectContaining({
					method: 'POST',
					body: JSON.stringify(swapInput)
				})
			);
		});

		it('should throw ApiError for 404 when swapping with non-existent rule', async () => {
			const swapInput = {
				rule_a_id: 'rule-123',
				rule_b_id: 'nonexistent'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: false,
				status: 404,
				statusText: 'Not Found',
				headers: new Headers({ 'content-length': '50' }),
				json: () =>
					Promise.resolve({
						error: 'not_found',
						message: 'Deterministic rule not found: nonexistent'
					})
			});

			await expect(
				post<{ success: boolean }>('/api/rules/deterministic/swap-priorities', swapInput)
			).rejects.toThrow('API request failed: 404 Not Found');
		});

		it('should throw ApiError for 400 when swapping same rule with itself', async () => {
			const swapInput = {
				rule_a_id: 'rule-123',
				rule_b_id: 'rule-123'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: false,
				status: 400,
				statusText: 'Bad Request',
				headers: new Headers({ 'content-length': '50' }),
				json: () =>
					Promise.resolve({
						error: 'bad_request',
						message: "Cannot swap a rule's priority with itself"
					})
			});

			await expect(
				post<{ success: boolean }>('/api/rules/deterministic/swap-priorities', swapInput)
			).rejects.toThrow('API request failed: 400 Bad Request');
		});
	});

	describe('LLM Rules', () => {
		const mockLlmRule: LlmRule = {
			id: 'llm-rule-123',
			org_id: 1,
			user_id: 1,
			name: 'LLM Test Rule',
			description: 'An LLM test rule',
			scope: 'global',
			scope_ref: null,
			rule_text: 'Archive all marketing emails',
			enabled: true,
			metadata_json: {},
			created_at: '2024-01-01T00:00:00Z',
			updated_at: '2024-01-01T00:00:00Z'
		};

		it('should call GET /api/rules/llm for list', async () => {
			const mockRules: LlmRule[] = [mockLlmRule];

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockRules)
			});

			const result = await get<LlmRule[]>('/api/rules/llm');

			expect(result).toEqual(mockRules);
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/llm',
				expect.any(Object)
			);
		});

		it('should call GET /api/rules/llm/{id} for single rule', async () => {
			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockLlmRule)
			});

			const result = await get<LlmRule>('/api/rules/llm/llm-rule-123');

			expect(result).toEqual(mockLlmRule);
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/llm/llm-rule-123',
				expect.any(Object)
			);
		});

		it('should call POST /api/rules/llm for create', async () => {
			const createInput = {
				name: 'New LLM Rule',
				scope: 'global' as const,
				rule_text: 'Mark important emails as starred'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 201,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve({ ...mockLlmRule, ...createInput })
			});

			const result = await post<LlmRule>('/api/rules/llm', createInput);

			expect(result.name).toBe('New LLM Rule');
			expect(result.rule_text).toBe('Mark important emails as starred');
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/llm',
				expect.objectContaining({
					method: 'POST',
					body: JSON.stringify(createInput)
				})
			);
		});

		it('should call PATCH /api/rules/llm/{id} for update', async () => {
			const updateInput = {
				enabled: false,
				rule_text: 'Updated rule text'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve({ ...mockLlmRule, ...updateInput })
			});

			const result = await patch<LlmRule>('/api/rules/llm/llm-rule-123', updateInput);

			expect(result.enabled).toBe(false);
			expect(result.rule_text).toBe('Updated rule text');
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/llm/llm-rule-123',
				expect.objectContaining({
					method: 'PATCH',
					body: JSON.stringify(updateInput)
				})
			);
		});

		it('should call DELETE /api/rules/llm/{id} for delete', async () => {
			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 204,
				headers: new Headers({ 'content-length': '0' }),
				json: () => Promise.reject(new Error('No content'))
			});

			const result = await del<void>('/api/rules/llm/llm-rule-123');

			expect(result).toBeUndefined();
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/llm/llm-rule-123',
				expect.objectContaining({
					method: 'DELETE'
				})
			);
		});
	});

	describe('Labels', () => {
		const mockLabels: LabelSummary[] = [
			{
				id: 'label-1',
				name: 'Important',
				account_id: 'acc-1',
				provider_label_id: 'IMPORTANT',
				label_type: 'system',
				description: null,
				colors: { background_color: '#ff0000', text_color: '#ffffff' }
			},
			{
				id: 'label-2',
				name: 'Work',
				account_id: 'acc-1',
				provider_label_id: 'Label_123',
				label_type: 'user',
				description: 'Work-related emails',
				colors: { background_color: null, text_color: null }
			}
		];

		it('should call GET /api/labels for list', async () => {
			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockLabels)
			});

			const result = await get<LabelSummary[]>('/api/labels');

			expect(result).toEqual(mockLabels);
			expect(result).toHaveLength(2);
			expect(fetch).toHaveBeenCalledWith('http://test-backend:8080/api/labels', expect.any(Object));
		});
	});

	describe('Error handling', () => {
		it('should throw ApiError for 404 response on GET rule', async () => {
			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: false,
				status: 404,
				statusText: 'Not Found',
				headers: new Headers({ 'content-length': '50' }),
				json: () => Promise.resolve({ error: 'not_found', message: 'Rule not found' })
			});

			await expect(get<DeterministicRule>('/api/rules/deterministic/nonexistent')).rejects.toThrow(
				'API request failed: 404 Not Found'
			);
		});

		it('should throw ApiError for 400 response on create with missing fields', async () => {
			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: false,
				status: 400,
				statusText: 'Bad Request',
				headers: new Headers({ 'content-length': '50' }),
				json: () => Promise.resolve({ error: 'bad_request', message: 'Name is required' })
			});

			await expect(post<DeterministicRule>('/api/rules/deterministic', {})).rejects.toThrow(
				'API request failed: 400 Bad Request'
			);
		});

		it('should throw ApiError for 500 response', async () => {
			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: false,
				status: 500,
				statusText: 'Internal Server Error',
				headers: new Headers({ 'content-length': '50' }),
				json: () => Promise.resolve({ error: 'internal_error', message: 'Database error' })
			});

			await expect(get<DeterministicRule[]>('/api/rules/deterministic')).rejects.toThrow(
				'API request failed: 500 Internal Server Error'
			);
		});
	});

	describe('Scope values', () => {
		it('should support global scope', async () => {
			const createInput = {
				name: 'Global Rule',
				scope: 'global' as const,
				conditions_json: {},
				action_type: 'archive'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 201,
				headers: new Headers({ 'content-length': '100' }),
				json: () =>
					Promise.resolve({
						id: 'rule-1',
						org_id: 1,
						user_id: 1,
						...createInput,
						scope_ref: null,
						priority: 100,
						enabled: true,
						disabled_reason: null,
						action_parameters_json: {},
						safe_mode: 'default',
						created_at: '2024-01-01T00:00:00Z',
						updated_at: '2024-01-01T00:00:00Z'
					})
			});

			const result = await post<DeterministicRule>('/api/rules/deterministic', createInput);
			expect(result.scope).toBe('global');
		});

		it('should support account scope with scope_ref', async () => {
			const createInput = {
				name: 'Account Rule',
				scope: 'account' as const,
				scope_ref: 'account-123',
				conditions_json: {},
				action_type: 'archive'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 201,
				headers: new Headers({ 'content-length': '100' }),
				json: () =>
					Promise.resolve({
						id: 'rule-1',
						org_id: 1,
						user_id: 1,
						...createInput,
						priority: 100,
						enabled: true,
						disabled_reason: null,
						action_parameters_json: {},
						safe_mode: 'default',
						created_at: '2024-01-01T00:00:00Z',
						updated_at: '2024-01-01T00:00:00Z'
					})
			});

			const result = await post<DeterministicRule>('/api/rules/deterministic', createInput);
			expect(result.scope).toBe('account');
			expect(result.scope_ref).toBe('account-123');
		});

		it('should support sender scope with scope_ref', async () => {
			const createInput = {
				name: 'Sender Rule',
				scope: 'sender' as const,
				scope_ref: 'sender@example.com',
				conditions_json: {},
				action_type: 'archive'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 201,
				headers: new Headers({ 'content-length': '100' }),
				json: () =>
					Promise.resolve({
						id: 'rule-1',
						org_id: 1,
						user_id: 1,
						...createInput,
						priority: 100,
						enabled: true,
						disabled_reason: null,
						action_parameters_json: {},
						safe_mode: 'default',
						created_at: '2024-01-01T00:00:00Z',
						updated_at: '2024-01-01T00:00:00Z'
					})
			});

			const result = await post<DeterministicRule>('/api/rules/deterministic', createInput);
			expect(result.scope).toBe('sender');
			expect(result.scope_ref).toBe('sender@example.com');
		});

		it('should support domain scope with scope_ref', async () => {
			const createInput = {
				name: 'Domain Rule',
				scope: 'domain' as const,
				scope_ref: 'example.com',
				conditions_json: {},
				action_type: 'archive'
			};

			originalFetch = globalThis.fetch;
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 201,
				headers: new Headers({ 'content-length': '100' }),
				json: () =>
					Promise.resolve({
						id: 'rule-1',
						org_id: 1,
						user_id: 1,
						...createInput,
						priority: 100,
						enabled: true,
						disabled_reason: null,
						action_parameters_json: {},
						safe_mode: 'default',
						created_at: '2024-01-01T00:00:00Z',
						updated_at: '2024-01-01T00:00:00Z'
					})
			});

			const result = await post<DeterministicRule>('/api/rules/deterministic', createInput);
			expect(result.scope).toBe('domain');
			expect(result.scope_ref).toBe('example.com');
		});
	});
});
