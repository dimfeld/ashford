<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { getLlmRule, createLlmRule, updateLlmRule } from '$lib/api/rules.remote';
	import type { LlmRule, RuleScope } from '$lib/types/generated';

	// UI Components
	import * as Card from '$lib/components/ui/card';
	import * as Select from '$lib/components/ui/select';
	import { Input } from '$lib/components/ui/input';
	import { Textarea } from '$lib/components/ui/textarea';
	import { Button } from '$lib/components/ui/button';
	import { Switch } from '$lib/components/ui/switch';
	import { Label } from '$lib/components/ui/label';
	import { Spinner } from '$lib/components/ui/spinner';

	// Icons
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import SaveIcon from '@lucide/svelte/icons/save';

	// ============================================================================
	// State
	// ============================================================================

	const id = $derived(page.params.id);
	const isNew = $derived(id === 'new');

	let isLoading = $state(true);
	let isSaving = $state(false);
	let error = $state<string | null>(null);

	// Form fields
	let name = $state('');
	let description = $state('');
	let scope = $state<RuleScope>('global');
	let scopeRef = $state('');
	let enabled = $state(true);
	let ruleText = $state('');

	// Validation errors
	let nameError = $state('');
	let ruleTextError = $state('');

	// ============================================================================
	// Constants
	// ============================================================================

	const scopeOptions: { value: RuleScope; label: string }[] = [
		{ value: 'global', label: 'Global' },
		{ value: 'account', label: 'Account' },
		{ value: 'sender', label: 'Sender' },
		{ value: 'domain', label: 'Domain' }
	];

	const scopeNeedsRef = $derived(scope !== 'global');
	const scopeRefPlaceholder = $derived.by(() => {
		switch (scope) {
			case 'account':
				return 'Account ID';
			case 'sender':
				return 'email@example.com';
			case 'domain':
				return 'example.com';
			default:
				return '';
		}
	});

	const ruleTextPlaceholder = `Write instructions for the AI in natural language. Examples:

- "Archive newsletters that I haven't read in the past week"
- "If an email is from my manager and contains 'urgent' in the subject, star it"
- "Label emails from GitHub as 'Development' if they're about pull requests"
- "Mark as read any automated notifications from JIRA"

Be specific about:
- What conditions should trigger the action
- What action should be taken
- Any exceptions or special cases`;

	// ============================================================================
	// Data Fetching
	// ============================================================================

	async function fetchData() {
		isLoading = true;
		error = null;

		try {
			if (!isNew && id) {
				const rule = await getLlmRule({ id });
				populateForm(rule);
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load rule';
			console.error('Error loading rule:', e);
		} finally {
			isLoading = false;
		}
	}

	function populateForm(rule: LlmRule) {
		name = rule.name;
		description = rule.description ?? '';
		scope = rule.scope;
		scopeRef = rule.scope_ref ?? '';
		enabled = rule.enabled;
		ruleText = rule.rule_text;
	}

	onMount(() => {
		fetchData();
	});

	// ============================================================================
	// Form Submission
	// ============================================================================

	function validateForm(): boolean {
		let isValid = true;
		nameError = '';
		ruleTextError = '';

		if (!name.trim()) {
			nameError = 'Name is required';
			isValid = false;
		}

		if (!ruleText.trim()) {
			ruleTextError = 'Rule text is required';
			isValid = false;
		}

		return isValid;
	}

	async function handleSubmit() {
		if (!validateForm()) {
			toast.error('Please fix the validation errors');
			return;
		}

		isSaving = true;

		try {
			const payload = {
				name: name.trim(),
				description: description.trim() || null,
				scope,
				scope_ref: scopeNeedsRef ? scopeRef.trim() || null : null,
				enabled,
				rule_text: ruleText.trim()
			};

			if (isNew) {
				await createLlmRule(payload);
				toast.success('Rule created successfully');
			} else if (id) {
				await updateLlmRule({ id, ...payload });
				toast.success('Rule updated successfully');
			}

			goto(resolve('/rules'));
		} catch (e) {
			const errorMsg = e instanceof Error ? e.message : 'Failed to save rule';
			toast.error(errorMsg);
			console.error('Error saving rule:', e);
		} finally {
			isSaving = false;
		}
	}
</script>

<svelte:head>
	<title>{isNew ? 'New LLM Rule' : 'Edit LLM Rule'} - Ashford</title>
</svelte:head>

<div class="space-y-6">
	<!-- Back Link -->
	<a
		href="/rules"
		class="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
	>
		<ArrowLeftIcon class="size-4" />
		Back to Rules
	</a>

	<!-- Page Header -->
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-semibold">
			{isNew ? 'New LLM Rule' : 'Edit LLM Rule'}
		</h1>
	</div>

	{#if isLoading}
		<div class="flex items-center justify-center py-12">
			<Spinner class="size-8" />
		</div>
	{:else if error && !isNew}
		<Card.Root>
			<Card.Header>
				<Card.Title class="text-destructive">Error Loading Rule</Card.Title>
				<Card.Description>{error}</Card.Description>
			</Card.Header>
			<Card.Footer>
				<Button onclick={() => fetchData()}>Try Again</Button>
			</Card.Footer>
		</Card.Root>
	{:else}
		<form
			onsubmit={(e) => {
				e.preventDefault();
				handleSubmit();
			}}
			class="space-y-6"
		>
			<!-- Basic Information -->
			<Card.Root>
				<Card.Header>
					<Card.Title>Basic Information</Card.Title>
				</Card.Header>
				<Card.Content class="space-y-4">
					<!-- Name -->
					<div class="space-y-2">
						<Label for="name">Name *</Label>
						<Input
							id="name"
							type="text"
							placeholder="e.g., Handle newsletter emails"
							bind:value={name}
							aria-invalid={!!nameError}
						/>
						{#if nameError}
							<p class="text-sm text-destructive">{nameError}</p>
						{/if}
					</div>

					<!-- Description -->
					<div class="space-y-2">
						<Label for="description">Description</Label>
						<Textarea
							id="description"
							placeholder="Optional description of what this rule does"
							bind:value={description}
							class="min-h-[80px]"
						/>
					</div>

					<!-- Enabled -->
					<div class="flex items-center gap-3">
						<Switch id="enabled" bind:checked={enabled} />
						<Label for="enabled">Enabled</Label>
					</div>
				</Card.Content>
			</Card.Root>

			<!-- Scope -->
			<Card.Root>
				<Card.Header>
					<Card.Title>Scope</Card.Title>
					<Card.Description>Define which emails this rule applies to</Card.Description>
				</Card.Header>
				<Card.Content class="space-y-4">
					<div class="flex gap-4">
						<div class="w-48">
							<Label class="mb-2">Scope Type</Label>
							<Select.Root
								type="single"
								value={scope}
								onValueChange={(value) => {
									if (value) scope = value as RuleScope;
								}}
							>
								<Select.Trigger class="w-full">
									{scopeOptions.find((s) => s.value === scope)?.label ?? scope}
								</Select.Trigger>
								<Select.Content>
									{#each scopeOptions as opt (opt.value)}
										<Select.Item value={opt.value} label={opt.label} />
									{/each}
								</Select.Content>
							</Select.Root>
						</div>

						{#if scopeNeedsRef}
							<div class="flex-1">
								<Label class="mb-2">Scope Reference</Label>
								<Input type="text" placeholder={scopeRefPlaceholder} bind:value={scopeRef} />
							</div>
						{/if}
					</div>
				</Card.Content>
			</Card.Root>

			<!-- Rule Text -->
			<Card.Root>
				<Card.Header>
					<Card.Title>Rule Instructions</Card.Title>
					<Card.Description>
						Write natural language instructions for the AI. Be specific about when the rule should
						trigger and what action should be taken.
					</Card.Description>
				</Card.Header>
				<Card.Content>
					<div class="space-y-2">
						<Label for="rule-text">Rule Text *</Label>
						<Textarea
							id="rule-text"
							placeholder={ruleTextPlaceholder}
							bind:value={ruleText}
							class="min-h-[200px] font-mono text-sm"
							aria-invalid={!!ruleTextError}
						/>
						{#if ruleTextError}
							<p class="text-sm text-destructive">{ruleTextError}</p>
						{/if}
					</div>
				</Card.Content>
			</Card.Root>

			<!-- Form Actions -->
			<div class="flex items-center gap-3">
				<Button type="submit" disabled={isSaving}>
					{#if isSaving}
						<Spinner class="mr-2 size-4" />
						Saving...
					{:else}
						<SaveIcon class="mr-2 size-4" />
						{isNew ? 'Create Rule' : 'Save Changes'}
					{/if}
				</Button>
				<Button variant="outline" href="/rules" disabled={isSaving}>Cancel</Button>
			</div>
		</form>
	{/if}
</div>
