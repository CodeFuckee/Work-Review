<script lang="ts">
  import { onMount } from 'svelte';
  import { push } from 'svelte-spa-router';
  import { locale, t } from '$lib/i18n/index.ts';
  import {
    buildAuthStateFromToken,
    clientLogin,
    clientRegister,
    getAdminUrl,
    getAuth,
    isTokenExpired,
    setAdminUrl,
    setAuth,
    type ClientAuthCallResult,
  } from '$lib/stores/auth.ts';
  import {
    findLoginValidationError,
    findRegisterValidationError,
  } from './authLogic.ts';
  import { deriveSyncKey, storeSyncKey } from '$lib/utils/configSync.ts';
  import SegmentedButton, { Segment } from '@smui/segmented-button';
  import Textfield from '@smui/textfield';
  import Button from '@smui/button';
  import MdIcon from '../../lib/components/MdIcon.svelte';

  type Mode = 'login' | 'register';

  let mode: Mode = 'login';
  // SegmentedButton 单选影子：mode 为业务真值，外部同步进选中段；点击经 handleSegSelect 写回
  let modeSel: string = 'login';
  $: modeSel = mode;
  let userName = '';
  let password = '';
  let confirmPassword = '';
  let name = '';
  let department = '';
  let serverUrl = getAdminUrl();
  let submitting = false;
  let errorMessage = '';
  $: currentLocale = $locale;

  onMount(() => {
    const existing = getAuth();
    if (!existing) return;
    userName = existing.userName || userName;
    // 已登录（token 未过期）访问 /auth 直接回主页
    if (!isTokenExpired(existing)) {
      push('/');
    }
  });

  // 登录/注册 Tab：单选语义（点击已选段不切换、不清空表单，等同 Carbon ContentSwitcher）
  function handleSegSelect(e: Event): void {
    const detail = (e as CustomEvent<{ index: number; selected: boolean; segmentId?: string }>).detail;
    const next: Mode = detail.segmentId === 'register' ? 'register' : 'login';
    if (!detail.selected || next === mode) {
      modeSel = mode;
      return;
    }
    switchMode(next);
  }

  function switchMode(next: Mode): void {
    mode = next;
    errorMessage = '';
    password = '';
    confirmPassword = '';
  }

  function describeFailure(result: ClientAuthCallResult, fallbackKey: string): string {
    if (result.network) return t('auth.errorNetwork');
    return t(fallbackKey, { message: result.message || '' });
  }

  function finishAuth(result: ClientAuthCallResult, fallbackName: string): void {
    if (!result.token) return;
    setAuth(buildAuthStateFromToken({
      token: result.token,
      userId: result.userId,
      userName: userName.trim() || result.userName,
      name: fallbackName || result.name,
    }));
    // 通知 App.svelte 刷新认证门并退出离线宽限提示
    window.dispatchEvent(new CustomEvent('auth-changed'));
    push('/');
  }

  async function submit(): Promise<void> {
    if (submitting) return;
    errorMessage = '';
    const adminUrl = serverUrl.trim().replace(/\/+$/, '');
    setAdminUrl(adminUrl);

    if (mode === 'login') {
      const invalidKey = findLoginValidationError(userName, password);
      if (invalidKey) {
        errorMessage = t(invalidKey);
        return;
      }
      submitting = true;
      let result: ClientAuthCallResult;
      try {
        result = await clientLogin(adminUrl, userName.trim(), password);
      } finally {
        submitting = false;
      }
      if (!result.ok) {
        errorMessage = describeFailure(result, 'auth.loginFailed');
        return;
      }
      // 派生并缓存同步密钥（跨设备配置同步用；登出时清除）
      try {
        await storeSyncKey(await deriveSyncKey(password, userName.trim()));
      } catch (e) {
        console.warn('派生同步密钥失败:', e);
      }
      finishAuth(result, '');
      return;
    }

    const invalidKey = findRegisterValidationError({ userName, password, confirmPassword, name });
    if (invalidKey) {
      errorMessage = t(invalidKey);
      return;
    }
    submitting = true;
    let registerResult: ClientAuthCallResult;
    try {
      registerResult = await clientRegister(
        adminUrl,
        userName.trim(),
        password,
        name.trim(),
        department.trim(),
      );
    } finally {
      submitting = false;
    }
    if (!registerResult.ok) {
      errorMessage = describeFailure(registerResult, 'auth.registerFailed');
      return;
    }
    // 服务端注册成功即返回 token，直接自动登录
    try {
      await storeSyncKey(await deriveSyncKey(password, userName.trim()));
    } catch (e) {
      console.warn('派生同步密钥失败:', e);
    }
    finishAuth(registerResult, name.trim());
  }
</script>

<div class="auth-carbon-frame">
  <div class="auth-carbon-card">
    <img src="/icons/256x256.png" alt={t('common.appName')} class="auth-carbon-logo" />
    <h1 class="auth-carbon-title">{t('common.appName')}</h1>
    <p class="auth-carbon-subtitle">
      {mode === 'login' ? t('auth.loginSubtitle') : t('auth.registerSubtitle')}
    </p>

    <SegmentedButton
      class="auth-seg"
      singleSelect
      segments={['login', 'register']}
      bind:selected={modeSel}
      onselected={handleSegSelect}
    >
      {#snippet segment(seg)}
        <Segment segment={seg}>
          {seg === 'login' ? t('auth.loginTab') : t('auth.registerTab')}
        </Segment>
      {/snippet}
    </SegmentedButton>

    <form
      class="auth-carbon-form"
      onsubmit={(e) => { e.preventDefault(); void submit(); }}
    >
      <Textfield
        class="auth-textfield"
        variant="outlined"
        bind:value={userName}
        label={t('auth.accountPlaceholder')}
        input$autocomplete="username"
        input$spellcheck={false}
      />

      {#if mode === 'register'}
        <Textfield
          class="auth-textfield"
          variant="outlined"
          bind:value={name}
          label={t('auth.namePlaceholder')}
          input$autocomplete="name"
        />
      {/if}

      <Textfield
        class="auth-textfield"
        variant="outlined"
        type="password"
        bind:value={password}
        label={t('auth.passwordPlaceholder')}
        input$autocomplete={mode === 'login' ? 'current-password' : 'new-password'}
      />

      {#if mode === 'register'}
        <Textfield
          class="auth-textfield"
          variant="outlined"
          type="password"
          bind:value={confirmPassword}
          label={t('auth.confirmPasswordPlaceholder')}
          input$autocomplete="new-password"
        />
        <Textfield
          class="auth-textfield"
          variant="outlined"
          bind:value={department}
          label={t('auth.departmentPlaceholder')}
        />
      {/if}

      {#if errorMessage}
        <div class="md-banner" role="alert">
          <MdIcon name="error" size={20} />
          <span class="md-banner-text">{errorMessage}</span>
        </div>
      {/if}

      <Button variant="raised" class="auth-submit" type="submit" disabled={submitting}>
        {mode === 'login' ? t('auth.loginAction') : t('auth.registerAction')}
      </Button>
    </form>

    <!-- 服务器设置折叠区：SMUI 无 accordion 包，用原生 details/summary 承接 Carbon Accordion 语义 -->
    <details class="auth-server">
      <summary class="auth-server-summary">
        <MdIcon name="dns" size={18} />
        <span class="auth-server-title">{t('auth.serverToggle')}</span>
        <span class="auth-server-chevron"><MdIcon name="expand_more" size={16} /></span>
      </summary>
      <div class="auth-server-body">
        <Textfield
          class="auth-textfield"
          variant="outlined"
          bind:value={serverUrl}
          label={t('auth.serverPlaceholder')}
          input$spellcheck={false}
        />
      </div>
    </details>
  </div>
</div>

<style>
  .auth-carbon-frame {
    display: flex;
    min-height: 100vh;
    align-items: center;
    justify-content: center;
    padding: 2rem 1rem;
    background: var(--md-theme-background);
  }

  .auth-carbon-card {
    width: 100%;
    max-width: 24rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 2rem 1.75rem;
    background: var(--md-theme-surface);
    border: 1px solid var(--md-theme-border-subtle);
    border-radius: 8px;
  }

  .auth-carbon-logo {
    width: 3.5rem;
    height: 3.5rem;
    border-radius: 12px;
    object-fit: cover;
  }

  .auth-carbon-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--md-theme-on-surface);
  }

  .auth-carbon-subtitle {
    font-size: 0.75rem;
    color: var(--md-theme-on-surface-medium);
  }

  /* 登录/注册分段选择：撑满卡片宽度，段等分 */
  .auth-seg {
    width: 100%;
  }

  .auth-seg :global(.mdc-segmented-button__segment) {
    flex: 1;
  }

  .auth-carbon-form {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.875rem;
    margin-top: 0.5rem;
  }

  .auth-textfield {
    width: 100%;
  }

  /* 错误提示条（MD banner 形态）：替代 Carbon InlineNotification */
  .md-banner {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.625rem 0.875rem;
    border: 1px solid var(--md-theme-border-subtle);
    border-radius: var(--radius-md, 8px);
    background: var(--md-theme-surface-variant);
    color: var(--md-theme-on-surface);
    font-size: 0.875rem;
  }

  .md-banner :global(.md-icon) {
    color: var(--md-theme-error);
    flex-shrink: 0;
  }

  .md-banner-text {
    min-width: 0;
  }

  .auth-submit {
    width: 100%;
  }

  .auth-submit :global(.mdc-button__label) {
    width: 100%;
  }

  /* 服务器设置折叠区（原生 details） */
  .auth-server {
    width: 100%;
    margin-top: 0.5rem;
    border-top: 1px solid var(--md-theme-border-subtle);
  }

  .auth-server-summary {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 0.25rem;
    color: var(--md-theme-on-surface-medium);
    font-size: 0.8125rem;
    cursor: pointer;
    list-style: none;
  }

  .auth-server-summary::-webkit-details-marker {
    display: none;
  }

  .auth-server-summary:hover {
    color: var(--md-theme-on-surface);
  }

  .auth-server-title {
    flex: 1;
  }

  .auth-server-chevron {
    display: inline-flex;
    transition: transform 160ms ease;
  }

  .auth-server[open] .auth-server-chevron {
    transform: rotate(180deg);
  }

  .auth-server-body {
    padding: 0.25rem 0 0.75rem;
  }
</style>
