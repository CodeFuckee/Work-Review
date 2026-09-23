<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { showToast } from '$lib/stores/toast.ts';
  import { locale, t } from '$lib/i18n/index.ts';

  type UiVisualStyle = 'a' | 'b' | 'c';

  interface AppearanceConfig {
    ui_visual_style: UiVisualStyle;
    background_image: string | null;
    background_opacity?: number | null;
    background_blur?: number | null;
  }

  type AppearanceChangeDetail = AppearanceConfig | {
    autosaved: true;
    config: AppearanceConfig;
  };

  export let config: AppearanceConfig;

  const dispatch = createEventDispatcher<{ change: AppearanceChangeDetail }>();
  $: currentLocale = $locale;

  let uiVisualStyleSaving = false;
  const UI_VISUAL_STYLE_OPTIONS: readonly {
    id: UiVisualStyle;
    titleKey: string;
    descriptionKey: string;
    badgeKey: string;
  }[] = [
    {
      id: 'a',
      titleKey: 'settingsAppearance.uiStyleATitle',
      descriptionKey: 'settingsAppearance.uiStyleADesc',
      badgeKey: 'settingsAppearance.uiStyleABadge',
    },
    {
      id: 'b',
      titleKey: 'settingsAppearance.uiStyleBTitle',
      descriptionKey: 'settingsAppearance.uiStyleBDesc',
      badgeKey: 'settingsAppearance.uiStyleBBadge',
    },
    {
      id: 'c',
      titleKey: 'settingsAppearance.uiStyleCTitle',
      descriptionKey: 'settingsAppearance.uiStyleCDesc',
      badgeKey: 'settingsAppearance.uiStyleCBadge',
    },
  ];
  let blurLabels: string[] = [];
  // === 背景图片 ===
  let bgPreview: string | null = null;
  let bgUploading = false;
  let appearanceDestroyed = false;

  $: {
    currentLocale;
    blurLabels = [
      t('settingsAppearance.blurClear'),
      t('settingsAppearance.blurLight'),
      t('settingsAppearance.blurMedium'),
    ];
  }
  onMount(async () => {
    try {
      const b64 = await invoke<string | null>('get_background_image');
      if (b64) bgPreview = `data:image/jpeg;base64,${b64}`;
    } catch (e) { /* ignore */ }
  });

  onDestroy(() => {
    appearanceDestroyed = true;
  });

  async function selectUiVisualStyle(styleId: UiVisualStyle) {
    if (uiVisualStyleSaving || config.ui_visual_style === styleId) {
      return;
    }

    // 与 Settings.svelte 的归一化默认值保持一致（缺省风格为 'c'）
    const previousStyle = config.ui_visual_style || 'c';
    uiVisualStyleSaving = true;
    config.ui_visual_style = styleId;
    dispatch('change', { autosaved: true, config });
    window.dispatchEvent(new CustomEvent('ui-visual-style-changed', {
      detail: { style: styleId },
    }));

    try {
      await invoke<void>('save_config', { config });
    } catch (e) {
      config.ui_visual_style = previousStyle;
      dispatch('change', { autosaved: true, config });
      window.dispatchEvent(new CustomEvent('ui-visual-style-changed', {
        detail: { style: previousStyle },
      }));
      console.error('保存界面风格失败:', e);
      showToast(t('settingsAppearance.uiVisualStyleSaveFailed', { error: e }), 'error');
    } finally {
      uiVisualStyleSaving = false;
    }
  }

  function handleBgFileSelect(event: Event) {
    const input = event.currentTarget;
    if (!(input instanceof HTMLInputElement)) return;
    const file = input.files?.[0];
    if (!file) return;
    if (!file.type.startsWith('image/')) return;
    if (file.size > 10 * 1024 * 1024) {
      showToast(t('settingsAppearance.imageTooLarge'), 'warning');
      return;
    }

    bgUploading = true;
    const reader = new FileReader();
    reader.onload = async () => {
      if (appearanceDestroyed) return;

      try {
        const b64Data = typeof reader.result === 'string' ? reader.result.split(',')[1] : null;
        if (!b64Data) {
          throw new Error(t('settingsAppearance.imageReadFailed'));
        }
        await invoke<void>('save_background_image', { data: b64Data });
        if (appearanceDestroyed) return;
        config.background_image = 'background.jpg';
        await invoke<void>('save_config', { config });
        const freshB64 = await invoke<string | null>('get_background_image');
        if (appearanceDestroyed) return;
        const imageUrl = freshB64 ? `data:image/jpeg;base64,${freshB64}` : null;
        bgPreview = imageUrl;
        dispatchBgEvent(imageUrl);
      } catch (e) {
        if (appearanceDestroyed) return;
        console.error('上传背景图失败:', e);
        showToast(t('settingsAppearance.uploadFailed', { error: e }), 'error');
      } finally {
        if (!appearanceDestroyed) {
          bgUploading = false;
        }
      }
    };
    reader.readAsDataURL(file);
  }

  async function clearBg() {
    try {
      await invoke<void>('clear_background_image');
      bgPreview = null;
      config.background_image = null;
      dispatchBgEvent(null);
      await invoke<void>('save_config', { config });
    } catch (e) {
      console.error('清除背景图失败:', e);
      showToast(t('settingsAppearance.clearFailed', { error: e }), 'error');
    }
  }

  function updateBgOpacity(val: string | number) {
    config.background_opacity = parseFloat(String(val));
    dispatch('change', config);
    dispatchBgEvent(bgPreview);
    saveConfigQuietly();
  }

  function updateBgBlur(val: string | number) {
    config.background_blur = parseInt(String(val), 10);
    dispatch('change', config);
    dispatchBgEvent(bgPreview);
    saveConfigQuietly();
  }

  function dispatchBgEvent(image: string | null) {
    window.dispatchEvent(new CustomEvent('background-changed', {
      detail: {
        image,
        opacity: config.background_opacity ?? 0.25,
        blur: config.background_blur ?? 1,
      }
    }));
  }

  async function saveConfigQuietly() {
    try {
      await invoke<void>('save_config', { config });
    } catch (e) {
      console.error('自动保存配置失败:', e);
    }
  }
</script>

<div class="settings-card" data-locale={currentLocale}>
  <div class="flex items-start justify-between gap-4">
    <div>
      <h3 class="settings-card-title">
        {t('settingsAppearance.uiVisualStyle')}
        <span class="ml-1.5 inline-flex items-center rounded-full border border-amber-200 bg-amber-50 px-1 py-px text-[10px] font-semibold uppercase tracking-[0.06em] text-amber-700 align-middle dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-200">Beta</span>
      </h3>
      <p class="settings-muted mt-1">{t('settingsAppearance.uiVisualStyleDesc')}</p>
    </div>
    {#if uiVisualStyleSaving}
      <span class="text-xs text-slate-400 dark:text-[#636c76]">{t('settingsAppearance.syncing')}</span>
    {/if}
  </div>

  <div class="mt-4 grid gap-3 md:grid-cols-3">
    {#each UI_VISUAL_STYLE_OPTIONS as option}
      <button
        type="button"
        class="settings-style-option {config.ui_visual_style === option.id ? 'settings-style-option-active' : ''}"
        on:click={() => selectUiVisualStyle(option.id)}
        aria-pressed={config.ui_visual_style === option.id}
      >
        <div class="settings-style-preview settings-style-preview--{option.id}" aria-hidden="true">
          <div class="settings-style-preview__sidebar"></div>
          <div class="settings-style-preview__topbar"></div>
          <div class="settings-style-preview__metric"></div>
          <div class="settings-style-preview__chart">
            <span></span>
            <span></span>
            <span></span>
          </div>
        </div>
        <div class="mt-3 flex items-center justify-between gap-2">
          <div class="text-sm font-semibold text-slate-900 dark:text-[#e6edf3]">
            {t(option.titleKey)}
          </div>
          <div class="flex items-center gap-1.5">
            {#if config.ui_visual_style === option.id}
              <span class="settings-style-current-mark">
                <svg viewBox="0 0 16 16" fill="none" aria-hidden="true">
                  <path d="M3.5 8.2L6.5 11L12.5 5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
                </svg>
                {t('settingsAppearance.uiStyleCurrent')}
              </span>
            {/if}
            <span class="rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.08em] text-slate-500 dark:bg-[#30363d] dark:text-[#adbac7]">
              {t(option.badgeKey)}
            </span>
          </div>
        </div>
        <p class="mt-2 text-xs leading-5 text-slate-500 dark:text-[#7d8590]">
          {t(option.descriptionKey)}
        </p>
        <p class="mt-2 text-[11px] font-medium text-slate-400 dark:text-[#636c76]">
          {t('settingsAppearance.uiVisualStyleApplyHint')}
        </p>
      </button>
    {/each}
  </div>
</div>

<!-- 背景图片 -->
<div class="settings-card" data-locale={currentLocale}>
  <h3 class="settings-card-title">{t('settingsAppearance.backgroundImage')}</h3>

  <div class="settings-section">
    <!-- 预览 + 上传 -->
    <div class="flex items-start gap-4">
      {#if bgPreview}
        <div class="w-32 h-20 rounded-lg overflow-hidden border border-slate-200 dark:border-[#30363d] flex-shrink-0">
          <img src={bgPreview} alt={t('settingsAppearance.bgPreviewAlt')} class="w-full h-full object-cover" />
        </div>
      {:else}
        <div class="w-32 h-20 rounded-lg border-2 border-dashed border-slate-200 dark:border-[#30363d] flex items-center justify-center flex-shrink-0">
          <span class="settings-subtle">{t('settingsAppearance.noBackground')}</span>
        </div>
      {/if}

      <div class="flex-1 settings-field">
        <label class="settings-action-secondary cursor-pointer">
          {#if bgUploading}
            <div class="animate-spin rounded-full h-3 w-3 border-2 border-slate-500 border-t-transparent"></div>
            {t('common.processing')}
          {:else}
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" /></svg>
            {t('settingsAppearance.chooseImage')}
          {/if}
          <input type="file" accept="image/*" class="hidden" on:change={handleBgFileSelect} disabled={bgUploading} />
        </label>
        {#if bgPreview}
          <button
            on:click={clearBg}
            class="settings-link-danger"
          >
            {t('settingsAppearance.clearBackground')}
          </button>
        {/if}
        <p class="settings-muted">{t('settingsAppearance.bgSupport')}</p>
      </div>
    </div>

    {#if bgPreview || config.background_image}
      <hr class="border-slate-200 dark:border-[#30363d]" />

      <!-- 显示强度 -->
      <div class="settings-block">
        <div class="flex items-center justify-between">
          <span class="settings-text">{t('settingsAppearance.bgStrength')}</span>
          <span class="settings-value">{Math.round((config.background_opacity ?? 0.25) * 100)}%</span>
        </div>
        <input
          type="range"
          min="0.05"
          max="0.60"
          step="0.01"
          value={config.background_opacity ?? 0.25}
          on:input={(e) => updateBgOpacity(e.currentTarget.value)}
          class="range-input"
        />
        <div class="flex justify-between text-[10px] settings-subtle">
          <span>{t('settingsAppearance.bgLight')}</span>
          <span>{t('settingsAppearance.bgStrong')}</span>
        </div>
      </div>

      <!-- 模糊度 -->
      <div class="settings-block">
        <div class="flex items-center justify-between">
          <span class="settings-text">{t('settingsAppearance.bgBlur')}</span>
          <span class="settings-muted">{blurLabels[config.background_blur ?? 1]}</span>
        </div>
        <div class="flex gap-2">
          {#each [0, 1, 2] as level}
            <button
              on:click={() => updateBgBlur(level)}
              class="segment-btn
                {(config.background_blur ?? 1) === level
                  ? 'settings-segment-active'
                  : 'settings-segment-base'}"
            >
              {blurLabels[level]}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>
