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
  import ContentSwitcher from 'carbon-components-svelte/src/ContentSwitcher/ContentSwitcher.svelte';
  import Switch from 'carbon-components-svelte/src/ContentSwitcher/Switch.svelte';
  import TextInput from 'carbon-components-svelte/src/TextInput/TextInput.svelte';
  import Button from 'carbon-components-svelte/src/Button/Button.svelte';
  import InlineNotification from 'carbon-components-svelte/src/Notification/InlineNotification.svelte';
  import Accordion from 'carbon-components-svelte/src/Accordion/Accordion.svelte';
  import AccordionItem from 'carbon-components-svelte/src/Accordion/AccordionItem.svelte';

  type Mode = 'login' | 'register';

  let mode: Mode = 'login';
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

    <ContentSwitcher
      selectedIndex={mode === 'login' ? 0 : 1}
      on:change={(e) => switchMode(e.detail === 0 ? 'login' : 'register')}
    >
      <Switch text={t('auth.loginTab')} id="login" />
      <Switch text={t('auth.registerTab')} id="register" />
    </ContentSwitcher>

    <form class="auth-carbon-form" on:submit|preventDefault={submit}>
      <TextInput
        bind:value={userName}
        labelText={t('auth.accountPlaceholder')}
        placeholder={t('auth.accountPlaceholder')}
        autocomplete="username"
        spellcheck={false}
      />

      {#if mode === 'register'}
        <TextInput
          bind:value={name}
          labelText={t('auth.namePlaceholder')}
          placeholder={t('auth.namePlaceholder')}
          autocomplete="name"
        />
      {/if}

      <TextInput
        type="password"
        bind:value={password}
        labelText={t('auth.passwordPlaceholder')}
        placeholder={t('auth.passwordPlaceholder')}
        autocomplete={mode === 'login' ? 'current-password' : 'new-password'}
      />

      {#if mode === 'register'}
        <TextInput
          type="password"
          bind:value={confirmPassword}
          labelText={t('auth.confirmPasswordPlaceholder')}
          placeholder={t('auth.confirmPasswordPlaceholder')}
          autocomplete="new-password"
        />
        <TextInput
          bind:value={department}
          labelText={t('auth.departmentPlaceholder')}
          placeholder={t('auth.departmentPlaceholder')}
        />
      {/if}

      {#if errorMessage}
        <InlineNotification kind="error" title={errorMessage} hideCloseButton={true} timeout={0} />
      {/if}

      <Button type="submit" fullWidth={true} loading={submitting}>
        {mode === 'login' ? t('auth.loginAction') : t('auth.registerAction')}
      </Button>
    </form>

    <Accordion align="start" size="sm">
      <AccordionItem title={t('auth.serverToggle')}>
        <TextInput
          bind:value={serverUrl}
          labelText={t('auth.serverToggle')}
          placeholder={t('auth.serverPlaceholder')}
          spellcheck={false}
        />
      </AccordionItem>
    </Accordion>
  </div>
</div>

<style>
  .auth-carbon-frame {
    display: flex;
    min-height: 100vh;
    align-items: center;
    justify-content: center;
    padding: 2rem 1rem;
    background: var(--cds-background);
  }

  .auth-carbon-card {
    width: 100%;
    max-width: 24rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 2rem 1.75rem;
    background: var(--cds-layer);
    border: 1px solid var(--cds-border-subtle);
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
    color: var(--cds-text-primary);
  }

  .auth-carbon-subtitle {
    font-size: 0.75rem;
    color: var(--cds-text-secondary);
  }

  .auth-carbon-form {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.875rem;
    margin-top: 0.5rem;
  }

  .auth-carbon-card :global(.bx--accordion) {
    width: 100%;
    margin-top: 0.5rem;
  }
</style>
