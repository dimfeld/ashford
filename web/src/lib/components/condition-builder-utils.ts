/**
 * Utility functions for the ConditionBuilder component.
 * These are extracted to enable unit testing of the pure logic functions.
 */

import type { LeafCondition, LogicalCondition, LogicalOperator } from '$lib/types/generated';

/**
 * Valid leaf condition type values.
 */
const LEAF_CONDITION_TYPES = [
	'sender_email',
	'sender_domain',
	'subject_contains',
	'subject_regex',
	'header_match',
	'label_present'
] as const;

/**
 * Internal representation of a condition row in the UI.
 */
export interface ConditionRow {
	id: string;
	type: LeafCondition['type'];
	value: string;
	// For header_match type
	header: string;
	pattern: string;
}

/**
 * Type for the conditions JSON, which can be a logical condition, leaf condition, or empty object.
 */
export type ConditionsJson = LogicalCondition | LeafCondition | Record<string, unknown>;

/**
 * Result from parsing conditions JSON.
 */
export interface ParsedConditions {
	operator: LogicalOperator;
	rows: ConditionRow[];
	/**
	 * Warnings generated during parsing, such as when nested logical conditions
	 * are detected and flattened.
	 */
	warnings: string[];
}

/**
 * Convert a LeafCondition to a ConditionRow for UI display.
 */
export function leafToRow(leaf: LeafCondition, id: string): ConditionRow {
	switch (leaf.type) {
		case 'header_match':
			return { id, type: 'header_match', value: '', header: leaf.header, pattern: leaf.pattern };
		case 'sender_email':
			return { id, type: 'sender_email', value: leaf.value, header: '', pattern: '' };
		case 'sender_domain':
			return { id, type: 'sender_domain', value: leaf.value, header: '', pattern: '' };
		case 'subject_contains':
			return { id, type: 'subject_contains', value: leaf.value, header: '', pattern: '' };
		case 'subject_regex':
			return { id, type: 'subject_regex', value: leaf.value, header: '', pattern: '' };
		case 'label_present':
			return { id, type: 'label_present', value: leaf.value, header: '', pattern: '' };
	}
}

/**
 * Convert a ConditionRow back to a LeafCondition for API submission.
 */
export function rowToLeaf(row: ConditionRow): LeafCondition {
	switch (row.type) {
		case 'sender_email':
			return { type: 'sender_email', value: row.value };
		case 'sender_domain':
			return { type: 'sender_domain', value: row.value };
		case 'subject_contains':
			return { type: 'subject_contains', value: row.value };
		case 'subject_regex':
			return { type: 'subject_regex', value: row.value };
		case 'header_match':
			return { type: 'header_match', header: row.header, pattern: row.pattern };
		case 'label_present':
			return { type: 'label_present', value: row.value };
	}
}

/**
 * Type guard to check if an object is a LogicalCondition.
 */
export function isLogicalCondition(obj: ConditionsJson): obj is LogicalCondition {
	return (
		typeof obj === 'object' &&
		obj !== null &&
		'op' in obj &&
		'children' in obj &&
		Array.isArray((obj as LogicalCondition).children)
	);
}

/**
 * Type guard to check if an object is a LeafCondition.
 * Validates that the 'type' property is one of the valid leaf condition types
 * and ensures it doesn't have LogicalCondition properties (op, children).
 */
export function isLeafCondition(obj: ConditionsJson): obj is LeafCondition {
	if (typeof obj !== 'object' || obj === null || !('type' in obj)) {
		return false;
	}
	// If it has 'op' and 'children' properties, it's a LogicalCondition, not a LeafCondition
	// even if it happens to have a 'type' property
	if ('op' in obj && 'children' in obj) {
		return false;
	}
	const typeValue = (obj as { type: unknown }).type;
	return (
		typeof typeValue === 'string' &&
		LEAF_CONDITION_TYPES.includes(typeValue as (typeof LEAF_CONDITION_TYPES)[number])
	);
}

/**
 * Recursively flatten all leaf conditions from a condition tree.
 * @param condition - The condition to flatten
 * @returns Array of leaf conditions found in the tree
 */
function flattenConditionToLeaves(condition: LogicalCondition | LeafCondition): LeafCondition[] {
	if (isLeafCondition(condition)) {
		return [condition];
	}

	if (isLogicalCondition(condition)) {
		const leaves: LeafCondition[] = [];
		for (const child of condition.children) {
			leaves.push(...flattenConditionToLeaves(child));
		}
		return leaves;
	}

	return [];
}

/**
 * Check if a logical condition has any nested logical conditions.
 * @param condition - The logical condition to check
 * @returns true if any child is a LogicalCondition
 */
function hasNestedLogicalConditions(condition: LogicalCondition): boolean {
	return condition.children.some((child) => isLogicalCondition(child));
}

/**
 * Parse conditions JSON from the API into the UI representation.
 * @param conditions - The conditions JSON from the API
 * @param generateId - Function to generate unique IDs for rows
 * @returns The parsed operator, rows, and any warnings
 */
export function parseConditionsJson(
	conditions: ConditionsJson,
	generateId: () => string
): ParsedConditions {
	const warnings: string[] = [];

	// Empty or null conditions
	if (!conditions || Object.keys(conditions).length === 0) {
		return { operator: 'and', rows: [], warnings };
	}

	// Check if it's a logical condition (has 'op' and 'children' properties)
	if (isLogicalCondition(conditions)) {
		const operator: LogicalOperator = conditions.op === 'or' ? 'or' : 'and';

		// Check for nested logical conditions
		if (hasNestedLogicalConditions(conditions)) {
			warnings.push(
				'This rule contains nested condition groups (including AND, OR, or NOT operators) which cannot be displayed in the visual editor. ' +
					'The conditions have been flattened into a single list, and NOT conditions have been converted to their positive equivalents (which may invert the rule logic). ' +
					'If you save this rule, the original nested structure will be lost. ' +
					'To preserve the original rule logic, cancel editing and modify the rule via the API instead.'
			);

			// Flatten all nested conditions to leaves
			const leaves = flattenConditionToLeaves(conditions);
			const rows = leaves.map((leaf) => leafToRow(leaf, generateId()));
			return { operator, rows, warnings };
		}

		// No nesting - process normally
		const rows = conditions.children
			.filter((child): child is LeafCondition => isLeafCondition(child))
			.map((child) => leafToRow(child, generateId()));
		return { operator, rows, warnings };
	}

	// Check if it's a single leaf condition
	if (isLeafCondition(conditions)) {
		return {
			operator: 'and',
			rows: [leafToRow(conditions, generateId())],
			warnings
		};
	}

	// Unknown format, return empty
	return { operator: 'and', rows: [], warnings };
}

/**
 * Build the conditions JSON from UI rows for API submission.
 * @param rows - The condition rows from the UI
 * @param operator - The logical operator ('and' or 'or')
 * @returns The conditions JSON ready for API submission
 */
export function buildConditionsJson(
	rows: ConditionRow[],
	operator: LogicalOperator
): ConditionsJson {
	if (rows.length === 0) {
		// Return an empty AND condition
		return { op: 'and', children: [] };
	}

	if (rows.length === 1) {
		// Single condition - return as leaf for simplicity
		return rowToLeaf(rows[0]);
	}

	// Multiple conditions - wrap in logical operator
	const children = rows.map(rowToLeaf);
	return {
		op: operator,
		children
	};
}

/**
 * Create a new empty condition row with default values.
 */
export function createEmptyRow(id: string): ConditionRow {
	return {
		id,
		type: 'sender_email',
		value: '',
		header: '',
		pattern: ''
	};
}
