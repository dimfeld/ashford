/**
 * Shared formatting functions for action-related UI components.
 * Used by both the actions list page and action detail page.
 */

import type { ActionStatus } from '$lib/types/generated';

/**
 * Format a timestamp string for display in a compact format.
 * Used in list views where space is limited.
 */
export function formatTimestampShort(dateStr: string): string {
	const date = new Date(dateStr);
	return date.toLocaleString(undefined, {
		month: 'short',
		day: 'numeric',
		hour: '2-digit',
		minute: '2-digit'
	});
}

/**
 * Format a timestamp string for display in a full format.
 * Used in detail views where more context is helpful.
 */
export function formatTimestampFull(dateStr: string): string {
	const date = new Date(dateStr);
	return date.toLocaleString(undefined, {
		weekday: 'short',
		month: 'short',
		day: 'numeric',
		year: 'numeric',
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit'
	});
}

/**
 * Format an action type for display (e.g., "apply_label" -> "Apply Label").
 */
export function formatActionType(actionType: string): string {
	return actionType
		.split('_')
		.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
		.join(' ');
}

/**
 * Format a confidence value (0.0-1.0) as a percentage string.
 * Returns 'N/A' for null values.
 */
export function formatConfidence(confidence: number | null): string {
	if (confidence === null) return 'N/A';
	return `${Math.round(confidence * 100)}%`;
}

/**
 * Get the appropriate color class for a confidence value.
 * Red for <50%, yellow for 50-80%, green for >=80%.
 */
export function getConfidenceColor(confidence: number | null): string {
	if (confidence === null) return 'text-muted-foreground';
	const percent = confidence * 100;
	if (percent < 50) return 'text-red-500';
	if (percent < 80) return 'text-yellow-500';
	return 'text-green-500';
}

/**
 * Get the appropriate badge variant for an action status.
 */
export function getStatusVariant(
	status: ActionStatus
): 'default' | 'secondary' | 'destructive' | 'outline' {
	switch (status) {
		case 'completed':
			return 'default';
		case 'queued':
		case 'executing':
		case 'approved_pending':
			return 'secondary';
		case 'failed':
		case 'canceled':
		case 'rejected':
			return 'destructive';
		default:
			return 'outline';
	}
}

/**
 * Get the display label for an action status.
 */
export function getStatusLabel(status: ActionStatus): string {
	switch (status) {
		case 'approved_pending':
			return 'Pending Approval';
		default:
			return status.charAt(0).toUpperCase() + status.slice(1);
	}
}

/**
 * Format sender information for display.
 * Returns name and email if both available, otherwise just name or email.
 */
export function formatSender(email: string | null, name: string | null): string {
	if (name && email) return `${name} <${email}>`;
	if (name) return name;
	if (email) return email;
	return 'Unknown sender';
}

/**
 * Format an object as pretty-printed JSON.
 */
export function formatJson(obj: Record<string, unknown>): string {
	return JSON.stringify(obj, null, 2);
}
