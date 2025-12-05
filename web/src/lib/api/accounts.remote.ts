/**
 * Remote functions for Accounts API.
 * These execute on the server but can be called transparently from client code.
 */

import { query } from '$app/server';
import * as v from 'valibot';
import { get } from './client';
import type { AccountSummary } from '$lib/types/generated';

// ============================================================================
// Schema Definitions
// ============================================================================

/**
 * Schema for listing accounts (no parameters needed).
 */
const listAccountsInputSchema = v.object({});

// ============================================================================
// Query Functions (Read Operations)
// ============================================================================

/**
 * Lists all accounts for the current user.
 *
 * @example
 * ```svelte
 * <script lang="ts">
 *   import { listAccounts } from '$lib/api/accounts.remote';
 *
 *   let accounts = await listAccounts({});
 * </script>
 * ```
 */
export const listAccounts = query(listAccountsInputSchema, async (): Promise<AccountSummary[]> => {
	return get<AccountSummary[]>('/api/accounts');
});
