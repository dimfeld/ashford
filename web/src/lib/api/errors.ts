/**
 * Custom error class for API errors with additional context.
 * This is in a separate file from client.ts so it can be imported in browser code.
 */
export class ApiError extends Error {
	constructor(
		message: string,
		public readonly status: number,
		public readonly statusText: string,
		public readonly body?: unknown
	) {
		super(message);
		this.name = 'ApiError';
	}
}
