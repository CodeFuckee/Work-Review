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
  import { clearStoredSyncKey } from '$lib/utils/configSync.ts';

  import SettingsGeneral from './components/SettingsGeneral.svelte';
  import SettingsAppearance from './components/SettingsAppearance.svelte';
  import SettingsAI from './components/SettingsAI.svelte';
  import SettingsGitlab from './components/SettingsGitlab.svelte';
  import SettingsPlane from './components/SettingsPlane.svelte';
  import SettingsFileMonitor from './components/SettingsFileMonitor.svelte';
  import SettingsSync from './components/SettingsSync.svelte';
  import Tabs from 'carbon-components-svelte/src/Tabs/Tabs.svelte';
  import Tab from 'carbon-components-svelte/src/Tabs/Tab.svelte';
  import Button from 'carbon-components-svelte/src/Button/Button.svelte';
  import TextInput from 'carbon-components-svelte/src/TextInput/TextInput.svelte';
  import InlineNotification from 'carbon-components-svelte/src/Notification/InlineNotification.svelte';
  import NotificationActionButton from 'carbon-components-svelte/src/Notification/NotificationActionButton.svelte';
  import Loading from 'carbon-components-svelte/src/Loading/Loading.svelte';
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
    localhost_api_enabled?: boolean;
    localhost_api_port?: number;
    localhost_api_host?: string | null;
    text_model_provider_cache?: Record<string, unknown>;
    node_devices?: unknown[];
    daily_report_export_dir?: string | null;
    daily_report_auto_export?: boolean;
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
    clearStoredSyncKey();
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

<div class="settings-carbon-shell" data-locale={currentLocale}>
  <div class="settings-carbon-header">
    <div>
      <h2 class="cds--type-heading-04">{t('settings.title')}</h2>
      <p class="settings-carbon-subtitle">{t('settings.subtitle')}</p>
    </div>
    <Button
      kind="primary"
      loading={saving}
      disabled={loading || saving}
      on:click={saveConfig}
    >
      {#if success}{t('settings.saved')}{:else}{t('settings.save')}{/if}
    </Button>
  </div>

  {#if loading}
    <div class="settings-carbon-loading">
      <Loading description={t('settings.loading') || 'Loading'} withOverlay={false} />
    </div>
  {:else if error}
    <InlineNotification
      kind="error"
      title={t('settings.loadError')}
      subtitle={error}
      hideCloseButton={true}
      timeout={0}
    >
      <NotificationActionButton slot="actions" on:click={loadConfig}>
        {t('settings.retry')}
      </NotificationActionButton>
    </InlineNotification>
  {:else if config}
    <!-- 账号区块 -->
    <section class="settings-carbon-card">
      <h3 class="cds--type-heading-02">{t('settings.accountTitle')}</h3>
      <p class="settings-carbon-card-desc">{t('settings.accountDesc')}</p>

      <div class="settings-account-row">
        {#if accountState}
          <div class="settings-account-id">
            <p class="settings-account-name">{accountState.name || accountState.userName}</p>
            <p class="settings-account-meta">{accountState.userName} · {t('settings.accountLoggedInAs')}</p>
          </div>
          <div class="settings-account-actions">
            <Button kind="ghost" size="small" on:click={() => push('/auth')}>{t('auth.reloginAction')}</Button>
            <Button kind="ghost" size="small" on:click={handleAccountLogout}>{t('auth.logoutAction')}</Button>
          </div>
        {:else}
          <p class="settings-account-meta">{t('settings.accountNotLoggedIn')}</p>
          <Button kind="ghost" size="small" on:click={() => push('/auth')}>{t('auth.loginTab')}</Button>
        {/if}
      </div>

      <div class="settings-account-server">
        <TextInput
          id="account-server-url"
          bind:value={accountServerUrl}
          labelText={t('settings.accountServerLabel')}
          placeholder="http://10.37.80.25:4191"
          spellcheck={false}
        />
        <Button
          kind="ghost"
          size="small"
          on:click={saveAccountServerUrl}
        >
          {accountServerSaved ? t('settings.saved') : t('settings.save')}
        </Button>
      </div>
      <p class="settings-carbon-hint">{t('settings.accountServerHint')}</p>
    </section>

    {#if settingsRuntimePlatform === 'macos'}
      <section class="settings-carbon-card">
        <SettingsSystem />
      </section>
    {/if}

    <!-- 设置页签（Carbon Tabs：0.113 重写版 —— 导航项 + slot="content" 面板区，
         面板须为 role=tabpanel 直接子级并由 hidden 按索引切换，内容按需懒渲染） -->
    <Tabs selected={0} on:change={(e) => { activeTab = tabs[e.detail]?.id ?? 'general'; }}>
      {#each tabs as tab (tab.id)}
        <Tab label={t(tab.labelKey) + (tab.beta ? ' · Beta' : '')} />
      {/each}
      <svelte:fragment slot="content">
        {#each tabs as tab (tab.id)}
          <div role="tabpanel" hidden={activeTab !== tab.id}>
            {#if activeTab === tab.id}
              <div class="settings-carbon-panel">
              {#if activeTab === 'general'}
                <SettingsSync bind:config on:change={() => dirty = true} />
                <SettingsGeneral bind:config on:change={() => dirty = true} />
              {:else if activeTab === 'appearance'}
                <SettingsAppearance bind:config on:change={handleSettingsChange} />
              {:else if activeTab === 'node'}
                <SettingsNodeGateway bind:config {dataDir} on:change={() => dirty = true} />
              {:else if activeTab === 'ai'}
                <section class="settings-carbon-card">
                  <h3 class="cds--type-heading-02">{t('settings.aiCardTitle')}</h3>
                  <p class="settings-carbon-card-desc">{t('settings.aiCardDescription')}</p>
                  <SettingsAI bind:config {providers} on:change={() => dirty = true} />
                </section>
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
            {/if}
          </div>
        {/each}
      </svelte:fragment>
    </Tabs>
  {/if}
</div>

<style>
  .settings-carbon-shell {
    min-height: 100%;
    background: var(--cds-background);
    color: var(--cds-text-primary);
    padding: 1.5rem clamp(1rem, 3vw, 2.5rem) 2.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .settings-carbon-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .settings-carbon-subtitle {
    color: var(--cds-text-secondary);
    font-size: 0.8125rem;
    margin-top: 0.125rem;
  }

  .settings-carbon-loading {
    display: flex;
    justify-content: center;
    padding: 3rem 0;
  }

  .settings-carbon-card {
    background: var(--cds-layer);
    border: 1px solid var(--cds-border-subtle);
    border-radius: 8px;
    padding: 1.25rem 1.375rem;
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .settings-carbon-card-desc {
    font-size: 0.8125rem;
    color: var(--cds-text-secondary);
  }

  .settings-carbon-hint {
    font-size: 0.6875rem;
    color: var(--cds-text-secondary);
  }

  .settings-account-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.75rem;
  }

  .settings-account-id {
    min-width: 0;
  }

  .settings-account-name {
    font-size: 0.875rem;
    font-weight: 600;
  }

  .settings-account-meta {
    font-size: 0.75rem;
    color: var(--cds-text-secondary);
  }

  .settings-account-actions {
    display: flex;
    gap: 0.375rem;
    margin-left: auto;
  }

  .settings-account-server {
    display: flex;
    align-items: end;
    gap: 0.5rem;
  }

  .settings-account-server :global(.bx--form-item) {
    flex: 1;
    min-width: 0;
  }

  .settings-carbon-panel {
    padding-top: 1rem;
  }
</style>
