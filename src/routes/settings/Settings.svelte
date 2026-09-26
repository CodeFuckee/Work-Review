<script lang="ts">
  import { onMount } from 'svelte';
  import type { ComponentProps } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { push } from 'svelte-spa-router';
  import { cache } from '../../lib/stores/cache.ts';
  import { locale, t } from '$lib/i18n/index.ts';
  import { formatUserError } from '$lib/utils/errorDisplay.ts';
  import { showToast } from '../../lib/stores/toast.ts';
  import { clearAuth, getAdminUrl, getAuth, setAdminUrl, type AuthState } from '$lib/stores/auth.ts';

  import SettingsGeneral from './components/SettingsGeneral.svelte';
  import SettingsAppearance from './components/SettingsAppearance.svelte';
  import SettingsAI from './components/SettingsAI.svelte';
  import SettingsGitlab from './components/SettingsGitlab.svelte';
  import SettingsPlane from './components/SettingsPlane.svelte';
  import SettingsFileMonitor from './components/SettingsFileMonitor.svelte';
  import SettingsNodeGateway from './components/SettingsNodeGateway.svelte';
  import SettingsSystem from './components/SettingsSystem.svelte';
  import SettingsPrivacy from './components/SettingsPrivacy.svelte';
  import SettingsStorage from './components/SettingsStorage.svelte';

  type GeneralConfig = ComponentProps<SettingsGeneral>['config'];
  type AppearanceConfig = ComponentProps<SettingsAppearance>['config'];
  type AiConfig = ComponentProps<SettingsAI>['config'];
  type NodeGatewayConfig = ComponentProps<SettingsNodeGateway>['config'];
  type PrivacyConfig = ComponentProps<SettingsPrivacy>['config'];
  type StorageConfig = ComponentProps<SettingsStorage>['config'];
  type AiProvider = NonNullable<ComponentProps<SettingsAI>['providers']>[number];
  type StorageStats = NonNullable<ComponentProps<SettingsStorage>['storageStats']>;

  interface LegacyModelConfig {
    provider: string;
    endpoint: string;
    api_key: string | null;
    model: string;
    vision_model?: string;
  }

  interface RootSettingsFields {
    ai_provider: LegacyModelConfig;
    text_model_profiles: unknown[];
    daily_report_custom_prompt: string;
    daily_report_prompt_presets: unknown[];
    vision_model: LegacyModelConfig;
    app_category_rules: unknown[];
    privacy: PrivacyConfig['privacy'] & { sensitive_keywords?: string[] };
    storage: StorageConfig['storage'] & {
      metadata_retention_days: number;
      jpeg_quality: number;
    };
  }

  type SettingsConfig = GeneralConfig
    & AppearanceConfig
    & AiConfig
    & NodeGatewayConfig
    & PrivacyConfig
    & StorageConfig
    & RootSettingsFields
    & ComponentProps<SettingsGitlab>['config']
    & ComponentProps<SettingsPlane>['config']
    & ComponentProps<SettingsFileMonitor>['config'];

  type NullableCredentialKey =
    | 'feishu_app_id'
    | 'feishu_app_secret'
    | 'feishu_verification_token'
    | 'wecom_corp_id'
    | 'wecom_token'
    | 'wecom_encoding_aes_key'
    | 'wecom_bot_id'
    | 'wecom_bot_secret'
    | 'dingtalk_app_secret';

  type DraftPrivacySettings = Partial<PrivacyConfig['privacy']> & {
    sensitive_keywords?: string[];
  };

  type DraftSettingsConfig = Omit<
    SettingsConfig,
    | NullableCredentialKey
    | 'ai_provider'
    | 'text_model'
    | 'text_model_profiles'
    | 'daily_report_custom_prompt'
    | 'daily_report_prompt_presets'
    | 'node_gateway'
    | 'vision_model'
    | 'storage'
    | 'app_category_rules'
    | 'privacy'
    | 'ui_visual_style'
  > & {
    [Key in NullableCredentialKey]: string | null;
  } & {
    ai_provider: LegacyModelConfig | null;
    text_model: AiConfig['text_model'] | null;
    text_model_profiles?: unknown[];
    daily_report_custom_prompt?: string;
    daily_report_prompt_presets?: unknown[];
    node_gateway: (Partial<NodeGatewayConfig['node_gateway']> & { device_name: string | null }) | null;
    vision_model: LegacyModelConfig | null;
    storage: (Partial<RootSettingsFields['storage']> & {
      screenshots_enabled?: boolean;
      screenshot_display_mode?: string;
      screenshot_width_mode?: string;
    }) | null;
    app_category_rules?: unknown[];
    privacy: DraftPrivacySettings | null;
    ui_visual_style: string;
  };

  type SettingsTabId = 'general' | 'appearance' | 'ai' | 'privacy' | 'storage' | 'node' | 'gitlab' | 'plane' | 'filemonitor';

  interface SettingsTab {
    id: SettingsTabId;
    labelKey: string;
    icon: SettingsTabId;
    beta?: boolean;
  }

  let config: SettingsConfig | null = null;
  let loading = true;
  let saving = false;
  let dirty = false;
  let configLoaded = false;
  let error: string | null = null;
  let success = false;
  let providers: AiProvider[] = [];
  let runningApps: string[] = [];
  let recentApps: string[] = [];
  let storageStats: StorageStats | null = null;
  let dataDir = '';
  let defaultDataDir = '';
  let settingsRuntimePlatform = '';
  let successTimer: ReturnType<typeof setTimeout> | null = null;
  $: currentLocale = $locale;

  // 当前激活的标签
  let activeTab: SettingsTabId = 'general';

  const tabs: SettingsTab[] = [
    { id: 'general', labelKey: 'settings.tabs.general', icon: 'general' },
    { id: 'appearance', labelKey: 'settings.tabs.appearance', icon: 'appearance' },
    { id: 'ai', labelKey: 'settings.tabs.ai', icon: 'ai' },
    { id: 'privacy', labelKey: 'settings.tabs.privacy', icon: 'privacy' },
    { id: 'storage', labelKey: 'settings.tabs.storage', icon: 'storage' },
    { id: 'node', labelKey: 'settings.tabs.node', icon: 'node', beta: true },
    { id: 'gitlab', labelKey: 'settings.tabs.gitlab', icon: 'gitlab' },
    { id: 'plane', labelKey: 'settings.tabs.plane', icon: 'plane' },
    { id: 'filemonitor', labelKey: 'settings.tabs.fileMonitor', icon: 'filemonitor' },
  ];

  // 加载配置
  async function loadConfig() {
    loading = true;
    error = null;
    try {
      const [loadedConfig, loadedProviders, loadedStorageStats, loadedDataDir, loadedDefaultDataDir, loadedRuntimePlatform] = await Promise.all([
        invoke<DraftSettingsConfig>('get_config'),
        invoke<AiProvider[]>('get_ai_providers'),
        invoke<StorageStats>('get_storage_stats'),
        invoke<string>('get_data_dir'),
        invoke<string>('get_default_data_dir'),
        invoke<string>('get_runtime_platform'),
      ]);

      providers = loadedProviders;
      storageStats = loadedStorageStats;
      dataDir = loadedDataDir;
      defaultDataDir = loadedDefaultDataDir;
      settingsRuntimePlatform = loadedRuntimePlatform;

      // 确保对象存在
      if (!loadedConfig.ai_provider) {
        loadedConfig.ai_provider = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'llava', vision_model: 'llava' };
      }
      if (!loadedConfig.text_model) {
        loadedConfig.text_model = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'qwen2.5' };
      }
      if (!loadedConfig.text_model_profiles) {
        loadedConfig.text_model_profiles = [];
      }
      if (!loadedConfig.text_model_provider_cache) {
        loadedConfig.text_model_provider_cache = {};
      }
      if (typeof loadedConfig.daily_report_custom_prompt !== 'string') {
        loadedConfig.daily_report_custom_prompt = '';
      }
      if (typeof loadedConfig.daily_report_export_dir !== 'string' && loadedConfig.daily_report_export_dir !== null) {
        loadedConfig.daily_report_export_dir = null;
      }
      if (typeof loadedConfig.daily_report_auto_export !== 'boolean') {
        loadedConfig.daily_report_auto_export = false;
      }
      if (!Array.isArray(loadedConfig.daily_report_prompt_presets)) {
        loadedConfig.daily_report_prompt_presets = [];
      }
      if (typeof loadedConfig.localhost_api_enabled !== 'boolean') {
        loadedConfig.localhost_api_enabled = false;
      }
      if (!Number.isInteger(loadedConfig.localhost_api_port) || loadedConfig.localhost_api_port <= 0) {
        loadedConfig.localhost_api_port = 47831;
      }
      if (typeof loadedConfig.localhost_api_host !== 'string' && loadedConfig.localhost_api_host !== null) {
        loadedConfig.localhost_api_host = null;
      }
      if (!['a', 'b', 'c'].includes(loadedConfig.ui_visual_style)) {
        loadedConfig.ui_visual_style = 'c';
      }
      if (!Array.isArray(loadedConfig.node_devices)) {
        loadedConfig.node_devices = [];
      }
      if (typeof loadedConfig.feishu_bot_enabled !== 'boolean') {
        loadedConfig.feishu_bot_enabled = false;
      }
      if (typeof loadedConfig.feishu_app_id !== 'string' && loadedConfig.feishu_app_id !== null) {
        loadedConfig.feishu_app_id = null;
      }
      if (typeof loadedConfig.feishu_app_secret !== 'string' && loadedConfig.feishu_app_secret !== null) {
        loadedConfig.feishu_app_secret = null;
      }
      if (typeof loadedConfig.feishu_verification_token !== 'string' && loadedConfig.feishu_verification_token !== null) {
        loadedConfig.feishu_verification_token = null;
      }
      if (typeof loadedConfig.wecom_bot_enabled !== 'boolean') {
        loadedConfig.wecom_bot_enabled = false;
      }
      if (typeof loadedConfig.wecom_corp_id !== 'string' && loadedConfig.wecom_corp_id !== null) {
        loadedConfig.wecom_corp_id = null;
      }
      if (typeof loadedConfig.wecom_token !== 'string' && loadedConfig.wecom_token !== null) {
        loadedConfig.wecom_token = null;
      }
      if (typeof loadedConfig.wecom_encoding_aes_key !== 'string' && loadedConfig.wecom_encoding_aes_key !== null) {
        loadedConfig.wecom_encoding_aes_key = null;
      }
      if (typeof loadedConfig.wecom_bot_id !== 'string' && loadedConfig.wecom_bot_id !== null) {
        loadedConfig.wecom_bot_id = null;
      }
      if (typeof loadedConfig.wecom_bot_secret !== 'string' && loadedConfig.wecom_bot_secret !== null) {
        loadedConfig.wecom_bot_secret = null;
      }
      if (typeof loadedConfig.dingtalk_bot_enabled !== 'boolean') {
        loadedConfig.dingtalk_bot_enabled = false;
      }
      if (typeof loadedConfig.dingtalk_app_secret !== 'string' && loadedConfig.dingtalk_app_secret !== null) {
        loadedConfig.dingtalk_app_secret = null;
      }
      if (!loadedConfig.node_gateway || typeof loadedConfig.node_gateway !== 'object') {
        loadedConfig.node_gateway = {
          device_name: null,
        };
      }
      if (
        typeof loadedConfig.node_gateway.device_name !== 'string' &&
        loadedConfig.node_gateway.device_name !== null
      ) {
        loadedConfig.node_gateway.device_name = null;
      }
      if (!loadedConfig.vision_model) {
        loadedConfig.vision_model = { provider: 'ollama', endpoint: 'http://localhost:11434', api_key: null, model: 'llava' };
      }
      if (typeof loadedConfig.lightweight_mode !== 'boolean') {
        loadedConfig.lightweight_mode = false;
      }
      if (typeof loadedConfig.auto_start_silent !== 'boolean') {
        loadedConfig.auto_start_silent = false;
      }
      if (!loadedConfig.storage) {
        loadedConfig.storage = {
          screenshot_retention_days: 7,
          metadata_retention_days: 30,
          storage_limit_mb: 2048,
          jpeg_quality: 85,
          max_image_width: 1280,
          screenshots_enabled: true,
          screenshot_display_mode: 'active_window',
          screenshot_width_mode: 'auto',
        };
      }
      if (typeof loadedConfig.storage.screenshots_enabled !== 'boolean') {
        loadedConfig.storage.screenshots_enabled = true;
      }
      if (!loadedConfig.storage.screenshot_display_mode) {
        loadedConfig.storage.screenshot_display_mode = 'active_window';
      }
      if (!loadedConfig.storage.screenshot_width_mode || !['auto', 'fixed'].includes(loadedConfig.storage.screenshot_width_mode)) {
        loadedConfig.storage.screenshot_width_mode = 'auto';
      }
      if (!loadedConfig.app_category_rules) loadedConfig.app_category_rules = [];
      if (!loadedConfig.privacy) loadedConfig.privacy = {};
      if (!loadedConfig.privacy.app_rules) loadedConfig.privacy.app_rules = [];
      if (!loadedConfig.privacy.excluded_keywords) loadedConfig.privacy.excluded_keywords = [];
      delete loadedConfig.privacy.sensitive_keywords;

      config = loadedConfig as SettingsConfig;
      cache.setConfig(config);
      // 后端配置已加载：此后缓存 store 的旧快照不得再覆盖 config
      //（防御「缓存里的旧配置整体写回、抹掉新增设置项」类回归）
      configLoaded = true;
    } catch (e) {
      error = formatUserError(e, t('common.loadFailedRetry'));
      console.error('加载配置失败:', e);
      settingsRuntimePlatform = '';
    } finally {
      loading = false;
    }
  }

  // 加载运行中的应用
  async function loadRunningApps() {
    try {
      runningApps = await invoke<string[]>('get_running_apps');
    } catch (e) {
      console.error('获取运行应用失败:', e);
      runningApps = [];
    }
  }

  // 加载历史应用列表
  async function loadRecentApps() {
    try {
      recentApps = await invoke<string[]>('get_recent_apps');
    } catch (e) {
      console.error('获取历史应用失败:', e);
      recentApps = [];
    }
  }

  // 保存配置
  async function saveConfig() {
    if (!config) return;
    const currentConfig = config;
    saving = true;
    error = null;
    success = false;

    try {
      delete currentConfig.privacy?.sensitive_keywords;
      await invoke<void>('save_config', { config: currentConfig });
      success = true;
      dirty = false;
      cache.setConfig(currentConfig);
      showToast(t('settings.saveSuccessToast'), 'success');
      
      if (successTimer !== null) clearTimeout(successTimer);
      successTimer = setTimeout(() => {
        success = false;
        successTimer = null;
      }, 3000);
    } catch (e) {
      error = formatUserError(e, t('common.loadFailedRetry'));
    } finally {
      saving = false;
    }
  }

  function handleSettingsChange(event: CustomEvent<unknown>) {
    const detail = event.detail;
    const autosaved = typeof detail === 'object'
      && detail !== null
      && 'autosaved' in detail
      && detail.autosaved === true;
    dirty = !autosaved;
  }

  // 清理缓存回调
  async function handleClearCache() {
    try {
      const [latestStats, latestDataDir] = await Promise.all([
        invoke<StorageStats>('get_storage_stats'),
        invoke<string>('get_data_dir'),
      ]);
      storageStats = latestStats;
      dataDir = latestDataDir;
    } catch (e) {
      console.error('刷新存储状态失败:', e);
    }
  }

  async function handleDataDirChanged() {
    try {
      const [latestStats, latestDataDir] = await Promise.all([
        invoke<StorageStats>('get_storage_stats'),
        invoke<string>('get_data_dir'),
      ]);
      storageStats = latestStats;
      dataDir = latestDataDir;
      cache.clear();
    } catch (e) {
      console.error('切换数据目录后刷新状态失败:', e);
    }
  }

  // 账号区块（凭证在 localStorage，与 config 保存流程无关）
  let accountState: AuthState | null = null;
  let accountServerUrl = '';
  let accountServerSaved = false;
  let accountServerSaveTimer: ReturnType<typeof setTimeout> | null = null;

  function refreshAccountState(): void {
    accountState = getAuth();
    accountServerUrl = getAdminUrl();
  }

  function saveAccountServerUrl(): void {
    setAdminUrl(accountServerUrl);
    accountServerUrl = getAdminUrl();
    accountServerSaved = true;
    if (accountServerSaveTimer !== null) clearTimeout(accountServerSaveTimer);
    accountServerSaveTimer = setTimeout(() => {
      accountServerSaved = false;
      accountServerSaveTimer = null;
    }, 2000);
  }

  function handleAccountLogout(): void {
    clearAuth();
    refreshAccountState();
    // 通知 App.svelte 刷新认证门（无凭证会强制跳回登录页）
    window.dispatchEvent(new CustomEvent('auth-changed'));
  }

  onMount(() => {
    refreshAccountState();
    const unsubscribeCache = cache.subscribe((state) => {
      if (!state.config) return;
      // 保存中或用户已编辑配置时，不覆盖（避免丢弃未保存的修改）
      if (saving) return;
      if (config && dirty) return;
      config = state.config as SettingsConfig;
    });

    loadConfig();
    loadRunningApps();
    loadRecentApps();

    return () => {
      unsubscribeCache();
      if (successTimer !== null) clearTimeout(successTimer);
      if (accountServerSaveTimer !== null) clearTimeout(accountServerSaveTimer);
    };
  });
</script>

<div class="page-shell settings-editorial-shell" data-locale={currentLocale}>
  <div class="page-header page-axis-operation">
    <div class="page-title-group">
      <div class="page-title-badge">
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>{t('settings.title')}</h2>
        <p>{t('settings.subtitle')}</p>
      </div>
    </div>

    <!-- 保存按钮 -->
    <div class="settings-save-dock">
      <button
        on:click={saveConfig}
        disabled={loading || saving}
        class="settings-action-primary px-4 rounded-xl"
      >
        {#if saving}
          <div class="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent"></div>
          {t('settings.saving')}
        {:else if success}
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" /></svg>
          {t('settings.saved')}
        {:else}
          {t('settings.save')}
        {/if}
      </button>
    </div>
  </div>

  {#if loading}
    <div class="flex justify-center py-12">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
    </div>
  {:else if error}
    <div class="page-banner-error mb-6">
      <div>
        <p class="font-semibold">{t('settings.loadError')}</p>
        <p class="text-sm mt-1">{error}</p>
      </div>
      <button on:click={loadConfig} class="page-action-brand">{t('settings.retry')}</button>
    </div>
  {:else if config}
    <div class="w-full settings-editorial-board page-axis-operation">
      <div class="settings-card settings-account">
        <h3 class="settings-card-title">{t('settings.accountTitle')}</h3>
        <p class="settings-card-desc">{t('settings.accountDesc')}</p>

        <div class="mt-4 flex flex-wrap items-center gap-3">
          {#if accountState}
            <div class="min-w-0">
              <p class="truncate text-sm font-semibold text-slate-900 dark:text-[#e6edf3]">
                {accountState.name || accountState.userName}
              </p>
              <p class="truncate text-xs text-slate-500 dark:text-slate-400">
                {accountState.userName} · {t('settings.accountLoggedInAs')}
              </p>
            </div>
            <div class="ms-auto flex items-center gap-2">
              <button type="button" on:click={() => push('/auth')} class="page-control-btn min-h-9 px-3">
                {t('auth.reloginAction')}
              </button>
              <button type="button" on:click={handleAccountLogout} class="page-control-btn min-h-9 px-3">
                {t('auth.logoutAction')}
              </button>
            </div>
          {:else}
            <p class="text-sm text-slate-500 dark:text-slate-400">{t('settings.accountNotLoggedIn')}</p>
            <button type="button" on:click={() => push('/auth')} class="page-control-btn ms-auto min-h-9 px-3">
              {t('auth.loginTab')}
            </button>
          {/if}
        </div>

        <div class="mt-4">
          <label class="settings-label" for="account-server-url">{t('settings.accountServerLabel')}</label>
          <div class="mt-1.5 flex items-center gap-2">
            <input
              id="account-server-url"
              bind:value={accountServerUrl}
              spellcheck="false"
              placeholder="http://10.37.80.25:4191"
              class="min-w-0 flex-1 rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
            />
            <button type="button" on:click={saveAccountServerUrl} class="page-control-btn min-h-9 shrink-0 px-3">
              {accountServerSaved ? t('settings.saved') : t('settings.save')}
            </button>
          </div>
          <p class="settings-subtle mt-1.5">{t('settings.accountServerHint')}</p>
        </div>
      </div>

      {#if settingsRuntimePlatform === 'macos'}
        <div class="settings-card settings-top-status-zone">
          <SettingsSystem />
        </div>
      {/if}

      <div class="settings-stage-layout">
        <nav class="settings-tab-rail" aria-label={t('settings.title')}>
          {#each tabs as tab}
            <button
              on:click={() => activeTab = tab.id}
              class="settings-tab-rail-item {activeTab === tab.id ? 'settings-tab-rail-item-active' : ''}"
              aria-current={activeTab === tab.id ? 'page' : undefined}
            >
              <span class="settings-tab-rail-icon">
                {#if tab.icon === 'general'}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>
                {:else if tab.icon === 'appearance'}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 3.75a8.25 8.25 0 100 16.5h1.15a1.85 1.85 0 001.05-3.37 1.15 1.15 0 01.64-2.1h1.01A4.4 4.4 0 0020.25 10.4 6.65 6.65 0 0013.6 3.75H12z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7.8 10.1h.01M9.75 7.4h.01M13 7.05h.01M15.9 9.25h.01" /></svg>
                {:else if tab.icon === 'ai'}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" /></svg>
                {:else if tab.icon === 'node'}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 7h14M5 12h14M5 17h10" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 5a2 2 0 012-2h12a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V5z" /></svg>
                {:else if tab.icon === 'privacy'}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" /></svg>
                {:else if tab.icon === 'storage'}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 5.625c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" /></svg>
                {:else if tab.icon === 'gitlab'}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 3.5l1.6 4.2h-3.2L12 3.5z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M6.8 7.7L3.8 15a.9.9 0 00.4 1.1l7.8 4.7-2.2-7.1-3-6z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17.2 7.7l3 7.3a.9.9 0 01-.4 1.1l-7.8 4.7 2.2-7.1 3-6z" /></svg>
                {:else if tab.icon === 'plane'}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M6 12L3.269 3.125A59.769 59.769 0 0121.485 12 59.768 59.768 0 013.27 20.875L5.999 12zm0 0h7.5" /></svg>
                {:else if tab.icon === 'filemonitor'}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>
                {/if}
              </span>
              <span class="inline-flex items-center gap-1 whitespace-nowrap">
                <span>{t(tab.labelKey)}</span>
                {#if tab.beta}
                  <span class="inline-flex items-center rounded-full border border-amber-200 bg-amber-50 px-1 py-px text-[10px] font-semibold uppercase tracking-[0.06em] text-amber-700 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-200">
                    Beta
                  </span>
                {/if}
              </span>
            </button>
          {/each}
        </nav>

        <div class="settings-stage-shell">
        {#if activeTab === 'general'}
          <SettingsGeneral bind:config on:change={() => dirty = true} />
        {:else if activeTab === 'appearance'}
          <SettingsAppearance bind:config on:change={handleSettingsChange} />
        {:else if activeTab === 'node'}
          <SettingsNodeGateway bind:config {dataDir} on:change={() => dirty = true} />
        {:else if activeTab === 'ai'}
          <div class="settings-card settings-ai-shell">
            <h3 class="settings-card-title">{t('settings.aiCardTitle')}</h3>
            <p class="settings-card-desc">{t('settings.aiCardDescription')}</p>
            <SettingsAI bind:config {providers} on:change={() => dirty = true} />
          </div>
        {:else if activeTab === 'privacy'}
          <SettingsPrivacy
            bind:config
            {runningApps}
            {recentApps}
            on:change={() => dirty = true}
            on:refresh-apps={() => { loadRunningApps(); loadRecentApps(); }}
          />
        {:else if activeTab === 'storage'}
          <SettingsStorage
            bind:config
            {storageStats}
            {dataDir}
            {defaultDataDir}
            on:change={() => dirty = true}
            on:clearCache={handleClearCache}
            on:dataDirChanged={handleDataDirChanged}
          />
        {:else if activeTab === 'gitlab'}
          <SettingsGitlab bind:config on:change={() => dirty = true} />
        {:else if activeTab === 'plane'}
          <SettingsPlane bind:config on:change={() => dirty = true} />
        {:else if activeTab === 'filemonitor'}
          <SettingsFileMonitor bind:config on:change={() => dirty = true} />
        {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
