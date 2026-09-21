// 登录/注册页的纯逻辑：JWT 过期解析、注册表单校验、认证门判定。
// 不依赖 localStorage / Tauri API，可在 node 测试环境直接运行。

export type AuthGate = 'allow' | 'grace' | 'login';

export interface RegisterFormInput {
  userName: string;
  password: string;
  confirmPassword: string;
  name: string;
}

function base64UrlDecodeToUtf8(input: string): string {
  const normalized = input.replace(/-/g, '+').replace(/_/g, '/');
  const padded = normalized + '='.repeat((4 - (normalized.length % 4)) % 4);
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new TextDecoder().decode(bytes);
}

/** 从 JWT 中解析 exp（秒级 Unix 时间戳）；格式非法或缺 exp 时返回 null */
export function parseTokenExpiresAt(token: string): number | null {
  const parts = token.split('.');
  const payloadPart = parts.length === 3 ? parts[1] : undefined;
  if (!payloadPart) return null;

  try {
    const payload = JSON.parse(base64UrlDecodeToUtf8(payloadPart)) as { exp?: unknown };
    if (typeof payload.exp === 'number' && Number.isFinite(payload.exp)) {
      return payload.exp;
    }
    return null;
  } catch {
    return null;
  }
}

/** 认证门判定：未登录 → 强制登录；token 过期 → 离线宽限；其余放行 */
export function resolveAuthGate(hasAuth: boolean, tokenExpired: boolean): AuthGate {
  if (!hasAuth) return 'login';
  return tokenExpired ? 'grace' : 'allow';
}

/** 登录表单校验：返回 i18n 键，通过返回 null */
export function findLoginValidationError(userName: string, password: string): string | null {
  if (!userName.trim() || !password) return 'auth.errorAccountRequired';
  return null;
}

/** 注册表单校验：返回 i18n 键，通过返回 null */
export function findRegisterValidationError(input: RegisterFormInput): string | null {
  if (!input.userName.trim()) return 'auth.errorUserNameRequired';
  if (!input.name.trim()) return 'auth.errorNameRequired';
  if (input.password.length < 6) return 'auth.errorPasswordTooShort';
  if (input.password !== input.confirmPassword) return 'auth.errorPasswordMismatch';
  return null;
}
