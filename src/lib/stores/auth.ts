// 客户端认证模块：JWT 存储、状态管理、Admin.Core 登录/注册 API 调用。
// 强制登录：启动时无本地凭证 → 跳转 /auth；
// 离线宽限：token 过期但本地有缓存时允许继续使用本地功能（grace）。
// 服务端端点（zhontai Admin.Core ClientAuthService）：
//   POST {admin}/api/admin/client-auth/login
//   POST {admin}/api/admin/client-auth/register

import { invoke } from '@tauri-apps/api/core';
import { parseTokenExpiresAt, type AuthGate, resolveAuthGate } from '../../routes/auth/authLogic.ts';

/** Rust 侧 client_http_post_json 命令的返回（delivered=false 即网络层失败） */
interface ClientHttpPostJsonResult {
  delivered: boolean;
  status?: number;
  body?: string;
  error?: string;
}

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
    const parsed = JSON.parse(raw) as Partial<AuthState> | null;
    // 形状校验：合法 JSON 但缺字段/错型（旧版或半损坏条目）时，
    // isTokenExpired 算出 number > undefined 恒为 false，坏凭证会被判成
    // allow 直接放行进主界面（admin 同步全挂）——不满足一律按无凭证走登录门
    if (
      !parsed ||
      typeof parsed.token !== 'string' ||
      parsed.token === '' ||
      typeof parsed.expiresAt !== 'number' ||
      !Number.isFinite(parsed.expiresAt)
    ) {
      return null;
    }
    return parsed as AuthState;
  } catch {
    return null;
  }
}

export function setAuth(state: AuthState): void {
  // 配额满/隐私模式下 setItem 可能抛错；此刻服务端已发放 token，
  // 缓存失败不应让登录流程崩成未处理 rejection——warn 后照常返回
  //（本次会话内内存状态仍可用，下次启动走重新登录）
  try {
    localStorage.setItem(AUTH_KEY, JSON.stringify(state));
  } catch (e) {
    console.warn('认证凭证写入 localStorage 失败（配额满/隐私模式）:', e);
  }
}

export function clearAuth(): void {
  localStorage.removeItem(AUTH_KEY);
}

export function getAdminUrl(): string {
  return localStorage.getItem(ADMIN_URL_KEY) || 'http://10.37.80.25:4191';
}

export function setAdminUrl(url: string): void {
  try {
    localStorage.setItem(ADMIN_URL_KEY, url.trim().replace(/\/+$/, ''));
  } catch (e) {
    console.warn('admin 地址写入 localStorage 失败:', e);
  }
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
  const url = `${adminUrl}${path}`;
  let httpStatus = 0;
  let rawBody: unknown;

  // 生产路径：登录/注册经 Rust 侧 reqwest 发请求（client_http_post_json 命令）。
  // WebView 页面源（Windows https://tauri.localhost / Linux tauri://localhost）都是
  // 安全上下文，页面 fetch 纯 HTTP 内网地址会被混合内容拦截且零网络请求
  // （v1.1.81 Windows、v1.2.22 Linux 两次复现）；Rust 侧不受 WebView 网络栈约束。
  try {
    const res = await invoke<ClientHttpPostJsonResult>('client_http_post_json', { url, body });
    if (!res.delivered) {
      console.warn('客户端认证请求异常（网络层）:', res.error);
      return { ok: false, network: true };
    }
    httpStatus = res.status ?? 0;
    rawBody = res.body;
  } catch (e) {
    // 浏览器预览（vite dev / README 截图 harness）无 Tauri 运行时：退回 fetch，
    // 行为与旧版一致（localhost 源无混合内容问题）。
    if (!(e instanceof Error) || !String(e.message ?? e).includes('__TAURI_INTERNALS__')) {
      console.warn('客户端认证请求异常:', e);
      return { ok: false, network: true };
    }
    try {
      const resp = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        // admin 服务挂起（建连但不响应）时 15 秒超时落入下方 network 分支，
        // 登录/注册按钮不再永久 pending
        signal: AbortSignal.timeout(15_000),
      });
      httpStatus = resp.status;
      rawBody = await resp.text();
    } catch (e2) {
      console.warn('客户端认证请求异常:', e2);
      return { ok: false, network: true };
    }
  }

  if (httpStatus < 200 || httpStatus >= 300) {
    return { ok: false, message: `HTTP ${httpStatus}` };
  }
  try {
    const data = (typeof rawBody === 'string' ? JSON.parse(rawBody) : rawBody) as {
      data?: ClientAuthResponse;
    } | ClientAuthResponse;
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
  } catch (e) {
    console.warn('客户端认证响应解析异常:', e);
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
