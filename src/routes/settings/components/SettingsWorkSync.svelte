<script lang="ts">
  // 多机工作记录同步卡：开关 + 机器命名 + 手动同步 + 状态行。
  // 同步范围=活动文字元数据（App/标题/时长/OCR，端侧加密），截图不同步；
  // 引擎在 src/lib/utils/workSync.ts（5 分钟轮询 + 日报生成前强制拉取），
  // 本卡只负责配置展示与手动触发。
  import { createEventDispatcher, onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { getAdminUrl, getAuth } from '$lib/stores/auth.ts';
  import { showToast } from '$lib/stores/toast.ts';
  import { locale, t } from '$lib/i18n/index.ts';
  import { syncWithStoredCredentials, type WorkSyncRunSummary } from '$lib/utils/workSync.ts';
  import { invoke } from '@tauri-apps/api/core';
  import Switch from '@smui/switch';
  import Button from '@smui/button';
  import CircularProgress from '@smui/circular-progress';
  import MdIcon from '../../../lib/components/MdIcon.svelte';

  // 刻意 any：与 SettingsSync 同款，Settings.svelte 的 SettingsConfig 是多面板
  // 交叉类型；卡片只触碰 work_sync_enabled / machine_name 两个键
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export let config: any;

  const dispatch = createEventDispatcher<{ change: void }>();
  $: currentLocale = $locale;

  // Switch 影子变量：bind 到有回退值的 $bindable prop 传 undefined 会抛
  // props_invalid_value（SMUI v8 踩坑），全部 ?? 兜底。
  // 与 SettingsGeneral 同款：input bind:value 直接绑定 config 字段（bind:config
  // 双向），change 只作脏标记。勿用「draft ≠ config 就重置 draft」的响应式回填
  // ——它会在用户键入的瞬间把输入抢回去（E2E-002 实测抓到）。
  let enabledSwitch = false;
  $: enabledSwitch = config.work_sync_enabled ?? false;

  function setEnabled(next: boolean): void {
    config.work_sync_enabled = next;
    dispatch('change');
  }

  function commitMachineName(): void {
    const value = config.machine_name;
    config.machine_name = typeof value === 'string' ? value.trim() : '';
    dispatch('change');
  }

  let working = false;
  let lastRun: WorkSyncRunSummary | null = null;
  let lastRunAt: number | null = null;
  let lastError = '';

  async function runNow(pullOnly: boolean): Promise<void> {
    if (working) return;
    const auth = getAuth();
    if (!auth?.token || !getAdminUrl()) {
      showToast(t('workSync.needLogin'), 'warning');
      return;
    }
    working = true;
    lastError = '';
    try {
      const summary = await syncWithStoredCredentials(
        config.work_sync_enabled ?? false,
        config.machine_name ?? '',
        pullOnly,
      );
      if (summary) {
        lastRun = summary;
        lastRunAt = Date.now();
        showToast(
          t('workSync.syncDone', {
            pushed: summary.pushedBlocks,
            imported: summary.importedActivities,
            reports: summary.pushedReports + summary.pulledReports,
          }),
          'success',
        );
      } else {
        showToast(t('workSync.syncSkipped'), 'info');
      }
    } catch (e) {
      lastError = String(e);
      showToast(t('workSync.syncFailed', { error: lastError.slice(0, 120) }), 'error');
    } finally {
      working = false;
    }
  }

  let machineId = '';
  onMount(async () => {
    try {
      const state = await invoke<{ machine_id: string }>('work_sync_get_state');
      machineId = state.machine_id;
    } catch {
      machineId = '';
    }
  });

  $: fmtTime = (ts: number): string =>
    get(locale) === 'zh-CN'
      ? new Date(ts).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
      : new Date(ts).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
</script>

<div class="settings-card settings-work-sync-card">
  <div class="settings-card-header">
    <MdIcon name="sync" size={20} />
    <h3>{t('workSync.cardTitle')}</h3>
    <span class="settings-card-subtitle">{t('workSync.cardSubtitle')}</span>
  </div>

  <div class="work-sync-row">
    <Switch
      bind:checked={enabledSwitch}
      onSMUISwitchChange={() => setEnabled(enabledSwitch)}
    >
      {t('workSync.enableLabel')}
    </Switch>
  </div>

  {#if enabledSwitch}
    <div class="work-sync-row">
      <label class="work-sync-label" for="work-sync-machine-name">{t('workSync.machineNameLabel')}</label>
      <input
        id="work-sync-machine-name"
        class="work-sync-input"
        type="text"
        maxlength="32"
        placeholder={t('workSync.machineNamePlaceholder')}
        bind:value={config.machine_name}
        onblur={commitMachineName}
      />
    </div>
    {#if machineId}
      <p class="work-sync-machine-id">{t('workSync.machineIdLabel', { id: machineId.slice(0, 8) })}</p>
    {/if}
    <p class="work-sync-hint">{t('workSync.hint')}</p>
  {/if}

  <div class="work-sync-actions">
    <Button
      variant="outlined"
      disabled={working || !(config.work_sync_enabled ?? false)}
      onclick={() => runNow(false)}
    >
      {#if working}
        <CircularProgress indeterminate />
      {:else}
        <MdIcon name="sync" size={16} />
      {/if}
      {t('workSync.syncNow')}
    </Button>
    {#if lastRunAt}
      <span class="work-sync-last-run">{t('workSync.lastRunAt', { time: fmtTime(lastRunAt) })}</span>
    {/if}
  </div>

  {#if lastError}
    <p class="work-sync-error">{lastError}</p>
  {/if}
</div>

<style>
  .work-sync-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 0;
  }

  .work-sync-label {
    font-size: 0.875rem;
    color: var(--md-theme-on-surface, #1d1b20);
    min-width: 72px;
  }

  .work-sync-input {
    flex: 1;
    max-width: 260px;
    padding: 8px 12px;
    border: 1px solid var(--md-theme-outline-variant, #cac4d0);
    border-radius: 8px;
    background: transparent;
    color: var(--md-theme-on-surface, #1d1b20);
    font-size: 0.875rem;
  }

  .work-sync-input:focus {
    outline: 2px solid var(--md-theme-primary, #1976d2);
    border-color: transparent;
  }

  .work-sync-machine-id,
  .work-sync-hint {
    font-size: 0.75rem;
    color: var(--md-theme-on-surface-variant, #49454f);
    margin: 4px 0 0;
    line-height: 1.5;
  }

  .work-sync-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 12px;
  }

  .work-sync-last-run {
    font-size: 0.75rem;
    color: var(--md-theme-on-surface-variant, #49454f);
  }

  .work-sync-error {
    font-size: 0.75rem;
    color: var(--md-theme-error, #b3261e);
    margin: 8px 0 0;
    word-break: break-all;
  }
</style>
