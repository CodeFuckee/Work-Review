<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { locale, t } from '$lib/i18n/index.ts';
  import { trapFocus } from '$lib/utils/focusTrap.ts';
  import {
    filterProjectOptions,
    normalizeProjectSelection,
    toggleProject,
  } from '../gitlabProjectPicker.ts';

  export let projects: string[] = [];
  export let initialSelected: string[] = [];

  const dispatch = createEventDispatcher<{ save: string[]; close: void }>();
  $: currentLocale = $locale;

  let query = '';
  let selected: string[] = normalizeProjectSelection(initialSelected);
  $: filtered = filterProjectOptions(projects, query);
  $: selectedKeys = new Set(selected.map((item) => item.toLowerCase()));

  function isSelected(path: string): boolean {
    return selectedKeys.has(path.trim().toLowerCase());
  }

  function onToggle(path: string) {
    selected = toggleProject(selected, path);
  }

  function selectAllFiltered() {
    const merged = normalizeProjectSelection([...selected, ...filtered]);
    selected = merged;
  }

  function clearSelection() {
    selected = [];
  }

  function save() {
    dispatch('save', normalizeProjectSelection(selected));
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
    aria-label={t('settingsGitlab.pickTitle')}
    class="flex max-h-[80vh] w-full max-w-xl flex-col rounded-xl border border-slate-200 bg-white shadow-xl dark:border-[#2d3843] dark:bg-[#10161d]"
  >
    <div class="flex items-center justify-between border-b border-slate-200 px-5 py-4 dark:border-[#2d3843]">
      <h3 class="text-sm font-semibold text-slate-900 dark:text-[#e6edf3]">
        {t('settingsGitlab.pickTitle')}
      </h3>
      <span class="text-xs text-slate-500 dark:text-slate-400">
        {t('settingsGitlab.pickCount', { selected: selected.length, total: projects.length })}
      </span>
    </div>

    <div class="space-y-3 px-5 py-4">
      <input
        type="search"
        placeholder={t('settingsGitlab.pickSearchPlaceholder')}
        bind:value={query}
        class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
      />

      <div class="flex items-center gap-2">
        <button
          type="button"
          on:click={selectAllFiltered}
          class="rounded-lg border border-slate-200 px-3 py-1.5 text-xs text-slate-600 hover:text-slate-900 dark:border-[#2d3843] dark:text-slate-400 dark:hover:text-slate-200"
        >
          {t('settingsGitlab.pickSelectAll')}
        </button>
        <button
          type="button"
          on:click={clearSelection}
          class="rounded-lg border border-slate-200 px-3 py-1.5 text-xs text-slate-600 hover:text-slate-900 dark:border-[#2d3843] dark:text-slate-400 dark:hover:text-slate-200"
        >
          {t('settingsGitlab.pickClear')}
        </button>
      </div>

      <div class="max-h-72 overflow-y-auto rounded-lg border border-slate-200 dark:border-[#2d3843]">
        {#if filtered.length === 0}
          <div class="px-3 py-6 text-center text-xs text-slate-400">
            {t('settingsGitlab.pickSearchPlaceholder')}
          </div>
        {:else}
          {#each filtered as project (project)}
            <label
              class="flex cursor-pointer items-center gap-2.5 px-3 py-2 text-xs text-slate-700 hover:bg-slate-50 dark:text-slate-300 dark:hover:bg-[#1a222c]"
            >
              <input
                type="checkbox"
                checked={isSelected(project)}
                on:change={() => onToggle(project)}
                class="h-4 w-4 rounded border-slate-300 accent-indigo-500"
              />
              <span class="break-all font-mono">{project}</span>
            </label>
          {/each}
        {/if}
      </div>

      <p class="text-xs text-slate-400 dark:text-slate-500">
        {t('settingsGitlab.pickCountAll')}
      </p>
    </div>

    <div class="flex items-center justify-end gap-2 border-t border-slate-200 px-5 py-4 dark:border-[#2d3843]">
      <button
        type="button"
        on:click={() => dispatch('close')}
        class="rounded-lg border border-slate-200 px-4 py-2 text-sm text-slate-600 hover:text-slate-900 dark:border-[#2d3843] dark:text-slate-400 dark:hover:text-slate-200"
      >
        {t('settingsGitlab.pickCancel')}
      </button>
      <button
        type="button"
        on:click={save}
        class="rounded-lg bg-indigo-500 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-600"
      >
        {t('settingsGitlab.pickSave')}
      </button>
    </div>
  </div>
</div>
