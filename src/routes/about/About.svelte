<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getVersion } from '@tauri-apps/api/app';
  import { locale, t } from '$lib/i18n/index.ts';
  import { runUpdateFlow } from '$lib/utils/updater.ts';
  import Button from '@smui/button';
  import '@smui/button/bare.css';
  import Switch from '@smui/switch';
  import '@smui/switch/bare.css';
  import CircularProgress from '@smui/circular-progress';
  import '@smui/circular-progress/bare.css';
  import MdIcon from '../../lib/components/MdIcon.svelte';

  interface UpdateSettings {
    autoCheck: boolean;
    lastCheckTime: number;
    checkIntervalHours: number;
  }

  let appVersion = '';
  let isCheckingUpdate = false;
  let autoCheckUpdate = true;
  // Switch 的受控影子：外部加载同步进开关，切换时经 setAutoCheck 走保存管线
  let autoCheckUpdateSwitch = false;
  $: autoCheckUpdateSwitch = autoCheckUpdate;
  let updateStatus = '';
  let updateStatusTimer: ReturnType<typeof setTimeout> | null = null;
  $: currentLocale = $locale;

  onMount(async () => {
    try {
      const version = await getVersion();
      appVersion = typeof version === 'string' && version.trim() ? version : '--';
    } catch (e) {
      console.error('获取版本失败:', e);
      appVersion = '--';
    }

    try {
      const settings = await invoke<UpdateSettings>('get_update_settings');
      autoCheckUpdate = settings?.autoCheck ?? true;
    } catch (e) {
      console.error('读取更新设置失败:', e);
    }
  });

  async function setAutoCheck(next: boolean) {
    const previous = autoCheckUpdate;
    autoCheckUpdate = next;
    try {
      const settings = await invoke<UpdateSettings>('get_update_settings');
      settings.autoCheck = autoCheckUpdate;
      await invoke('save_update_settings', { settings });
    } catch (e) {
      console.error('保存更新设置失败:', e);
      autoCheckUpdate = previous;
    }
  }

  async function openDataDir() {
    try {
      await invoke('open_data_dir');
    } catch (e) {
      console.error('打开目录失败:', e);
    }
  }

  async function checkForUpdates() {
    if (isCheckingUpdate) return;

    isCheckingUpdate = true;
    updateStatus = t('about.checkingUpdates');

    await runUpdateFlow({
      onStatusChange: (status) => {
        updateStatus = status;
      },
    });

    isCheckingUpdate = false;
    if (updateStatus) {
      if (updateStatusTimer !== null) clearTimeout(updateStatusTimer);
      updateStatusTimer = setTimeout(() => {
        updateStatus = '';
        updateStatusTimer = null;
      }, 3000);
    }
  }

  onDestroy(() => {
    if (updateStatusTimer !== null) clearTimeout(updateStatusTimer);
  });
</script>

<div class="about-carbon-shell" data-locale={currentLocale}>
  <div class="about-carbon-grid">
    <h2 class="cds--type-heading-04 about-page-title">{t('about.pageTitle')}</h2>
    <p class="cds--type-body-short-01 about-page-subtitle">{t('about.pageSubtitle')}</p>

    <section class="about-card">
      <div class="about-brand">
        <img src="/icons/256x256.png" alt={t('common.appName')} class="about-brand-logo" />
        <div class="about-brand-copy">
          <div class="about-brand-title-row">
            <h1 class="cds--type-heading-03">{t('common.appName')}</h1>
            <span class="md-chip md-chip-tonal">v{appVersion}</span>
          </div>
          <p class="about-brand-description">{t('about.description')}</p>
        </div>
      </div>

      <Button variant="outlined" onclick={openDataDir}>
        <span class="mdc-button__icon"><MdIcon name="folder" size={18} /></span>
        {t('about.openDataDir')}
      </Button>

      <div class="about-update-grid">
        <div class="about-update-unit">
          <span class="about-update-label">{t('about.currentVersionTag')}</span>
          <strong class="about-update-value">v{appVersion}</strong>
        </div>

        <div class="about-update-unit">
          <span class="about-update-label">{t('about.autoCheckUpdate')}</span>
          <Switch bind:checked={autoCheckUpdateSwitch} onchange={() => setAutoCheck(autoCheckUpdateSwitch)} />
        </div>

        <div class="about-update-unit">
          <span class="about-update-label">{t('about.updateStatus')}</span>
          <Button
            variant="text"
            disabled={isCheckingUpdate}
            onclick={checkForUpdates}
          >
            <span class="mdc-button__icon"><MdIcon name="refresh" size={18} /></span>
            {#if isCheckingUpdate}{t('about.checkingUpdates')}{:else}{t('about.checkUpdates')}{/if}
          </Button>
        </div>
      </div>

      {#if updateStatus}
        <div class="about-update-feedback" role="status" aria-live="polite">
          <span class="about-inline-loading"><CircularProgress indeterminate class="about-loading-spinner" />{updateStatus}</span>
        </div>
      {/if}
    </section>

    <section class="about-card">
      <h2 class="cds--type-heading-03 about-principles-title">{t('about.productPrinciplesTitle')}</h2>
      <div class="about-trust-grid">
        <article class="about-trust-card">
          <span class="about-trust-kicker">01</span>
          <h3 class="cds--type-heading-02">{t('about.localFirstTitle')}</h3>
          <p class="about-trust-copy">{t('about.localFirstCopy')}</p>
        </article>
        <article class="about-trust-card">
          <span class="about-trust-kicker">02</span>
          <h3 class="cds--type-heading-02">{t('about.timelineTrustTitle')}</h3>
          <p class="about-trust-copy">{t('about.timelineTrustCopy')}</p>
        </article>
        <article class="about-trust-card">
          <span class="about-trust-kicker">03</span>
          <h3 class="cds--type-heading-02">{t('about.reportTrustTitle')}</h3>
          <p class="about-trust-copy">{t('about.reportTrustCopy')}</p>
        </article>
      </div>
    </section>

    <section class="about-tech-stack">
      <span class="md-chip md-chip-outlined">Tauri 2</span>
      <span class="md-chip md-chip-outlined">Svelte 5</span>
      <span class="md-chip md-chip-outlined">Material Design</span>
      <span class="md-chip md-chip-outlined">Rust</span>
      <span class="md-chip md-chip-outlined">SQLite</span>
    </section>
  </div>
</div>

<style>
  .about-carbon-shell {
    min-height: 100%;
    background: var(--cds-background);
    color: var(--cds-text-primary);
    padding: 2rem clamp(1rem, 4vw, 3rem);
  }

  .about-carbon-grid {
    max-width: 56rem;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .about-page-subtitle {
    color: var(--cds-text-secondary);
    margin-top: -0.75rem;
  }

  .about-card {
    background: var(--cds-layer);
    border: 1px solid var(--cds-border-subtle);
    border-radius: 8px;
    padding: 1.75rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .about-brand {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    text-align: center;
  }

  .about-brand-logo {
    width: 4rem;
    height: 4rem;
    border-radius: 12px;
    object-fit: cover;
  }

  .about-brand-title-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  .about-brand-description {
    color: var(--cds-text-secondary);
    font-size: 0.875rem;
    margin-top: 0.375rem;
  }

  .about-update-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
    gap: 0.75rem;
    border-top: 1px solid var(--cds-border-subtle);
    padding-top: 1.25rem;
  }

  .about-update-unit {
    display: flex;
    min-height: 4.5rem;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  .about-update-label {
    font-size: 0.75rem;
    color: var(--cds-text-secondary);
  }

  .about-update-value {
    font-size: 0.875rem;
    font-weight: 600;
  }

  .about-update-feedback {
    display: flex;
    justify-content: center;
  }

  .about-principles-title {
    text-align: center;
  }

  .about-trust-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
    gap: 0.875rem;
  }

  .about-trust-card {
    background: var(--cds-layer-alternate);
    border: 1px solid var(--cds-border-subtle);
    border-radius: 8px;
    padding: 1.125rem;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .about-trust-kicker {
    font-size: 0.6875rem;
    letter-spacing: 0.08em;
    color: var(--cds-text-secondary);
  }

  .about-trust-copy {
    font-size: 0.8125rem;
    color: var(--cds-text-secondary);
    line-height: 1.6;
  }

  .about-tech-stack {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.5rem;
  }

  .about-loading-spinner {
    width: 18px;
    height: 18px;
    --md-circular-progress-size: 18px;
    --md-circular-progress-active-indicator-color: var(--md-theme-primary);
    vertical-align: -3px;
    margin-inline-end: 0.375rem;
  }
</style>
