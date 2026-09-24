<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { locale, t } from '$lib/i18n/index.ts';

  interface PlaneSettings {
    enabled: boolean;
    url: string | null;
    api_key: string | null;
    workspace_slug: string | null;
  }

  interface PlanePreviewItem {
    name: string;
    project: string;
    state: string;
    updated_at: string;
  }

  export let config: { plane?: PlaneSettings | null };

  const dispatch = createEventDispatcher<{ change: typeof config }>();
  $: currentLocale = $locale;

  let showKey = false;
  let testing = false;
  let testResult: { ok: boolean; message: string; detail: string } | null = null;

  let previewing = false;
  let previewLoaded = false;
  let previewItems: PlanePreviewItem[] = [];
  let previewError: string | null = null;

  function plane(): PlaneSettings {
    if (!config.plane) {
      config.plane = {
        enabled: false,
        url: null,
        api_key: null,
        workspace_slug: null,
      };
    }
    return config.plane;
  }

  function handleChange() {
    dispatch('change', config);
  }

  function onUrlInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value.trim();
    plane().url = value === '' ? null : value;
    handleChange();
  }

  function onApiKeyInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value;
    plane().api_key = value === '' ? null : value;
    handleChange();
  }

  function onWorkspaceSlugInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value.trim();
    plane().workspace_slug = value === '' ? null : value;
    handleChange();
  }

  function onEnabledToggle() {
    plane().enabled = !plane().enabled;
    handleChange();
  }

  function hasCredentials(): boolean {
    const current = plane();
    return Boolean(
      (current.url ?? '').trim() && (current.api_key ?? '').trim() && (current.workspace_slug ?? '').trim(),
    );
  }

  async function testConnection() {
    const current = plane();
    testing = true;
    testResult = null;
    try {
      const result = await invoke<{
        ok: boolean;
        workspace_name: string;
        message: string | null;
      }>('plane_test_connection', {
        url: current.url ?? '',
        apiKey: current.api_key ?? '',
        workspaceSlug: current.workspace_slug ?? '',
      });
      if (result.ok) {
        testResult = {
          ok: true,
          message: result.workspace_name || 'OK',
          detail: '',
        };
      } else {
        testResult = {
          ok: false,
          message: t('settingsPlane.testFailed'),
          detail: result.message ?? '',
        };
      }
    } catch (e) {
      testResult = {
        ok: false,
        message: t('settingsPlane.testFailed'),
        detail: String(e),
      };
    } finally {
      testing = false;
    }
  }

  async function fetchPreview() {
    const current = plane();
    previewing = true;
    previewLoaded = false;
    previewError = null;
    previewItems = [];
    try {
      const result = await invoke<{
        ok: boolean;
        items: PlanePreviewItem[];
        message: string | null;
      }>('plane_fetch_preview', {
        url: current.url ?? '',
        apiKey: current.api_key ?? '',
        workspaceSlug: current.workspace_slug ?? '',
        limit: 20,
      });
      if (!result.ok) {
        previewError = result.message ?? t('settingsPlane.previewFailed');
        return;
      }
      previewItems = result.items;
      previewLoaded = true;
    } catch (e) {
      previewError = `${t('settingsPlane.previewFailed')}: ${String(e)}`;
    } finally {
      previewing = false;
    }
  }
</script>

<div class="settings-card" data-locale={currentLocale}>
  <h3 class="settings-card-title">{t('settingsPlane.title')}</h3>
  <p class="settings-card-desc">{t('settingsPlane.description')}</p>

  <div class="settings-section">
    <div class="settings-block">
      <div class="flex items-center justify-between">
        <div>
          <div class="settings-text">{t('settingsPlane.enabledLabel')}</div>
          <div class="settings-muted">{t('settingsPlane.enabledHint')}</div>
        </div>
        <button
          type="button"
          on:click={onEnabledToggle}
          class="switch-track {plane().enabled ? 'bg-emerald-500' : 'bg-slate-300 dark:bg-[#484f58]'}"
          role="switch"
          aria-label={t('settingsPlane.enabledLabel')}
          aria-checked={plane().enabled}
        >
          <span class="switch-thumb {plane().enabled ? 'translate-x-5' : 'translate-x-0'}"></span>
        </button>
      </div>
    </div>

    <div class="settings-block">
      <label class="block">
        <span class="settings-text">{t('settingsPlane.urlLabel')}</span>
        <input
          type="url"
          placeholder="https://plane.example.com"
          value={plane().url ?? ''}
          on:input={onUrlInput}
          class="mt-2 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
        />
        <span class="settings-muted">{t('settingsPlane.urlHint')}</span>
      </label>
    </div>

    <div class="settings-block">
      <label class="block">
        <span class="settings-text">{t('settingsPlane.apiKeyLabel')}</span>
        <div class="mt-2 flex items-center gap-2">
          <input
            type={showKey ? 'text' : 'password'}
            value={plane().api_key ?? ''}
            on:input={onApiKeyInput}
            class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
            autocomplete="off"
          />
          <button
            type="button"
            on:click={() => (showKey = !showKey)}
            class="shrink-0 rounded-lg border border-slate-200 px-3 py-2 text-xs text-slate-500 hover:text-slate-700 dark:border-[#2d3843] dark:text-slate-400 dark:hover:text-slate-200"
          >
            {showKey ? t('settingsPlane.hideKey') : t('settingsPlane.showKey')}
          </button>
        </div>
        <span class="settings-muted">{t('settingsPlane.apiKeyHint')}</span>
      </label>
    </div>

    <div class="settings-block">
      <label class="block">
        <span class="settings-text">{t('settingsPlane.workspaceSlugLabel')}</span>
        <input
          type="text"
          placeholder="my-team"
          value={plane().workspace_slug ?? ''}
          on:input={onWorkspaceSlugInput}
          class="mt-2 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
        />
        <span class="settings-muted">{t('settingsPlane.workspaceSlugHint')}</span>
      </label>
    </div>

    <div class="settings-block">
      <button
        type="button"
        on:click={testConnection}
        disabled={testing || !hasCredentials()}
        class="page-action-brand"
      >
        {#if testing}
          <span class="inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-white border-t-transparent"></span>
          {t('settingsPlane.testing')}
        {:else}
          {t('settingsPlane.testConnection')}
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

    <div class="settings-block">
      <div class="flex items-center justify-between gap-2">
        <button
          type="button"
          on:click={fetchPreview}
          disabled={previewing || !hasCredentials()}
          class="page-action-brand"
        >
          {#if previewing}
            <span class="inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-white border-t-transparent"></span>
            {t('settingsPlane.previewing')}
          {:else}
            {t('settingsPlane.previewButton')}
          {/if}
        </button>
      </div>

      {#if previewError}
        <p class="mt-3 break-all rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:border-rose-500/30 dark:bg-rose-500/10 dark:text-rose-200">
          {previewError}
        </p>
      {:else if previewItems.length === 0 && previewLoaded}
        <p class="settings-muted mt-3">{t('settingsPlane.previewEmpty')}</p>
      {/if}

      {#if previewItems.length > 0}
        <ul class="mt-3 divide-y divide-slate-100 rounded-lg border border-slate-200 dark:divide-[#222b34] dark:border-[#2d3843]">
          {#each previewItems as item (item.project + item.name + item.updated_at)}
            <li class="px-3 py-2">
              <div class="truncate text-sm font-medium text-slate-900 dark:text-[#e6edf3]">{item.name}</div>
              <div class="mt-0.5 truncate text-xs text-slate-500 dark:text-slate-400">
                {item.project}{item.state ? ` · ${item.state}` : ''}{item.updated_at ? ` · ${item.updated_at}` : ''}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
</div>
