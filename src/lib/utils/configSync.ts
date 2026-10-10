// 跨设备配置同步纯函数：payload 抽取/回填、逐块差异、账号派生密钥加解密。
// 设计契约（设置页「配置同步」卡）：
// - 同步范围仅「连接类」配置（GitLab/Plane/AI/机器人凭证/localhost API），
//   机器绑定项（数据目录/开机自启/主题/工作时间段）一律不纳入；
// - 云端只存不可读密文：AES-GCM，密钥 = PBKDF2-SHA256(登录密码, 账号盐)，
//   服务端数据库泄露不泄露凭据；重登后按同参数重派生即可解旧密文。

export type SyncPayload = Record<string, unknown>;

export type BlockDiffState = 'both-empty' | 'local-only' | 'cloud-only' | 'same' | 'differs';

/** 同步块 → 纳入同步的配置键清单（互不重叠，新增块时在此登记） */
export const SYNC_BLOCK_KEYS: Record<string, string[]> = {
  gitlab: ['gitlab'],
  plane: ['plane'],
  ai: [
    'ai_provider',
    'text_model',
    'text_model_profiles',
    'vision_model',
    'ai_mode',
    'assistant_search_provider',
    'assistant_search_api_key',
    'assistant_web_access_enabled',
    'assistant_timeout_secs',
    'assistant_memory_enabled',
    'embedding_provider',
    'embedding_endpoint',
    'embedding_model',
    'embedding_api_key',
    'memory_semantic_enabled',
    'daily_report_custom_prompt',
    'daily_report_prompt_presets',
    'report_generation_timeout_secs',
  ],
  bots: [
    'feishu_bot_enabled',
    'feishu_app_id',
    'feishu_app_secret',
    'feishu_verification_token',
    'wecom_bot_enabled',
    'wecom_corp_id',
    'wecom_token',
    'wecom_encoding_aes_key',
    'wecom_bot_id',
    'wecom_bot_secret',
    'dingtalk_bot_enabled',
    'dingtalk_app_secret',
  ],
  localhost: ['localhost_api_enabled', 'localhost_api_port', 'localhost_api_host'],
};

export const SYNC_BLOCK_LABELS: Record<string, string> = {
  gitlab: 'GitLab',
  plane: 'Plane',
  ai: 'AI 模型与提示词',
  bots: '机器人凭证',
  localhost: 'localhost API',
};

export const SYNC_KEY_STORAGE = 'yue-review-sync-key';

/** 键值视为「未配置」：undefined / null / 空串 */
function isUnset(value: unknown): boolean {
  return value === undefined || value === null || value === '';
}

/** 从本机 config 抽取同步载荷：仅收集有值的键，未配置键不出现（空块 = 空对象） */
export function extractSyncPayload(config: Record<string, unknown>): SyncPayload {
  const payload: SyncPayload = {};
  for (const keys of Object.values(SYNC_BLOCK_KEYS)) {
    for (const key of keys) {
      if (key in config && !isUnset(config[key])) {
        payload[key] = config[key];
      }
    }
  }
  return payload;
}

/** 把载荷回填进 config（返回新对象，深拷贝载荷值；载荷缺的键保持原样） */
export function applySyncPayload(
  config: Record<string, unknown>,
  payload: SyncPayload,
): Record<string, unknown> {
  const merged: Record<string, unknown> = { ...config };
  for (const [key, value] of Object.entries(payload)) {
    if (value !== undefined && typeof value === 'object' && value !== null) {
      merged[key] = structuredClone(value);
    } else if (value !== undefined) {
      merged[key] = value;
    }
  }
  return merged;
}

function blockHasContent(payload: SyncPayload | null, block: string): boolean {
  const keys = SYNC_BLOCK_KEYS[block];
  if (!keys || !payload) return false;
  return keys.some((key) => key in payload && !isUnset(payload[key]));
}

function blockEquals(local: SyncPayload | null, cloud: SyncPayload | null, block: string): boolean {
  const keys = SYNC_BLOCK_KEYS[block];
  if (!keys) return true;
  for (const key of keys) {
    const l = local && key in local ? JSON.stringify(local[key]) : undefined;
    const r = cloud && key in cloud ? JSON.stringify(cloud[key]) : undefined;
    if (l !== r) return false;
  }
  return true;
}

/** 逐块差异判定（供拉取/上传前的差异预览对话框） */
export function diffSyncBlocks(
  localPayload: SyncPayload | null,
  cloudPayload: SyncPayload | null,
): Record<string, BlockDiffState> {
  const result: Record<string, BlockDiffState> = {};
  for (const block of Object.keys(SYNC_BLOCK_KEYS)) {
    const localHas = blockHasContent(localPayload, block);
    const cloudHas = blockHasContent(cloudPayload, block);
    if (!localHas && !cloudHas) {
      result[block] = 'both-empty';
    } else if (!localHas) {
      result[block] = 'cloud-only';
    } else if (!cloudHas) {
      result[block] = 'local-only';
    } else {
      result[block] = blockEquals(localPayload, cloudPayload, block) ? 'same' : 'differs';
    }
  }
  return result;
}

/** 登录密码 + 账号名派生 AES-GCM 密钥（PBKDF2-SHA256，150k 迭代）；
    重登后按同参数重派生即可解旧密文（确定性） */
export async function deriveSyncKey(password: string, userName: string): Promise<CryptoKey> {
  const encoder = new TextEncoder();
  const salt = encoder.encode(`yue-review-sync:${userName}`);
  const baseKey = await crypto.subtle.importKey('raw', encoder.encode(password), 'PBKDF2', false, [
    'deriveKey',
  ]);
  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt, iterations: 150_000, hash: 'SHA-256' },
    baseKey,
    { name: 'AES-GCM', length: 256 },
    true,
    ['encrypt', 'decrypt'],
  );
}

function toB64(bytes: Uint8Array): string {
  let text = '';
  for (const byte of bytes) text += String.fromCharCode(byte);
  return btoa(text);
}

function fromB64(b64: string): Uint8Array<ArrayBuffer> {
  const raw = atob(b64);
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i += 1) bytes[i] = raw.charCodeAt(i);
  return bytes;
}

/** AES-GCM 加密明文，随机 12 字节 IV；返回 b64(cipher) + b64(iv) */
export async function encryptPayload(
  key: CryptoKey,
  plaintext: string,
): Promise<{ cipher: string; iv: string }> {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const cipher = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    key,
    new TextEncoder().encode(plaintext),
  );
  return { cipher: toB64(new Uint8Array(cipher)), iv: toB64(iv) };
}

/** 解密 b64 密文（密钥不符/密文被篡改时抛错） */
export async function decryptPayload(key: CryptoKey, cipher: string, iv: string): Promise<string> {
  const plain = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: fromB64(iv) },
    key,
    fromB64(cipher),
  );
  return new TextDecoder().decode(plain);
}

/** 派生密钥导出为 b64 存入 localStorage（与 JWT 同生命周期：登出时一并清除） */
export async function storeSyncKey(key: CryptoKey): Promise<void> {
  const raw = await crypto.subtle.exportKey('raw', key);
  localStorage.setItem(SYNC_KEY_STORAGE, toB64(new Uint8Array(raw)));
}

/** 取回存储的派生密钥；未存储/损坏时返回 null（调用方应引导重新登录派生） */
export async function getStoredSyncKey(): Promise<CryptoKey | null> {
  const b64 = localStorage.getItem(SYNC_KEY_STORAGE);
  if (!b64) return null;
  try {
    return await crypto.subtle.importKey('raw', fromB64(b64), { name: 'AES-GCM' }, false, [
      'encrypt',
      'decrypt',
    ]);
  } catch {
    return null;
  }
}

export function clearStoredSyncKey(): void {
  localStorage.removeItem(SYNC_KEY_STORAGE);
}
