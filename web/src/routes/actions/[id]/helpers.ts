/**
 * Helper functions for the action detail page.
 * Re-exports shared formatting functions and provides detail-page-specific helpers.
 */

// Re-export shared formatting functions for backwards compatibility
export {
	formatActionType,
	formatConfidence,
	getConfidenceColor,
	getStatusVariant,
	getStatusLabel,
	formatSender,
	formatJson,
	formatTimestampFull
} from '$lib/formatting/actions';

// Alias for backwards compatibility with existing code
import { formatTimestampFull } from '$lib/formatting/actions';

/**
 * Format a timestamp string for display.
 * @deprecated Use formatTimestampFull from '$lib/formatting/actions' directly
 */
export const formatTimestamp = formatTimestampFull;
