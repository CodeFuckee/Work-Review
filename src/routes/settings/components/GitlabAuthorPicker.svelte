<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { locale, t } from '$lib/i18n/index.ts';
  import { trapFocus } from '$lib/utils/focusTrap.ts';
  import {
    filterAuthorOptions,
    toggleAuthor,
    type GitlabAuthorOption,
  } from '../gitlabAuthorPicker.ts';

  export let authors: GitlabAuthorOption[] = [];
  export let initialSelected: string[] = [];

  const dispatch = createEventDispatcher<{ save: string[]; close: void }>();
  $: currentLocale = $locale;

  let query = '';
  let selected: string[] = [...initialSelected];
  $: filtered = filterAuthorOptions(authors, query);
  $: selectedKeys = new Set(selected.map((item) => item.trim().toLowerCase()));

  function isSelected(email: string): boolean {
    return selectedKeys.has(email.trim().toLowerCase());
  }

  function onToggle(email: string) {
    selected = toggleAuthor(selected, email);
  }

  function selectAllFiltered() {
    selected = [...selected, ...filtered.map((author) => author.email)];
  }

  function clearSelection() {
    selected = [];
  }

  function save() {
    dispatch('save', selected);
  }

  function onBackdropKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      dispatch('close');
    }
  }
</script>

<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
  on:keydown={onBackdropKeydown}
  on:click|self={() => dispatch('close')}
  role="presentation"
>
  <div
    use:trapFocus
    role="dialog"
    aria-modal="true"
    aria-label={t('settingsGitlab.pickAuthorTitle')}
    class="flex max-h-[80vh] w-full max-w-xl flex-col rounded-xl border border-slate-200 bg-white shadow-xl dark:border-[#2d3843] dark:bg-[#10161d]"
  >
    <div class="flex items-center justify-between border-b border-slate-200 px-5 py-4 dark:border-[#2d3843]">
      <h3 class="text-sm font-semibold text-slate-900 dark:text-[#e6edf3]">
        {t('settingsGitlab.pickAuthorTitle')}
      </h3>
      <span class="text-xs text-slate-500 dark:text-slate-400">
        {t('settingsGitlab.pickAuthorCount', { selected: selected.length, total: authors.length })}
      </span>
    </div>

    <div class="space-y-3 px-5 py-4">
      <input
        type="search"
        placeholder={t('settingsGitlab.pickAuthorSearchPlaceholder')}
        bind:value={query}
        class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
      />

      <div class="flex items-center gap-2">
        <button
          type="button"
          on:click={selectAllFiltered}
          class="rounded-lg border border-slate-200 px-3 py-1.5 text-xs text-slate-600 hover:text-slate-900 dark:border-[#2d3843] dark:text-slate-400 dark:hover:text-slate-200"
        >
          {t('settingsGitlab.pickAuthorSelectAll')}
        </button>
        <button
          type="button"
          on:click={clearSelection}
          class="rounded-lg border border-slate-200 px-3 py-1.5 text-xs text-slate-600 hover:text-slate-900 dark:border-[#2d3843] dark:text-slate-400 dark:hover:text-slate-200"
        >
          {t('settingsGitlab.pickAuthorClear')}
        </button>
      </div>

      <div class="max-h-72 overflow-y-auto rounded-lg border border-slate-200 dark:border-[#2d3843]">
        {#if filtered.length === 0}
          <div class="px-3 py-6 text-center text-xs text-slate-400">
            {t('settingsGitlab.pickAuthorSearchPlaceholder')}
          </div>
        {:else}
          {#each filtered as author (author.email)}
            <label
              class="flex cursor-pointer items-center gap-2.5 px-3 py-2 text-xs text-slate-700 hover:bg-slate-50 dark:text-slate-300 dark:hover:bg-[#1a222c]"
            >
              <input
                type="checkbox"
                checked={isSelected(author.email)}
                on:change={() => onToggle(author.email)}
                class="h-4 w-4 shrink-0 rounded border-slate-300 accent-indigo-500"
              />
              <span class="min-w-0 flex-1 truncate">
                <span class="font-medium">{author.name}</span>
                <span class="ml-1 font-mono text-slate-500 dark:text-slate-400">({author.email})</span>
              </span>
              {#if author.is_current_user}
                <span class="shrink-0 rounded-full bg-indigo-100 px-2 py-0.5 text-[10px] font-medium text-indigo-600 dark:bg-indigo-500/15 dark:text-indigo-300">
                  {t('settingsGitlab.pickAuthorCurrentUser')}
                </span>
              {/if}
              <span class="shrink-0 text-slate-400 dark:text-slate-500">
                {t('settingsGitlab.pickAuthorCommitCount', { count: author.commits })}
              </span>
            </label>
          {/each}
        {/if}
      </div>

      <p class="text-xs text-slate-400 dark:text-slate-500">
        {t('settingsGitlab.pickAuthorHint')}
      </p>
    </div>

    <div class="flex items-center justify-end gap-2 border-t border-slate-200 px-5 py-4 dark:border-[#2d3843]">
      <button
        type="button"
        on:click={() => dispatch('close')}
        class="rounded-lg border border-slate-200 px-4 py-2 text-sm text-slate-600 hover:text-slate-900 dark:border-[#2d3843] dark:text-slate-400 dark:hover:text-slate-200"
      >
        {t('settingsGitlab.pickAuthorCancel')}
      </button>
      <button
        type="button"
        on:click={save}
        class="rounded-lg bg-indigo-500 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-600"
      >
        {t('settingsGitlab.pickAuthorSave')}
      </button>
    </div>
  </div>
</div>
