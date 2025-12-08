/**
 * Tests for ConditionBuilder utility functions.
 */

import { describe, it, expect } from 'vitest';
import type { LeafCondition, LogicalCondition } from '$lib/types/generated';
import {
	leafToRow,
	rowToLeaf,
	parseConditionsJson,
	buildConditionsJson,
	isLogicalCondition,
	isLeafCondition,
	createEmptyRow,
	type ConditionRow
} from './condition-builder-utils';

describe('ConditionBuilder Utilities', () => {
	// Helper to generate IDs for testing
	let idCounter = 0;
	const generateId = () => `test-id-${idCounter++}`;

	describe('leafToRow', () => {
		it('should convert sender_email leaf to row', () => {
			const leaf: LeafCondition = { type: 'sender_email', value: 'user@example.com' };
			const row = leafToRow(leaf, 'row-1');

			expect(row).toEqual({
				id: 'row-1',
				type: 'sender_email',
				value: 'user@example.com',
				header: '',
				pattern: ''
			});
		});

		it('should convert sender_domain leaf to row', () => {
			const leaf: LeafCondition = { type: 'sender_domain', value: 'example.com' };
			const row = leafToRow(leaf, 'row-2');

			expect(row).toEqual({
				id: 'row-2',
				type: 'sender_domain',
				value: 'example.com',
				header: '',
				pattern: ''
			});
		});

		it('should convert subject_contains leaf to row', () => {
			const leaf: LeafCondition = { type: 'subject_contains', value: 'invoice' };
			const row = leafToRow(leaf, 'row-3');

			expect(row).toEqual({
				id: 'row-3',
				type: 'subject_contains',
				value: 'invoice',
				header: '',
				pattern: ''
			});
		});

		it('should convert subject_regex leaf to row', () => {
			const leaf: LeafCondition = { type: 'subject_regex', value: '^Re:.*' };
			const row = leafToRow(leaf, 'row-4');

			expect(row).toEqual({
				id: 'row-4',
				type: 'subject_regex',
				value: '^Re:.*',
				header: '',
				pattern: ''
			});
		});

		it('should convert header_match leaf to row with header and pattern', () => {
			const leaf: LeafCondition = { type: 'header_match', header: 'X-Priority', pattern: '^1$' };
			const row = leafToRow(leaf, 'row-5');

			expect(row).toEqual({
				id: 'row-5',
				type: 'header_match',
				value: '',
				header: 'X-Priority',
				pattern: '^1$'
			});
		});

		it('should convert label_present leaf to row', () => {
			const leaf: LeafCondition = { type: 'label_present', value: 'label-123' };
			const row = leafToRow(leaf, 'row-6');

			expect(row).toEqual({
				id: 'row-6',
				type: 'label_present',
				value: 'label-123',
				header: '',
				pattern: ''
			});
		});
	});

	describe('rowToLeaf', () => {
		it('should convert sender_email row to leaf', () => {
			const row: ConditionRow = {
				id: 'row-1',
				type: 'sender_email',
				value: '*@amazon.com',
				header: '',
				pattern: ''
			};
			const leaf = rowToLeaf(row);

			expect(leaf).toEqual({ type: 'sender_email', value: '*@amazon.com' });
		});

		it('should convert sender_domain row to leaf', () => {
			const row: ConditionRow = {
				id: 'row-2',
				type: 'sender_domain',
				value: 'amazon.com',
				header: '',
				pattern: ''
			};
			const leaf = rowToLeaf(row);

			expect(leaf).toEqual({ type: 'sender_domain', value: 'amazon.com' });
		});

		it('should convert subject_contains row to leaf', () => {
			const row: ConditionRow = {
				id: 'row-3',
				type: 'subject_contains',
				value: 'urgent',
				header: '',
				pattern: ''
			};
			const leaf = rowToLeaf(row);

			expect(leaf).toEqual({ type: 'subject_contains', value: 'urgent' });
		});

		it('should convert subject_regex row to leaf', () => {
			const row: ConditionRow = {
				id: 'row-4',
				type: 'subject_regex',
				value: '\\bmeeting\\b',
				header: '',
				pattern: ''
			};
			const leaf = rowToLeaf(row);

			expect(leaf).toEqual({ type: 'subject_regex', value: '\\bmeeting\\b' });
		});

		it('should convert header_match row to leaf using header and pattern fields', () => {
			const row: ConditionRow = {
				id: 'row-5',
				type: 'header_match',
				value: '',
				header: 'X-Mailer',
				pattern: 'Outlook'
			};
			const leaf = rowToLeaf(row);

			expect(leaf).toEqual({ type: 'header_match', header: 'X-Mailer', pattern: 'Outlook' });
		});

		it('should convert label_present row to leaf', () => {
			const row: ConditionRow = {
				id: 'row-6',
				type: 'label_present',
				value: 'INBOX',
				header: '',
				pattern: ''
			};
			const leaf = rowToLeaf(row);

			expect(leaf).toEqual({ type: 'label_present', value: 'INBOX' });
		});
	});

	describe('isLogicalCondition', () => {
		it('should return true for valid logical condition with AND operator', () => {
			const condition: LogicalCondition = {
				op: 'and',
				children: [{ type: 'sender_domain', value: 'test.com' }]
			};
			expect(isLogicalCondition(condition)).toBe(true);
		});

		it('should return true for valid logical condition with OR operator', () => {
			const condition: LogicalCondition = {
				op: 'or',
				children: []
			};
			expect(isLogicalCondition(condition)).toBe(true);
		});

		it('should return false for leaf condition', () => {
			const leaf: LeafCondition = { type: 'sender_email', value: 'test@test.com' };
			expect(isLogicalCondition(leaf)).toBe(false);
		});

		it('should return false for empty object', () => {
			expect(isLogicalCondition({})).toBe(false);
		});

		it('should return false for null', () => {
			expect(isLogicalCondition(null as unknown as Record<string, unknown>)).toBe(false);
		});

		it('should return false for array input', () => {
			const arrayInput = [{ op: 'and', children: [] }];
			expect(isLogicalCondition(arrayInput as unknown as Record<string, unknown>)).toBe(false);
		});
	});

	describe('isLeafCondition', () => {
		it('should return true for sender_email condition', () => {
			const leaf: LeafCondition = { type: 'sender_email', value: 'test@test.com' };
			expect(isLeafCondition(leaf)).toBe(true);
		});

		it('should return true for sender_domain condition', () => {
			const leaf: LeafCondition = { type: 'sender_domain', value: 'example.com' };
			expect(isLeafCondition(leaf)).toBe(true);
		});

		it('should return true for subject_contains condition', () => {
			const leaf: LeafCondition = { type: 'subject_contains', value: 'invoice' };
			expect(isLeafCondition(leaf)).toBe(true);
		});

		it('should return true for subject_regex condition', () => {
			const leaf: LeafCondition = { type: 'subject_regex', value: '^Re:.*' };
			expect(isLeafCondition(leaf)).toBe(true);
		});

		it('should return true for header_match condition', () => {
			const leaf: LeafCondition = { type: 'header_match', header: 'X-Custom', pattern: '.*' };
			expect(isLeafCondition(leaf)).toBe(true);
		});

		it('should return true for label_present condition', () => {
			const leaf: LeafCondition = { type: 'label_present', value: 'INBOX' };
			expect(isLeafCondition(leaf)).toBe(true);
		});

		it('should return false for logical condition', () => {
			const condition: LogicalCondition = {
				op: 'and',
				children: []
			};
			expect(isLeafCondition(condition)).toBe(false);
		});

		it('should return false for empty object', () => {
			expect(isLeafCondition({})).toBe(false);
		});

		it('should return false for object with invalid type value', () => {
			const invalidCondition = { type: 'invalid_type', value: 'test' };
			expect(isLeafCondition(invalidCondition)).toBe(false);
		});

		it('should return false for object with type property but not a string', () => {
			const invalidCondition = { type: 123, value: 'test' };
			expect(isLeafCondition(invalidCondition)).toBe(false);
		});

		it('should return false for logical condition with extra type property', () => {
			// This tests the fix for Task 25 - a malformed LogicalCondition with an extra type property
			const malformedCondition = {
				op: 'and',
				children: [],
				type: 'and' // extra type property that shouldn't make it a LeafCondition
			};
			expect(isLeafCondition(malformedCondition)).toBe(false);
		});

		// Additional edge cases for Task 25 - type guard validation
		it('should return false for object with null type property', () => {
			const invalidCondition = { type: null, value: 'test' };
			expect(isLeafCondition(invalidCondition)).toBe(false);
		});

		it('should return false for object with undefined type property', () => {
			const invalidCondition = { type: undefined, value: 'test' };
			expect(isLeafCondition(invalidCondition)).toBe(false);
		});

		it('should return false for object with empty string type', () => {
			const invalidCondition = { type: '', value: 'test' };
			expect(isLeafCondition(invalidCondition)).toBe(false);
		});

		it('should return false for array input', () => {
			const arrayInput = [{ type: 'sender_email', value: 'test@test.com' }];
			expect(isLeafCondition(arrayInput as unknown as Record<string, unknown>)).toBe(false);
		});

		it('should return false for object with type as boolean', () => {
			const invalidCondition = { type: true, value: 'test' };
			expect(isLeafCondition(invalidCondition)).toBe(false);
		});

		it('should return false for object with type as object', () => {
			const invalidCondition = { type: { name: 'sender_email' }, value: 'test' };
			expect(isLeafCondition(invalidCondition)).toBe(false);
		});

		it('should return false for object with type as array', () => {
			const invalidCondition = { type: ['sender_email'], value: 'test' };
			expect(isLeafCondition(invalidCondition)).toBe(false);
		});

		it('should return false for logical condition with type property matching a valid leaf type', () => {
			// Even if type matches a valid leaf type, if it has op and children it's a logical condition
			const malformedCondition = {
				op: 'or',
				children: [{ type: 'sender_email', value: 'test@test.com' }],
				type: 'sender_email' // matches a valid leaf type but this is still a logical condition
			};
			expect(isLeafCondition(malformedCondition)).toBe(false);
		});
	});

	describe('parseConditionsJson', () => {
		it('should return empty rows for empty object', () => {
			const result = parseConditionsJson({}, generateId);

			expect(result.operator).toBe('and');
			expect(result.rows).toEqual([]);
			expect(result.warnings).toEqual([]);
		});

		it('should return empty rows for null-like input', () => {
			const result = parseConditionsJson(null as unknown as Record<string, unknown>, generateId);

			expect(result.operator).toBe('and');
			expect(result.rows).toEqual([]);
			expect(result.warnings).toEqual([]);
		});

		it('should parse single leaf condition', () => {
			const leaf: LeafCondition = { type: 'sender_domain', value: 'amazon.com' };
			const result = parseConditionsJson(leaf, generateId);

			expect(result.operator).toBe('and');
			expect(result.rows).toHaveLength(1);
			expect(result.rows[0].type).toBe('sender_domain');
			expect(result.rows[0].value).toBe('amazon.com');
			expect(result.warnings).toEqual([]);
		});

		it('should parse AND logical condition', () => {
			const condition: LogicalCondition = {
				op: 'and',
				children: [
					{ type: 'sender_domain', value: 'amazon.com' },
					{ type: 'subject_contains', value: 'shipped' }
				]
			};
			const result = parseConditionsJson(condition, generateId);

			expect(result.operator).toBe('and');
			expect(result.rows).toHaveLength(2);
			expect(result.rows[0].type).toBe('sender_domain');
			expect(result.rows[0].value).toBe('amazon.com');
			expect(result.rows[1].type).toBe('subject_contains');
			expect(result.rows[1].value).toBe('shipped');
			expect(result.warnings).toEqual([]);
		});

		it('should parse OR logical condition', () => {
			const condition: LogicalCondition = {
				op: 'or',
				children: [
					{ type: 'sender_email', value: 'boss@work.com' },
					{ type: 'subject_contains', value: 'urgent' }
				]
			};
			const result = parseConditionsJson(condition, generateId);

			expect(result.operator).toBe('or');
			expect(result.rows).toHaveLength(2);
			expect(result.warnings).toEqual([]);
		});

		it('should handle empty children array', () => {
			const condition: LogicalCondition = {
				op: 'and',
				children: []
			};
			const result = parseConditionsJson(condition, generateId);

			expect(result.operator).toBe('and');
			expect(result.rows).toEqual([]);
			expect(result.warnings).toEqual([]);
		});

		it('should flatten nested logical conditions and emit warning', () => {
			// Nested condition: AND(sender_domain, OR(subject_contains, label_present))
			const nestedCondition: LogicalCondition = {
				op: 'and',
				children: [
					{ type: 'sender_domain', value: 'amazon.com' },
					{
						op: 'or',
						children: [
							{ type: 'subject_contains', value: 'shipped' },
							{ type: 'label_present', value: 'Important' }
						]
					}
				]
			};
			const result = parseConditionsJson(nestedCondition, generateId);

			// Should flatten to 3 leaf conditions
			expect(result.operator).toBe('and');
			expect(result.rows).toHaveLength(3);
			expect(result.rows[0].type).toBe('sender_domain');
			expect(result.rows[0].value).toBe('amazon.com');
			expect(result.rows[1].type).toBe('subject_contains');
			expect(result.rows[1].value).toBe('shipped');
			expect(result.rows[2].type).toBe('label_present');
			expect(result.rows[2].value).toBe('Important');

			// Should emit a warning about flattening
			expect(result.warnings).toHaveLength(1);
			expect(result.warnings[0]).toContain('nested condition groups');
		});

		it('should flatten deeply nested logical conditions and emit warning', () => {
			// Deeply nested: AND(leaf, AND(leaf, OR(leaf, leaf)))
			const deeplyNestedCondition: LogicalCondition = {
				op: 'and',
				children: [
					{ type: 'sender_email', value: 'test@test.com' },
					{
						op: 'and',
						children: [
							{ type: 'sender_domain', value: 'example.com' },
							{
								op: 'or',
								children: [
									{ type: 'subject_contains', value: 'urgent' },
									{ type: 'subject_regex', value: '^Re:' }
								]
							}
						]
					}
				]
			};
			const result = parseConditionsJson(deeplyNestedCondition, generateId);

			// Should flatten to 4 leaf conditions
			expect(result.rows).toHaveLength(4);
			expect(result.rows.map((r) => r.type)).toEqual([
				'sender_email',
				'sender_domain',
				'subject_contains',
				'subject_regex'
			]);

			// Should emit a warning
			expect(result.warnings).toHaveLength(1);
		});

		it('should not emit warning for flat logical condition with only leaves', () => {
			const flatCondition: LogicalCondition = {
				op: 'and',
				children: [
					{ type: 'sender_domain', value: 'test.com' },
					{ type: 'subject_contains', value: 'important' },
					{ type: 'label_present', value: 'INBOX' }
				]
			};
			const result = parseConditionsJson(flatCondition, generateId);

			expect(result.rows).toHaveLength(3);
			expect(result.warnings).toEqual([]);
		});

		// Additional edge cases for Task 21 - nested condition flattening
		it('should flatten NOT condition with a single leaf child', () => {
			// NOT conditions wrap a single child
			const notCondition: LogicalCondition = {
				op: 'and',
				children: [
					{ type: 'sender_domain', value: 'example.com' },
					{
						op: 'not',
						children: [{ type: 'label_present', value: 'spam' }]
					}
				]
			};
			const result = parseConditionsJson(notCondition, generateId);

			// Should flatten to 2 leaf conditions (the NOT semantic is lost)
			expect(result.rows).toHaveLength(2);
			expect(result.rows[0].type).toBe('sender_domain');
			expect(result.rows[1].type).toBe('label_present');
			expect(result.warnings).toHaveLength(1);
			expect(result.warnings[0]).toContain('nested condition groups');
		});

		it('should flatten deeply nested NOT conditions', () => {
			// NOT(NOT(leaf)) - double negation
			const doubleNotCondition: LogicalCondition = {
				op: 'and',
				children: [
					{
						op: 'not',
						children: [
							{
								op: 'not',
								children: [{ type: 'subject_contains', value: 'spam' }]
							}
						]
					}
				]
			};
			const result = parseConditionsJson(doubleNotCondition, generateId);

			// Should flatten to 1 leaf condition
			expect(result.rows).toHaveLength(1);
			expect(result.rows[0].type).toBe('subject_contains');
			expect(result.rows[0].value).toBe('spam');
			expect(result.warnings).toHaveLength(1);
		});

		it('should flatten multiple nested logical conditions at the same level', () => {
			// AND(OR(leaf, leaf), OR(leaf, leaf))
			const condition: LogicalCondition = {
				op: 'and',
				children: [
					{
						op: 'or',
						children: [
							{ type: 'sender_domain', value: 'amazon.com' },
							{ type: 'sender_domain', value: 'amazon.co.uk' }
						]
					},
					{
						op: 'or',
						children: [
							{ type: 'subject_contains', value: 'shipped' },
							{ type: 'subject_contains', value: 'delivered' }
						]
					}
				]
			};
			const result = parseConditionsJson(condition, generateId);

			// Should flatten all 4 leaf conditions
			expect(result.rows).toHaveLength(4);
			expect(result.rows.map((r) => r.type)).toEqual([
				'sender_domain',
				'sender_domain',
				'subject_contains',
				'subject_contains'
			]);
			expect(result.warnings).toHaveLength(1);
		});

		it('should handle empty nested logical condition', () => {
			// AND(leaf, OR()) - nested condition with no children
			const condition: LogicalCondition = {
				op: 'and',
				children: [
					{ type: 'sender_domain', value: 'example.com' },
					{
						op: 'or',
						children: []
					}
				]
			};
			const result = parseConditionsJson(condition, generateId);

			// Should have 1 leaf condition and emit warning
			expect(result.rows).toHaveLength(1);
			expect(result.rows[0].type).toBe('sender_domain');
			expect(result.warnings).toHaveLength(1);
		});

		it('should flatten 4+ levels of nesting', () => {
			// AND(AND(AND(AND(leaf))))
			const veryDeeplyNested: LogicalCondition = {
				op: 'and',
				children: [
					{
						op: 'and',
						children: [
							{
								op: 'and',
								children: [
									{
										op: 'and',
										children: [{ type: 'sender_email', value: 'deep@test.com' }]
									}
								]
							}
						]
					}
				]
			};
			const result = parseConditionsJson(veryDeeplyNested, generateId);

			// Should find the leaf at the bottom
			expect(result.rows).toHaveLength(1);
			expect(result.rows[0].type).toBe('sender_email');
			expect(result.rows[0].value).toBe('deep@test.com');
			expect(result.warnings).toHaveLength(1);
		});

		it('should flatten mixed AND/OR nesting at different levels', () => {
			// OR(AND(leaf, leaf), AND(leaf, NOT(leaf)))
			const mixedCondition: LogicalCondition = {
				op: 'or',
				children: [
					{
						op: 'and',
						children: [
							{ type: 'sender_domain', value: 'work.com' },
							{ type: 'subject_contains', value: 'meeting' }
						]
					},
					{
						op: 'and',
						children: [
							{ type: 'sender_email', value: 'boss@work.com' },
							{
								op: 'not',
								children: [{ type: 'label_present', value: 'archived' }]
							}
						]
					}
				]
			};
			const result = parseConditionsJson(mixedCondition, generateId);

			// Should flatten all 4 leaf conditions
			expect(result.rows).toHaveLength(4);
			expect(result.operator).toBe('or'); // top-level operator preserved
			expect(result.warnings).toHaveLength(1);
		});

		it('should handle nested condition where only some children are logical', () => {
			// AND(leaf, OR(leaf, leaf), leaf) - middle child is nested
			const condition: LogicalCondition = {
				op: 'and',
				children: [
					{ type: 'sender_domain', value: 'first.com' },
					{
						op: 'or',
						children: [
							{ type: 'subject_contains', value: 'a' },
							{ type: 'subject_contains', value: 'b' }
						]
					},
					{ type: 'sender_domain', value: 'last.com' }
				]
			};
			const result = parseConditionsJson(condition, generateId);

			// Should flatten to 4 leaves
			expect(result.rows).toHaveLength(4);
			expect(result.rows.map((r) => r.value)).toEqual(['first.com', 'a', 'b', 'last.com']);
			expect(result.warnings).toHaveLength(1);
		});

		it('should return empty rows for unknown format', () => {
			const unknownFormat = { foo: 'bar', baz: 123 };
			const result = parseConditionsJson(unknownFormat, generateId);

			expect(result.operator).toBe('and');
			expect(result.rows).toEqual([]);
			expect(result.warnings).toEqual([]);
		});
	});

	describe('buildConditionsJson', () => {
		it('should return empty AND condition for empty rows', () => {
			const result = buildConditionsJson([], 'and');

			expect(result).toEqual({ op: 'and', children: [] });
		});

		it('should return single leaf for one row', () => {
			const rows: ConditionRow[] = [
				{ id: 'row-1', type: 'sender_domain', value: 'example.com', header: '', pattern: '' }
			];
			const result = buildConditionsJson(rows, 'and');

			expect(result).toEqual({ type: 'sender_domain', value: 'example.com' });
		});

		it('should return logical AND condition for multiple rows', () => {
			const rows: ConditionRow[] = [
				{ id: 'row-1', type: 'sender_domain', value: 'amazon.com', header: '', pattern: '' },
				{ id: 'row-2', type: 'subject_contains', value: 'package', header: '', pattern: '' }
			];
			const result = buildConditionsJson(rows, 'and');

			expect(result).toEqual({
				op: 'and',
				children: [
					{ type: 'sender_domain', value: 'amazon.com' },
					{ type: 'subject_contains', value: 'package' }
				]
			});
		});

		it('should return logical OR condition when operator is or', () => {
			const rows: ConditionRow[] = [
				{ id: 'row-1', type: 'sender_email', value: 'support@amazon.com', header: '', pattern: '' },
				{ id: 'row-2', type: 'sender_email', value: 'noreply@amazon.com', header: '', pattern: '' }
			];
			const result = buildConditionsJson(rows, 'or');

			expect(result).toEqual({
				op: 'or',
				children: [
					{ type: 'sender_email', value: 'support@amazon.com' },
					{ type: 'sender_email', value: 'noreply@amazon.com' }
				]
			});
		});

		it('should handle header_match rows correctly', () => {
			const rows: ConditionRow[] = [
				{ id: 'row-1', type: 'header_match', value: '', header: 'X-Priority', pattern: '1' }
			];
			const result = buildConditionsJson(rows, 'and');

			expect(result).toEqual({ type: 'header_match', header: 'X-Priority', pattern: '1' });
		});
	});

	describe('createEmptyRow', () => {
		it('should create a row with default sender_email type', () => {
			const row = createEmptyRow('new-row');

			expect(row).toEqual({
				id: 'new-row',
				type: 'sender_email',
				value: '',
				header: '',
				pattern: ''
			});
		});
	});

	describe('round-trip conversion', () => {
		it('should preserve data through leaf -> row -> leaf conversion for all types', () => {
			const testCases: LeafCondition[] = [
				{ type: 'sender_email', value: 'test@example.com' },
				{ type: 'sender_domain', value: 'example.com' },
				{ type: 'subject_contains', value: 'test subject' },
				{ type: 'subject_regex', value: '^test.*' },
				{ type: 'header_match', header: 'X-Test', pattern: 'value' },
				{ type: 'label_present', value: 'INBOX' }
			];

			for (const originalLeaf of testCases) {
				const row = leafToRow(originalLeaf, 'test-id');
				const convertedLeaf = rowToLeaf(row);
				expect(convertedLeaf).toEqual(originalLeaf);
			}
		});

		it('should preserve data through parse -> build conversion', () => {
			const originalCondition: LogicalCondition = {
				op: 'and',
				children: [
					{ type: 'sender_domain', value: 'test.com' },
					{ type: 'subject_contains', value: 'important' }
				]
			};

			const parsed = parseConditionsJson(originalCondition, generateId);
			const rebuilt = buildConditionsJson(parsed.rows, parsed.operator);

			expect(rebuilt).toEqual(originalCondition);
		});

		it('should preserve single leaf through parse -> build', () => {
			const originalLeaf: LeafCondition = { type: 'sender_email', value: 'user@test.com' };

			const parsed = parseConditionsJson(originalLeaf, generateId);
			const rebuilt = buildConditionsJson(parsed.rows, parsed.operator);

			expect(rebuilt).toEqual(originalLeaf);
		});
	});
});
