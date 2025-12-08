<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import {
		getDeterministicRule,
		getLabels,
		createDeterministicRule,
		updateDeterministicRule
	} from '$lib/api/rules.remote';
	import type { DeterministicRule, RuleScope, SafeMode, LabelSummary } from '$lib/types/generated';

	// UI Components
	import * as Card from '$lib/components/ui/card';
	import * as Select from '$lib/components/ui/select';
	import { Input } from '$lib/components/ui/input';
	import { Textarea } from '$lib/components/ui/textarea';
	import { Button } from '$lib/components/ui/button';
	import { Switch } from '$lib/components/ui/switch';
	import { Label } from '$lib/components/ui/label';
	import { Spinner } from '$lib/components/ui/spinner';
	import ConditionBuilder from '$lib/components/ConditionBuilder.svelte';

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
	let labels = $state<LabelSummary[]>([]);

	// Form fields
	let name = $state('');
	let description = $state('');
	let scope = $state<RuleScope>('global');
	let scopeRef = $state('');
	let priority = $state(100);
	let enabled = $state(true);
	let safeMode = $state<SafeMode>('default');
	let actionType = $state('');
	let actionParameters = $state<Record<string, unknown>>({});
	let conditionsJson = $state<Record<string, unknown>>({ op: 'and', children: [] });

	// Track previous action type to detect actual changes
	let previousActionType = $state<string>('');

	// Validation errors
	let nameError = $state('');
	let actionTypeError = $state('');
	let actionParamsError = $state('');

	// ============================================================================
	// Constants
	// ============================================================================

	const scopeOptions: { value: RuleScope; label: string }[] = [
		{ value: 'global', label: 'Global' },
		{ value: 'account', label: 'Account' },
		{ value: 'sender', label: 'Sender' },
		{ value: 'domain', label: 'Domain' }
	];

	const safeModeOptions: { value: SafeMode; label: string; description: string }[] = [
		{ value: 'default', label: 'Default', description: 'Standard safety enforcement' },
		{ value: 'always_safe', label: 'Always Safe', description: 'Skip approval for this rule' },
		{
			value: 'dangerous_override',
			label: 'Dangerous Override',
			description: 'Allow dangerous actions'
		}
	];

	const actionTypes: { value: string; label: string; category: string }[] = [
		// Safe actions
		{ value: 'apply_label', label: 'Apply Label', category: 'Safe' },
		{ value: 'remove_label', label: 'Remove Label', category: 'Safe' },
		{ value: 'mark_read', label: 'Mark Read', category: 'Safe' },
		{ value: 'mark_unread', label: 'Mark Unread', category: 'Safe' },
		{ value: 'archive', label: 'Archive', category: 'Safe' },
		{ value: 'trash', label: 'Move to Trash', category: 'Safe' },
		{ value: 'restore', label: 'Restore', category: 'Safe' },
		{ value: 'move', label: 'Move', category: 'Safe' },
		{ value: 'none', label: 'None', category: 'Safe' },
		// Reversible actions
		{ value: 'star', label: 'Star', category: 'Reversible' },
		{ value: 'unstar', label: 'Unstar', category: 'Reversible' },
		{ value: 'snooze', label: 'Snooze', category: 'Reversible' },
		{ value: 'add_note', label: 'Add Note', category: 'Reversible' },
		{ value: 'create_task', label: 'Create Task', category: 'Reversible' },
		// Dangerous actions
		{ value: 'delete', label: 'Delete', category: 'Dangerous' },
		{ value: 'forward', label: 'Forward', category: 'Dangerous' },
		{ value: 'auto_reply', label: 'Auto Reply', category: 'Dangerous' },
		{ value: 'escalate', label: 'Escalate', category: 'Dangerous' }
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

	// ============================================================================
	// Data Fetching
	// ============================================================================

	async function fetchData() {
		isLoading = true;
		error = null;

		try {
			// Always fetch labels for the condition builder
			labels = await getLabels({});

			if (!isNew && id) {
				// Fetch existing rule
				const rule = await getDeterministicRule({ id });
				populateForm(rule);
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load data';
			console.error('Error loading data:', e);
		} finally {
			isLoading = false;
		}
	}

	function populateForm(rule: DeterministicRule) {
		name = rule.name;
		description = rule.description ?? '';
		scope = rule.scope;
		scopeRef = rule.scope_ref ?? '';
		priority = rule.priority;
		enabled = rule.enabled;
		safeMode = rule.safe_mode;
		actionType = rule.action_type;
		actionParameters = rule.action_parameters_json;
		conditionsJson = rule.conditions_json;
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
		actionTypeError = '';
		actionParamsError = '';

		if (!name.trim()) {
			nameError = 'Name is required';
			isValid = false;
		}

		if (!actionType) {
			actionTypeError = 'Action type is required';
			isValid = false;
		}

		// Validate required action parameters based on action type
		if (actionType) {
			const labelId = actionParameters.label_id as string | undefined;
			const to = actionParameters.to as string | undefined;
			const body = actionParameters.body as string | undefined;
			const until = actionParameters.until as string | undefined;

			if (
				(actionType === 'apply_label' || actionType === 'remove_label' || actionType === 'move') &&
				!labelId?.trim()
			) {
				actionParamsError = 'Label is required for this action';
				isValid = false;
			} else if (actionType === 'forward' && !to?.trim()) {
				actionParamsError = 'Email address is required for forward action';
				isValid = false;
			} else if ((actionType === 'auto_reply' || actionType === 'add_note') && !body?.trim()) {
				actionParamsError =
					actionType === 'auto_reply'
						? 'Reply body is required for auto reply action'
						: 'Note content is required for add note action';
				isValid = false;
			} else if (actionType === 'snooze' && !until?.trim()) {
				actionParamsError = 'Snooze time is required for snooze action';
				isValid = false;
			}
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
				priority,
				enabled,
				safe_mode: safeMode,
				action_type: actionType,
				action_parameters_json: actionParameters,
				conditions_json: conditionsJson
			};

			if (isNew) {
				await createDeterministicRule(payload);
				toast.success('Rule created successfully');
			} else if (id) {
				await updateDeterministicRule({ id, ...payload });
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

	// ============================================================================
	// Action Parameters
	// ============================================================================

	function handleConditionsChange(newConditions: Record<string, unknown>) {
		conditionsJson = newConditions;
	}

	function handleConditionWarnings(warnings: string[]) {
		// Display each warning as a toast so the user is aware of flattened conditions
		for (const warning of warnings) {
			toast.warning(warning, { duration: 10000 });
		}
	}

	function getActionTypeLabel(value: string): string {
		return actionTypes.find((at) => at.value === value)?.label ?? value;
	}

	// Determine which parameter fields to show based on action type
	const actionNeedsLabelId = $derived(
		actionType === 'apply_label' || actionType === 'remove_label' || actionType === 'move'
	);
	const actionNeedsEmail = $derived(actionType === 'forward');
	const actionNeedsBody = $derived(actionType === 'auto_reply' || actionType === 'add_note');
	const actionNeedsDateTime = $derived(actionType === 'snooze');

	// Reset action parameters only when action type actually changes (user interaction)
	$effect(() => {
		if (actionType !== previousActionType) {
			// Only reset if switching action types after initial load
			if (previousActionType !== '' && !isLoading) {
				actionParameters = {};
				// Clear any parameter-related validation errors
				actionParamsError = '';
			}
			previousActionType = actionType;
		}
	});
</script>

<svelte:head>
	<title>{isNew ? 'New Deterministic Rule' : 'Edit Deterministic Rule'} - Ashford</title>
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
			{isNew ? 'New Deterministic Rule' : 'Edit Deterministic Rule'}
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
							placeholder="e.g., Archive Amazon shipping notifications"
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

					<!-- Priority -->
					<div class="space-y-2">
						<Label for="priority">Priority</Label>
						<Input
							id="priority"
							type="number"
							bind:value={priority}
							class="w-32"
							min={0}
							max={1000}
						/>
						<p class="text-xs text-muted-foreground">
							Lower numbers execute first. Default is 100.
						</p>
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

			<!-- Conditions -->
			<Card.Root>
				<Card.Header>
					<Card.Title>Conditions</Card.Title>
					<Card.Description>
						Define when this rule should trigger. If no conditions are set, the rule applies to all
						emails within its scope.
					</Card.Description>
				</Card.Header>
				<Card.Content>
					<ConditionBuilder
						conditions={conditionsJson}
						{labels}
						onchange={handleConditionsChange}
						onwarnings={handleConditionWarnings}
					/>
				</Card.Content>
			</Card.Root>

			<!-- Action -->
			<Card.Root>
				<Card.Header>
					<Card.Title>Action</Card.Title>
					<Card.Description>What should happen when conditions are met</Card.Description>
				</Card.Header>
				<Card.Content class="space-y-4">
					<!-- Action Type -->
					<div class="space-y-2">
						<Label>Action Type *</Label>
						<Select.Root
							type="single"
							value={actionType}
							onValueChange={(value) => {
								if (value) actionType = value;
							}}
						>
							<Select.Trigger class="w-64" aria-invalid={!!actionTypeError}>
								{actionType ? getActionTypeLabel(actionType) : 'Select an action...'}
							</Select.Trigger>
							<Select.Content>
								<Select.Group>
									<Select.GroupHeading>Safe Actions</Select.GroupHeading>
									{#each actionTypes.filter((at) => at.category === 'Safe') as at (at.value)}
										<Select.Item value={at.value} label={at.label} />
									{/each}
								</Select.Group>
								<Select.Separator />
								<Select.Group>
									<Select.GroupHeading>Reversible Actions</Select.GroupHeading>
									{#each actionTypes.filter((at) => at.category === 'Reversible') as at (at.value)}
										<Select.Item value={at.value} label={at.label} />
									{/each}
								</Select.Group>
								<Select.Separator />
								<Select.Group>
									<Select.GroupHeading>Dangerous Actions</Select.GroupHeading>
									{#each actionTypes.filter((at) => at.category === 'Dangerous') as at (at.value)}
										<Select.Item value={at.value} label={at.label} />
									{/each}
								</Select.Group>
							</Select.Content>
						</Select.Root>
						{#if actionTypeError}
							<p class="text-sm text-destructive">{actionTypeError}</p>
						{/if}
					</div>

					<!-- Action Parameters (dynamic based on action type) -->
					{#if actionNeedsLabelId}
						<div class="space-y-2">
							<Label>Label</Label>
							{#if labels.length > 0}
								<Select.Root
									type="single"
									value={actionParameters.label_id as string | undefined}
									onValueChange={(value) => {
										if (value) actionParameters = { ...actionParameters, label_id: value };
									}}
								>
									<Select.Trigger class="w-64">
										{#if actionParameters.label_id}
											{labels.find((l) => l.id === actionParameters.label_id)?.name ??
												actionParameters.label_id}
										{:else}
											Select a label...
										{/if}
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
									value={(actionParameters.label_id as string) ?? ''}
									oninput={(e) =>
										(actionParameters = { ...actionParameters, label_id: e.currentTarget.value })}
								/>
							{/if}
						</div>
					{/if}

					{#if actionNeedsEmail}
						<div class="space-y-2">
							<Label>Forward To</Label>
							<Input
								type="email"
								placeholder="recipient@example.com"
								value={(actionParameters.to as string) ?? ''}
								oninput={(e) =>
									(actionParameters = { ...actionParameters, to: e.currentTarget.value })}
							/>
						</div>
					{/if}

					{#if actionNeedsBody}
						<div class="space-y-2">
							<Label>{actionType === 'auto_reply' ? 'Reply Body' : 'Note'}</Label>
							<Textarea
								placeholder={actionType === 'auto_reply'
									? 'Your auto-reply message...'
									: 'Note content...'}
								value={(actionParameters.body as string) ?? ''}
								oninput={(e) =>
									(actionParameters = { ...actionParameters, body: e.currentTarget.value })}
								class="min-h-[120px]"
							/>
						</div>
					{/if}

					{#if actionNeedsDateTime}
						<div class="space-y-2">
							<Label>Snooze Until</Label>
							<Input
								type="datetime-local"
								value={(actionParameters.until as string) ?? ''}
								oninput={(e) =>
									(actionParameters = { ...actionParameters, until: e.currentTarget.value })}
							/>
						</div>
					{/if}

					{#if actionParamsError}
						<p class="text-sm text-destructive">{actionParamsError}</p>
					{/if}

					<!-- Safe Mode -->
					<div class="space-y-2">
						<Label>Safe Mode</Label>
						<Select.Root
							type="single"
							value={safeMode}
							onValueChange={(value) => {
								if (value) safeMode = value as SafeMode;
							}}
						>
							<Select.Trigger class="w-64">
								{safeModeOptions.find((sm) => sm.value === safeMode)?.label ?? safeMode}
							</Select.Trigger>
							<Select.Content>
								{#each safeModeOptions as opt (opt.value)}
									<Select.Item value={opt.value} label={opt.label} />
								{/each}
							</Select.Content>
						</Select.Root>
						<p class="text-xs text-muted-foreground">
							{safeModeOptions.find((sm) => sm.value === safeMode)?.description}
						</p>
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
