<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getVersion } from '@tauri-apps/api/app';
  import { locale, t } from '$lib/i18n/index.ts';
  import { runUpdateFlow } from '$lib/utils/updater.ts';

  interface UpdateSettings {
    autoCheck: boolean;
    lastCheckTime: number;
    checkIntervalHours: number;
  }

  let appVersion = '';
  let isCheckingUpdate = false;
  let autoCheckUpdate = true;
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

  async function toggleAutoCheck() {
    autoCheckUpdate = !autoCheckUpdate;
    try {
      const settings = await invoke<UpdateSettings>('get_update_settings');
      settings.autoCheck = autoCheckUpdate;
      await invoke('save_update_settings', { settings });
    } catch (e) {
      console.error('保存更新设置失败:', e);
      autoCheckUpdate = !autoCheckUpdate;
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

<div class="page-shell about-editorial-shell" data-locale={currentLocale}>
  <div class="page-header page-axis-operation">
    <div class="page-title-group">
      <div class="page-title-badge">
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <circle cx="12" cy="12" r="8.5" stroke-width="1.8" />
          <path stroke-linecap="round" stroke-width="1.8" d="M12 10.5v5" />
          <circle cx="12" cy="7.5" r=".8" fill="currentColor" stroke="none" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>{t('about.pageTitle')}</h2>
        <p>{t('about.pageSubtitle')}</p>
      </div>
    </div>
  </div>

  <div class="w-full about-minimal-shell page-axis-reading">
    <section class="page-card about-brand-card">
      <div class="about-brand-identity flex flex-col items-center">
        <div class="about-brand-mark">
          <img src="/icons/256x256.png" alt={t('common.appName')} class="h-16 w-16 rounded-lg object-cover" />
        </div>

        <div class="about-brand-copy">
          <div class="about-brand-title-row flex items-center justify-center">
            <h1 class="about-brand-title">{t('common.appName')}</h1>
            <span class="about-version-badge">v{appVersion}</span>
          </div>
          <p class="about-brand-description">{t('about.description')}</p>
        </div>
      </div>

      <div class="about-action-row mt-6">
        <button type="button" on:click={openDataDir} class="page-action-secondary min-h-10 px-4 py-2">
          <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
          <span class="leading-none">{t('about.openDataDir')}</span>
        </button>
      </div>

      <div class="about-update-grid mt-7 grid grid-cols-1 pt-5 sm:grid-cols-3">
        <div class="about-update-unit flex min-h-20 flex-col items-center justify-center gap-2 px-4 py-3">
          <span class="text-xs font-medium text-slate-400 dark:text-[#7d8590]">{t('about.currentVersionTag')}</span>
          <strong class="text-sm font-semibold text-slate-700 dark:text-[#c9d1d9]">v{appVersion}</strong>
        </div>

        <div class="about-update-unit flex min-h-20 flex-col items-center justify-center gap-2 px-4 py-3">
          <span class="text-xs font-medium text-slate-400 dark:text-[#7d8590]">{t('about.autoCheckUpdate')}</span>
          <button
            type="button"
            role="switch"
            aria-checked={autoCheckUpdate}
            aria-label={t('about.autoCheckUpdate')}
            title={t('about.autoCheckUpdate')}
            on:click={toggleAutoCheck}
            class="relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors duration-200 {autoCheckUpdate ? 'bg-blue-500' : 'bg-slate-300 dark:bg-[#484f58]'}"
          >
            <span class="pointer-events-none inline-block h-4 w-4 rounded-full bg-white shadow-sm transition-transform duration-200 dark:shadow-none {autoCheckUpdate ? 'translate-x-[18px]' : 'translate-x-[2px]'}"></span>
          </button>
        </div>

        <div class="about-update-unit flex min-h-20 flex-col items-center justify-center gap-2 px-4 py-3">
          <span class="text-xs font-medium text-slate-400 dark:text-[#7d8590]">{t('about.updateStatus')}</span>
          <button
            type="button"
            on:click={checkForUpdates}
            disabled={isCheckingUpdate}
            class="page-control-btn min-h-9 px-3 disabled:opacity-50"
          >
            {#if isCheckingUpdate}
              <svg class="h-3.5 w-3.5 shrink-0 animate-spin" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
              <span class="leading-none">{t('about.checkingUpdates')}</span>
            {:else}
              <svg class="h-3.5 w-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>
              <span class="leading-none">{t('about.checkUpdates')}</span>
            {/if}
          </button>
        </div>
      </div>

      {#if updateStatus}
        <div class="about-update-feedback mt-4 text-center" role="status" aria-live="polite">
          <p class="text-sm text-slate-500 dark:text-[#7d8590]">{updateStatus}</p>
        </div>
      {/if}
    </section>

    <section class="page-card about-principles-card">
      <h2 class="about-principles-title page-section-title text-center">{t('about.productPrinciplesTitle')}</h2>
      <div class="about-trust-grid">
        <article class="about-trust-card">
          <span class="about-trust-kicker">01</span>
          <h3 class="about-trust-title">{t('about.localFirstTitle')}</h3>
          <p class="about-trust-copy">{t('about.localFirstCopy')}</p>
        </article>
        <article class="about-trust-card">
          <span class="about-trust-kicker">02</span>
          <h3 class="about-trust-title">{t('about.timelineTrustTitle')}</h3>
          <p class="about-trust-copy">{t('about.timelineTrustCopy')}</p>
        </article>
        <article class="about-trust-card">
          <span class="about-trust-kicker">03</span>
          <h3 class="about-trust-title">{t('about.reportTrustTitle')}</h3>
          <p class="about-trust-copy">{t('about.reportTrustCopy')}</p>
        </article>
      </div>
    </section>

    <section class="page-card about-tech-stack">
      <span class="about-tech-item">Tauri 2</span>
      <span class="about-tech-item">Svelte</span>
      <span class="about-tech-item">Rust</span>
      <span class="about-tech-item">SQLite</span>
    </section>
  </div>
</div>
