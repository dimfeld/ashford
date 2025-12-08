<script lang="ts">
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import {
		getDeterministicRules,
		getLlmRules,
		updateDeterministicRule,
		updateLlmRule,
		deleteDeterministicRule,
		deleteLlmRule,
		swapDeterministicRulePriorities
	} from '$lib/api/rules.remote';
	import type {
		DeterministicRule,
		LlmRule,
		RuleScope,
		Condition,
		LeafCondition
	} from '$lib/types/generated';

	// UI Components
	import * as Tabs from '$lib/components/ui/tabs';
	import * as Table from '$lib/components/ui/table';
	import * as Empty from '$lib/components/ui/empty';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Switch } from '$lib/components/ui/switch';
	import { Spinner } from '$lib/components/ui/spinner';

	// Icons
	import PlusIcon from '@lucide/svelte/icons/plus';
	import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import BookRuledIcon from '@lucide/svelte/icons/book-open';

	// ============================================================================
	// State
	// ============================================================================

	let activeTab = $state<string>('deterministic');

	// Deterministic rules state
	let deterministicRules = $state<DeterministicRule[]>([]);
	let deterministicLoading = $state<boolean>(true);
	let deterministicError = $state<string | null>(null);

	// LLM rules state
	let llmRules = $state<LlmRule[]>([]);
	let llmLoading = $state<boolean>(true);
	let llmError = $state<string | null>(null);

	// Track rules being toggled (for optimistic UI)
	let togglingRules = $state<Set<string>>(new Set());

	// Track rules being reordered
	let reorderingRules = $state<Set<string>>(new Set());

	// Delete dialog state
	let deleteDialogOpen = $state<boolean>(false);
	let ruleToDelete = $state<{ id: string; name: string; type: 'deterministic' | 'llm' } | null>(
		null
	);
	let isDeleting = $state<boolean>(false);

	// ============================================================================
	// Data Fetching
	// ============================================================================

	async function fetchDeterministicRules() {
		deterministicLoading = true;
		deterministicError = null;

		try {
			deterministicRules = await getDeterministicRules({});
		} catch (e) {
			deterministicError = e instanceof Error ? e.message : 'Failed to fetch deterministic rules';
			console.error('Error fetching deterministic rules:', e);
		} finally {
			deterministicLoading = false;
		}
	}

	async function fetchLlmRules() {
		llmLoading = true;
		llmError = null;

		try {
			llmRules = await getLlmRules({});
		} catch (e) {
			llmError = e instanceof Error ? e.message : 'Failed to fetch LLM rules';
			console.error('Error fetching LLM rules:', e);
		} finally {
			llmLoading = false;
		}
	}

	// Fetch both on mount
	onMount(() => {
		fetchDeterministicRules();
		fetchLlmRules();
	});

	// ============================================================================
	// Enable/Disable Toggle (Optimistic Updates)
	// ============================================================================

	async function toggleDeterministicRule(rule: DeterministicRule) {
		if (togglingRules.has(rule.id)) return;

		const originalEnabled = rule.enabled;
		const newEnabled = !originalEnabled;

		// Optimistic update
		togglingRules.add(rule.id);
		togglingRules = togglingRules;
		deterministicRules = deterministicRules.map((r) =>
			r.id === rule.id ? { ...r, enabled: newEnabled } : r
		);

		try {
			await updateDeterministicRule({ id: rule.id, enabled: newEnabled });
			toast.success(`Rule "${rule.name}" ${newEnabled ? 'enabled' : 'disabled'}`);
		} catch (e) {
			// Revert on error
			deterministicRules = deterministicRules.map((r) =>
				r.id === rule.id ? { ...r, enabled: originalEnabled } : r
			);
			const errorMessage = e instanceof Error ? e.message : 'Failed to update rule';
			toast.error(errorMessage);
			console.error('Error toggling deterministic rule:', e);
		} finally {
			togglingRules.delete(rule.id);
			togglingRules = togglingRules;
		}
	}

	async function toggleLlmRule(rule: LlmRule) {
		if (togglingRules.has(rule.id)) return;

		const originalEnabled = rule.enabled;
		const newEnabled = !originalEnabled;

		// Optimistic update
		togglingRules.add(rule.id);
		togglingRules = togglingRules;
		llmRules = llmRules.map((r) => (r.id === rule.id ? { ...r, enabled: newEnabled } : r));

		try {
			await updateLlmRule({ id: rule.id, enabled: newEnabled });
			toast.success(`Rule "${rule.name}" ${newEnabled ? 'enabled' : 'disabled'}`);
		} catch (e) {
			// Revert on error
			llmRules = llmRules.map((r) => (r.id === rule.id ? { ...r, enabled: originalEnabled } : r));
			const errorMessage = e instanceof Error ? e.message : 'Failed to update rule';
			toast.error(errorMessage);
			console.error('Error toggling LLM rule:', e);
		} finally {
			togglingRules.delete(rule.id);
			togglingRules = togglingRules;
		}
	}

	// ============================================================================
	// Priority Reordering
	// ============================================================================

	async function moveDeterministicRule(index: number, direction: 'up' | 'down') {
		const targetIndex = direction === 'up' ? index - 1 : index + 1;

		// Bounds check
		if (targetIndex < 0 || targetIndex >= deterministicRules.length) return;

		const currentRule = deterministicRules[index];
		const adjacentRule = deterministicRules[targetIndex];

		if (reorderingRules.has(currentRule.id) || reorderingRules.has(adjacentRule.id)) return;

		// Save original priorities
		const currentPriority = currentRule.priority;
		const adjacentPriority = adjacentRule.priority;

		// Mark as reordering
		reorderingRules.add(currentRule.id);
		reorderingRules.add(adjacentRule.id);
		reorderingRules = reorderingRules;

		// Optimistic update: swap positions in the array
		const newRules = [...deterministicRules];
		newRules[index] = { ...adjacentRule, priority: currentPriority };
		newRules[targetIndex] = { ...currentRule, priority: adjacentPriority };
		deterministicRules = newRules;

		try {
			// Use atomic swap endpoint to avoid race condition
			await swapDeterministicRulePriorities({
				rule_a_id: currentRule.id,
				rule_b_id: adjacentRule.id
			});
		} catch (e) {
			// Revert on error
			const revertedRules = [...deterministicRules];
			revertedRules[index] = { ...currentRule, priority: currentPriority };
			revertedRules[targetIndex] = { ...adjacentRule, priority: adjacentPriority };
			deterministicRules = revertedRules;

			const errorMessage = e instanceof Error ? e.message : 'Failed to reorder rules';
			toast.error(errorMessage);
			console.error('Error reordering rules:', e);
		} finally {
			reorderingRules.delete(currentRule.id);
			reorderingRules.delete(adjacentRule.id);
			reorderingRules = reorderingRules;
		}
	}

	// ============================================================================
	// Delete
	// ============================================================================

	function confirmDelete(rule: DeterministicRule | LlmRule, type: 'deterministic' | 'llm') {
		ruleToDelete = { id: rule.id, name: rule.name, type };
		deleteDialogOpen = true;
	}

	async function executeDelete() {
		if (!ruleToDelete || isDeleting) return;

		isDeleting = true;

		try {
			if (ruleToDelete.type === 'deterministic') {
				await deleteDeterministicRule({ id: ruleToDelete.id });
				deterministicRules = deterministicRules.filter((r) => r.id !== ruleToDelete!.id);
			} else {
				await deleteLlmRule({ id: ruleToDelete.id });
				llmRules = llmRules.filter((r) => r.id !== ruleToDelete!.id);
			}
			toast.success(`Rule "${ruleToDelete.name}" deleted`);
			deleteDialogOpen = false;
			ruleToDelete = null;
		} catch (e) {
			const errorMessage = e instanceof Error ? e.message : 'Failed to delete rule';
			toast.error(errorMessage);
			console.error('Error deleting rule:', e);
		} finally {
			isDeleting = false;
		}
	}

	// ============================================================================
	// Helpers
	// ============================================================================

	function formatScope(scope: RuleScope, scopeRef: string | null): string {
		if (scope === 'global') return 'Global';
		if (!scopeRef) return scope.charAt(0).toUpperCase() + scope.slice(1);
		return `${scope.charAt(0).toUpperCase() + scope.slice(1)}: ${scopeRef}`;
	}

	function getScopeBadgeVariant(scope: RuleScope): 'default' | 'secondary' | 'outline' {
		switch (scope) {
			case 'global':
				return 'default';
			case 'account':
				return 'secondary';
			default:
				return 'outline';
		}
	}

	function summarizeConditions(conditionsJson: Record<string, unknown>): string {
		// Try to parse as Condition structure
		const condition = conditionsJson as Condition;

		// Check if it's a logical condition (has 'op' property)
		if ('op' in condition && 'children' in condition) {
			const children = condition.children as LeafCondition[];
			const count = children.length;
			const op = condition.op === 'and' ? 'ALL' : 'ANY';
			if (count === 0) return 'No conditions';
			if (count === 1) return formatLeafCondition(children[0]);
			return `${op} of ${count} conditions`;
		}

		// It's a single leaf condition
		if ('type' in condition) {
			return formatLeafCondition(condition as LeafCondition);
		}

		return 'Custom conditions';
	}

	function formatLeafCondition(leaf: LeafCondition): string {
		switch (leaf.type) {
			case 'sender_email':
				return `Sender: ${leaf.value}`;
			case 'sender_domain':
				return `Domain: ${leaf.value}`;
			case 'subject_contains':
				return `Subject contains: ${leaf.value}`;
			case 'subject_regex':
				return `Subject regex: ${leaf.value}`;
			case 'header_match':
				return `Header ${leaf.header}: ${leaf.pattern}`;
			case 'label_present':
				return `Has label: ${leaf.value}`;
			default:
				return 'Unknown condition';
		}
	}

	function truncateText(text: string, maxLength: number = 60): string {
		if (text.length <= maxLength) return text;
		return text.slice(0, maxLength) + '...';
	}
</script>

<svelte:head>
	<title>Rules - Ashford</title>
</svelte:head>

<div class="space-y-4">
	<!-- Page Header -->
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-semibold">Rules</h1>
	</div>

	<!-- Tabs -->
	<Tabs.Root bind:value={activeTab}>
		<div class="flex items-center justify-between">
			<Tabs.List>
				<Tabs.Trigger value="deterministic">Deterministic Rules</Tabs.Trigger>
				<Tabs.Trigger value="llm">LLM Rules</Tabs.Trigger>
			</Tabs.List>

			<div class="flex items-center gap-2">
				{#if activeTab === 'deterministic'}
					<Button variant="outline" size="sm" onclick={() => fetchDeterministicRules()}>
						<RefreshCwIcon class="mr-2 size-4" />
						Refresh
					</Button>
					<Button size="sm" href="/rules/deterministic/new">
						<PlusIcon class="mr-2 size-4" />
						New Rule
					</Button>
				{:else}
					<Button variant="outline" size="sm" onclick={() => fetchLlmRules()}>
						<RefreshCwIcon class="mr-2 size-4" />
						Refresh
					</Button>
					<Button size="sm" href="/rules/llm/new">
						<PlusIcon class="mr-2 size-4" />
						New Rule
					</Button>
				{/if}
			</div>
		</div>

		<!-- Deterministic Rules Tab -->
		<Tabs.Content value="deterministic" class="mt-4">
			{#if deterministicLoading}
				<div class="flex items-center justify-center py-12">
					<Spinner class="size-8" />
				</div>
			{:else if deterministicError}
				<Empty.Root class="border">
					<Empty.Header>
						<Empty.Title>Error Loading Rules</Empty.Title>
						<Empty.Description>{deterministicError}</Empty.Description>
					</Empty.Header>
					<Empty.Content>
						<Button onclick={() => fetchDeterministicRules()}>Try Again</Button>
					</Empty.Content>
				</Empty.Root>
			{:else if deterministicRules.length === 0}
				<Empty.Root class="border">
					<Empty.Media>
						<BookRuledIcon class="size-12 text-muted-foreground" />
					</Empty.Media>
					<Empty.Header>
						<Empty.Title>No Deterministic Rules</Empty.Title>
						<Empty.Description>
							Create your first deterministic rule to automatically process emails based on
							conditions.
						</Empty.Description>
					</Empty.Header>
					<Empty.Content>
						<Button href="/rules/deterministic/new">
							<PlusIcon class="mr-2 size-4" />
							Create Rule
						</Button>
					</Empty.Content>
				</Empty.Root>
			{:else}
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head class="w-[50px]">Priority</Table.Head>
							<Table.Head>Name</Table.Head>
							<Table.Head>Scope</Table.Head>
							<Table.Head>Conditions</Table.Head>
							<Table.Head>Action</Table.Head>
							<Table.Head class="w-[100px]">Enabled</Table.Head>
							<Table.Head class="w-[100px]">Actions</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each deterministicRules as rule, index (rule.id)}
							<Table.Row class="hover:bg-muted/50">
								<Table.Cell>
									<div class="flex items-center gap-1">
										<Button
											variant="ghost"
											size="icon-sm"
											disabled={index === 0 || reorderingRules.has(rule.id)}
											onclick={() => moveDeterministicRule(index, 'up')}
										>
											<ChevronUpIcon class="size-4" />
										</Button>
										<Button
											variant="ghost"
											size="icon-sm"
											disabled={index === deterministicRules.length - 1 ||
												reorderingRules.has(rule.id)}
											onclick={() => moveDeterministicRule(index, 'down')}
										>
											<ChevronDownIcon class="size-4" />
										</Button>
									</div>
								</Table.Cell>
								<Table.Cell>
									<a href="/rules/deterministic/{rule.id}" class="font-medium hover:underline">
										{rule.name}
									</a>
									{#if rule.disabled_reason}
										<p class="text-xs text-muted-foreground mt-0.5" title={rule.disabled_reason}>
											{truncateText(rule.disabled_reason, 40)}
										</p>
									{/if}
								</Table.Cell>
								<Table.Cell>
									<Badge variant={getScopeBadgeVariant(rule.scope)}>
										{formatScope(rule.scope, rule.scope_ref)}
									</Badge>
								</Table.Cell>
								<Table.Cell class="max-w-[200px]">
									<span
										class="text-sm text-muted-foreground"
										title={JSON.stringify(rule.conditions_json)}
									>
										{summarizeConditions(rule.conditions_json)}
									</span>
								</Table.Cell>
								<Table.Cell>
									<span class="text-sm">{rule.action_type}</span>
								</Table.Cell>
								<Table.Cell>
									<Switch
										checked={rule.enabled}
										disabled={togglingRules.has(rule.id)}
										onCheckedChange={() => toggleDeterministicRule(rule)}
									/>
								</Table.Cell>
								<Table.Cell>
									<Button
										variant="ghost"
										size="icon-sm"
										onclick={() => confirmDelete(rule, 'deterministic')}
									>
										<TrashIcon class="size-4 text-destructive" />
									</Button>
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			{/if}
		</Tabs.Content>

		<!-- LLM Rules Tab -->
		<Tabs.Content value="llm" class="mt-4">
			{#if llmLoading}
				<div class="flex items-center justify-center py-12">
					<Spinner class="size-8" />
				</div>
			{:else if llmError}
				<Empty.Root class="border">
					<Empty.Header>
						<Empty.Title>Error Loading Rules</Empty.Title>
						<Empty.Description>{llmError}</Empty.Description>
					</Empty.Header>
					<Empty.Content>
						<Button onclick={() => fetchLlmRules()}>Try Again</Button>
					</Empty.Content>
				</Empty.Root>
			{:else if llmRules.length === 0}
				<Empty.Root class="border">
					<Empty.Media>
						<BookRuledIcon class="size-12 text-muted-foreground" />
					</Empty.Media>
					<Empty.Header>
						<Empty.Title>No LLM Rules</Empty.Title>
						<Empty.Description>
							Create your first LLM rule to let AI help process your emails using natural language
							instructions.
						</Empty.Description>
					</Empty.Header>
					<Empty.Content>
						<Button href="/rules/llm/new">
							<PlusIcon class="mr-2 size-4" />
							Create Rule
						</Button>
					</Empty.Content>
				</Empty.Root>
			{:else}
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head>Name</Table.Head>
							<Table.Head>Scope</Table.Head>
							<Table.Head>Rule Text</Table.Head>
							<Table.Head class="w-[100px]">Enabled</Table.Head>
							<Table.Head class="w-[100px]">Actions</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each llmRules as rule (rule.id)}
							<Table.Row class="hover:bg-muted/50">
								<Table.Cell>
									<a href="/rules/llm/{rule.id}" class="font-medium hover:underline">
										{rule.name}
									</a>
								</Table.Cell>
								<Table.Cell>
									<Badge variant={getScopeBadgeVariant(rule.scope)}>
										{formatScope(rule.scope, rule.scope_ref)}
									</Badge>
								</Table.Cell>
								<Table.Cell class="max-w-[300px]">
									<span class="text-sm text-muted-foreground" title={rule.rule_text}>
										{truncateText(rule.rule_text)}
									</span>
								</Table.Cell>
								<Table.Cell>
									<Switch
										checked={rule.enabled}
										disabled={togglingRules.has(rule.id)}
										onCheckedChange={() => toggleLlmRule(rule)}
									/>
								</Table.Cell>
								<Table.Cell>
									<Button variant="ghost" size="icon-sm" onclick={() => confirmDelete(rule, 'llm')}>
										<TrashIcon class="size-4 text-destructive" />
									</Button>
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			{/if}
		</Tabs.Content>
	</Tabs.Root>
</div>

<!-- Delete Confirmation Dialog -->
<AlertDialog.Root bind:open={deleteDialogOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Delete Rule</AlertDialog.Title>
			<AlertDialog.Description>
				Are you sure you want to delete "{ruleToDelete?.name}"? This action cannot be undone.
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel disabled={isDeleting}>Cancel</AlertDialog.Cancel>
			<AlertDialog.Action
				onclick={executeDelete}
				disabled={isDeleting}
				class="bg-destructive text-white hover:bg-destructive/90"
			>
				{#if isDeleting}
					<Spinner class="mr-2 size-4" />
					Deleting...
				{:else}
					Delete
				{/if}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
