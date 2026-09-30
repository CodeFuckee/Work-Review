export interface TimelineActivityLike {
  appName?: unknown;
  app_name?: unknown;
  windowTitle?: unknown;
  window_title?: unknown;
}

function trimmed(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

function normalizeComparable(value: unknown): string {
  return trimmed(value)
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fa5]+/gi, '');
}

// 安装器词表：isGenericInstallerToken 与 isInstallerLikeName 共用，
// 内联两份曾逐字重复，增删词项易只改一处
const INSTALLER_TOKENS = ['setup', 'install', 'installer', 'uninstall'];

function isGenericInstallerToken(value: unknown): boolean {
  return INSTALLER_TOKENS.includes(normalizeComparable(value));
}

function isInstallerLikeName(value: unknown): boolean {
  const nameWithoutExtension = trimmed(value).replace(/\.(?:exe|msi|pkg|dmg|app)$/i, '');
  const tokens = nameWithoutExtension
    .toLowerCase()
    .split(/[^a-z0-9\u4e00-\u9fa5]+/i)
    .filter(Boolean);

  if (tokens.some((token) => INSTALLER_TOKENS.includes(token))) {
    return true;
  }

  const comparable = normalizeComparable(nameWithoutExtension);
  return INSTALLER_TOKENS.some((token) => comparable.endsWith(token));
}

function isCompactRawToken(value: unknown): boolean {
  const source = trimmed(value);
  return Boolean(source) && /^[a-z0-9_.-]+$/i.test(source) && !/[A-Z]/.test(source);
}

export function getPreferredTimelineAppName(activity: TimelineActivityLike = {}): string {
  const rawAppName = trimmed(activity.appName || activity.app_name);
  const rawTitle = trimmed(activity.windowTitle || activity.window_title);
  if (!rawTitle) {
    return rawAppName;
  }

  const appComparable = normalizeComparable(rawAppName);
  const titleComparable = normalizeComparable(rawTitle);

  if (isGenericInstallerToken(rawAppName) && rawTitle.length > rawAppName.length) {
    return rawTitle;
  }

  if (
    isInstallerLikeName(rawAppName)
    && titleComparable
    && appComparable
    && (
      appComparable.includes(titleComparable)
      || (
        (appComparable.startsWith('workreview') || appComparable.startsWith('dailyreport') || appComparable.startsWith('yuereview'))
        && (titleComparable.startsWith('workreview') || titleComparable.startsWith('dailyreport') || titleComparable.startsWith('yuereview'))
        && appComparable.endsWith('setup')
        && titleComparable.endsWith('setup')
      )
    )
  ) {
    return rawTitle;
  }

  if (appComparable && titleComparable === appComparable && rawTitle !== rawAppName) {
    return rawTitle;
  }

  if (
    isCompactRawToken(rawAppName)
    && titleComparable
    && appComparable
    && (appComparable.includes(titleComparable) || titleComparable.includes(appComparable))
    && rawTitle.length >= rawAppName.length
  ) {
    return rawTitle;
  }

  return rawAppName || rawTitle;
}

export function shouldPreferTimelineFallbackIcon(activity: TimelineActivityLike = {}): boolean {
  const rawAppName = trimmed(activity.appName || activity.app_name);
  if (!rawAppName) {
    return false;
  }

  return isGenericInstallerToken(rawAppName) || isInstallerLikeName(rawAppName);
}
