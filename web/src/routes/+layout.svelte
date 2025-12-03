<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { page } from '$app/state';
	import { ModeWatcher, toggleMode, mode } from 'mode-watcher';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import { Toaster } from '$lib/components/ui/sonner';

	// Icons
	import HistoryIcon from '@lucide/svelte/icons/history';
	import BookRuledIcon from '@lucide/svelte/icons/book-open';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import SunIcon from '@lucide/svelte/icons/sun';
	import MoonIcon from '@lucide/svelte/icons/moon';

	let { children } = $props();

	const navItems = [
		{ title: 'Actions', href: '/actions', icon: HistoryIcon },
		{ title: 'Rules', href: '/rules', icon: BookRuledIcon },
		{ title: 'Settings', href: '/settings', icon: SettingsIcon }
	];

	// Check if current path matches or starts with the nav item href
	function isActive(href: string): boolean {
		const pathname = page.url.pathname;
		if (href === '/') {
			return pathname === '/';
		}
		return pathname === href || pathname.startsWith(`${href}/`);
	}
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<ModeWatcher />

<Sidebar.Provider>
	<Sidebar.Sidebar>
		<Sidebar.Header>
			<div class="flex items-center gap-2 px-2 py-1.5">
				<span class="text-lg font-semibold">Ashford</span>
			</div>
		</Sidebar.Header>

		<Sidebar.Content>
			<Sidebar.Group>
				<Sidebar.GroupLabel>Navigation</Sidebar.GroupLabel>
				<Sidebar.Menu>
					{#each navItems as item (item.href)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton isActive={isActive(item.href)} tooltipContent={item.title}>
								{#snippet child({ props })}
									<a href={item.href} {...props}>
										<item.icon />
										<span>{item.title}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.Group>
		</Sidebar.Content>

		<Sidebar.Footer>
			<Sidebar.Menu>
				<Sidebar.MenuItem>
					<Sidebar.MenuButton onclick={() => toggleMode()} tooltipContent="Toggle theme">
						{#if mode.current === 'dark'}
							<SunIcon />
							<span>Light Mode</span>
						{:else}
							<MoonIcon />
							<span>Dark Mode</span>
						{/if}
					</Sidebar.MenuButton>
				</Sidebar.MenuItem>
			</Sidebar.Menu>
		</Sidebar.Footer>

		<Sidebar.Rail />
	</Sidebar.Sidebar>

	<Sidebar.Inset>
		<header class="flex h-14 shrink-0 items-center gap-2 border-b px-4">
			<Sidebar.Trigger class="-ml-1" />
		</header>
		<main class="flex-1 overflow-auto p-4">
			{@render children()}
		</main>
	</Sidebar.Inset>
</Sidebar.Provider>

<Toaster />
