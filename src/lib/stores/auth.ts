// 客户端认证模块：JWT 存储、状态管理、Admin.Core 登录/注册 API 调用。
// 强制登录：启动时无本地凭证 → 跳转 /auth；
// 离线宽限：token 过期但本地有缓存时允许继续使用本地功能（grace）。
// 服务端端点（zhontai Admin.Core ClientAuthService）：
//   POST {admin}/api/admin/client-auth/login
//   POST {admin}/api/admin/client-auth/register

import { parseTokenExpiresAt, type AuthGate, resolveAuthGate } from '../../routes/auth/authLogic.ts';

const AUTH_KEY = 'yue-review-auth';
const ADMIN_URL_KEY = 'yue-review-admin-url';
// token 无 exp 字段时的兜底有效期（秒）
const FALLBACK_TOKEN_TTL_SECONDS = 12 * 60 * 60;

export interface AuthState {
  token: string;
  userId: number;
  userName: string;
  name: string;
  expiresAt: number;
}

export interface ClientAuthCallResult {
  ok: boolean;
  token?: string;
  userId?: number;
  userName?: string;
  name?: string;
  /** 服务端返回的业务错误信息 */
  message?: string;
  /** 网络不通/服务器不可达 */
  network?: boolean;
}

export function getAuth(): AuthState | null {
  try {
    const raw = localStorage.getItem(AUTH_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as AuthState;
  } catch {
    return null;
  }
}

export function setAuth(state: AuthState): void {
  localStorage.setItem(AUTH_KEY, JSON.stringify(state));
}

export function clearAuth(): void {
  localStorage.removeItem(AUTH_KEY);
}

export function getAdminUrl(): string {
  return localStorage.getItem(ADMIN_URL_KEY) || 'http://10.37.80.25:4191';
}

export function setAdminUrl(url: string): void {
  localStorage.setItem(ADMIN_URL_KEY, url.trim().replace(/\/+$/, ''));
}

export function isTokenExpired(state: AuthState): boolean {
  // expiresAt 与 JWT exp 同为秒级 Unix 时间戳
  return Date.now() / 1000 > state.expiresAt;
}

/** 从本地存储判定认证门：login=强制登录 / grace=过期离线宽限 / allow=放行 */
export function resolveAuthGateFromStorage(): AuthGate {
  const auth = getAuth();
  if (!auth) return 'login';
  return resolveAuthGate(true, isTokenExpired(auth));
}

/** 用登录/注册返回的 token 构造本地认证状态（expiresAt 优先取 JWT exp） */
export function buildAuthStateFromToken(params: {
  token: string;
  userId?: number;
  userName?: string;
  name?: string;
}): AuthState {
  const expiresAt = parseTokenExpiresAt(params.token)
    ?? Math.floor(Date.now() / 1000) + FALLBACK_TOKEN_TTL_SECONDS;
  return {
    token: params.token,
    userId: params.userId ?? 0,
    userName: params.userName ?? '',
    name: params.name ?? '',
    expiresAt,
  };
}

interface ClientAuthResponse {
  success?: boolean;
  token?: string | null;
  userId?: number;
  userName?: string | null;
  name?: string | null;
  message?: string | null;
}

async function postClientAuth(
  adminUrl: string,
  path: string,
  body: Record<string, string>,
): Promise<ClientAuthCallResult> {
  try {
    const resp = await fetch(`${adminUrl}${path}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!resp.ok) {
      return { ok: false, message: `HTTP ${resp.status}` };
    }
    const data = (await resp.json()) as { data?: ClientAuthResponse } | ClientAuthResponse;
    // Admin.Core 统一包装为 { success, code, msg, data }，登录结果在 data.data
    const inner: ClientAuthResponse = (('data' in data && data.data) ? data.data : data) as ClientAuthResponse;
    if (inner.success === false) {
      return { ok: false, message: inner.message || '' };
    }
    if (!inner.token) {
      return { ok: false, message: '未获取到登录凭证' };
    }
    return {
      ok: true,
      token: inner.token,
      userId: inner.userId,
      userName: inner.userName ?? undefined,
      name: inner.name ?? undefined,
    };
  } catch {
    return { ok: false, network: true };
  }
}

export function clientLogin(
  adminUrl: string,
  userName: string,
  password: string,
): Promise<ClientAuthCallResult> {
  return postClientAuth(`${adminUrl}`, '/api/admin/client-auth/login', { userName, password });
}

export function clientRegister(
  adminUrl: string,
  userName: string,
  password: string,
  name: string,
  department: string,
): Promise<ClientAuthCallResult> {
  return postClientAuth(`${adminUrl}`, '/api/admin/client-auth/register', {
    userName,
    password,
    name,
    department,
  });
}
