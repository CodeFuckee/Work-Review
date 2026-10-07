import test from 'node:test';
import assert from 'node:assert/strict';

// 由于 auth.ts 使用了 localStorage（浏览器 API），这里只做纯逻辑测试：
// 校验常量、端点路径与关键调用结构；纯函数逻辑见 src/routes/auth/authLogic.test.ts。

test('AUTH_KEY 和 ADMIN_URL_KEY 常量不应为空', async () => {
  const source = await import('node:fs').then(fs =>
    fs.readFileSync(new URL('./auth.ts', import.meta.url), 'utf8'),
  );
  assert.match(source, /AUTH_KEY\s*=\s*'[^']+'/);
  assert.match(source, /ADMIN_URL_KEY\s*=\s*'[^']+'/);
});

test('clientLogin 应调用 Admin.Core client-auth/login 端点', async () => {
  const source = await import('node:fs').then(fs =>
    fs.readFileSync(new URL('./auth.ts', import.meta.url), 'utf8'),
  );
  assert.match(source, /clientLogin/);
  assert.match(source, /\/api\/admin\/client-auth\/login/);
  assert.match(source, /userName|password/);
});

test('clientRegister 应调用 Admin.Core client-auth/register 端点', async () => {
  const source = await import('node:fs').then(fs =>
    fs.readFileSync(new URL('./auth.ts', import.meta.url), 'utf8'),
  );
  assert.match(source, /clientRegister/);
  assert.match(source, /\/api\/admin\/client-auth\/register/);
});

// 回归：v1.1.81（Windows）与 v1.2.22（Linux）两度因 WebView 安全上下文的混合内容拦截
// 导致登录页 fetch 纯 HTTP 内网地址零请求失败。登录/注册必须走 Rust 侧
// client_http_post_json（reqwest），fetch 仅作为无 Tauri 运行时的浏览器预览兜底。
test('登录/注册请求应经 Rust 侧 client_http_post_json 发出（绕开 WebView 混合内容拦截）', async () => {
  const source = await import('node:fs').then(fs =>
    fs.readFileSync(new URL('./auth.ts', import.meta.url), 'utf8'),
  );
  assert.match(source, /invoke<ClientHttpPostJsonResult>\('client_http_post_json'/);
  // 浏览器预览兜底必须保留（vite dev / README 截图 harness 无 __TAURI_INTERNALS__）
  assert.match(source, /__TAURI_INTERNALS__/);
});

test('认证状态持久化与过期判定应成对出现', async () => {
  const source = await import('node:fs').then(fs =>
    fs.readFileSync(new URL('./auth.ts', import.meta.url), 'utf8'),
  );
  assert.match(source, /setAuth/);
  assert.match(source, /getAuth/);
  assert.match(source, /clearAuth/);
  assert.match(source, /isTokenExpired/);
  assert.match(source, /parseTokenExpiresAt/);
  assert.match(source, /resolveAuthGateFromStorage/);
  assert.match(source, /buildAuthStateFromToken/);
});
