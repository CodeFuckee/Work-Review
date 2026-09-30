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

  type Mode = 'login' | 'register';

  let mode: Mode = 'login';
  let userName = '';
  let password = '';
  let confirmPassword = '';
  let name = '';
  let department = '';
  let serverUrl = getAdminUrl();
  let showServerSettings = false;
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

<div class="flex h-full min-h-0 items-center justify-center overflow-auto bg-gradient-to-br from-slate-100 via-indigo-50 to-sky-100 px-4 py-8 dark:from-[#0d1117] dark:via-[#10161d] dark:to-[#0d1420]">
  <div class="w-full max-w-sm rounded-xl border border-slate-200 bg-white/90 p-7 shadow-xl backdrop-blur dark:border-[#2d3843] dark:bg-[#10161d]/90">
    <div class="flex flex-col items-center gap-2 text-center">
      <img src="/icons/256x256.png" alt={t('common.appName')} class="h-14 w-14 rounded-xl object-cover" />
      <h1 class="text-lg font-semibold text-slate-900 dark:text-[#e6edf3]">{t('common.appName')}</h1>
      <p class="text-xs text-slate-500 dark:text-slate-400">
        {mode === 'login' ? t('auth.loginSubtitle') : t('auth.registerSubtitle')}
      </p>
    </div>

    <div class="mt-5 grid grid-cols-2 gap-1 rounded-xl bg-slate-100 p-1 dark:bg-[#1a222c]" role="tablist" aria-label={t('auth.loginTab')}>
      <button
        type="button"
        role="tab"
        aria-selected={mode === 'login'}
        on:click={() => switchMode('login')}
        class="rounded-lg px-3 py-1.5 text-sm font-medium transition-colors {mode === 'login'
          ? 'bg-white text-slate-900 shadow-sm dark:bg-[#2d3843] dark:text-[#e6edf3]'
          : 'text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200'}"
      >
        {t('auth.loginTab')}
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={mode === 'register'}
        on:click={() => switchMode('register')}
        class="rounded-lg px-3 py-1.5 text-sm font-medium transition-colors {mode === 'register'
          ? 'bg-white text-slate-900 shadow-sm dark:bg-[#2d3843] dark:text-[#e6edf3]'
          : 'text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200'}"
      >
        {t('auth.registerTab')}
      </button>
    </div>

    <form class="mt-5 space-y-3" on:submit|preventDefault={submit}>
      <input
        bind:value={userName}
        placeholder={t('auth.accountPlaceholder')}
        autocomplete="username"
        spellcheck="false"
        class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
      />

      {#if mode === 'register'}
        <input
          bind:value={name}
          placeholder={t('auth.namePlaceholder')}
          autocomplete="name"
          class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
        />
      {/if}

      <input
        type="password"
        bind:value={password}
        placeholder={t('auth.passwordPlaceholder')}
        autocomplete={mode === 'login' ? 'current-password' : 'new-password'}
        class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
      />

      {#if mode === 'register'}
        <input
          type="password"
          bind:value={confirmPassword}
          placeholder={t('auth.confirmPasswordPlaceholder')}
          autocomplete="new-password"
          class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
        />
        <input
          bind:value={department}
          placeholder={t('auth.departmentPlaceholder')}
          class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
        />
      {/if}

      {#if errorMessage}
        <p class="rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-300" role="alert">
          {errorMessage}
        </p>
      {/if}

      <button
        type="submit"
        disabled={submitting}
        class="flex w-full items-center justify-center gap-2 rounded-lg bg-indigo-500 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-600 disabled:opacity-60"
      >
        {#if submitting}
          <svg class="h-4 w-4 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          {mode === 'login' ? t('auth.loggingIn') : t('auth.registering')}
        {:else}
          {mode === 'login' ? t('auth.loginAction') : t('auth.registerAction')}
        {/if}
      </button>
    </form>

    <div class="mt-4 border-t border-slate-200 pt-3 dark:border-[#2d3843]">
      <button
        type="button"
        on:click={() => (showServerSettings = !showServerSettings)}
        class="inline-flex items-center gap-1 text-xs text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200"
        aria-expanded={showServerSettings}
      >
        <svg class="h-3 w-3 transition-transform {showServerSettings ? 'rotate-90' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
        </svg>
        {t('auth.serverToggle')}
      </button>
      {#if showServerSettings}
        <input
          bind:value={serverUrl}
          placeholder={t('auth.serverPlaceholder')}
          spellcheck="false"
          class="mt-2 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-indigo-400 dark:border-[#2d3843] dark:bg-[#10161d] dark:text-[#e6edf3]"
        />
      {/if}
    </div>
  </div>
</div>
