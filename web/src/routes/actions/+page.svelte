<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { SvelteURLSearchParams } from 'svelte/reactivity';
	import { listActions } from '$lib/api/actions.remote';
	import { listAccounts } from '$lib/api/accounts.remote';
	import { TIME_WINDOWS, ACTION_STATUSES } from '$lib/api/actions.constants';
	import type {
		ActionListItem,
		ActionStatus,
		AccountSummary,
		PaginatedResponse
	} from '$lib/types/generated';

	// Shared formatting functions
	import {
		formatTimestampShort,
		formatActionType,
		formatConfidence,
		getConfidenceColor,
		getStatusVariant,
		getStatusLabel
	} from '$lib/formatting/actions';

	// UI Components
	import * as Table from '$lib/components/ui/table';
	import * as Select from '$lib/components/ui/select';
	import * as Empty from '$lib/components/ui/empty';
	import * as ToggleGroup from '$lib/components/ui/toggle-group';
	import * as Pagination from '$lib/components/ui/pagination';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { Spinner } from '$lib/components/ui/spinner';
	import { Label } from '$lib/components/ui/label';

	// Icons
	import InboxIcon from '@lucide/svelte/icons/inbox';
	import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';

	// ============================================================================
	// Filter State (synced with URL params)
	// ============================================================================

	// Read initial values from URL
	function getUrlParam(key: string): string | null {
		return page.url.searchParams.get(key);
	}

	function getUrlParamArray(key: string): string[] {
		const value = getUrlParam(key);
		return value ? value.split(',').filter(Boolean) : [];
	}

	// Filter state
	let timeWindow = $state<string | undefined>(getUrlParam('time_window') || undefined);
	let accountId = $state<string | undefined>(getUrlParam('account_id') || undefined);
	let sender = $state<string>(getUrlParam('sender') || '');
	let senderDebounced = $state<string>(getUrlParam('sender') || '');
	let actionTypes = $state<string[]>(getUrlParamArray('action_type'));
	let statuses = $state<string[]>(getUrlParamArray('status'));
	let minConfidence = $state<string>(getUrlParam('min_confidence') || '');
	let maxConfidence = $state<string>(getUrlParam('max_confidence') || '');

	// Debounce sender input to avoid excessive API calls on each keystroke
	const SENDER_DEBOUNCE_MS = 300;
	$effect(() => {
		const value = sender;
		const timeoutId = setTimeout(() => {
			senderDebounced = value;
		}, SENDER_DEBOUNCE_MS);
		return () => clearTimeout(timeoutId);
	});

	// Pagination state
	let currentPage = $state<number>(Number(getUrlParam('page')) || 1);
	let itemsPerPage = $state<number>(Number(getUrlParam('limit')) || 20);

	// Loading and data state
	let isLoading = $state<boolean>(true);
	let isPolling = $state<boolean>(false);
	let data = $state<PaginatedResponse<ActionListItem> | null>(null);
	let error = $state<string | null>(null);

	// Accounts state (for filter dropdown)
	let accounts = $state<AccountSummary[]>([]);
	let accountsLoading = $state<boolean>(true);

	// Available action types (we'll collect these from the data or use a predefined list)
	const AVAILABLE_ACTION_TYPES = [
		'apply_label',
		'mark_read',
		'mark_unread',
		'archive',
		'delete',
		'move',
		'star',
		'unstar',
		'forward',
		'auto_reply',
		'create_task',
		'snooze',
		'add_note',
		'escalate',
		'none'
	];

	// ============================================================================
	// URL Sync
	// ============================================================================

	function updateUrl() {
		const params = new SvelteURLSearchParams();

		if (timeWindow) params.set('time_window', timeWindow);
		if (accountId) params.set('account_id', accountId);
		if (senderDebounced.trim()) params.set('sender', senderDebounced.trim());
		if (actionTypes.length > 0) params.set('action_type', actionTypes.join(','));
		if (statuses.length > 0) params.set('status', statuses.join(','));
		if (minConfidence.trim()) params.set('min_confidence', minConfidence.trim());
		if (maxConfidence.trim()) params.set('max_confidence', maxConfidence.trim());
		if (currentPage > 1) params.set('page', String(currentPage));
		if (itemsPerPage !== 20) params.set('limit', String(itemsPerPage));

		const queryString = params.toString();
		const newUrl = queryString ? `?${queryString}` : page.url.pathname;
		goto(resolve(newUrl as '/actions'), { replaceState: true, keepFocus: true });
	}

	// ============================================================================
	// Data Fetching
	// ============================================================================

	async function fetchActions(options: { showLoading?: boolean } = {}) {
		const { showLoading = true } = options;

		if (showLoading) {
			isLoading = true;
		} else {
			isPolling = true;
		}
		error = null;

		try {
			const minConf = minConfidence.trim() ? Number(minConfidence) : undefined;
			const maxConf = maxConfidence.trim() ? Number(maxConfidence) : undefined;

			data = await listActions({
				timeWindow: timeWindow as '24h' | '7d' | '30d' | undefined,
				accountId: accountId || undefined,
				sender: senderDebounced.trim() || undefined,
				actionTypes: actionTypes.length > 0 ? actionTypes : undefined,
				statuses: statuses.length > 0 ? (statuses as ActionStatus[]) : undefined,
				minConfidence: minConf,
				maxConfidence: maxConf,
				limit: itemsPerPage,
				offset: (currentPage - 1) * itemsPerPage
			});
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch actions';
			console.error('Error fetching actions:', e);
		} finally {
			isLoading = false;
			isPolling = false;
		}
	}

	// Fetch accounts list on mount
	$effect(() => {
		async function fetchAccountsList() {
			try {
				accounts = await listAccounts({});
			} catch (e) {
				console.error('Error fetching accounts:', e);
			} finally {
				accountsLoading = false;
			}
		}
		fetchAccountsList();
	});

	// Initial fetch and refetch on filter changes
	$effect(() => {
		// Track all filter dependencies (use senderDebounced to avoid fetching on every keystroke)
		void [
			timeWindow,
			accountId,
			senderDebounced,
			actionTypes,
			statuses,
			minConfidence,
			maxConfidence,
			currentPage,
			itemsPerPage
		];
		fetchActions();
		updateUrl();
	});

	// Polling effect (10 second interval)
	$effect(() => {
		let intervalId: ReturnType<typeof setInterval> | undefined;

		function startPolling() {
			intervalId = setInterval(() => {
				if (document.visibilityState === 'visible') {
					fetchActions({ showLoading: false });
				}
			}, 10000);
		}

		function handleVisibilityChange() {
			if (document.visibilityState === 'visible') {
				// Immediately fetch when becoming visible
				fetchActions({ showLoading: false });
			}
		}

		startPolling();
		document.addEventListener('visibilitychange', handleVisibilityChange);

		return () => {
			if (intervalId) clearInterval(intervalId);
			document.removeEventListener('visibilitychange', handleVisibilityChange);
		};
	});

	// ============================================================================
	// Filter Handlers
	// ============================================================================

	function handleTimeWindowChange(value: string | undefined) {
		// Convert empty string (from "All" button) to undefined
		timeWindow = value || undefined;
		currentPage = 1; // Reset to first page on filter change
	}

	function handleStatusToggle(status: string) {
		if (statuses.includes(status)) {
			statuses = statuses.filter((s) => s !== status);
		} else {
			statuses = [...statuses, status];
		}
		currentPage = 1;
	}

	function clearFilters() {
		timeWindow = undefined;
		accountId = undefined;
		sender = '';
		senderDebounced = ''; // Clear immediately without waiting for debounce
		actionTypes = [];
		statuses = [];
		minConfidence = '';
		maxConfidence = '';
		currentPage = 1;
	}

	// ============================================================================
	// Page-Specific Helpers
	// ============================================================================

	function truncateSubject(subject: string | null, maxLength: number = 50): string {
		if (!subject) return '(No subject)';
		if (subject.length <= maxLength) return subject;
		return subject.slice(0, maxLength) + '...';
	}

	/**
	 * Format sender for list view - only show name or email, not both.
	 * This differs from the shared formatSender which shows "Name <email>".
	 */
	function formatSenderShort(email: string | null, name: string | null): string {
		if (name) return name;
		if (email) return email;
		return 'Unknown';
	}

	// Derived values
	const totalPages = $derived(data ? Math.ceil(data.total / itemsPerPage) : 0);
	const hasFilters = $derived(
		!!timeWindow ||
			!!accountId ||
			!!sender.trim() ||
			actionTypes.length > 0 ||
			statuses.length > 0 ||
			!!minConfidence.trim() ||
			!!maxConfidence.trim()
	);
</script>

<svelte:head>
	<title>Actions - Ashford</title>
</svelte:head>

<div class="space-y-4">
	<!-- Page Header -->
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-semibold">Actions History</h1>
		<div class="flex items-center gap-2">
			{#if isPolling}
				<Spinner class="size-4" />
			{/if}
			<Button variant="outline" size="sm" onclick={() => fetchActions()}>
				<RefreshCwIcon class="mr-2 size-4" />
				Refresh
			</Button>
		</div>
	</div>

	<!-- Filters -->
	<div class="rounded-lg border bg-card p-4">
		<div class="flex flex-wrap items-end gap-4">
			<!-- Time Window Toggle -->
			<div class="space-y-1.5">
				<Label class="text-xs text-muted-foreground">Time Window</Label>
				<ToggleGroup.Root
					type="single"
					value={timeWindow}
					onValueChange={handleTimeWindowChange}
					variant="outline"
				>
					{#each TIME_WINDOWS as tw (tw)}
						<ToggleGroup.Item value={tw} class="px-3">
							{tw}
						</ToggleGroup.Item>
					{/each}
					<ToggleGroup.Item value="" class="px-3">All</ToggleGroup.Item>
				</ToggleGroup.Root>
			</div>

			<!-- Account Select -->
			<div class="space-y-1.5">
				<Label class="text-xs text-muted-foreground">Account</Label>
				<Select.Root
					type="single"
					value={accountId ?? ''}
					onValueChange={(v) => {
						accountId = v || undefined;
						currentPage = 1;
					}}
				>
					<Select.Trigger class="w-48" disabled={accountsLoading}>
						{#if accountsLoading}
							<span class="text-muted-foreground">Loading...</span>
						{:else if !accountId}
							<span class="text-muted-foreground">All accounts</span>
						{:else}
							{accounts.find((a) => a.id === accountId)?.email ?? 'Unknown'}
						{/if}
					</Select.Trigger>
					<Select.Content>
						<Select.Item value="">All accounts</Select.Item>
						{#each accounts as account (account.id)}
							<Select.Item value={account.id}>
								{account.display_name ?? account.email}
							</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>

			<!-- Sender Input -->
			<div class="space-y-1.5">
				<Label class="text-xs text-muted-foreground">Sender</Label>
				<Input type="text" placeholder="Email or domain" bind:value={sender} class="w-48" />
			</div>

			<!-- Action Type Select -->
			<div class="space-y-1.5">
				<Label class="text-xs text-muted-foreground">Action Type</Label>
				<Select.Root
					type="multiple"
					value={actionTypes}
					onValueChange={(v) => {
						actionTypes = v;
						currentPage = 1;
					}}
				>
					<Select.Trigger class="w-40">
						{#if actionTypes.length === 0}
							<span class="text-muted-foreground">Any type</span>
						{:else if actionTypes.length === 1}
							{formatActionType(actionTypes[0])}
						{:else}
							{actionTypes.length} types
						{/if}
					</Select.Trigger>
					<Select.Content>
						{#each AVAILABLE_ACTION_TYPES as actionType (actionType)}
							<Select.Item value={actionType}>
								{formatActionType(actionType)}
							</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>

			<!-- Confidence Range -->
			<div class="space-y-1.5">
				<Label class="text-xs text-muted-foreground">Confidence %</Label>
				<div class="flex items-center gap-1">
					<Input
						type="number"
						placeholder="Min"
						min="0"
						max="100"
						bind:value={minConfidence}
						class="w-20"
					/>
					<span class="text-muted-foreground">-</span>
					<Input
						type="number"
						placeholder="Max"
						min="0"
						max="100"
						bind:value={maxConfidence}
						class="w-20"
					/>
				</div>
			</div>

			<!-- Clear Filters -->
			{#if hasFilters}
				<Button variant="ghost" size="sm" onclick={clearFilters}>Clear filters</Button>
			{/if}
		</div>

		<!-- Status Checkboxes -->
		<div class="mt-4 flex flex-wrap items-center gap-4">
			<span class="text-xs text-muted-foreground">Status:</span>
			{#each ACTION_STATUSES as status (status)}
				<label class="flex items-center gap-2 text-sm">
					<Checkbox
						checked={statuses.includes(status)}
						onCheckedChange={() => handleStatusToggle(status)}
					/>
					{getStatusLabel(status)}
				</label>
			{/each}
		</div>
	</div>

	<!-- Content -->
	{#if isLoading && !data}
		<!-- Initial Loading State -->
		<div class="flex items-center justify-center py-12">
			<Spinner class="size-8" />
		</div>
	{:else if error}
		<!-- Error State -->
		<Empty.Root class="border">
			<Empty.Header>
				<Empty.Title>Error Loading Actions</Empty.Title>
				<Empty.Description>{error}</Empty.Description>
			</Empty.Header>
			<Empty.Content>
				<Button onclick={() => fetchActions()}>Try Again</Button>
			</Empty.Content>
		</Empty.Root>
	{:else if data && data.items.length === 0}
		<!-- Empty State -->
		<Empty.Root class="border">
			<Empty.Media>
				<InboxIcon class="size-12 text-muted-foreground" />
			</Empty.Media>
			<Empty.Header>
				<Empty.Title>No Actions Found</Empty.Title>
				<Empty.Description>
					{#if hasFilters}
						No actions match your current filters. Try adjusting your search criteria.
					{:else}
						No actions have been recorded yet.
					{/if}
				</Empty.Description>
			</Empty.Header>
			{#if hasFilters}
				<Empty.Content>
					<Button variant="outline" onclick={clearFilters}>Clear Filters</Button>
				</Empty.Content>
			{/if}
		</Empty.Root>
	{:else if data}
		<!-- Actions Table -->
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Timestamp</Table.Head>
					<Table.Head>Subject</Table.Head>
					<Table.Head>Sender</Table.Head>
					<Table.Head>Action</Table.Head>
					<Table.Head>Confidence</Table.Head>
					<Table.Head>Status</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each data.items as action (action.id)}
					<Table.Row
						class="cursor-pointer"
						onclick={() => goto(resolve('/actions/[id]', { id: action.id }))}
					>
						<Table.Cell class="font-mono text-sm">
							{formatTimestampShort(action.created_at)}
						</Table.Cell>
						<Table.Cell class="max-w-[200px]" title={action.message_subject || undefined}>
							{truncateSubject(action.message_subject)}
						</Table.Cell>
						<Table.Cell class="max-w-[150px]" title={action.message_from_email || undefined}>
							{formatSenderShort(action.message_from_email, action.message_from_name)}
						</Table.Cell>
						<Table.Cell>
							{formatActionType(action.action_type)}
						</Table.Cell>
						<Table.Cell class={getConfidenceColor(action.confidence)}>
							{formatConfidence(action.confidence)}
						</Table.Cell>
						<Table.Cell>
							<Badge variant={getStatusVariant(action.status)}>
								{getStatusLabel(action.status)}
							</Badge>
						</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>

		<!-- Pagination -->
		<div class="flex items-center justify-between">
			<div class="text-sm text-muted-foreground">
				Showing {(currentPage - 1) * itemsPerPage + 1} to {Math.min(
					currentPage * itemsPerPage,
					data.total
				)} of {data.total} actions
			</div>
			<div class="flex items-center gap-4">
				<!-- Items per page selector -->
				<div class="flex items-center gap-2">
					<span class="text-sm text-muted-foreground">Per page:</span>
					<Select.Root
						type="single"
						value={String(itemsPerPage)}
						onValueChange={(v) => {
							itemsPerPage = Number(v);
							currentPage = 1;
						}}
					>
						<Select.Trigger class="w-20">
							{itemsPerPage}
						</Select.Trigger>
						<Select.Content>
							<Select.Item value="10">10</Select.Item>
							<Select.Item value="25">25</Select.Item>
							<Select.Item value="50">50</Select.Item>
						</Select.Content>
					</Select.Root>
				</div>

				<!-- Page navigation -->
				{#if totalPages > 1}
					<Pagination.Root count={data.total} perPage={itemsPerPage} bind:page={currentPage}>
						{#snippet children({ pages })}
							<Pagination.Content>
								<Pagination.PrevButton />
								{#each pages as p (p.key)}
									{#if p.type === 'ellipsis'}
										<Pagination.Ellipsis />
									{:else}
										<Pagination.Item>
											<Pagination.Link page={p} isActive={currentPage === p.value}>
												{p.value}
											</Pagination.Link>
										</Pagination.Item>
									{/if}
								{/each}
								<Pagination.NextButton />
							</Pagination.Content>
						{/snippet}
					</Pagination.Root>
				{/if}
			</div>
		</div>
	{/if}
</div>
