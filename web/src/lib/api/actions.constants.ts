/**
 * Constants for the Actions API.
 * These are extracted from actions.remote.ts since remote function files
 * can only export remote functions.
 */

import type { ActionStatus } from '$lib/types/generated';

/**
 * All possible action statuses for filtering.
 */
export const ACTION_STATUSES = [
	'queued',
	'executing',
	'completed',
	'failed',
	'canceled',
	'rejected',
	'approved_pending'
] as const satisfies readonly ActionStatus[];

/**
 * Time window options for filtering actions.
 */
export const TIME_WINDOWS = ['24h', '7d', '30d'] as const;

export type TimeWindow = (typeof TIME_WINDOWS)[number];
