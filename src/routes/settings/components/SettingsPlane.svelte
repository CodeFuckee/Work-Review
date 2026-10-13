<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { locale, t } from '$lib/i18n/index.ts';
  // SMUI v8 Switch：change 事件实为 SMUISwitchChange（onchange 不触发），用 onSMUISwitchChange
  import Switch from '@smui/switch';

  interface PlaneSettings {
    enabled: boolean;
    url: string | null;
    api_key: string | null;
    workspace_slug: string | null;
    project_filter: string[] | null;
    allow_insecure_http: boolean;
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

  // Switch 受控影子：外部加载同步进开关，切换时回调把影子值经既有函数写回 config
  let planeEnabledSwitch = false;
  let planeAllowInsecureSwitch = false;
  $: planeEnabledSwitch = Boolean(config.plane?.enabled);
  $: planeAllowInsecureSwitch = Boolean(config.plane?.allow_insecure_http);

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
        project_filter: null,
        allow_insecure_http: false,
      };
    }
    return config.plane;
  }

  function handleChange() {
    dispatch('change', config);
  }

  function onUrlInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value.trim();
    config.plane = { ...plane(), url: value === '' ? null : value };
    handleChange();
  }

  function onApiKeyInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value;
    config.plane = { ...plane(), api_key: value === '' ? null : value };
    handleChange();
  }

  function onWorkspaceSlugInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value.trim();
    config.plane = { ...plane(), workspace_slug: value === '' ? null : value };
    handleChange();
  }

  // 项目允许列表：一行（或逗号分隔）一个项目名；空 = 全部项目。
  // 与 GitLab 仓库过滤的输入解析一致。
  function onFilterInput(event: Event) {
    const value = (event.currentTarget as HTMLTextAreaElement).value;
    const lines = value
      .split(/[\n,]/)
      .map((line) => line.trim())
      .filter((line) => line !== '');
    config.plane = { ...plane(), project_filter: lines };
    handleChange();
  }

  // 注意：Svelte 4 对「函数调用表达式.属性 = x」这类赋值无法自动失效依赖，
  // 必须以 config 根为赋值目标（config.plane = 新对象）视图才会更新
  function onEnabledToggle() {
    const current = plane();
    config.plane = { ...current, enabled: !current.enabled };
    handleChange();
  }

  function onAllowInsecureToggle() {
    const current = plane();
    config.plane = { ...current, allow_insecure_http: !current.allow_insecure_http };
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
        allowInsecureHttp: current.allow_insecure_http === true,
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
        allowInsecureHttp: current.allow_insecure_http === true,
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
        <Switch bind:checked={planeEnabledSwitch} onSMUISwitchChange={onEnabledToggle} aria-label={t('settingsPlane.enabledLabel')} />
      </div>
    </div>

    <div class="settings-block">
      <label class="block">
        <span class="settings-text">{t('settingsPlane.urlLabel')}</span>
        <input
          type="url"
          placeholder="https://plane.example.com"
          value={config.plane?.url ?? ''}
          on:input={onUrlInput}
          class="mt-2 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
        />
        <span class="settings-muted">{t('settingsPlane.urlHint')}</span>
        <span class="settings-muted">{t('settingsPlane.schemeHint')}</span>
      </label>
    </div>

    <div class="settings-block">
      <label class="block">
        <span class="settings-text">{t('settingsPlane.apiKeyLabel')}</span>
        <div class="mt-2 flex items-center gap-2">
          <input
            type={showKey ? 'text' : 'password'}
            value={config.plane?.api_key ?? ''}
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
          value={config.plane?.workspace_slug ?? ''}
          on:input={onWorkspaceSlugInput}
          class="mt-2 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
        />
        <span class="settings-muted">{t('settingsPlane.workspaceSlugHint')}</span>
      </label>
    </div>

    <div class="settings-block">
      <div class="flex items-center justify-between">
        <div>
          <div class="settings-text">{t('settingsPlane.allowInsecureLabel')}</div>
          <div class="settings-muted">{t('settingsPlane.allowInsecureHint')}</div>
        </div>
        <Switch bind:checked={planeAllowInsecureSwitch} onSMUISwitchChange={onAllowInsecureToggle} aria-label={t('settingsPlane.allowInsecureLabel')} />
      </div>
    </div>

    <div class="settings-block">
      <span class="settings-text">{t('settingsPlane.filterLabel')}</span>
      <textarea
        rows="3"
        placeholder="project-a&#10;project-b"
        value={(config.plane?.project_filter ?? []).join('\n')}
        on:input={onFilterInput}
        class="mt-2 w-full resize-y rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
      />
      <span class="settings-muted">{t('settingsPlane.filterHint')}</span>
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

      <p class="settings-muted mt-2">{t('settingsPlane.previewLimitHint')}</p>

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
