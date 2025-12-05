<script lang="ts">
	import { page } from '$app/state';
	import { getAction, undoAction } from '$lib/api/actions.remote';
	import { ApiError } from '$lib/api/errors';
	import { toast } from 'svelte-sonner';
	import type { ActionDetail } from '$lib/types/generated';

	// UI Components
	import * as Card from '$lib/components/ui/card';
	import * as Collapsible from '$lib/components/ui/collapsible';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Spinner } from '$lib/components/ui/spinner';

	// Icons
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
	import UndoIcon from '@lucide/svelte/icons/undo-2';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';

	// Helper functions (extracted for testability)
	import {
		formatTimestamp,
		formatActionType,
		formatConfidence,
		getConfidenceColor,
		getStatusVariant,
		getStatusLabel,
		formatSender,
		formatJson
	} from './helpers';

	// ============================================================================
	// State
	// ============================================================================

	const id = $derived(page.params.id);

	let isLoading = $state<boolean>(true);
	let error = $state<string | null>(null);
	let action = $state<ActionDetail | null>(null);
	let isUndoing = $state<boolean>(false);
	let isJsonOpen = $state<boolean>(true);
	let isParamsOpen = $state<boolean>(false);

	// ============================================================================
	// Data Fetching
	// ============================================================================

	async function fetchAction() {
		if (!id) {
			error = 'No action ID provided';
			isLoading = false;
			return;
		}

		isLoading = true;
		error = null;

		try {
			action = await getAction({ id });
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch action details';
			console.error('Error fetching action:', e);
		} finally {
			isLoading = false;
		}
	}

	// Fetch on mount and when id changes
	$effect(() => {
		void id;
		fetchAction();
	});

	// ============================================================================
	// Undo Handler
	// ============================================================================

	async function handleUndo() {
		if (!action || !action.can_undo || isUndoing) return;

		isUndoing = true;

		try {
			await undoAction({ actionId: action.id });
			toast.success('Action undo queued');
			// Refresh the action data to show updated state
			await fetchAction();
		} catch (e) {
			// Extract descriptive error message from API response body if available
			let errorMessage = 'Failed to undo action';
			if (e instanceof ApiError && e.body && typeof e.body === 'object' && 'message' in e.body) {
				errorMessage = (e.body as { message: string }).message;
			} else if (e instanceof Error) {
				errorMessage = e.message;
			}
			toast.error(errorMessage);
			console.error('Error undoing action:', e);
		} finally {
			isUndoing = false;
		}
	}
</script>

<svelte:head>
	<title>Action Details - Ashford</title>
</svelte:head>

<div class="space-y-6">
	<!-- Back Link -->
	<a
		href="/actions"
		class="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
	>
		<ArrowLeftIcon class="size-4" />
		Back to Actions
	</a>

	{#if isLoading}
		<!-- Loading State -->
		<div class="flex items-center justify-center py-12">
			<Spinner class="size-8" />
		</div>
	{:else if error}
		<!-- Error State -->
		<Card.Root>
			<Card.Header>
				<Card.Title class="text-destructive">Error Loading Action</Card.Title>
				<Card.Description>{error}</Card.Description>
			</Card.Header>
			<Card.Footer>
				<Button onclick={() => fetchAction()}>Try Again</Button>
			</Card.Footer>
		</Card.Root>
	{:else if action}
		<!-- Header with Badges -->
		<div class="flex flex-wrap items-center gap-3">
			<h1 class="text-2xl font-semibold">{formatActionType(action.action_type)}</h1>
			<Badge variant={getStatusVariant(action.status)}>
				{getStatusLabel(action.status)}
			</Badge>
			{#if action.has_been_undone}
				<Badge variant="outline">Undone</Badge>
			{/if}
		</div>

		<!-- Summary Card -->
		<Card.Root>
			<Card.Header>
				<Card.Title>Summary</Card.Title>
			</Card.Header>
			<Card.Content>
				<dl class="grid grid-cols-1 gap-4 sm:grid-cols-2">
					<div>
						<dt class="text-sm font-medium text-muted-foreground">Timestamp</dt>
						<dd class="mt-1 font-mono text-sm">{formatTimestamp(action.created_at)}</dd>
					</div>
					<div>
						<dt class="text-sm font-medium text-muted-foreground">Confidence</dt>
						<dd class="mt-1 text-sm {getConfidenceColor(action.decision?.confidence ?? null)}">
							{formatConfidence(action.decision?.confidence ?? null)}
						</dd>
					</div>
					<div class="sm:col-span-2">
						<dt class="text-sm font-medium text-muted-foreground">Subject</dt>
						<dd class="mt-1 text-sm">{action.message_subject || '(No subject)'}</dd>
					</div>
					<div class="sm:col-span-2">
						<dt class="text-sm font-medium text-muted-foreground">Sender</dt>
						<dd class="mt-1 text-sm">
							{formatSender(action.message_from_email, action.message_from_name)}
						</dd>
					</div>
					{#if action.executed_at}
						<div>
							<dt class="text-sm font-medium text-muted-foreground">Executed At</dt>
							<dd class="mt-1 font-mono text-sm">{formatTimestamp(action.executed_at)}</dd>
						</div>
					{/if}
					{#if action.error_message}
						<div class="sm:col-span-2">
							<dt class="text-sm font-medium text-muted-foreground">Error</dt>
							<dd class="mt-1 text-sm text-destructive">{action.error_message}</dd>
						</div>
					{/if}
				</dl>
			</Card.Content>
		</Card.Root>

		<!-- Rationale Section -->
		<Card.Root>
			<Card.Header>
				<Card.Title>Rationale</Card.Title>
			</Card.Header>
			<Card.Content>
				{#if action.decision?.rationale}
					<p class="whitespace-pre-wrap text-sm">{action.decision.rationale}</p>
				{:else}
					<p class="text-sm text-muted-foreground italic">No rationale provided</p>
				{/if}
			</Card.Content>
		</Card.Root>

		<!-- Decision JSON (Collapsible) -->
		{#if action.decision?.decision_json}
			<Collapsible.Root bind:open={isJsonOpen}>
				<Card.Root>
					<Collapsible.Trigger class="w-full">
						<Card.Header class="cursor-pointer hover:bg-muted/50">
							<div class="flex items-center justify-between">
								<Card.Title>Decision JSON</Card.Title>
								{#if isJsonOpen}
									<ChevronDownIcon class="size-5 text-muted-foreground" />
								{:else}
									<ChevronRightIcon class="size-5 text-muted-foreground" />
								{/if}
							</div>
						</Card.Header>
					</Collapsible.Trigger>
					<Collapsible.Content>
						<Card.Content>
							<pre class="overflow-x-auto rounded-md bg-muted p-4 text-sm"><code
									>{formatJson(action.decision.decision_json)}</code
								></pre>
						</Card.Content>
					</Collapsible.Content>
				</Card.Root>
			</Collapsible.Root>
		{/if}

		<!-- Action Parameters JSON (Collapsible) -->
		{#if action.parameters_json && Object.keys(action.parameters_json).length > 0}
			<Collapsible.Root bind:open={isParamsOpen}>
				<Card.Root>
					<Collapsible.Trigger class="w-full">
						<Card.Header class="cursor-pointer hover:bg-muted/50">
							<div class="flex items-center justify-between">
								<Card.Title>Action Parameters</Card.Title>
								{#if isParamsOpen}
									<ChevronDownIcon class="size-5 text-muted-foreground" />
								{:else}
									<ChevronRightIcon class="size-5 text-muted-foreground" />
								{/if}
							</div>
						</Card.Header>
					</Collapsible.Trigger>
					<Collapsible.Content>
						<Card.Content>
							<pre class="overflow-x-auto rounded-md bg-muted p-4 text-sm"><code
									>{formatJson(action.parameters_json)}</code
								></pre>
						</Card.Content>
					</Collapsible.Content>
				</Card.Root>
			</Collapsible.Root>
		{/if}

		<!-- Undone By Link -->
		{#if action.has_been_undone && action.undo_action_id}
			<Card.Root>
				<Card.Content class="py-4">
					<p class="text-sm">
						This action was undone by
						<a
							href="/actions/{action.undo_action_id}"
							class="font-medium text-primary underline hover:no-underline"
						>
							action {action.undo_action_id.slice(0, 8)}...
						</a>
					</p>
				</Card.Content>
			</Card.Root>
		{/if}

		<!-- Action Buttons -->
		<div class="flex flex-wrap gap-3">
			{#if action.gmail_link}
				<Button
					variant="outline"
					href={action.gmail_link}
					target="_blank"
					rel="noopener noreferrer"
				>
					<ExternalLinkIcon class="mr-2 size-4" />
					Open in Gmail
				</Button>
			{/if}

			{#if action.can_undo}
				<Button variant="default" onclick={handleUndo} disabled={isUndoing}>
					{#if isUndoing}
						<Spinner class="mr-2 size-4" />
						Undoing...
					{:else}
						<UndoIcon class="mr-2 size-4" />
						Undo Action
					{/if}
				</Button>
			{/if}
		</div>
	{/if}
</div>
