/**
 * Tests for shared action formatting functions.
 */

import { describe, it, expect } from 'vitest';
import {
	formatTimestampShort,
	formatTimestampFull,
	formatActionType,
	formatConfidence,
	getConfidenceColor,
	getStatusVariant,
	getStatusLabel,
	formatSender,
	formatJson
} from './actions';
import type { ActionStatus } from '$lib/types/generated';

describe('Shared Action Formatting Functions', () => {
	describe('formatTimestampShort', () => {
		it('should format a date string in short format', () => {
			// Use a fixed date string and check that the output contains expected patterns
			const dateStr = '2024-03-15T14:30:00Z';
			const result = formatTimestampShort(dateStr);
			// Should include time in HH:MM format
			expect(result).toMatch(/\d{1,2}:\d{2}/);
			// Should include month abbreviation
			expect(result).toMatch(/[A-Z][a-z]{2}/);
			// Should include a numeric day
			expect(result).toMatch(/\d{1,2}/);
		});

		it('should format date correctly according to local timezone', () => {
			const result = formatTimestampShort('2024-12-25T08:00:00Z');
			// The result should match the locale's representation
			// Just verify it's non-empty and has expected format
			expect(result.length).toBeGreaterThan(0);
			expect(result).toMatch(/[A-Z][a-z]{2}/); // Month abbreviation
		});
	});

	describe('formatTimestampFull', () => {
		it('should format a date string in full format', () => {
			const result = formatTimestampFull('2024-03-15T14:30:00Z');
			// Should include weekday
			expect(result).toMatch(/Fri|Thu|Wed/); // Depending on timezone
			// Should include month and day
			expect(result).toMatch(/Mar/);
			expect(result).toMatch(/15/);
			// Should include year
			expect(result).toMatch(/2024/);
			// Should include time with seconds
			expect(result).toMatch(/\d{1,2}:\d{2}:\d{2}/);
		});
	});

	describe('formatActionType', () => {
		it('should capitalize single word action types', () => {
			expect(formatActionType('archive')).toBe('Archive');
		});

		it('should split and capitalize multi-word action types', () => {
			expect(formatActionType('apply_label')).toBe('Apply Label');
		});

		it('should handle multiple underscores', () => {
			expect(formatActionType('mark_as_read')).toBe('Mark As Read');
		});

		it('should handle empty string', () => {
			expect(formatActionType('')).toBe('');
		});
	});

	describe('formatConfidence', () => {
		it('should return N/A for null confidence', () => {
			expect(formatConfidence(null)).toBe('N/A');
		});

		it('should convert 0.0 to 0%', () => {
			expect(formatConfidence(0)).toBe('0%');
		});

		it('should convert 1.0 to 100%', () => {
			expect(formatConfidence(1.0)).toBe('100%');
		});

		it('should convert 0.5 to 50%', () => {
			expect(formatConfidence(0.5)).toBe('50%');
		});

		it('should round to nearest integer', () => {
			expect(formatConfidence(0.333)).toBe('33%');
			expect(formatConfidence(0.666)).toBe('67%');
		});
	});

	describe('getConfidenceColor', () => {
		it('should return muted color for null confidence', () => {
			expect(getConfidenceColor(null)).toBe('text-muted-foreground');
		});

		it('should return red for confidence below 50%', () => {
			expect(getConfidenceColor(0.49)).toBe('text-red-500');
			expect(getConfidenceColor(0.1)).toBe('text-red-500');
			expect(getConfidenceColor(0)).toBe('text-red-500');
		});

		it('should return yellow for confidence between 50% and 80%', () => {
			expect(getConfidenceColor(0.5)).toBe('text-yellow-500');
			expect(getConfidenceColor(0.65)).toBe('text-yellow-500');
			expect(getConfidenceColor(0.79)).toBe('text-yellow-500');
		});

		it('should return green for confidence at or above 80%', () => {
			expect(getConfidenceColor(0.8)).toBe('text-green-500');
			expect(getConfidenceColor(0.9)).toBe('text-green-500');
			expect(getConfidenceColor(1.0)).toBe('text-green-500');
		});
	});

	describe('getStatusVariant', () => {
		it('should return default for completed status', () => {
			expect(getStatusVariant('completed')).toBe('default');
		});

		it('should return secondary for pending statuses', () => {
			expect(getStatusVariant('queued')).toBe('secondary');
			expect(getStatusVariant('executing')).toBe('secondary');
			expect(getStatusVariant('approved_pending')).toBe('secondary');
		});

		it('should return destructive for error/terminal statuses', () => {
			expect(getStatusVariant('failed')).toBe('destructive');
			expect(getStatusVariant('canceled')).toBe('destructive');
			expect(getStatusVariant('rejected')).toBe('destructive');
		});

		it('should return outline for unknown status', () => {
			expect(getStatusVariant('unknown' as ActionStatus)).toBe('outline');
		});
	});

	describe('getStatusLabel', () => {
		it('should return "Pending Approval" for approved_pending', () => {
			expect(getStatusLabel('approved_pending')).toBe('Pending Approval');
		});

		it('should capitalize single-word statuses', () => {
			expect(getStatusLabel('completed')).toBe('Completed');
			expect(getStatusLabel('queued')).toBe('Queued');
			expect(getStatusLabel('failed')).toBe('Failed');
		});
	});

	describe('formatSender', () => {
		it('should format with both name and email', () => {
			expect(formatSender('user@example.com', 'John Doe')).toBe('John Doe <user@example.com>');
		});

		it('should return only name when email is null', () => {
			expect(formatSender(null, 'John Doe')).toBe('John Doe');
		});

		it('should return only email when name is null', () => {
			expect(formatSender('user@example.com', null)).toBe('user@example.com');
		});

		it('should return "Unknown sender" when both are null', () => {
			expect(formatSender(null, null)).toBe('Unknown sender');
		});
	});

	describe('formatJson', () => {
		it('should format empty object', () => {
			expect(formatJson({})).toBe('{}');
		});

		it('should format object with properties', () => {
			const result = formatJson({ key: 'value' });
			expect(result).toBe('{\n  "key": "value"\n}');
		});

		it('should format nested objects', () => {
			const result = formatJson({ outer: { inner: 'value' } });
			expect(result).toContain('"outer"');
			expect(result).toContain('"inner"');
			expect(result).toContain('"value"');
		});
	});
});
