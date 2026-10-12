import test from 'node:test';
import assert from 'node:assert/strict';

// 跨设备配置同步纯函数测试：payload 抽取/回填、逐块差异、账号派生密钥加解密。
// WebCrypto 用 Node 全局 crypto.subtle（Node 20+），localStorage 用注入桩。

function installLocalStorageStub(): void {
  const store = new Map<string, string>();
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: (k: string) => (store.has(k) ? (store.get(k) as string) : null),
      setItem: (k: string, v: string) => void store.set(k, String(v)),
      removeItem: (k: string) => void store.delete(k),
    },
  });
}

test('配置同步模块应导出块定义与全部纯函数', async () => {
  installLocalStorageStub();
  const mod = await import('./configSync.ts');
  assert.ok(mod.SYNC_BLOCK_KEYS);
  assert.ok(mod.SYNC_BLOCK_LABELS);
  assert.equal(typeof mod.extractSyncPayload, 'function');
  assert.equal(typeof mod.applySyncPayload, 'function');
  assert.equal(typeof mod.diffSyncBlocks, 'function');
  assert.equal(typeof mod.deriveSyncKey, 'function');
  assert.equal(typeof mod.encryptPayload, 'function');
  assert.equal(typeof mod.decryptPayload, 'function');
  assert.equal(typeof mod.storeSyncKey, 'function');
  assert.equal(typeof mod.getStoredSyncKey, 'function');
  assert.equal(typeof mod.clearStoredSyncKey, 'function');
});

test('同步块定义应覆盖五类连接配置且互不重叠', async () => {
  installLocalStorageStub();
  const { SYNC_BLOCK_KEYS } = await import('./configSync.ts');
  const blocks = Object.keys(SYNC_BLOCK_KEYS);
  assert.ok(blocks.includes('gitlab') && blocks.includes('plane') && blocks.includes('ai') && blocks.includes('bots') && blocks.includes('localhost'));
  const seen = new Set<string>();
  for (const keys of Object.values(SYNC_BLOCK_KEYS)) {
    for (const k of keys) {
      assert.ok(!seen.has(k), `配置键 ${k} 重复出现在多个块`);
      seen.add(k);
    }
  }
  // 关键字段抽检
  assert.ok(SYNC_BLOCK_KEYS.gitlab.includes('gitlab'));
  assert.ok(SYNC_BLOCK_KEYS.plane.includes('plane'));
  assert.ok(SYNC_BLOCK_KEYS.ai.includes('text_model'));
  assert.ok(SYNC_BLOCK_KEYS.ai.includes('daily_report_custom_prompt'));
  assert.ok(SYNC_BLOCK_KEYS.bots.includes('feishu_app_secret'));
  assert.ok(SYNC_BLOCK_KEYS.bots.includes('wecom_corp_id'));
  assert.ok(SYNC_BLOCK_KEYS.bots.includes('dingtalk_app_secret'));
  assert.ok(SYNC_BLOCK_KEYS.localhost.includes('localhost_api_port'));
});

test('extractSyncPayload 应提取已配置块并跳过未配置键', async () => {
  installLocalStorageStub();
  const { extractSyncPayload } = await import('./configSync.ts');
  const config = {
    gitlab: { url: 'https://git.example.com', token: 'glpat-x', project_filter: null, work_authors: null, report_enabled: true },
    text_model: { provider: 'deepseek', endpoint: 'https://api.deepseek.com', api_key: 'sk-1', model: 'deepseek-chat' },
    localhost_api_enabled: true,
    localhost_api_port: 47831,
    // 未配置：plane 缺失、theme 属机器类不纳入
    theme: 'dark',
  };
  const payload = extractSyncPayload(config);
  assert.ok(payload.gitlab, 'gitlab 块应被提取');
  assert.equal((payload.gitlab as { url: string }).url, 'https://git.example.com');
  assert.ok(payload.text_model, 'AI 块应被提取');
  assert.equal((payload.text_model as { model: string }).model, 'deepseek-chat');
  assert.equal(payload.localhost_api_enabled, true);
  assert.equal(payload.plane, undefined, '未配置块不应出现');
  assert.equal(payload.theme, undefined, '机器类配置不得混入同步载荷');
});

test('extractSyncPayload 空配置应返回空载荷', async () => {
  installLocalStorageStub();
  const { extractSyncPayload } = await import('./configSync.ts');
  assert.deepEqual(extractSyncPayload({}), {});
});

test('applySyncPayload 应把载荷回填进配置且不污染原对象', async () => {
  installLocalStorageStub();
  const { applySyncPayload } = await import('./configSync.ts');
  const original = { theme: 'dark', gitlab: null as unknown };
  const payload = {
    gitlab: { url: 'https://git.example.com', token: 'glpat-x' },
    text_model: { model: 'deepseek-chat' },
  };
  const merged = applySyncPayload(original as Record<string, unknown>, payload);
  assert.deepEqual(merged.gitlab, payload.gitlab);
  assert.deepEqual(merged.text_model, payload.text_model);
  assert.equal(merged.theme, 'dark', '非同步键应保留');
  assert.equal(original.gitlab, null, '原 config 不得被修改');
  // 引用隔离：修改 merged 不影响 payload
  (merged.gitlab as { url: string }).url = 'changed';
  assert.equal(payload.gitlab.url, 'https://git.example.com');
});

test('applySyncPayload 载荷缺块时本机对应块应保持不变', async () => {
  installLocalStorageStub();
  const { applySyncPayload } = await import('./configSync.ts');
  const original = { plane: { enabled: true, url: 'https://plane.local' }, gitlab: null };
  const merged = applySyncPayload(original as Record<string, unknown>, { gitlab: { url: 'x' } });
  assert.deepEqual(merged.plane, { enabled: true, url: 'https://plane.local' }, '载荷缺失的块不得被动');
  assert.ok(merged.gitlab);
});

test('diffSyncBlocks 应区分五种块状态', async () => {
  installLocalStorageStub();
  const { diffSyncBlocks } = await import('./configSync.ts');
  const local = {
    gitlab: { url: 'https://a.com', token: 't1' },
    plane: { enabled: true },
    text_model: { model: 'm1' },
  };
  const cloud = {
    gitlab: { url: 'https://a.com', token: 't1' },
    text_model: { model: 'm2' },
    bots: undefined,
    wecom_corp_id: 'w1',
  };
  const diff = diffSyncBlocks(local, cloud);
  assert.equal(diff.gitlab, 'same', '两端一致 → same');
  assert.equal(diff.plane, 'local-only', '本机有云端无 → local-only');
  assert.equal(diff.ai, 'differs', '模型不同 → differs');
  assert.equal(diff.bots, 'cloud-only', '本机无云端有 → cloud-only');
  assert.equal(diff.localhost, 'both-empty', '两端皆空 → both-empty');
});

test('diffSyncBlocks 任一端为空载荷时应全部按空处理', async () => {
  installLocalStorageStub();
  const { diffSyncBlocks } = await import('./configSync.ts');
  const local = { gitlab: { url: 'x' } };
  const d1 = diffSyncBlocks(local, null);
  assert.equal(d1.gitlab, 'local-only');
  const d2 = diffSyncBlocks(null, local);
  assert.equal(d2.gitlab, 'cloud-only');
  const d3 = diffSyncBlocks(null, null);
  assert.equal(d3.gitlab, 'both-empty');
});

test('派生密钥加解密应可往返且与账号绑定', async () => {
  installLocalStorageStub();
  const { deriveSyncKey, encryptPayload, decryptPayload } = await import('./configSync.ts');
  const keyA = await deriveSyncKey('password-1', 'alice');
  const plaintext = JSON.stringify({ gitlab: { token: 'glpat-secret' } });
  const { cipher, iv } = await encryptPayload(keyA, plaintext);
  assert.notEqual(cipher, plaintext);
  assert.ok(iv.length > 0);
  const round = await decryptPayload(keyA, cipher, iv);
  assert.equal(round, plaintext);
  // 不同账号派生不同密钥 → 解密失败
  const keyB = await deriveSyncKey('password-1', 'bob');
  await assert.rejects(() => decryptPayload(keyB, cipher, iv));
  // 同账号不同密码 → 解密失败
  const keyA2 = await deriveSyncKey('password-2', 'alice');
  await assert.rejects(() => decryptPayload(keyA2, cipher, iv));
});

test('密文被篡改时解密应失败', async () => {
  installLocalStorageStub();
  const { deriveSyncKey, encryptPayload, decryptPayload } = await import('./configSync.ts');
  const key = await deriveSyncKey('pw', 'alice');
  const { cipher, iv } = await encryptPayload(key, '{"a":1}');
  const bytes = Uint8Array.from(atob(cipher), (c) => c.charCodeAt(0));
  bytes[bytes.length - 1] ^= 0xff;
  const tampered = btoa(String.fromCharCode(...bytes));
  await assert.rejects(() => decryptPayload(key, tampered, iv));
});

test('同步密钥的存取应走 localStorage 且可清除', async () => {
  installLocalStorageStub();
  const { deriveSyncKey, storeSyncKey, getStoredSyncKey, clearStoredSyncKey, SYNC_KEY_STORAGE } = await import('./configSync.ts');
  const key = await deriveSyncKey('pw', 'alice');
  await storeSyncKey(key);
  const restored = await getStoredSyncKey();
  assert.ok(restored, '存储后应能取回');
  assert.equal(restored.type, 'secret');
  assert.ok(localStorage.getItem(SYNC_KEY_STORAGE), '底层应落 localStorage');
  clearStoredSyncKey();
  assert.equal(await getStoredSyncKey(), null);
  assert.equal(localStorage.getItem(SYNC_KEY_STORAGE), null);
});

test('重复派生同账号密钥应得到可用等价密钥（PBKDF2 确定性）', async () => {
  installLocalStorageStub();
  const { deriveSyncKey, encryptPayload, decryptPayload } = await import('./configSync.ts');
  const k1 = await deriveSyncKey('pw', 'alice');
  const { cipher, iv } = await encryptPayload(k1, 'hello');
  const k2 = await deriveSyncKey('pw', 'alice');
  assert.equal(await decryptPayload(k2, cipher, iv), 'hello', '重登后重新派生的密钥应能解开旧密文');
});

test('同步 API 认证失败响应应可被识别（success:false 语义）', async () => {
  installLocalStorageStub();
  const { isSyncAuthFailure } = await import('./configSync.ts');
  // 服务端 _user.Id<=0 时返回 200 + {success:false, 全空字段}（DTO 注释约定：
  // false = 未登录/登录过期，调用端应引导重新登录）
  assert.equal(isSyncAuthFailure({ success: false, payload: null, iv: null, updatedTime: null }), true, '认证失败必须被识别');
  // 从未上传：success:true + 全空字段——不是认证失败，是「云端暂无配置」
  assert.equal(isSyncAuthFailure({ success: true, payload: null, iv: null, updatedTime: null }), false);
  // 有配置
  assert.equal(isSyncAuthFailure({ success: true, payload: 'x', iv: 'y', updatedTime: '2026-10-11 12:00:00' }), false);
  // 防御：缺 success 字段（旧服务端/异常响应）与非法入参一律不算认证失败
  assert.equal(isSyncAuthFailure({ payload: null }), false);
  assert.equal(isSyncAuthFailure(null), false);
  assert.equal(isSyncAuthFailure('oops'), false);
});
