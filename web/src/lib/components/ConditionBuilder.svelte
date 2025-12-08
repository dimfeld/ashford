<script lang="ts">
	import { onMount } from 'svelte';
	import type { LeafCondition, LabelSummary, LogicalOperator } from '$lib/types/generated';
	import * as Select from '$lib/components/ui/select';
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import { Label } from '$lib/components/ui/label';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import TrashIcon from '@lucide/svelte/icons/trash-2';

	import {
		parseConditionsJson,
		buildConditionsJson as buildJson,
		createEmptyRow,
		type ConditionRow,
		type ConditionsJson
	} from './condition-builder-utils';

	// ============================================================================
	// Types
	// ============================================================================

	type ConditionType = LeafCondition['type'];

	// ============================================================================
	// Props
	// ============================================================================

	interface Props {
		/** Initial conditions JSON to populate the builder */
		conditions?: ConditionsJson;
		/** Available labels for the label_present condition type */
		labels?: LabelSummary[];
		/** Callback when conditions change */
		onchange?: (conditionsJson: ConditionsJson) => void;
		/** Callback when warnings are generated during parsing (e.g., nested conditions flattened) */
		onwarnings?: (warnings: string[]) => void;
	}

	let { conditions = {}, labels = [], onchange, onwarnings }: Props = $props();

	// ============================================================================
	// State
	// ============================================================================

	let logicalOperator = $state<LogicalOperator>('and');
	let conditionRows = $state<ConditionRow[]>([]);

	// Unique ID counter for rows
	let idCounter = $state(0);
	const generateId = () => `condition-${idCounter++}`;

	// ============================================================================
	// Condition Type Definitions
	// ============================================================================

	const conditionTypes: { value: ConditionType; label: string; description: string }[] = [
		{ value: 'sender_email', label: 'Sender Email', description: 'Match sender email address' },
		{ value: 'sender_domain', label: 'Sender Domain', description: 'Match sender domain' },
		{
			value: 'subject_contains',
			label: 'Subject Contains',
			description: 'Subject contains text'
		},
		{ value: 'subject_regex', label: 'Subject Regex', description: 'Subject matches regex' },
		{ value: 'header_match', label: 'Header Match', description: 'Header matches regex' },
		{ value: 'label_present', label: 'Label Present', description: 'Email has label' }
	];

	// ============================================================================
	// Initialization
	// ============================================================================

	function initializeFromConditions() {
		const parsed = parseConditionsJson(conditions, generateId);
		logicalOperator = parsed.operator;
		conditionRows = parsed.rows;
		// Emit any warnings from parsing (e.g., nested conditions flattened)
		if (parsed.warnings.length > 0) {
			onwarnings?.(parsed.warnings);
		}
	}

	// Initialize on mount and emit initial state
	onMount(() => {
		initializeFromConditions();
		// Emit the initial state so parent has normalized conditions
		emitChange();
	});

	// ============================================================================
	// Row Operations
	// ============================================================================

	function addCondition() {
		conditionRows = [...conditionRows, createEmptyRow(generateId())];
		emitChange();
	}

	function removeCondition(id: string) {
		conditionRows = conditionRows.filter((row) => row.id !== id);
		emitChange();
	}

	function updateConditionType(id: string, newType: ConditionType) {
		conditionRows = conditionRows.map((row) => {
			if (row.id === id) {
				return { ...row, type: newType, value: '', header: '', pattern: '' };
			}
			return row;
		});
		emitChange();
	}

	function updateConditionValue(
		id: string,
		field: 'value' | 'header' | 'pattern',
		newValue: string
	) {
		conditionRows = conditionRows.map((row) => {
			if (row.id === id) {
				return { ...row, [field]: newValue };
			}
			return row;
		});
		emitChange();
	}

	function toggleOperator() {
		logicalOperator = logicalOperator === 'and' ? 'or' : 'and';
		emitChange();
	}

	// ============================================================================
	// Output Generation
	// ============================================================================

	function emitChange() {
		onchange?.(buildJson(conditionRows, logicalOperator));
	}

	// ============================================================================
	// Helpers
	// ============================================================================

	function getConditionTypeLabel(type: ConditionType): string {
		return conditionTypes.find((ct) => ct.value === type)?.label ?? type;
	}

	function getLabelName(labelId: string): string {
		return labels.find((l) => l.id === labelId)?.name ?? labelId;
	}
</script>

<div class="space-y-4">
	<!-- Logical Operator Toggle -->
	{#if conditionRows.length > 1}
		<div class="flex items-center gap-3 rounded-md bg-muted/50 p-3">
			<span class="text-sm font-medium">Match</span>
			<Button
				variant={logicalOperator === 'and' ? 'default' : 'outline'}
				size="sm"
				onclick={toggleOperator}
			>
				ALL conditions
			</Button>
			<span class="text-sm text-muted-foreground">or</span>
			<Button
				variant={logicalOperator === 'or' ? 'default' : 'outline'}
				size="sm"
				onclick={toggleOperator}
			>
				ANY condition
			</Button>
		</div>
	{/if}

	<!-- Condition Rows -->
	<div class="space-y-3">
		{#each conditionRows as row (row.id)}
			<div class="flex flex-wrap items-start gap-3 rounded-md border p-3">
				<!-- Condition Type Dropdown -->
				<div class="w-48">
					<Label class="mb-1.5 text-xs text-muted-foreground">Condition Type</Label>
					<Select.Root
						type="single"
						value={row.type}
						onValueChange={(value) => {
							if (value) updateConditionType(row.id, value as ConditionType);
						}}
					>
						<Select.Trigger class="w-full">
							{getConditionTypeLabel(row.type)}
						</Select.Trigger>
						<Select.Content>
							{#each conditionTypes as ct (ct.value)}
								<Select.Item value={ct.value} label={ct.label} />
							{/each}
						</Select.Content>
					</Select.Root>
				</div>

				<!-- Value Input(s) based on type -->
				<div class="flex-1 min-w-[200px]">
					{#if row.type === 'header_match'}
						<!-- Header Match: Two inputs -->
						<div class="flex gap-2">
							<div class="flex-1">
								<Label class="mb-1.5 text-xs text-muted-foreground">Header Name</Label>
								<Input
									type="text"
									placeholder="e.g., X-Priority"
									value={row.header}
									oninput={(e) => updateConditionValue(row.id, 'header', e.currentTarget.value)}
								/>
							</div>
							<div class="flex-1">
								<Label class="mb-1.5 text-xs text-muted-foreground">Pattern (regex)</Label>
								<Input
									type="text"
									placeholder="e.g., ^1$"
									value={row.pattern}
									oninput={(e) => updateConditionValue(row.id, 'pattern', e.currentTarget.value)}
								/>
							</div>
						</div>
					{:else if row.type === 'label_present'}
						<!-- Label Present: Dropdown of available labels -->
						<Label class="mb-1.5 text-xs text-muted-foreground">Label</Label>
						{#if labels.length > 0}
							<Select.Root
								type="single"
								value={row.value}
								onValueChange={(value) => {
									if (value) updateConditionValue(row.id, 'value', value);
								}}
							>
								<Select.Trigger class="w-full">
									{row.value ? getLabelName(row.value) : 'Select a label...'}
								</Select.Trigger>
								<Select.Content>
									{#each labels as label (label.id)}
										<Select.Item value={label.id} label={label.name} />
									{/each}
								</Select.Content>
							</Select.Root>
						{:else}
							<Input
								type="text"
								placeholder="Label ID"
								value={row.value}
								oninput={(e) => updateConditionValue(row.id, 'value', e.currentTarget.value)}
							/>
						{/if}
					{:else if row.type === 'sender_email'}
						<Label class="mb-1.5 text-xs text-muted-foreground">Email Address</Label>
						<Input
							type="text"
							placeholder="e.g., *@example.com"
							value={row.value}
							oninput={(e) => updateConditionValue(row.id, 'value', e.currentTarget.value)}
						/>
					{:else if row.type === 'sender_domain'}
						<Label class="mb-1.5 text-xs text-muted-foreground">Domain</Label>
						<Input
							type="text"
							placeholder="e.g., amazon.com"
							value={row.value}
							oninput={(e) => updateConditionValue(row.id, 'value', e.currentTarget.value)}
						/>
					{:else if row.type === 'subject_contains'}
						<Label class="mb-1.5 text-xs text-muted-foreground">Text to Match</Label>
						<Input
							type="text"
							placeholder="e.g., invoice"
							value={row.value}
							oninput={(e) => updateConditionValue(row.id, 'value', e.currentTarget.value)}
						/>
					{:else if row.type === 'subject_regex'}
						<Label class="mb-1.5 text-xs text-muted-foreground">Regex Pattern</Label>
						<Input
							type="text"
							placeholder="e.g., ^Re:.*"
							value={row.value}
							oninput={(e) => updateConditionValue(row.id, 'value', e.currentTarget.value)}
						/>
					{/if}
				</div>

				<!-- Remove Button -->
				<div class="pt-6">
					<Button
						variant="ghost"
						size="icon"
						onclick={() => removeCondition(row.id)}
						title="Remove condition"
					>
						<TrashIcon class="size-4 text-destructive" />
					</Button>
				</div>
			</div>
		{/each}
	</div>

	<!-- Add Condition Button -->
	<Button variant="outline" onclick={addCondition}>
		<PlusIcon class="mr-2 size-4" />
		Add Condition
	</Button>
</div>
