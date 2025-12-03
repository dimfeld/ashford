/**
 * Tests for the API client module.
 *
 * These tests verify the behavior of the fetch wrapper including:
 * - Successful requests
 * - Error handling (4xx, 5xx, network errors)
 * - Timeout handling
 * - Query string building
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ApiError, get, post, patch, put, del, buildQueryString } from './client';

// Mock the $env/dynamic/private module
vi.mock('$env/dynamic/private', () => ({
	env: {
		BACKEND_URL: 'http://test-backend:8080'
	}
}));

describe('API Client', () => {
	let originalFetch: typeof fetch;

	beforeEach(() => {
		originalFetch = globalThis.fetch;
	});

	afterEach(() => {
		globalThis.fetch = originalFetch;
		vi.clearAllMocks();
	});

	describe('successful requests', () => {
		it('should make a GET request and return parsed JSON', async () => {
			const mockResponse = { id: '123', name: 'Test Action' };
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockResponse)
			});

			const result = await get<typeof mockResponse>('/api/actions/123');

			expect(result).toEqual(mockResponse);
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/actions/123',
				expect.objectContaining({
					method: 'GET',
					headers: expect.objectContaining({
						'Content-Type': 'application/json',
						Accept: 'application/json'
					})
				})
			);
		});

		it('should make a POST request with body', async () => {
			const requestBody = { actionId: '123', reason: 'Test undo' };
			const mockResponse = { undoActionId: '456', status: 'queued' };
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockResponse)
			});

			const result = await post<typeof mockResponse>('/api/actions/123/undo', requestBody);

			expect(result).toEqual(mockResponse);
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/actions/123/undo',
				expect.objectContaining({
					method: 'POST',
					body: JSON.stringify(requestBody)
				})
			);
		});

		it('should make a PATCH request with body', async () => {
			const requestBody = { name: 'Updated Rule' };
			const mockResponse = { id: '123', name: 'Updated Rule' };
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockResponse)
			});

			const result = await patch<typeof mockResponse>('/api/rules/123', requestBody);

			expect(result).toEqual(mockResponse);
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/123',
				expect.objectContaining({
					method: 'PATCH',
					body: JSON.stringify(requestBody)
				})
			);
		});

		it('should make a PUT request with body', async () => {
			const requestBody = { name: 'New Rule', enabled: true };
			const mockResponse = { id: '123', ...requestBody };
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(mockResponse)
			});

			const result = await put<typeof mockResponse>('/api/rules/123', requestBody);

			expect(result).toEqual(mockResponse);
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/123',
				expect.objectContaining({
					method: 'PUT',
					body: JSON.stringify(requestBody)
				})
			);
		});

		it('should make a DELETE request', async () => {
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 204,
				headers: new Headers({ 'content-length': '0' }),
				json: () => Promise.resolve({})
			});

			const result = await del<void>('/api/rules/123');

			expect(result).toBeUndefined();
			expect(fetch).toHaveBeenCalledWith(
				'http://test-backend:8080/api/rules/123',
				expect.objectContaining({
					method: 'DELETE'
				})
			);
		});

		it('should handle 204 No Content responses', async () => {
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 204,
				headers: new Headers({ 'content-length': '0' }),
				json: () => Promise.reject(new Error('No content'))
			});

			const result = await post<void>('/api/actions/123/undo');

			expect(result).toBeUndefined();
		});
	});

	describe('error handling', () => {
		it('should throw ApiError for 4xx responses', async () => {
			const errorBody = { error: 'Not found', message: 'Action not found' };
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: false,
				status: 404,
				statusText: 'Not Found',
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve(errorBody)
			});

			await expect(get('/api/actions/999')).rejects.toThrow(ApiError);

			try {
				await get('/api/actions/999');
			} catch (e) {
				expect(e).toBeInstanceOf(ApiError);
				const error = e as ApiError;
				expect(error.status).toBe(404);
				expect(error.statusText).toBe('Not Found');
				expect(error.body).toEqual(errorBody);
			}
		});

		it('should throw ApiError for 5xx responses', async () => {
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: false,
				status: 500,
				statusText: 'Internal Server Error',
				headers: new Headers({ 'content-length': '0' }),
				json: () => Promise.reject(new Error('No JSON')),
				text: () => Promise.resolve('Server error occurred')
			});

			await expect(get('/api/actions')).rejects.toThrow(ApiError);

			try {
				await get('/api/actions');
			} catch (e) {
				expect(e).toBeInstanceOf(ApiError);
				const error = e as ApiError;
				expect(error.status).toBe(500);
				expect(error.statusText).toBe('Internal Server Error');
				expect(error.body).toBe('Server error occurred');
			}
		});

		it('should throw ApiError for network errors', async () => {
			globalThis.fetch = vi.fn().mockRejectedValue(new TypeError('Failed to fetch'));

			await expect(get('/api/actions')).rejects.toThrow(ApiError);

			try {
				await get('/api/actions');
			} catch (e) {
				expect(e).toBeInstanceOf(ApiError);
				const error = e as ApiError;
				expect(error.status).toBe(0);
				expect(error.statusText).toBe('Network Error');
				expect(error.message).toContain('Network error');
			}
		});

		it('should throw ApiError for abort/timeout', async () => {
			const abortError = new Error('Aborted');
			abortError.name = 'AbortError';
			globalThis.fetch = vi.fn().mockRejectedValue(abortError);

			await expect(get('/api/actions')).rejects.toThrow(ApiError);

			try {
				await get('/api/actions');
			} catch (e) {
				expect(e).toBeInstanceOf(ApiError);
				const error = e as ApiError;
				expect(error.status).toBe(0);
				expect(error.statusText).toBe('Aborted');
				expect(error.message).toContain('timed out or was aborted');
			}
		});

		it('should pass custom headers to the request', async () => {
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve({})
			});

			await get('/api/actions', {
				headers: { 'X-Custom-Header': 'custom-value' }
			});

			expect(fetch).toHaveBeenCalledWith(
				expect.any(String),
				expect.objectContaining({
					headers: expect.objectContaining({
						'X-Custom-Header': 'custom-value'
					})
				})
			);
		});

		it('should use user-provided AbortSignal directly', async () => {
			const controller = new AbortController();
			globalThis.fetch = vi.fn().mockResolvedValue({
				ok: true,
				status: 200,
				headers: new Headers({ 'content-length': '100' }),
				json: () => Promise.resolve({ data: 'test' })
			});

			await get('/api/actions', { signal: controller.signal });

			expect(fetch).toHaveBeenCalledWith(
				expect.any(String),
				expect.objectContaining({
					signal: controller.signal
				})
			);
		});

		it('should abort request when user-provided signal is aborted', async () => {
			const controller = new AbortController();
			const abortError = new Error('Aborted');
			abortError.name = 'AbortError';
			globalThis.fetch = vi.fn().mockRejectedValue(abortError);

			controller.abort();

			await expect(get('/api/actions', { signal: controller.signal })).rejects.toThrow(ApiError);
		});
	});

	describe('buildQueryString', () => {
		it('should build query string from object', () => {
			const params = {
				status: 'completed',
				limit: 10,
				offset: 0
			};

			const result = buildQueryString(params);

			expect(result).toBe('?status=completed&limit=10&offset=0');
		});

		it('should filter out undefined and null values', () => {
			const params = {
				status: 'completed',
				limit: undefined,
				offset: null,
				accountId: 'acc123'
			};

			const result = buildQueryString(params);

			expect(result).toBe('?status=completed&accountId=acc123');
		});

		it('should return empty string for empty params', () => {
			expect(buildQueryString({})).toBe('');
		});

		it('should return empty string when all params are undefined/null', () => {
			const params = {
				a: undefined,
				b: null
			};

			expect(buildQueryString(params)).toBe('');
		});

		it('should handle boolean values', () => {
			const params = {
				enabled: true,
				disabled: false
			};

			const result = buildQueryString(params);

			expect(result).toBe('?enabled=true&disabled=false');
		});
	});
});
