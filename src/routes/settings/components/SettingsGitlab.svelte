<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { locale, t } from '$lib/i18n/index.ts';
  import { parseWorkAuthorsInput } from '../gitlabWorkAuthors.ts';
  import {
    mergeWorkAuthorsWithPicked,
    preselectedAuthorEmails,
    type GitlabAuthorOption,
  } from '../gitlabAuthorPicker.ts';
  import GitlabProjectPicker from './GitlabProjectPicker.svelte';
  import GitlabAuthorPicker from './GitlabAuthorPicker.svelte';
  import Toggle from 'carbon-components-svelte/src/Toggle/Toggle.svelte';

  interface GitlabSettings {
    url: string | null;
    token: string | null;
    project_filter: string[] | null;
    work_authors: string[] | null;
    report_enabled: boolean;
  }

  export let config: { gitlab?: GitlabSettings | null };

  const dispatch = createEventDispatcher<{ change: typeof config }>();
  $: currentLocale = $locale;

  let showToken = false;
  let testing = false;
  let testResult: { ok: boolean; message: string | null; detail: string } | null = null;

  let showProjectPicker = false;
  let pickerLoading = false;
  let pickerProjects: string[] = [];
  let pickerError: string | null = null;

  let showAuthorPicker = false;
  let authorPickerLoading = false;
  let authorOptions: GitlabAuthorOption[] = [];
  let authorPickerError: string | null = null;
  let authorPickerTruncated = false;

  function gitlab(): GitlabSettings {
    if (!config.gitlab) {
      config.gitlab = {
        url: null,
        token: null,
        project_filter: null,
        work_authors: null,
        report_enabled: true,
      };
    }
    return config.gitlab;
  }

  function handleChange() {
    dispatch('change', config);
  }

  function onUrlInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value.trim();
    gitlab().url = value === '' ? null : value;
    handleChange();
  }

  function onTokenInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value;
    gitlab().token = value === '' ? null : value;
    handleChange();
  }

  function onFilterInput(event: Event) {
    const value = (event.currentTarget as HTMLTextAreaElement).value;
    const lines = value
      .split(/[\n,]/)
      .map((line) => line.trim())
      .filter((line) => line !== '');
    gitlab().project_filter = lines;
    handleChange();
  }

  function onAuthorsInput(event: Event) {
    const value = (event.currentTarget as HTMLTextAreaElement).value;
    gitlab().work_authors = parseWorkAuthorsInput(value);
    handleChange();
  }

  async function openProjectPicker() {
    const current = gitlab();
    pickerLoading = true;
    pickerError = null;
    try {
      const result = await invoke<{
        ok: boolean;
        projects: { path_with_namespace: string }[];
        message: string | null;
      }>('gitlab_list_projects', {
        url: current.url ?? '',
        token: current.token ?? '',
      });
      if (!result.ok) {
        pickerError = result.message ?? t('settingsGitlab.pickLoadFailed');
        return;
      }
      pickerProjects = result.projects.map((p) => p.path_with_namespace);
      showProjectPicker = true;
    } catch (e) {
      pickerError = `${t('settingsGitlab.pickLoadFailed')}: ${String(e)}`;
    } finally {
      pickerLoading = false;
    }
  }

  function onPickerSave(event: CustomEvent<string[]>) {
    gitlab().project_filter = event.detail;
    handleChange();
    showProjectPicker = false;
  }

  async function openAuthorPicker() {
    const current = gitlab();
    authorPickerLoading = true;
    authorPickerError = null;
    authorPickerTruncated = false;
    try {
      const result = await invoke<{
        ok: boolean;
        authors: GitlabAuthorOption[];
        truncated: boolean;
        message: string | null;
      }>('gitlab_list_authors', {
        url: current.url ?? '',
        token: current.token ?? '',
        projectFilter: current.project_filter ?? [],
      });
      if (!result.ok) {
        authorPickerError = result.message ?? t('settingsGitlab.pickAuthorLoadFailed');
        return;
      }
      authorOptions = result.authors;
      authorPickerTruncated = result.truncated;
      showAuthorPicker = true;
    } catch (e) {
      authorPickerError = `${t('settingsGitlab.pickAuthorLoadFailed')}: ${String(e)}`;
    } finally {
      authorPickerLoading = false;
    }
  }

  function onAuthorPickerSave(event: CustomEvent<string[]>) {
    gitlab().work_authors = mergeWorkAuthorsWithPicked(gitlab().work_authors ?? [], event.detail);
    handleChange();
    showAuthorPicker = false;
  }

  function onEnabledToggle() {
    gitlab().report_enabled = !gitlab().report_enabled;
    handleChange();
  }

  async function testConnection() {
    const current = gitlab();
    testing = true;
    testResult = null;
    try {
      const result = await invoke<{
        ok: boolean;
        user_name: string;
        username: string;
        project_count: number;
        allowed_count: number;
        message: string | null;
      }>('gitlab_test_connection', {
        url: current.url ?? '',
        token: current.token ?? '',
        projectFilter: current.project_filter ?? [],
      });
      if (result.ok) {
        const auth = result.user_name || result.username;
        testResult = {
          ok: true,
          message: auth ? `${result.user_name || result.username}` : 'OK',
          detail: `${t('settingsGitlab.testProjectCount', { count: result.project_count })}${result.allowed_count !== result.project_count ? ' · ' + t('settingsGitlab.testAllowedCount', { count: result.allowed_count }) : ''}`,
        };
      } else {
        testResult = {
          ok: false,
          message: t('settingsGitlab.testFailed'),
          detail: result.message ?? '',
        };
      }
    } catch (e) {
      testResult = {
        ok: false,
        message: t('settingsGitlab.testFailed'),
        detail: String(e),
      };
    } finally {
      testing = false;
    }
  }
</script>

<div class="settings-card" data-locale={currentLocale}>
  <h3 class="settings-card-title">{t('settingsGitlab.title')}</h3>
  <p class="settings-card-desc">{t('settingsGitlab.description')}</p>

  <div class="settings-section">
    <div class="settings-block">
      <label class="block">
        <span class="settings-text">{t('settingsGitlab.urlLabel')}</span>
        <input
          type="url"
          placeholder="https://git.example.com"
          value={gitlab().url ?? ''}
          on:input={onUrlInput}
          class="mt-2 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
        />
        <span class="settings-muted">{t('settingsGitlab.urlHint')}</span>
      </label>
    </div>

    <div class="settings-block">
      <label class="block">
        <span class="settings-text">{t('settingsGitlab.tokenLabel')}</span>
        <div class="mt-2 flex items-center gap-2">
          <input
            type={showToken ? 'text' : 'password'}
            value={gitlab().token ?? ''}
            on:input={onTokenInput}
            class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
            autocomplete="off"
          />
          <button
            type="button"
            on:click={() => (showToken = !showToken)}
            class="shrink-0 rounded-lg border border-slate-200 px-3 py-2 text-xs text-slate-500 hover:text-slate-700 dark:border-[#2d3843] dark:text-slate-400 dark:hover:text-slate-200"
          >
            {showToken ? t('settingsGitlab.hideToken') : t('settingsGitlab.showToken')}
          </button>
        </div>
        <span class="settings-muted">{t('settingsGitlab.tokenHint')}</span>
      </label>
    </div>

    <div class="settings-block">
      <div class="flex items-center justify-between gap-2">
        <span class="settings-text">{t('settingsGitlab.filterLabel')}</span>
        <button
          type="button"
          on:click={openProjectPicker}
          disabled={pickerLoading || !((gitlab().url ?? '').trim() && (gitlab().token ?? '').trim())}
          class="shrink-0 rounded-lg border border-slate-200 px-3 py-1.5 text-xs text-slate-600 hover:text-slate-900 disabled:cursor-not-allowed disabled:opacity-50 dark:border-[#2d3843] dark:text-slate-400 dark:hover:text-slate-200"
        >
          {#if pickerLoading}
            <span class="inline-block h-3 w-3 animate-spin rounded-full border-2 border-slate-400 border-t-transparent"></span>
          {/if}
          {t('settingsGitlab.pickButton')}
        </button>
      </div>
      <textarea
        rows="3"
        placeholder="group/project-a&#10;group/project-b"
        value={(gitlab().project_filter ?? []).join('\n')}
        on:input={onFilterInput}
        class="mt-2 w-full resize-y rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
      ></textarea>
      {#if pickerError}
        <p class="mt-1 break-all text-xs text-rose-500">{pickerError}</p>
      {/if}
      <span class="settings-muted">{t('settingsGitlab.filterHint')}</span>
    </div>

    <div class="settings-block">
      <div class="flex items-center justify-between gap-2">
        <span class="settings-text">{t('settingsGitlab.authorsLabel')}</span>
        <button
          type="button"
          on:click={openAuthorPicker}
          disabled={authorPickerLoading || !((gitlab().url ?? '').trim() && (gitlab().token ?? '').trim())}
          class="shrink-0 rounded-lg border border-slate-200 px-3 py-1.5 text-xs text-slate-600 hover:text-slate-900 disabled:cursor-not-allowed disabled:opacity-50 dark:border-[#2d3843] dark:text-slate-400 dark:hover:text-slate-200"
        >
          {#if authorPickerLoading}
            <span class="inline-block h-3 w-3 animate-spin rounded-full border-2 border-slate-400 border-t-transparent"></span>
          {/if}
          {t('settingsGitlab.authorsPickButton')}
        </button>
      </div>
      <textarea
        rows="3"
        placeholder={t('settingsGitlab.authorsPlaceholder')}
        value={(gitlab().work_authors ?? []).join('\n')}
        on:input={onAuthorsInput}
        class="mt-2 w-full resize-y rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
      ></textarea>
      {#if authorPickerError}
        <p class="mt-1 break-all text-xs text-rose-500">{authorPickerError}</p>
      {/if}
      {#if authorPickerTruncated}
        <p class="mt-1 break-all text-xs text-amber-600 dark:text-amber-400">
          {t('settingsGitlab.pickAuthorTruncated')}
        </p>
      {/if}
      <span class="settings-muted">{t('settingsGitlab.authorsHint')}</span>
    </div>

    <div class="settings-block">
      <div class="flex items-center justify-between">
        <div>
          <div class="settings-text">{t('settingsGitlab.enabledLabel')}</div>
          <div class="settings-muted">{t('settingsGitlab.enabledHint')}</div>
        </div>
        <Toggle size="sm" toggled={gitlab().report_enabled} labelText={t('settingsGitlab.enabledLabel')} hideLabel={true} on:toggle={onEnabledToggle} />
      </div>
    </div>

    <div class="settings-block">
      <button
        type="button"
        on:click={testConnection}
        disabled={testing || !((gitlab().url ?? '').trim() && (gitlab().token ?? '').trim())}
        class="page-action-brand"
      >
        {#if testing}
          <span class="inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-white border-t-transparent"></span>
          {t('settingsGitlab.testing')}
        {:else}
          {t('settingsGitlab.testConnection')}
        {/if}
      </button>

      {#if testResult}
        <div class="mt-3 rounded-lg border px-3 py-2 text-sm {testResult.ok ? 'border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-200' : 'border-rose-200 bg-rose-50 text-rose-700 dark:border-rose-500/30 dark:bg-rose-500/10 dark:text-rose-200'}">
          <div class="font-semibold">{testResult.message}</div>
          {#if testResult.detail}
            <div class="mt-0.5 break-all text-xs opacity-90">{testResult.detail}</div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div>

{#if showProjectPicker}
  <GitlabProjectPicker
    projects={pickerProjects}
    initialSelected={gitlab().project_filter ?? []}
    on:save={onPickerSave}
    on:close={() => (showProjectPicker = false)}
  />
{/if}

{#if showAuthorPicker}
  <GitlabAuthorPicker
    authors={authorOptions}
    initialSelected={preselectedAuthorEmails(authorOptions, gitlab().work_authors ?? [])}
    on:save={onAuthorPickerSave}
    on:close={() => (showAuthorPicker = false)}
  />
{/if}