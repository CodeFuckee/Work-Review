<script lang="ts">
  // 跨设备配置同步卡（连接类配置整包推/拉）：
  // - 云端仅存客户端加密密文（账号派生密钥 AES-GCM），服务端不可读；
  // - 拉取先弹逐块差异预览，确认后应用并走常规保存管线；
  // - HTTP 经 Rust 命令发出（绕开 WebView 混合内容拦截，与登录同款约束）。
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getAdminUrl, getAuth } from '$lib/stores/auth.ts';
  import { showToast } from '$lib/stores/toast.ts';
  import { locale, t } from '$lib/i18n/index.ts';
  import {
    SYNC_BLOCK_LABELS,
    SYNC_BLOCK_KEYS,
    applySyncPayload,
    decryptPayload,
    diffSyncBlocks,
    deriveSyncKey,
    encryptPayload,
    extractSyncPayload,
    getStoredSyncKey,
    isSyncAuthFailure,
    type SyncCloudMeta,
    type SyncPayload,
  } from '$lib/utils/configSync.ts';
  import Button from '@smui/button';
  import CircularProgress from '@smui/circular-progress';
  import Dialog, { Title, Content, Actions } from '@smui/dialog';
  import MdIcon from '../../../lib/components/MdIcon.svelte';

  interface ClientHttpPostJsonResult {
    delivered: boolean;
    status?: number;
    body?: string;
    error?: string;
  }

  // 刻意 any：Settings.svelte 的 SettingsConfig 是多面板交叉类型，
  // 与 Record<string, unknown> 结构不兼容；卡片只经 extract/apply 触碰同步键
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export let config: any;

  const dispatch = createEventDispatcher<{ change: Record<string, unknown> }>();
  $: currentLocale = $locale;

  let cloudMeta: SyncCloudMeta | null = null;
  let statusLoading = false;
  let working = false;
  let showDiffModal = false;
  let pendingPull: SyncPayload | null = null;
  // token 失效（服务端 401 / 旧版 200+success:false）：状态区改显重新登录提示，
  // 上传/拉取不再走业务路径——此前只看 HTTP 状态码，认证失败被当成空配置，
  // 上传假成功、状态恒显「云端暂无配置」（v1.2.26 用户实测事故）
  let authExpired = false;

  $: auth = getAuth();
  $: canSync = Boolean(auth?.token && getAdminUrl());
  let hasSyncKey = false;
  async function refreshSyncKey(): Promise<void> { hasSyncKey = Boolean(await getStoredSyncKey()); }

  // SMUI Dialog 受控关闭（scrim 点击/Esc）：弹层关闭即丢弃待拉取载荷
  $: if (!showDiffModal) {
    pendingPull = null;
  }

  function blockLabel(block: string): string {
    return SYNC_BLOCK_LABELS[block] ?? block;
  }

  async function refreshCloudStatus(): Promise<void> {
    const currentAuth = getAuth();
    if (!currentAuth?.token || !getAdminUrl()) return;
    statusLoading = true;
    try {
      const res = await invoke<ClientHttpPostJsonResult>('client_http_get_json', {
        url: `${getAdminUrl()}/api/admin/client-config/get-sync-config`,
        authToken: currentAuth.token,
      });
      // 未认证：新版服务端 401 / 旧版 200+success:false——静默置位（状态区显示
      // 重新登录提示），不打扰 toast
      if (res.status === 401) {
        authExpired = true;
        cloudMeta = null;
        return;
      }
      if (!res.delivered || res.status !== 200 || !res.body) {
        cloudMeta = null;
        return;
      }
      const parsed = JSON.parse(res.body) as { data?: SyncCloudMeta };
      if (isSyncAuthFailure(parsed.data)) {
        authExpired = true;
        cloudMeta = null;
        return;
      }
      authExpired = false;
      cloudMeta = parsed.data ?? null;
    } catch (e) {
      console.warn('获取云端配置状态失败:', e);
      cloudMeta = null;
    } finally {
      statusLoading = false;
    }
  }

  onMount(() => {
    void refreshCloudStatus();
    void refreshSyncKey();
  });

  async function handlePull(): Promise<void> {
    if (working) return;
    const key = await getStoredSyncKey();
    if (!key) {
      showToast(t('settingsSync.needRelogin'), 'warning');
      return;
    }
    working = true;
    try {
      const currentAuth = getAuth();
      const res = await invoke<ClientHttpPostJsonResult>('client_http_get_json', {
        url: `${getAdminUrl()}/api/admin/client-config/get-sync-config`,
        authToken: currentAuth?.token,
      });
      if (!res.delivered) {
        showToast(t('settingsSync.networkError', { error: res.error ?? '' }), 'error');
        return;
      }
      // 未认证：新版 401 / 旧版 200+success:false——引导重新登录，绝不当作空配置
      if (res.status === 401) {
        showToast(t('settingsSync.authExpired'), 'warning');
        authExpired = true;
        return;
      }
      if (res.status !== 200 || !res.body) {
        showToast(t('settingsSync.networkError', { error: `HTTP ${res.status}` }), 'error');
        return;
      }
      const parsed = JSON.parse(res.body) as { data?: SyncCloudMeta };
      const meta = parsed.data;
      if (isSyncAuthFailure(meta)) {
        showToast(t('settingsSync.authExpired'), 'warning');
        authExpired = true;
        return;
      }
      if (!meta?.payload || !meta.iv) {
        showToast(t('settingsSync.cloudEmpty'), 'info');
        return;
      }
      const plaintext = await decryptPayload(key, meta.payload, meta.iv);
      pendingPull = JSON.parse(plaintext) as SyncPayload;
      showDiffModal = true;
    } catch (e) {
      console.warn('拉取云端配置失败:', e);
      showToast(t('settingsSync.pullFailed', { error: e }), 'error');
    } finally {
      working = false;
    }
  }

  async function confirmPull(): Promise<void> {
    if (!pendingPull) return;
    const merged = applySyncPayload(config, pendingPull);
    // 清空被覆盖的键：云端缺失/为空的块在本机也应清空（整包覆盖语义）
    for (const keys of Object.values(SYNC_BLOCK_KEYS)) {
      for (const key of keys) {
        if (!(key in pendingPull)) merged[key] = undefined;
      }
    }
    dispatch('change', merged);
    try {
      await invoke<void>('save_config', { config: merged });
    } catch (e) {
      console.warn('保存同步配置失败:', e);
    }
    showDiffModal = false;
    pendingPull = null;
    showToast(t('settingsSync.pullApplied'), 'success');
    void refreshCloudStatus();
  }

  function cancelPull(): void {
    showDiffModal = false;
    pendingPull = null;
  }

  async function handlePush(): Promise<void> {
    if (working) return;
    const key = await getStoredSyncKey();
    if (!key) {
      showToast(t('settingsSync.needRelogin'), 'warning');
      return;
    }
    const payload = extractSyncPayload(config);
    if (Object.keys(payload).length === 0) {
      showToast(t('settingsSync.nothingToPush'), 'warning');
      return;
    }
    working = true;
    try {
      const plaintext = JSON.stringify(payload);
      const { cipher, iv } = await encryptPayload(key, plaintext);
      const currentAuth = getAuth();
      const res = await invoke<ClientHttpPostJsonResult>('client_http_post_json', {
        url: `${getAdminUrl()}/api/admin/client-config/save-sync-config`,
        body: { payload: cipher, iv },
        authToken: currentAuth?.token,
      });
      // 未认证：新版 401 / 旧版 200+success:false——服务端什么都没存，
      // 必须引导重新登录，绝不能弹「上传成功」假成功
      if (res.status === 401) {
        showToast(t('settingsSync.authExpired'), 'warning');
        authExpired = true;
        return;
      }
      if (!res.delivered || res.status !== 200) {
        showToast(t('settingsSync.pushFailed', { error: `HTTP ${res.status ?? ''}` }), 'error');
        return;
      }
      try {
        const parsed = JSON.parse(res.body ?? '') as { data?: SyncCloudMeta };
        if (isSyncAuthFailure(parsed.data)) {
          showToast(t('settingsSync.authExpired'), 'warning');
          authExpired = true;
          return;
        }
      } catch {
        // 响应体不是 JSON：按非 200 同类处理
        showToast(t('settingsSync.pushFailed', { error: `HTTP ${res.status}` }), 'error');
        return;
      }
      showToast(t('settingsSync.pushDone'), 'success');
      void refreshCloudStatus();
    } catch (e) {
      showToast(t('settingsSync.pushFailed', { error: e }), 'error');
    } finally {
      working = false;
    }
  }

  function diffStateLabel(state: string): string {
    switch (state) {
      case 'same': return t('settingsSync.diffSame');
      case 'local-only': return t('settingsSync.diffLocalOnly');
      case 'cloud-only': return t('settingsSync.diffCloudOnly');
      case 'differs': return t('settingsSync.diffDiffers');
      default: return t('settingsSync.diffBothEmpty');
    }
  }

  // 差异状态 → md-chip 变体（differs/local-only 需语义色，其余走全局 outlined/tonal）
  function diffChipClass(state: string): string {
    switch (state) {
      case 'differs': return 'sync-diff-chip-differs';
      case 'cloud-only': return 'md-chip-tonal';
      case 'local-only': return 'sync-diff-chip-local';
      default: return 'md-chip-outlined';
    }
  }
</script>

<div class="sync-carbon" data-locale={currentLocale}>
  <div class="sync-head">
    <div>
      <h3 class="cds--type-heading-02">{t('settingsSync.title')}</h3>
      <p class="sync-desc">{t('settingsSync.desc')}</p>
    </div>
    {#if statusLoading}
      <span class="sync-loading"><CircularProgress indeterminate /></span>
    {/if}
  </div>

  {#if !auth?.token}
    <p class="sync-hint">{t('settingsSync.loginRequired')}</p>
  {:else if !hasSyncKey}
    <p class="sync-hint">{t('settingsSync.needRelogin')}</p>
  {:else}
    <div class="sync-status-row">
      <span class="sync-status-label">{t('settingsSync.cloudStatus')}</span>
      {#if authExpired}
        <span class="sync-hint">{t('settingsSync.authExpired')}</span>
      {:else if cloudMeta?.updatedTime}
        <span class="md-chip md-chip-tonal">{t('settingsSync.cloudUpdatedAt', { time: cloudMeta.updatedTime })}</span>
      {:else if cloudMeta}
        <span class="md-chip md-chip-outlined">{t('settingsSync.cloudEmpty')}</span>
      {/if}
    </div>
    <div class="sync-actions">
      <Button variant="outlined" disabled={working} onclick={handlePull}>
        <span class="mdc-button__icon"><MdIcon name="download" size={18} /></span>
        {t('settingsSync.pull')}
      </Button>
      <Button variant="raised" disabled={working} onclick={handlePush}>
        <span class="mdc-button__icon"><MdIcon name="upload" size={18} /></span>
        {t('settingsSync.push')}
      </Button>
    </div>
    <p class="sync-hint">{t('settingsSync.scopeHint')}</p>
  {/if}
</div>

{#if showDiffModal && pendingPull}
  <Dialog bind:open={showDiffModal} aria-labelledby="sync-diff-title">
    <Title id="sync-diff-title" class="sync-diff-title">{t('settingsSync.diffTitle')}</Title>
    <Content>
      <p class="sync-diff-intro">{t('settingsSync.diffIntro')}</p>
      <div class="sync-diff-list">
        {#each Object.entries(diffSyncBlocks(extractSyncPayload(config), pendingPull)) as [block, state] (block)}
          <div class="sync-diff-row">
            <span class="sync-diff-block">{blockLabel(block)}</span>
            <span class="md-chip {diffChipClass(state)}">{diffStateLabel(state)}</span>
          </div>
        {/each}
      </div>
    </Content>
    <Actions>
      <Button class="sync-diff-btn" onclick={cancelPull}>{t('common.cancel')}</Button>
      <Button
        class="sync-diff-btn"
        variant="raised"
        onclick={() => { void confirmPull(); }}
      >
        {t('settingsSync.applyPull')}
      </Button>
    </Actions>
  </Dialog>
{/if}

<style>
  .sync-carbon {
    background: var(--md-theme-surface);
    border: 1px solid var(--md-theme-border-subtle);
    border-radius: 8px;
    padding: 1.25rem 1.375rem;
    display: flex;
    flex-direction: column;
    gap: 0.875rem;
  }

  .sync-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .sync-desc {
    font-size: 0.8125rem;
    color: var(--md-theme-on-surface-medium);
    margin-top: 0.125rem;
  }

  .sync-hint {
    font-size: 0.75rem;
    color: var(--md-theme-on-surface-medium);
  }

  .sync-loading {
    display: inline-flex;
    width: 18px;
    height: 18px;
    --md-circular-progress-size: 18px;
    --mdc-circular-progress-active-indicator-color: var(--md-theme-primary);
    vertical-align: -3px;
  }

  .sync-status-row {
    display: flex;
    align-items: center;
    gap: 0.625rem;
  }

  .sync-status-label {
    font-size: 0.8125rem;
  }

  .sync-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .sync-diff-title {
    font-size: 1.125rem;
    font-weight: 500;
  }

  .sync-diff-intro {
    font-size: 0.8125rem;
    color: var(--md-theme-on-surface-medium);
    margin-bottom: 0.75rem;
  }

  .sync-diff-list {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .sync-diff-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .sync-diff-block {
    font-size: 0.8125rem;
    font-weight: 500;
  }

  /* 差异语义色 chip：底色用半透明色块（明暗主题均成立），文字走主题令牌 */
  .sync-diff-chip-differs {
    background: rgba(176, 0, 32, 0.14);
    color: var(--md-theme-error);
  }

  .sync-diff-chip-local {
    background: rgba(67, 160, 71, 0.16);
    color: #2e7d32;
  }

  .sync-diff-btn :global(.mdc-button__label) {
    font-size: 0.875rem;
  }
</style>
