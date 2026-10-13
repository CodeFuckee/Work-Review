<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { showToast } from '$lib/stores/toast.ts';
  import { locale, t } from '$lib/i18n/index.ts';
  import { cache } from '$lib/stores/cache.ts';
  import SegmentedButton, { Segment } from '@smui/segmented-button';
  import Slider from '@smui/slider';
  import Button from '@smui/button';
  import Radio from '@smui/radio';
  import FormField from '@smui/form-field';
  import MdIcon from '../../../lib/components/MdIcon.svelte';

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

  // Radio 组影子：config.theme 外部同步进单选组，切换经 commitTheme 走保存管线
  let themeSel = 'system';
  $: themeSel = (config.theme as string) || 'system';
  // Slider/SegmentedButton 影子：外部加载同步进控件，拖动/点击经回调写回 config
  let opacitySlider = 0.25;
  $: opacitySlider = config.background_opacity ?? 0.25;
  let blurSel = 1;
  $: blurSel = config.background_blur ?? 1;

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

  // 界面主题（跟随系统/浅色/深色）：写 config.theme 并经 cache 回流到 App 壳层的 MD 主题桥
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

  // Radio 组 change（原生冒泡）：bind:group 已更新 themeSel，此处提交
  function commitTheme(): void {
    void selectTheme(themeSel as ThemePreference);
  }

  // 模糊分段选择：单选语义，误点已选段（取消选中）时恢复当前值
  function handleBlurSelect(e: Event): void {
    const detail = (e as CustomEvent<{ index: number; selected: boolean; segmentId?: string }>).detail;
    if (!detail.selected) {
      blurSel = config.background_blur ?? 1;
      return;
    }
    updateBgBlur(Number(detail.segmentId));
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
    <div class="appearance-theme-group">
      <FormField>
        <Radio bind:group={themeSel} value="system" onchange={commitTheme} />
        {#snippet label()}<span class="appearance-theme-label">{t('sidebar.themeTitle.system')}</span>{/snippet}
      </FormField>
      <FormField>
        <Radio bind:group={themeSel} value="light" onchange={commitTheme} />
        {#snippet label()}<span class="appearance-theme-label">{t('sidebar.themeTitle.light')}</span>{/snippet}
      </FormField>
      <FormField>
        <Radio bind:group={themeSel} value="dark" onchange={commitTheme} />
        {#snippet label()}<span class="appearance-theme-label">{t('sidebar.themeTitle.dark')}</span>{/snippet}
      </FormField>
    </div>
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
          <Button variant="outlined" disabled={bgUploading}>
            <span class="mdc-button__icon"><MdIcon name="image" size={18} /></span>
            {t('settingsAppearance.chooseImage')}
          </Button>
          <input type="file" accept="image/*" class="appearance-file-input" onchange={handleBgFileSelect} disabled={bgUploading} />
        </label>
        {#if bgPreview}
          <Button variant="text" onclick={clearBg}>{t('settingsAppearance.clearBackground')}</Button>
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
          class="appearance-slider"
          min={0.05}
          max={0.6}
          step={0.01}
          bind:value={opacitySlider}
          oninput={() => updateBgOpacity(opacitySlider)}
        />
      </div>

      <div class="appearance-slider-block">
        <div class="appearance-slider-head">
          <span>{t('settingsAppearance.bgBlur')}</span>
          <span class="appearance-slider-value">{blurLabels[config.background_blur ?? 1]}</span>
        </div>
        <SegmentedButton
          class="appearance-seg"
          singleSelect
          segments={[0, 1, 2]}
          bind:selected={blurSel}
          onselected={handleBlurSelect}
        >
          {#snippet segment(level)}
            <Segment segment={level}>{blurLabels[level]}</Segment>
          {/snippet}
        </SegmentedButton>
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
    background: var(--md-theme-surface);
    border: 1px solid var(--md-theme-border-subtle);
    border-radius: 8px;
    padding: 1.25rem 1.375rem;
    display: flex;
    flex-direction: column;
    gap: 0.875rem;
  }

  .appearance-card-desc {
    font-size: 0.8125rem;
    color: var(--md-theme-on-surface-medium);
    margin-top: -0.5rem;
  }

  /* 主题单选组：纵排 + 表单域对齐（替代 Carbon RadioButtonGroup 的 label 布局） */
  .appearance-theme-group {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    margin-top: -0.5rem;
  }

  .appearance-theme-group :global(.mdc-form-field) {
    align-items: center;
  }

  .appearance-theme-label {
    font-size: 0.875rem;
    color: var(--md-theme-on-surface);
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
    border: 1px solid var(--md-theme-border-subtle);
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
    border: 1px dashed var(--md-theme-border-subtle);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    font-size: 0.75rem;
    color: var(--md-theme-on-surface-medium);
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
    color: var(--md-theme-on-surface-medium);
  }

  .appearance-slider-block {
    border-top: 1px solid var(--md-theme-border-subtle);
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
    color: var(--md-theme-on-surface-medium);
  }

  .appearance-slider {
    width: 100%;
  }

  /* 模糊档位分段选择：撑满滑块区宽度，段等分 */
  .appearance-seg {
    width: 100%;
  }

  .appearance-seg :global(.mdc-segmented-button__segment) {
    flex: 1;
  }
</style>
