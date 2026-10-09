<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { showToast } from '$lib/stores/toast.ts';
  import { locale, t } from '$lib/i18n/index.ts';
  import { cache } from '$lib/stores/cache.ts';
  import ContentSwitcher from 'carbon-components-svelte/src/ContentSwitcher/ContentSwitcher.svelte';
  import Switch from 'carbon-components-svelte/src/ContentSwitcher/Switch.svelte';
  import Slider from 'carbon-components-svelte/src/Slider/Slider.svelte';
  import Button from 'carbon-components-svelte/src/Button/Button.svelte';
  import RadioButtonGroup from 'carbon-components-svelte/src/RadioButtonGroup/RadioButtonGroup.svelte';
  import RadioButton from 'carbon-components-svelte/src/RadioButton/RadioButton.svelte';
  import ImageIcon from 'carbon-icons-svelte/lib/Image.svelte';

  type ThemePreference = 'system' | 'light' | 'dark';

  interface AppearanceConfig {
    theme?: ThemePreference;
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

  // 界面主题（跟随系统/浅色/深色）：写 config.theme 并经 cache 回流到 App 壳层的 Carbon 主题桥
  async function selectTheme(preference: ThemePreference) {
    const previous = config.theme;
    if (preference === previous) return;
    config.theme = preference;
    dispatch('change', { autosaved: true, config });
    try {
      await invoke<void>('save_config', { config });
      cache.setConfig(config as never);
    } catch (e) {
      config.theme = previous;
      dispatch('change', { autosaved: true, config });
      console.error('保存主题设置失败:', e);
      showToast(t('settingsAppearance.uiVisualStyleSaveFailed', { error: e }), 'error');
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

<div class="appearance-carbon" data-locale={currentLocale}>
  <!-- 界面主题 -->
  <section class="appearance-card">
    <h3 class="cds--type-heading-02">{t('settingsAppearance.uiVisualStyle')}</h3>
    <p class="appearance-card-desc">{t('settingsAppearance.uiVisualStyleDesc')}</p>
    <RadioButtonGroup
      selected={(config.theme as string) || 'system'}
      on:change={(e) => selectTheme(String(e.detail) as ThemePreference)}
    >
      <RadioButton value="system" labelText={t('sidebar.themeTitle.system')} />
      <RadioButton value="light" labelText={t('sidebar.themeTitle.light')} />
      <RadioButton value="dark" labelText={t('sidebar.themeTitle.dark')} />
    </RadioButtonGroup>
  </section>

  <!-- 背景图片 -->
  <section class="appearance-card">
    <h3 class="cds--type-heading-02">{t('settingsAppearance.backgroundImage')}</h3>

    <div class="appearance-bg-row">
      {#if bgPreview}
        <div class="appearance-bg-preview">
          <img src={bgPreview} alt={t('settingsAppearance.bgPreviewAlt')} />
        </div>
      {:else}
        <div class="appearance-bg-empty">
          <span>{t('settingsAppearance.noBackground')}</span>
        </div>
      {/if}

      <div class="appearance-bg-actions">
        <label class="appearance-upload-label">
          <Button kind="tertiary" size="small" icon={ImageIcon} loading={bgUploading}>
            {t('settingsAppearance.chooseImage')}
          </Button>
          <input type="file" accept="image/*" class="appearance-file-input" on:change={handleBgFileSelect} disabled={bgUploading} />
        </label>
        {#if bgPreview}
          <Button kind="ghost" size="small" on:click={clearBg}>{t('settingsAppearance.clearBackground')}</Button>
        {/if}
        <p class="appearance-hint">{t('settingsAppearance.bgSupport')}</p>
      </div>
    </div>

    {#if bgPreview || config.background_image}
      <div class="appearance-slider-block">
        <div class="appearance-slider-head">
          <span>{t('settingsAppearance.bgStrength')}</span>
          <span class="appearance-slider-value">{Math.round((config.background_opacity ?? 0.25) * 100)}%</span>
        </div>
        <Slider
          min={0.05}
          max={0.6}
          step={0.01}
          value={config.background_opacity ?? 0.25}
          labelText=""
          hideTextInput={true}
          on:input={(e) => updateBgOpacity(e.detail as number)}
        />
      </div>

      <div class="appearance-slider-block">
        <div class="appearance-slider-head">
          <span>{t('settingsAppearance.bgBlur')}</span>
          <span class="appearance-slider-value">{blurLabels[config.background_blur ?? 1]}</span>
        </div>
        <ContentSwitcher
          selectedIndex={(config.background_blur ?? 1) as number}
          on:change={(e) => updateBgBlur(e.detail as number)}
        >
          {#each [0, 1, 2] as level, i (level)}
            <Switch text={blurLabels[i]} id={'blur-' + level} />
          {/each}
        </ContentSwitcher>
      </div>
    {/if}
  </section>
</div>

<style>
  .appearance-carbon {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .appearance-card {
    background: var(--cds-layer);
    border: 1px solid var(--cds-border-subtle);
    border-radius: 8px;
    padding: 1.25rem 1.375rem;
    display: flex;
    flex-direction: column;
    gap: 0.875rem;
  }

  .appearance-card-desc {
    font-size: 0.8125rem;
    color: var(--cds-text-secondary);
    margin-top: -0.5rem;
  }

  .appearance-bg-row {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
  }

  .appearance-bg-preview {
    width: 8rem;
    height: 5rem;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid var(--cds-border-subtle);
    flex-shrink: 0;
  }

  .appearance-bg-preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .appearance-bg-empty {
    width: 8rem;
    height: 5rem;
    border: 1px dashed var(--cds-border-subtle);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    font-size: 0.75rem;
    color: var(--cds-text-secondary);
  }

  .appearance-bg-actions {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.625rem;
  }

  .appearance-upload-label {
    display: inline-flex;
    cursor: pointer;
  }

  .appearance-file-input {
    display: none;
  }

  .appearance-hint {
    width: 100%;
    font-size: 0.6875rem;
    color: var(--cds-text-secondary);
  }

  .appearance-slider-block {
    border-top: 1px solid var(--cds-border-subtle);
    padding-top: 0.875rem;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .appearance-slider-head {
    display: flex;
    justify-content: space-between;
    font-size: 0.8125rem;
  }

  .appearance-slider-value {
    color: var(--cds-text-secondary);
  }
</style>
