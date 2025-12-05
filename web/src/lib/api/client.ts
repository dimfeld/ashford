/**
 * API client for communicating with the Rust backend.
 * This module runs server-side only (in remote functions).
 */

import { env } from '$env/dynamic/private';
import { ApiError } from './errors';

// Re-export ApiError for backwards compatibility
export { ApiError };

/**
 * Base URL for the backend API.
 * Configured via BACKEND_URL environment variable, defaults to localhost.
 */
const baseUrl = env.BACKEND_URL || 'http://127.0.0.1:17800';

/**
 * Options for API requests.
 */
export interface RequestOptions {
	/**
	 * Additional headers to include in the request.
	 */
	headers?: Record<string, string>;
	/**
	 * Request timeout in milliseconds. Defaults to 30000 (30 seconds).
	 * Note: This is ignored when `signal` is provided.
	 */
	timeout?: number;
	/**
	 * AbortSignal to cancel the request. When provided, the `timeout` option is ignored.
	 */
	signal?: AbortSignal;
}

/**
 * Performs a fetch request to the backend API with error handling.
 *
 * @param method - HTTP method
 * @param path - API path (will be appended to base URL)
 * @param body - Optional request body (will be JSON-serialized)
 * @param options - Additional request options
 * @returns The parsed JSON response
 * @throws {ApiError} When the request fails or returns a non-2xx status
 */
async function request<T>(
	method: 'GET' | 'POST' | 'PATCH' | 'PUT' | 'DELETE',
	path: string,
	body?: unknown,
	options: RequestOptions = {}
): Promise<T> {
	const url = `${baseUrl}${path}`;
	const { headers = {}, timeout = 30000, signal } = options;

	// Only create timeout controller if user didn't provide their own signal
	let timeoutId: ReturnType<typeof setTimeout> | undefined;
	let requestSignal: AbortSignal;

	if (signal) {
		// Use user-provided signal directly (no timeout applied)
		requestSignal = signal;
	} else {
		// Create our own abort controller with timeout
		const controller = new AbortController();
		timeoutId = setTimeout(() => controller.abort(), timeout);
		requestSignal = controller.signal;
	}

	try {
		const response = await fetch(url, {
			method,
			headers: {
				'Content-Type': 'application/json',
				Accept: 'application/json',
				...headers
			},
			body: body !== undefined ? JSON.stringify(body) : undefined,
			signal: requestSignal
		});

		if (timeoutId !== undefined) {
			clearTimeout(timeoutId);
		}

		if (!response.ok) {
			// Try to parse error body for additional context
			let errorBody: unknown;
			try {
				errorBody = await response.json();
			} catch {
				// Response might not be JSON
				try {
					errorBody = await response.text();
				} catch {
					// Ignore if we can't read the body
				}
			}

			throw new ApiError(
				`API request failed: ${response.status} ${response.statusText}`,
				response.status,
				response.statusText,
				errorBody
			);
		}

		// Handle empty responses (e.g., 204 No Content)
		const contentLength = response.headers.get('content-length');
		if (contentLength === '0' || response.status === 204) {
			return undefined as T;
		}

		return (await response.json()) as T;
	} catch (error) {
		if (timeoutId !== undefined) {
			clearTimeout(timeoutId);
		}

		// Re-throw ApiErrors as-is
		if (error instanceof ApiError) {
			throw error;
		}

		// Handle abort/timeout
		if (error instanceof Error && error.name === 'AbortError') {
			throw new ApiError('Request timed out or was aborted', 0, 'Aborted');
		}

		// Handle network errors
		if (error instanceof TypeError) {
			throw new ApiError(`Network error: ${error.message}`, 0, 'Network Error');
		}

		// Re-throw unknown errors
		throw error;
	}
}

/**
 * Performs a GET request to the backend API.
 *
 * @param path - API path (e.g., '/api/actions')
 * @param options - Additional request options
 * @returns The parsed JSON response
 */
export function get<T>(path: string, options?: RequestOptions): Promise<T> {
	return request<T>('GET', path, undefined, options);
}

/**
 * Performs a POST request to the backend API.
 *
 * @param path - API path (e.g., '/api/actions')
 * @param body - Request body (will be JSON-serialized)
 * @param options - Additional request options
 * @returns The parsed JSON response
 */
export function post<T>(path: string, body?: unknown, options?: RequestOptions): Promise<T> {
	return request<T>('POST', path, body, options);
}

/**
 * Performs a PATCH request to the backend API.
 *
 * @param path - API path (e.g., '/api/rules/deterministic/123')
 * @param body - Request body (will be JSON-serialized)
 * @param options - Additional request options
 * @returns The parsed JSON response
 */
export function patch<T>(path: string, body?: unknown, options?: RequestOptions): Promise<T> {
	return request<T>('PATCH', path, body, options);
}

/**
 * Performs a PUT request to the backend API.
 *
 * @param path - API path
 * @param body - Request body (will be JSON-serialized)
 * @param options - Additional request options
 * @returns The parsed JSON response
 */
export function put<T>(path: string, body?: unknown, options?: RequestOptions): Promise<T> {
	return request<T>('PUT', path, body, options);
}

/**
 * Performs a DELETE request to the backend API.
 *
 * @param path - API path (e.g., '/api/rules/deterministic/123')
 * @param options - Additional request options
 * @returns The parsed JSON response
 */
export function del<T>(path: string, options?: RequestOptions): Promise<T> {
	return request<T>('DELETE', path, undefined, options);
}

/**
 * Builds a query string from an object of parameters.
 * Filters out undefined and null values.
 *
 * @param params - Object of query parameters
 * @returns Query string (including leading '?') or empty string if no params
 */
export function buildQueryString(
	params: Record<string, string | number | boolean | undefined | null>
): string {
	const entries = Object.entries(params)
		.filter(([, value]) => value !== undefined && value !== null)
		.map(([key, value]) => [key, String(value)]);

	if (entries.length === 0) {
		return '';
	}

	const searchParams = new URLSearchParams(entries);
	return `?${searchParams.toString()}`;
}
