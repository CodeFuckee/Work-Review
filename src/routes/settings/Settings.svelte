<script lang="ts">
  import { onMount, setContext, afterUpdate } from 'svelte';
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
  import SettingsWorkSync from './components/SettingsWorkSync.svelte';
  // SMUI v8：Tab/Button/Textfield/CircularProgress；bare.css 集中在 src/smui.css 引入
  import Tab from '@smui/tab';
  import Button from '@smui/button';
  import Textfield from '@smui/textfield';
  import CircularProgress from '@smui/circular-progress';
  import MdIcon from '../../lib/components/MdIcon.svelte';
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

  // ═══ SMUI Tab（@smui/tab v8 未导出 TabBar）═══
  // Tab 初始化强依赖 TabBar 提供的 context（focusOnActivate / initialActive），
  // 这里手动等价提供：初始激活 'general'；后续激活态由 activeTab 经 afterUpdate
  // 驱动各 Tab 的 activate/deactivate（受控语义等价 Carbon Tabs 的 selected）。
  setContext('SMUI:tab:focusOnActivate', false);
  setContext('SMUI:tab:initialActive', { active: 'general' as SettingsTabId | null, key: (key: unknown) => key });

  // Tab 组件实例只导出 activate/deactivate/focus/getElement（active 是内部状态，
  // 激活与否用根元素上的 mdc-tab--active 类判断）
  interface TabHandle {
    activate(previousIndicatorClientRect: DOMRect | undefined, skipFocus: boolean): void;
    deactivate(): void;
    getElement(): HTMLButtonElement;
  }

  let tabRefs: (TabHandle | undefined)[] = [];
  $: activeTabIndex = tabs.findIndex((tab) => tab.id === activeTab);
  afterUpdate(() => {
    tabRefs.forEach((ref, index) => {
      if (!ref) return;
      const isActive = ref.getElement().classList.contains('mdc-tab--active');
      if (index === activeTabIndex && !isActive) ref.activate(undefined, true);
      else if (index !== activeTabIndex && isActive) ref.deactivate();
    });
  });

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
      variant="raised"
      disabled={loading || saving}
      onclick={saveConfig}
    >
      {#if saving}
        <span class="inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
      {/if}
      {#if success}{t('settings.saved')}{:else}{t('settings.save')}{/if}
    </Button>
  </div>

  {#if loading}
    <div class="settings-carbon-loading" role="status" aria-label={t('settings.loading') || 'Loading'}>
      <CircularProgress indeterminate class="settings-loading-spinner" />
    </div>
  {:else if error}
    <!-- MD banner 形态：替代 Carbon InlineNotification（错误 + 重试动作按钮） -->
    <div class="md-banner" role="alert">
      <MdIcon name="error" size={20} />
      <span class="md-banner-copy"><strong>{t('settings.loadError')}</strong>{error}</span>
      <button class="md-text-btn" onclick={loadConfig}>{t('settings.retry')}</button>
    </div>
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
            <Button variant="text" onclick={() => push('/auth')}>{t('auth.reloginAction')}</Button>
            <Button variant="text" onclick={handleAccountLogout}>{t('auth.logoutAction')}</Button>
          </div>
        {:else}
          <p class="settings-account-meta">{t('settings.accountNotLoggedIn')}</p>
          <Button variant="text" onclick={() => push('/auth')}>{t('auth.loginTab')}</Button>
        {/if}
      </div>

      <div class="settings-account-server">
        <Textfield
          class="settings-account-server-field"
          input$id="account-server-url"
          variant="outlined"
          bind:value={accountServerUrl}
          label={t('settings.accountServerLabel')}
          placeholder="http://10.37.80.25:4191"
          input$spellcheck={false}
        />
        <Button
          variant="text"
          onclick={saveAccountServerUrl}
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

    <!-- 设置页签（SMUI Tab）：@smui/tab v8 未导出 TabBar，Tab 所需 context 由上方 setContext
         等价提供，激活态由 activeTab 经 afterUpdate 驱动（等价 Carbon 受控 selected 语义）；
         下方 9 个 role=tabpanel div 维持 hidden 按需懒渲染架构 -->
    <div class="settings-tab-strip" role="tablist">
      {#each tabs as tab, index (tab.id)}
        <Tab
          bind:this={tabRefs[index]}
          tab={tab.id}
          tabindex={0}
          onclick={() => { activeTab = tab.id; }}
        >
          <span class="mdc-tab__text-label">{t(tab.labelKey) + (tab.beta ? ' · Beta' : '')}</span>
        </Tab>
      {/each}
    </div>
    {#each tabs as tab (tab.id)}
      <div role="tabpanel" hidden={activeTab !== tab.id}>
        {#if activeTab === tab.id}
          <div class="settings-carbon-panel">
          {#if activeTab === 'general'}
            <SettingsSync bind:config on:change={() => dirty = true} />
            <SettingsWorkSync bind:config on:change={() => dirty = true} />
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
  {/if}
</div>

<style>
  .settings-carbon-shell {
    min-height: 100%;
    background: var(--md-theme-background);
    color: var(--md-theme-on-surface);
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
    color: var(--md-theme-on-surface-medium);
    font-size: 0.8125rem;
    margin-top: 0.125rem;
  }

  .settings-carbon-loading {
    display: flex;
    justify-content: center;
    padding: 3rem 0;
  }

  /* class 经 prop 落在 CircularProgress 子组件根元素上，需 :global 才能命中 */
  :global(.settings-loading-spinner) {
    width: 28px;
    height: 28px;
    --md-circular-progress-size: 28px;
  }

  /* MD banner 形态（与 App.svelte 壳层的 grace banner 同款，此处为本组件局部副本） */
  .md-banner {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.625rem 0.875rem;
    border: 1px solid var(--md-theme-border-subtle);
    border-radius: 8px;
    background: var(--md-theme-surface-variant);
    color: var(--md-theme-on-surface);
    font-size: 0.875rem;
  }

  .md-banner-copy {
    min-width: 0;
  }

  .md-banner-copy strong {
    margin-inline-end: 0.375rem;
  }

  .md-text-btn {
    margin-inline-start: auto;
    padding: 0.375rem 0.75rem;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--md-theme-primary);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }

  .md-text-btn:hover {
    background: var(--md-theme-border-subtle);
  }

  /* SMUI Tab 页签条：@smui/tab 无 TabBar 容器样式，这里补条状布局 */
  .settings-tab-strip {
    display: flex;
    align-items: stretch;
    overflow-x: auto;
    border-bottom: 1px solid var(--md-theme-border-subtle);
  }

  .settings-carbon-card {
    background: var(--md-theme-surface);
    border: 1px solid var(--md-theme-border-subtle);
    border-radius: 8px;
    padding: 1.25rem 1.375rem;
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .settings-carbon-card-desc {
    font-size: 0.8125rem;
    color: var(--md-theme-on-surface-medium);
  }

  .settings-carbon-hint {
    font-size: 0.6875rem;
    color: var(--md-theme-on-surface-medium);
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
    color: var(--md-theme-on-surface-medium);
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

  .settings-account-server :global(.mdc-text-field) {
    flex: 1;
    min-width: 0;
  }

  .settings-carbon-panel {
    padding-top: 1rem;
  }
</style>
