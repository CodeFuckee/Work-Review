<script lang="ts">
  import { onMount } from 'svelte';
  import Router, { location, push } from 'svelte-spa-router';
  import { wrap } from 'svelte-spa-router/wrap';
  import Toast from './lib/components/Toast.svelte';
  import ConfirmDialog from './lib/components/ConfirmDialog.svelte';
  import CarbonTheme from 'carbon-components-svelte/src/Theme/Theme.svelte';
  import Header from 'carbon-components-svelte/src/UIShell/Header.svelte';
  import SideNav from 'carbon-components-svelte/src/UIShell/SideNav.svelte';
  import SideNavItems from 'carbon-components-svelte/src/UIShell/SideNavItems.svelte';
  import SideNavLink from 'carbon-components-svelte/src/UIShell/SideNavLink.svelte';
  import HeaderGlobalAction from 'carbon-components-svelte/src/UIShell/HeaderGlobalAction.svelte';
  import Content from 'carbon-components-svelte/src/UIShell/Content.svelte';
  import InlineNotification from 'carbon-components-svelte/src/Notification/InlineNotification.svelte';
  import NotificationActionButton from 'carbon-components-svelte/src/Notification/NotificationActionButton.svelte';
  import OverflowMenu from 'carbon-components-svelte/src/OverflowMenu/OverflowMenu.svelte';
  import OverflowMenuItem from 'carbon-components-svelte/src/OverflowMenu/OverflowMenuItem.svelte';
  import HomeIcon from 'carbon-icons-svelte/lib/Home.svelte';
  import TimeIcon from 'carbon-icons-svelte/lib/Time.svelte';
  import ReportIcon from 'carbon-icons-svelte/lib/Report.svelte';
  import ChatIcon from 'carbon-icons-svelte/lib/Chat.svelte';
  import SettingsIcon from 'carbon-icons-svelte/lib/Settings.svelte';
  import InformationIcon from 'carbon-icons-svelte/lib/Information.svelte';
  import RecordingFilledIcon from 'carbon-icons-svelte/lib/RecordingFilled.svelte';
  import RecordingIcon from 'carbon-icons-svelte/lib/Recording.svelte';
  import SubtractIcon from 'carbon-icons-svelte/lib/Subtract.svelte';
  import CloseIcon from 'carbon-icons-svelte/lib/Close.svelte';
  import PauseIcon from 'carbon-icons-svelte/lib/Pause.svelte';
  import PlayIcon from 'carbon-icons-svelte/lib/Play.svelte';
  import TranslateIcon from 'carbon-icons-svelte/lib/Translate.svelte';
  import LightIcon from 'carbon-icons-svelte/lib/Light.svelte';
  import AsleepIcon from 'carbon-icons-svelte/lib/Asleep.svelte';
  import LaptopIcon from 'carbon-icons-svelte/lib/Laptop.svelte';
  import CheckmarkIcon from 'carbon-icons-svelte/lib/Checkmark.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { cache, getLocalDate } from './lib/stores/cache.ts';
  import { recordingStore } from './lib/stores/recording.ts';
  import { resolveAuthGateFromStorage } from './lib/stores/auth.ts';
  import { applyLocaleToDocument, initializeLocale, locale, setLocale, t, type Locale } from '$lib/i18n/index.ts';
  import { preloadAppIcons, type AppIconInvoke } from './lib/stores/iconCache.ts';
  import { runUpdateFlow } from './lib/utils/updater.ts';
  import { isTimelineActivity } from './routes/timeline/timelineData.ts';
  import { timelineGateway } from './routes/timeline/timelineGateway.ts';

  type ThemePreference = 'system' | 'light' | 'dark';

  interface WorkTimeSegment {
    end_hour?: number;
    end_minute?: number;
  }

  interface RuntimeConfig {
    lightweight_mode?: boolean;
    theme?: ThemePreference;
    ui_visual_style?: string;
    background_image?: string | null;
    background_opacity?: number;
    background_blur?: number;
    daily_report_auto_generate_time?: string | null;
    work_end_hour?: number;
    work_end_minute?: number;
    work_time_segments?: WorkTimeSegment[];
    memory_enabled?: boolean;
    memory_last_synthesis_date?: string | null;
  }

  interface DailyStatsPreview {
    browser_usage?: Array<{
      browser_name: string;
      executable_path?: string | null;
    }>;
    app_usage?: Array<{
      app_name: string;
      executable_path?: string | null;
    }>;
  }

  interface RecordingStateChangedPayload {
    isRecording: boolean;
    isPaused: boolean;
  }

  interface BackgroundChangeDetail {
    image?: string | null;
    opacity?: number;
    blur?: number;
  }

  interface AutoReportWorkEnd {
    hour: number;
    minute: number;
  }

  interface AutoReportCandidate extends AutoReportWorkEnd {
    score: number;
  }

  interface BrowserPreviewWindow {
    label: string;
    startDragging: () => Promise<void>;
    close: () => Promise<void>;
    hide: () => Promise<void>;
    minimize: () => Promise<void>;
    isMaximized: () => Promise<boolean>;
    unmaximize: () => Promise<void>;
    maximize: () => Promise<void>;
    isVisible: () => Promise<boolean>;
    onResized: (handler: () => void) => Promise<() => void>;
  }

  function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
  }

  function isTheme(value: unknown): value is ThemePreference {
    return value === 'system' || value === 'light' || value === 'dark';
  }

  function isRuntimeConfig(value: unknown): value is RuntimeConfig {
    if (!isRecord(value)) return false;

    const optionalStringFields = [
      'ui_visual_style',
      'background_image',
      'daily_report_auto_generate_time',
      'memory_last_synthesis_date',
    ];
    const optionalNumberFields = [
      'background_opacity',
      'background_blur',
      'work_end_hour',
      'work_end_minute',
    ];
    const optionalBooleanFields = ['lightweight_mode', 'memory_enabled'];

    if (optionalStringFields.some((field) => (
      value[field] !== undefined
      && value[field] !== null
      && typeof value[field] !== 'string'
    ))) return false;
    if (optionalNumberFields.some((field) => (
      value[field] !== undefined && typeof value[field] !== 'number'
    ))) return false;
    if (optionalBooleanFields.some((field) => (
      value[field] !== undefined && typeof value[field] !== 'boolean'
    ))) return false;
    if (value.theme !== undefined && !isTheme(value.theme)) return false;
    if (
      value.work_time_segments !== undefined
      && (
        !Array.isArray(value.work_time_segments)
        || value.work_time_segments.some((segment) => (
          !isRecord(segment)
          || (segment.end_hour !== undefined && typeof segment.end_hour !== 'number')
          || (segment.end_minute !== undefined && typeof segment.end_minute !== 'number')
        ))
      )
    ) return false;

    return true;
  }

  function isBoolean(value: unknown): value is boolean {
    return typeof value === 'boolean';
  }

  function isRecordingStateChangedPayload(
    value: unknown,
  ): value is RecordingStateChangedPayload {
    return isRecord(value)
      && typeof value.isRecording === 'boolean'
      && typeof value.isPaused === 'boolean';
  }

  function isRecordingStateTuple(value: unknown): value is [boolean, boolean] {
    return Array.isArray(value)
      && value.length === 2
      && value.every((item) => typeof item === 'boolean');
  }

  async function getRuntimeConfig(): Promise<RuntimeConfig> {
    const payload = await invoke<unknown>('get_config');
    if (!isRuntimeConfig(payload)) {
      throw new TypeError('配置载荷格式无效');
    }
    return payload;
  }

  // Development debug log: not output in production build to avoid polluting user console
  const devLog = (...args: unknown[]): void => {
    if (import.meta.env.DEV) console.log(...args);
  };

  function createBrowserPreviewWindow(): BrowserPreviewWindow {
    return {
      label: 'main',
      startDragging: async () => {},
      close: async () => {},
      hide: async () => {},
      minimize: async () => {},
      isMaximized: async () => false,
      unmaximize: async () => {},
      maximize: async () => {},
      isVisible: async () => true,
      onResized: async () => () => {},
    };
  }

  function getSafeCurrentWebviewWindow() {
    try {
      return getCurrentWebviewWindow();
    } catch (e) {
      console.warn('Tauri window metadata missing in current environment, switched to browser preview mode:', e);
      return createBrowserPreviewWindow();
    }
  }

  async function safeListen<T>(
    eventName: string,
    guard: (payload: unknown) => payload is T,
    handler: (payload: T) => void | Promise<void>,
  ): Promise<() => void> {
    try {
      return await listen<unknown>(eventName, (event) => {
        if (!guard(event.payload)) {
          console.warn(`Tauri 事件 ${eventName} 的载荷格式无效，已忽略`);
          return;
        }
        try {
          const result = handler(event.payload);
          void Promise.resolve(result).catch((error) => {
            console.error(`处理 Tauri 事件 ${eventName} 失败:`, error);
          });
        } catch (error) {
          console.error(`处理 Tauri 事件 ${eventName} 失败:`, error);
        }
      });
    } catch (e) {
      console.warn(`Cannot register Tauri event ${eventName} in current environment, skipped:`, e);
      return () => {};
    }
  }

  const appWindow = getSafeCurrentWebviewWindow();

  // 視窗拖拽（Linux WebKitGTK 不支援 -webkit-app-region: drag，改用 Tauri API）
  let lastDragClick = 0;
  let windowMaximized = false;
  async function startDrag(e: MouseEvent): Promise<void> {
    const target = e.target;
    if (e.button !== 0 || (target instanceof Element && target.closest('button'))) return;
    const now = Date.now();
    if (now - lastDragClick < 350) {
      lastDragClick = 0;
      await maximizeWindow();
      return;
    }
    lastDragClick = now;
    await appWindow.startDragging();
  }

  // 窗口控制函数
  async function closeWindow(): Promise<void> {
    if (runtimeConfig?.lightweight_mode) {
      await appWindow.close();
      return;
    }

    await appWindow.hide();
    syncMainWindowVisibility(false);
  }

  async function minimizeWindow(): Promise<void> {
    await appWindow.minimize();
  }

  async function refreshMaximizedState(): Promise<void> {
    try {
      windowMaximized = await appWindow.isMaximized();
    } catch (e) {
      console.warn('读取窗口最大化状态失败:', e);
    }
  }

  async function maximizeWindow(): Promise<void> {
    const isMaximized = await appWindow.isMaximized();
    if (isMaximized) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
    await refreshMaximizedState();
  }

  // 预加载核心数据
  const invokeAppIcon: AppIconInvoke = (command, args) => invoke<string>(command, {
    appName: args.appName,
    executablePath: args.executablePath,
  });

  async function preloadApp(): Promise<void> {
    devLog('开始预加载数据...');
    const today = getLocalDate();
    
    // 并行预加载：概览、时间线(今天)、日报(今天)
    Promise.all([
      // 1. 概览
      invoke<DailyStatsPreview>('get_today_stats').then(stats => {
        cache.setOverview(stats);

        preloadAppIcons(
          (stats?.browser_usage || []).map((browser) => ({
            appName: browser.browser_name,
            executablePath: browser.executable_path,
          })),
          invokeAppIcon,
          { priority: true }
        );

        preloadAppIcons(
          (stats?.app_usage || []).slice(0, 6).map((app) => ({
            appName: app.app_name,
            executablePath: app.executable_path,
          })),
          invokeAppIcon
        );
      }),
      // 2. 时间线 (今天) - 仅预加载前 20 条
      Promise.all([
        timelineGateway.getPage({ date: today, limit: 20, offset: 0 }),
        timelineGateway.getHourlySummaries(today),
      ]).then(([activities, summaries]) => cache.setTimeline(
        today,
        activities,
        summaries,
      )),
      // 3. 日报 (今天) - 检查是否已存在（必须带上当前语言，否则会把中文日报缓存到其他语言的 key 下）
      invoke<unknown>('get_saved_report', { date: today, locale: $locale }).then(report => {
        if (report) cache.setReport(`${today}:${$locale}`, report);
      })
    ]).then(() => {
      devLog('预加载完成');
    }).catch(e => {
      console.warn('预加载部分失败:', e);
    });
  }

  const routes = {
    '/': wrap({ asyncComponent: () => import('./routes/Overview.svelte') }),
    '/auth': wrap({ asyncComponent: () => import('./routes/auth/Auth.svelte') }),
    '/timeline': wrap({ asyncComponent: () => import('./routes/timeline/Timeline.svelte') }),
    '/timeline/summary/:date': wrap({ asyncComponent: () => import('./routes/timeline/Summary.svelte') }),
    '/timeline/summary': wrap({ asyncComponent: () => import('./routes/timeline/Summary.svelte') }),
    '/report': wrap({ asyncComponent: () => import('./routes/report/Report.svelte') }),
    '/ask': wrap({ asyncComponent: () => import('./routes/ask/Ask.svelte') }),
    '/settings': wrap({ asyncComponent: () => import('./routes/settings/Settings.svelte') }),
    '/about': wrap({ asyncComponent: () => import('./routes/about/About.svelte') }),
  };

  let theme: ThemePreference = 'system';
  let isDark = false;
  // Carbon 当前主题（g10=浅 / g90=深），由 applyTheme 桥接
  let carbonTheme: 'g10' | 'g90' = 'g10';
  let isRecording = true;
  let isPaused = false;
  let platform = '';
  let backgroundImage: string | null = null;
  let backgroundOpacity = 0.25;
  let backgroundBlur = 1;
  let runtimeConfig: RuntimeConfig | null = null;
  let unsubscribeLocale: () => void = () => {};
  $: currentLocale = $locale;

  // 认证门：login=未登录（强制跳登录页）/ grace=token 过期（离线宽限）/ allow=放行
  type AuthGateState = 'allow' | 'grace' | 'login';
  let authGate: AuthGateState = 'allow';

  // ── Carbon UIShell 壳层：导航 / 录制 / 语言 / 主题 ──
  let isSideNavOpen = true;
  interface ShellNavItem {
    path: string;
    labelKey: string;
    icon: typeof HomeIcon;
  }
  const shellNavItems: ShellNavItem[] = [
    { path: '/', labelKey: 'sidebar.nav.overview', icon: HomeIcon },
    { path: '/timeline', labelKey: 'sidebar.nav.timeline', icon: TimeIcon },
    { path: '/report', labelKey: 'sidebar.nav.report', icon: ReportIcon },
    { path: '/ask', labelKey: 'sidebar.nav.ask', icon: ChatIcon },
    { path: '/settings', labelKey: 'sidebar.nav.settings', icon: SettingsIcon },
    { path: '/about', labelKey: 'sidebar.nav.about', icon: InformationIcon },
  ];
  $: shellActiveStates = shellNavItems.reduce<Record<string, boolean>>((acc, item) => {
    const loc = $location || '/';
    acc[item.path] = item.path === '/'
      ? loc === '/'
      : loc === item.path || loc.startsWith(item.path + '/');
    return acc;
  }, {});

  // 语言菜单数据（label 为语言缩写，labelKey 为全名 i18n 键）
  const localeChoices: Array<{ value: Locale; label: string; labelKey: string }> = [
    { value: 'zh-CN', label: 'ZH', labelKey: 'sidebar.localeNames.zhCN' },
    { value: 'en', label: 'EN', labelKey: 'sidebar.localeNames.en' },
    { value: 'zh-TW', label: 'TW', labelKey: 'sidebar.localeNames.zhTW' },
    { value: 'ar', label: 'AR', labelKey: 'sidebar.localeNames.ar' },
  ];
  // 壳层模板的 i18n 取值：显式依赖 $locale 以便语言切换时更新（壳层不在 {#key} 重建范围内）
  $: tLocale = (key: string, params?: Record<string, unknown>): string => {
    void $locale;
    return t(key, params);
  };

  async function toggleRecording(): Promise<void> {
    try {
      await invoke(isPaused ? 'resume_recording' : 'pause_recording');
    } catch (e) {
      console.error('切换录制状态失败:', e);
    }
  }

  async function selectLocale(next: Locale): Promise<void> {
    const normalized = setLocale(next);
    invoke('set_app_locale', { locale: normalized }).catch((error) => {
      console.warn('同步后端语言失败:', error);
    });
  }
  $: isAuthRoute = $location === '/auth';
  $: if (authGate === 'login' && $location && $location !== '/auth') {
    void push('/auth');
  }

  function refreshAuthGate(): void {
    authGate = resolveAuthGateFromStorage() as AuthGateState;
  }

  function detectSystemTheme(): boolean {
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  }

  function applyTheme(newTheme: ThemePreference): void {
    theme = newTheme;
    isDark = theme === 'system' ? detectSystemTheme() : theme === 'dark';

    // Carbon 主题桥：浅色 g10 / 深色 g90（Theme 组件在 <html> 上写 theme 属性 + color-scheme）。
    // html.dark 保留至全站迁移收尾（过渡期旧语义类仍依赖 .dark 变量层），收尾时移除。
    carbonTheme = isDark ? 'g90' : 'g10';
    if (isDark) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }

  async function persistTheme(preference: ThemePreference): Promise<void> {
    applyTheme(preference);
    try {
      const config = await getRuntimeConfig();
      config.theme = preference;
      await invoke('save_config', { config });
      cache.setConfig(config);
    } catch (e) {
      console.error('保存主题配置失败:', e);
    }
  }

  // 壳层主题按钮：system → light → dark 循环
  function cycleTheme(): void {
    const order: ThemePreference[] = ['system', 'light', 'dark'];
    const next = order[(order.indexOf(theme) + 1) % order.length] ?? 'system';
    void persistTheme(next);
  }

  async function loadBackground() {
    try {
      const config = await getRuntimeConfig();
      backgroundOpacity = config.background_opacity ?? 0.25;
      backgroundBlur = config.background_blur ?? 1;
      if (config.background_image) {
        const b64 = await invoke<string | null>('get_background_image');
        if (b64) {
          backgroundImage = `data:image/jpeg;base64,${b64}`;
        }
      } else {
        backgroundImage = null;
      }
    } catch (e) {
      console.warn('加载背景图失败:', e);
    }
  }

  // 实时响应设置页的背景参数变更（不需要保存即可生效）
  function handleBackgroundChanged(e: Event): void {
    if (!(e instanceof CustomEvent)) return;
    const d: BackgroundChangeDetail = e.detail;
    if (d) {
      if (d.image !== undefined) backgroundImage = d.image;
      if (d.opacity !== undefined) backgroundOpacity = d.opacity;
      if (d.blur !== undefined) backgroundBlur = d.blur;
    }
  }

  function syncMainWindowVisibility(visible: boolean): void {
    document.body.classList.toggle('app-window-hidden', visible === false);
  }

  // 阻止文件拖拽到窗口时 WebView 导航到文件 URL
  function preventFileDrop(e: DragEvent): void {
    e.preventDefault();
  }

  // #126: 屏蔽 WebView 原生右键菜单，避免暴露浏览器默认上下文菜单
  function preventNativeContextMenu(e: MouseEvent): void {
    e.preventDefault();
  }

  function normalizeTimePart(value: unknown, upperBound: number): number {
    const parsed = Number.parseInt(String(value), 10);
    if (!Number.isFinite(parsed)) return 0;
    return Math.min(Math.max(parsed, 0), upperBound);
  }

  function resolveAutoReportWorkEnd(
    config: RuntimeConfig | null | undefined,
  ): AutoReportWorkEnd {
    // 优先使用用户自定义的日报生成时间
    const customTime = config?.daily_report_auto_generate_time;
    if (customTime && typeof customTime === 'string') {
      const parts = customTime.split(':').map(Number);
      if (parts.length >= 2 && Number.isFinite(parts[0]) && Number.isFinite(parts[1])) {
        return { hour: Math.min(Math.max(parts[0], 0), 23), minute: Math.min(Math.max(parts[1], 0), 59) };
      }
    }

    const fallbackHour = normalizeTimePart(config?.work_end_hour ?? 18, 23);
    const fallbackMinute = normalizeTimePart(config?.work_end_minute ?? 0, 59);
    const segments = Array.isArray(config?.work_time_segments) ? config.work_time_segments : [];
    if (segments.length === 0) {
      return { hour: fallbackHour, minute: fallbackMinute };
    }

    const latest = segments.reduce<AutoReportCandidate | null>((best, segment) => {
      const hour = normalizeTimePart(segment?.end_hour, 23);
      const minute = normalizeTimePart(segment?.end_minute, 59);
      const score = hour * 60 + minute;
      if (!best || score > best.score) {
        return { score, hour, minute };
      }
      return best;
    }, null);

    if (!latest) {
      return { hour: fallbackHour, minute: fallbackMinute };
    }
    return { hour: latest.hour, minute: latest.minute };
  }

  onMount(() => {
    // 全局阻止文件拖放导致页面导航（如拖入 PDF 会替换整个应用）
    window.addEventListener('dragover', preventFileDrop);
    window.addEventListener('drop', preventFileDrop);
    window.addEventListener('contextmenu', preventNativeContextMenu);

    initializeLocale();
    refreshAuthGate();
    const handleAuthChanged = () => refreshAuthGate();
    window.addEventListener('auth-changed', handleAuthChanged);
    unsubscribeLocale = locale.subscribe((nextLocale) => {
      applyLocaleToDocument(nextLocale);
    });

    let disposed = false;
    const pendingCleanup: Array<() => void> = [];

    // #118: 主窗口隐藏（静默驻留/轻量）时暂停 CSS 动画，降低后台 WebView2 GPU 占用
    safeListen('main-window-visibility', isBoolean, (payload) => {
      syncMainWindowVisibility(payload);
    }).then((unlisten) => {
      if (disposed) {
        try { if (unlisten) unlisten(); } catch {}
      } else {
        pendingCleanup.push(unlisten);
      }
    });

    // 同步注册的 locale subscription 立即可清理
    pendingCleanup.push(() => unsubscribeLocale());
    pendingCleanup.push(() => window.removeEventListener('auth-changed', handleAuthChanged));
    pendingCleanup.push(() => window.removeEventListener('dragover', preventFileDrop));
    pendingCleanup.push(() => window.removeEventListener('drop', preventFileDrop));
    pendingCleanup.push(() => window.removeEventListener('contextmenu', preventNativeContextMenu));

    (async () => {
      try {
        const visible = await appWindow.isVisible();
        if (!disposed) syncMainWindowVisibility(visible);
      } catch (e) {
        console.warn('同步主窗口可见性失败:', e);
      }
      if (disposed) return;

      // 获取平台信息
      try {
        platform = await invoke<string>('get_platform');
        devLog('当前平台:', platform);
      } catch (e) {
        console.error('获取平台信息失败:', e);
      }
      try {
        await refreshMaximizedState();
        const unlistenResized = await appWindow.onResized(() => {
          void refreshMaximizedState();
        });
        if (disposed) {
          try { unlistenResized(); } catch {}
        } else {
          pendingCleanup.push(unlistenResized);
        }
      } catch (e) {
        console.warn('监听窗口尺寸失败:', e);
      }
      if (disposed) return;

      // 加载配置并应用主题
      let config: RuntimeConfig;
      try {
        config = await getRuntimeConfig();
        runtimeConfig = config;
        cache.setConfig(config);
        applyTheme(config.theme || 'system');
      } catch (e) {
        console.error('加载配置失败:', e);
        applyTheme('system');
        config = { work_end_hour: 18 };
        runtimeConfig = config;
      }
      if (disposed) return;

      // 加载背景图
      loadBackground();

      try {
        const recordingState = await invoke<unknown>('get_recording_state');
        if (!isRecordingStateTuple(recordingState)) {
          throw new TypeError('录制状态载荷格式无效');
        }
        const [recording, paused] = recordingState;
        isRecording = recording;
        isPaused = paused;
        recordingStore.setState(recording, paused);
      } catch (e) {
        console.error('获取录制状态失败:', e);
      }
      if (disposed) return;

      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const handleSystemThemeChange = () => {
        if (theme === 'system') applyTheme('system');
      };
      mediaQuery.addEventListener('change', handleSystemThemeChange);
      pendingCleanup.push(() => mediaQuery.removeEventListener('change', handleSystemThemeChange));

      const unsubscribeCache = cache.subscribe((state) => {
        if (!isRuntimeConfig(state.config)) return;
        runtimeConfig = state.config;

        if (state.config.theme && state.config.theme !== theme) {
          applyTheme(state.config.theme);
        }

      });
      pendingCleanup.push(unsubscribeCache);

      const unlistenRecordingState = await safeListen(
        'recording-state-changed',
        isRecordingStateChangedPayload,
        (payload) => {
          isRecording = payload.isRecording;
          isPaused = payload.isPaused;
          recordingStore.setState(payload.isRecording, payload.isPaused);
        },
      );
      if (disposed) { try { if (unlistenRecordingState) unlistenRecordingState(); } catch {} return; }
      pendingCleanup.push(unlistenRecordingState);

      const unlistenConfigChanged = await safeListen(
        'config-changed',
        isRuntimeConfig,
        (payload) => {
          runtimeConfig = payload;
          cache.setConfig(payload);
        },
      );
      if (disposed) { try { if (unlistenConfigChanged) unlistenConfigChanged(); } catch {} return; }
      pendingCleanup.push(unlistenConfigChanged);

      // 监听背景图更新事件（来自设置页，实时预览）
      const handleBgChange = (e: Event) => handleBackgroundChanged(e);
      window.addEventListener('background-changed', handleBgChange);
      pendingCleanup.push(() => window.removeEventListener('background-changed', handleBgChange));


      // 启动预加载
      preloadApp();

      // 启动后延迟执行一次自动更新检查，避免阻塞首屏渲染
      const autoUpdateTimer = setTimeout(async () => {
        try {
          const shouldCheck = await invoke<boolean>('should_check_updates');
          if (!shouldCheck) return;

          await runUpdateFlow({
            silentWhenUpToDate: true,
            confirmBeforeDownload: true,
            onStatusChange: () => {},
          });
        } catch (e) {
          console.warn('自动检查更新失败:', e);
        }
      }, 2000);
      pendingCleanup.push(() => clearTimeout(autoUpdateTimer));

      // 日报自动生成检测：每分钟检查一次
      let lastAutoGenDate: string | null = null;  // 防止同一天重复触发
      let autoGenRunning = false;  // 防止并发生成
      let memorySynthRunning = false;  // 防止洞察合成并发
      const autoReportTimer = setInterval(async () => {
        if (autoGenRunning) return;  // 上一轮还没完成，跳过
        const now = new Date();
        const currentTotalMinutes = now.getHours() * 60 + now.getMinutes();
        const today = getLocalDate();

        // 检查是否达到或已过工作结束时间
        const { hour: workEndHour, minute: workEndMinute } =
          resolveAutoReportWorkEnd(runtimeConfig);
        const workEndTotalMinutes = workEndHour * 60 + workEndMinute;

        // 条件：当前时间 >= 工作结束时间，且今天未自动生成过
        if (currentTotalMinutes >= workEndTotalMinutes && lastAutoGenDate !== today) {
          try {
            // 检查今日是否已有日报
            const existingReport = await invoke<unknown>('get_saved_report', { date: today, locale: currentLocale });
            if (!existingReport) {
              devLog('工作结束时间到达，自动生成日报...');
              autoGenRunning = true;
              try {
                await invoke('generate_report', { date: today, force: false, locale: currentLocale });
                cache.invalidate('report', `${today}:${currentLocale}`);
                lastAutoGenDate = today;
                devLog('日报自动生成完成');
              } finally {
                autoGenRunning = false;
              }
            } else {
              lastAutoGenDate = today;  // 已有日报，标记今天不再触发
            }
          } catch (e) {
            console.warn('日报自动生成失败:', e);
          }
        }

        // AI 工作记忆：每天工作结束后自动合成洞察
        if (currentTotalMinutes >= workEndTotalMinutes && !memorySynthRunning) {
          try {
            const config = await getRuntimeConfig();
            if (config.memory_enabled && config.memory_last_synthesis_date !== today) {
              memorySynthRunning = true;
              try {
                await invoke('synthesize_insights', {});
                // 合成可能耗时较长，重新拉取最新配置再写入，避免覆盖期间用户改动的其他设置
                const freshConfig = await getRuntimeConfig();
                await invoke('save_config', { config: { ...freshConfig, memory_last_synthesis_date: today } });
                devLog('工作记忆合成完成');
              } finally {
                memorySynthRunning = false;
              }
            }
          } catch (e) {
            console.warn('工作记忆合成失败:', e);
          }
        }
      }, 60000);  // 每分钟检查一次
      pendingCleanup.push(() => clearInterval(autoReportTimer));

      const unlisten = await safeListen(
        'screenshot-taken',
        isTimelineActivity,
        (payload) => {
          devLog('截屏完成:', payload);

          // 1. 增量更新时间线缓存
          cache.addActivity(payload);

          // 2. 使概览缓存过期（下次访问或当前页面监听时刷新）
          cache.invalidate('overview');

          // 3. 发射自定义事件，通知当前页面实时更新
          window.dispatchEvent(new CustomEvent('activity-added', { detail: payload }));

          // 4. 抢先预热当前应用图标，浏览器记录优先级更高
          preloadAppIcons(
            [{
              appName: payload.app_name,
              executablePath: payload.executable_path,
            }],
            invokeAppIcon,
            { priority: Boolean(payload.browser_url) }
          );
        },
      );
      if (disposed) { try { if (unlisten) unlisten(); } catch {} return; }
      pendingCleanup.push(unlisten);
    })();

    return () => {
      disposed = true;
      pendingCleanup.forEach(fn => { try { fn(); } catch {} });
    };
  });
</script>

<CarbonTheme theme={carbonTheme} />

{#if isAuthRoute}
  <!-- 登录页：无壳层，仅保留顶部拖拽条与窗口控制（Windows/Linux） -->
  <div class="relative flex h-screen flex-col overflow-hidden">
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <header
      class="flex h-7 shrink-0 items-start justify-end"
      data-tauri-drag-region
      style="-webkit-app-region: drag;"
      on:mousedown={startDrag}
    >
      {#if platform && platform !== 'macos'}
        <div class="flex h-7 items-stretch" style="-webkit-app-region: no-drag;">
          <button class="shell-window-btn" on:click={minimizeWindow} title={t('window.minimize')} aria-label={t('window.minimize')}>
            <SubtractIcon size={14} />
          </button>
          <button class="shell-window-btn" on:click={maximizeWindow} title={windowMaximized ? t('window.restore') : t('window.maximize')} aria-label={windowMaximized ? t('window.restore') : t('window.maximize')}>
            <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.2" aria-hidden="true">
              <rect x="3" y="3" width="10" height="10" />
            </svg>
          </button>
          <button class="shell-window-btn shell-window-btn-close" on:click={closeWindow} title={t('window.close')} aria-label={t('window.close')}>
            <CloseIcon size={14} />
          </button>
        </div>
      {/if}
    </header>
    <main class="min-h-0 flex-1 overflow-auto">
      {#key currentLocale}
        <Router {routes} />
      {/key}
    </main>
    <Toast />
    <ConfirmDialog />
  </div>
{:else}
  <Header
    platformName={tLocale('common.appName')}
    href="#/"
    uiShellAriaLabel={tLocale('common.appName')}
    bind:isSideNavOpen
    data-tauri-drag-region
  >
    <!-- 录制状态（点击暂停/恢复） -->
    <HeaderGlobalAction
      icon={isRecording && !isPaused ? RecordingFilledIcon : isPaused ? PlayIcon : RecordingIcon}
      aria-label={isPaused ? tLocale('sidebar.resume') : tLocale('sidebar.pause')}
      title={(isRecording && !isPaused ? tLocale('sidebar.recordingStatus') + ' · ' : isPaused ? tLocale('sidebar.recordingStatus') + ' · ' : '') + (isPaused ? tLocale('sidebar.resume') : tLocale('sidebar.pause'))}
      on:click={toggleRecording}
    />
    <!-- 语言切换 -->
    <OverflowMenu
      direction="top"
      flipped={true}
      icon={TranslateIcon}
      iconDescription={tLocale('sidebar.localeButtonTitle')}
    >
      {#each localeChoices as option (option.value)}
        <OverflowMenuItem
          icon={currentLocale === option.value ? CheckmarkIcon : undefined}
          text="{option.label} · {tLocale(option.labelKey)}"
          primaryFocus={currentLocale === option.value}
          on:click={() => selectLocale(option.value)}
        />
      {/each}
    </OverflowMenu>
    <!-- 主题循环（跟随系统 / 浅色 / 深色） -->
    <HeaderGlobalAction
      icon={theme === 'system' ? LaptopIcon : theme === 'light' ? LightIcon : AsleepIcon}
      aria-label={tLocale('sidebar.themeTitle.' + theme)}
      title={tLocale('sidebar.themeTitle.' + theme)}
      on:click={cycleTheme}
    />
    <!-- 窗口控制（仅 Windows/Linux，macOS 用原生控件） -->
    {#if platform && platform !== 'macos'}
      <div class="shell-window-controls" role="group" aria-label="Window">
        <button class="shell-window-btn" on:click={minimizeWindow} title={t('window.minimize')} aria-label={t('window.minimize')}>
          <SubtractIcon size={14} />
        </button>
        <button class="shell-window-btn" on:click={maximizeWindow} title={windowMaximized ? t('window.restore') : t('window.maximize')} aria-label={windowMaximized ? t('window.restore') : t('window.maximize')}>
          <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.2" aria-hidden="true">
            {#if windowMaximized}
              <rect x="5" y="3" width="8" height="8" />
              <path d="M3 6V13h8" />
            {:else}
              <rect x="3" y="3" width="10" height="10" />
            {/if}
          </svg>
        </button>
        <button class="shell-window-btn shell-window-btn-close" on:click={closeWindow} title={t('window.close')} aria-label={t('window.close')}>
          <CloseIcon size={14} />
        </button>
      </div>
    {/if}
  </Header>

  <SideNav ariaLabel={tLocale('common.appName')} isOpen={isSideNavOpen} border={true}>
    <SideNavItems>
      {#each shellNavItems as item (item.path)}
        <SideNavLink
          href={'#' + item.path}
          icon={item.icon}
          isSelected={shellActiveStates[item.path]}
          text={tLocale(item.labelKey)}
        >
          {tLocale(item.labelKey)}
        </SideNavLink>
      {/each}
    </SideNavItems>
  </SideNav>

  <Content>
    <!-- 背景图层（设置-外观配置；图片全强度 + 半透明遮罩控制显隐） -->
    {#if backgroundImage}
      <div class="pointer-events-none fixed inset-0 z-0 overflow-hidden">
        <div
          class="absolute inset-[-20px] bg-cover bg-center bg-no-repeat"
          style="background-image: url({backgroundImage}); filter: blur({backgroundBlur === 0 ? 0 : backgroundBlur === 1 ? 8 : 16}px);"
        ></div>
        <div
          class="absolute inset-0 transition-opacity duration-300"
          style="opacity: {Math.max(0, 1 - backgroundOpacity)}; background: var(--cds-background);"
        ></div>
      </div>
    {/if}

    <div class="app-content-scroll relative z-10 flex flex-col">
      {#if authGate === 'grace'}
        <div class="px-4 pt-3">
          <InlineNotification kind="warning" title={tLocale('auth.graceNotice')} hideCloseButton={true} timeout={0}>
            <NotificationActionButton slot="actions" on:click={() => push('/auth')}>
              {tLocale('auth.reloginAction')}
            </NotificationActionButton>
          </InlineNotification>
        </div>
      {/if}
      <div>
        {#key currentLocale}
          <Router {routes} />
        {/key}
      </div>
    </div>
  </Content>
  <Toast />
  <ConfirmDialog />
{/if}

<style>
  /* Content 内滚动容器：Carbon Header 固定 3rem，内容区独立滚动
     （迁移时丢失了旧 .app-shell-main-scroll 的滚动职责，导致高页面被裁切） */
  .app-content-scroll {
    height: calc(100vh - 3rem);
    overflow-y: auto;
    overflow-x: hidden;
  }

  /* 壳层窗口控制按钮：置于 Carbon Header 内右侧，主区可点、不可触发拖拽 */
  .shell-window-controls {
    display: flex;
    align-items: stretch;
    align-self: stretch;
    height: 3rem;
    margin-left: auto;
    -webkit-app-region: no-drag;
  }

  .shell-window-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2.75rem;
    border: none;
    background: transparent;
    color: var(--cds-text-primary, #161616);
    cursor: pointer;
  }

  .shell-window-btn:hover {
    background: var(--cds-layer-hover, rgba(0, 0, 0, 0.06));
  }

  .shell-window-btn-close:hover {
    background: #da1e28;
    color: #fff;
  }

  /* 登录页窗口按钮（浅底） */
  header .shell-window-btn {
    width: 2.75rem;
    height: 1.75rem;
  }
</style>
