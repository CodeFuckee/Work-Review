// 多机同步双机闭环联调：模拟公司机/家里机两台客户端对本地 admin-core 的
// 完整推拉（注册→派生密钥→加密活动块→推→对方拉→解密比对→日报 LWW）。
// 与 TS 引擎同款 HTTP 形状与加解密参数（PBKDF2 150k / AES-GCM / b64）。
// 运行：node --import tsx tests/integration/work-sync-roundtrip.mjs [baseUrl]
const BASE = process.argv[2] ?? 'http://localhost:18010';

const enc = new TextEncoder();
const dec = new TextDecoder();

function b64(bytes) {
  let s = '';
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s);
}
function unb64(text) {
  return Uint8Array.from(atob(text), (c) => c.charCodeAt(0));
}

async function deriveKey(password, userName) {
  const base = await crypto.subtle.importKey('raw', enc.encode(password), 'PBKDF2', false, ['deriveKey']);
  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt: enc.encode(`yue-review-sync:${userName}`), iterations: 150_000, hash: 'SHA-256' },
    base,
    { name: 'AES-GCM', length: 256 },
    true,
    ['encrypt', 'decrypt'],
  );
}

async function encryptPayload(key, plaintext) {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const cipher = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, enc.encode(plaintext));
  return { cipher: b64(new Uint8Array(cipher)), iv: b64(iv) };
}

async function decryptPayload(key, cipher, iv) {
  return dec.decode(await crypto.subtle.decrypt({ name: 'AES-GCM', iv: unb64(iv) }, key, unb64(cipher)));
}

async function registerOrLogin(userName) {
  await fetch(`${BASE}/api/admin/client-auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ userName, password: 'e2e-sync-123', name: userName }),
  }).catch(() => {});
  const res = await fetch(`${BASE}/api/admin/client-auth/login`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ userName, password: 'e2e-sync-123' }),
  });
  if (!res.ok) throw new Error(`login failed: ${res.status}`);
  const body = await res.json();
  const token = body?.data?.token;
  if (!token) throw new Error(`no token: ${JSON.stringify(body).slice(0, 200)}`);
  return { token, authed: (path, payload) => fetch(`${BASE}${path}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` },
    body: JSON.stringify(payload),
  }) };
}

async function main() {
  const suffix = Date.now().toString(36);
  const account = `syncrt_${suffix}`;
  const { token, authed } = await registerOrLogin(account);
  const key = await deriveKey('e2e-sync-123', account);
  const assert = (cond, msg) => { if (!cond) throw new Error(`断言失败: ${msg}`); console.log(`  ✓ ${msg}`); };

  // ── 公司机推 3 块 ──
  const companyActivities = Array.from({ length: 3 }, (_, i) => ({
    uid: `machCompany:u${i}`,
    timestamp: 1_760_000_000 + i * 60,
    app_name: 'IDEA',
    window_title: `公司写代码 ${i}`,
    ocr_text: null,
    category: 'development',
    duration: 60,
    browser_url: null,
    semantic_category: null,
    semantic_confidence: null,
  }));
  for (let seq = 1; seq <= 3; seq += 1) {
    const chunk = seq === 3 ? companyActivities.slice(0, 1) : companyActivities.slice(seq - 1, seq);
    const { cipher, iv } = await encryptPayload(key, JSON.stringify(chunk));
    const res = await authed('/api/admin/activities-sync/push-activities', {
      machineId: 'machCompany', machineName: '公司电脑',
      blocks: [{ seq, cipher, iv }],
    });
    const body = await res.json();
    assert(res.ok && body?.data?.acceptedSeqs?.length === 1, `公司机块 ${seq} 推送成功`);
  }

  // ── 家里机拉取并解密比对 ──
  {
    const res = await authed('/api/admin/activities-sync/pull-activities', { cursors: {} });
    const data = (await res.json())?.data;
    assert(res.ok && data.blocks.length === 3, '家里机拉到 3 块');
    assert(data.latestSeqByMachine.machCompany === 3, 'latestSeqByMachine 指向 3');
    const plain = JSON.parse(await decryptPayload(key, data.blocks[0].cipher, data.blocks[0].iv));
    assert(plain[0].uid === 'machCompany:u0', '解密回原活动载荷');
    assert(plain[0].window_title === '公司写代码 0', '窗口标题完整');
  }

  // ── 家里机推日报（v2），公司机旧版本被拒 ──
  const report = { content: '# 全天日报', ai_mode: 'cloud', model_name: null, fallback_reason: null, created_at: 1 };
  {
    const { cipher, iv } = await encryptPayload(key, JSON.stringify(report));
    const res = await authed('/api/admin/activities-sync/save-daily-report', {
      date: '2026-10-14', locale: 'zh-CN', cipher, iv, machineId: 'machHome', machineName: '家里电脑', updatedAt: 2_000,
    });
    assert(res.ok && (await res.json())?.data?.accepted === true, '家里机日报 v2 被接受');
  }
  {
    const { cipher, iv } = await encryptPayload(key, JSON.stringify({ ...report, content: '# 旧版' }));
    const res = await authed('/api/admin/activities-sync/save-daily-report', {
      date: '2026-10-14', locale: 'zh-CN', cipher, iv, machineId: 'machCompany', machineName: '公司电脑', updatedAt: 1_000,
    });
    const data = (await res.json())?.data;
    assert(res.ok && data.accepted === false && data.updatedAt === 2_000, '公司机旧版日报被拒（LWW）');
  }

  // ── 公司机增量拉取日报 + 机器清单 ──
  {
    const res = await authed('/api/admin/activities-sync/pull-daily-reports', { sinceUpdatedAt: 0 });
    const data = (await res.json())?.data;
    assert(data.reports.length === 1, '公司机拉到 1 份日报变更');
    const plain = JSON.parse(await decryptPayload(key, data.reports[0].cipher, data.reports[0].iv));
    assert(plain.content === '# 全天日报', '日报解密内容正确');
    assert(data.reports[0].machineId === 'machHome', '来源机器为家里机');
  }
  {
    const listRes = await fetch(`${BASE}/api/admin/activities-sync/list-machines`, {
      headers: { authorization: `Bearer ${token}` },
    });
    const machines = (await listRes.json())?.data?.machines;
    assert(machines.length === 2, '机器清单含两台机器');
    const names = machines.map((m) => m.machineName).sort();
    assert(names[0] === '公司电脑' && names[1] === '家里电脑', `机器名注册正确: ${names}`);
  }

  console.log('\n✅ 双机闭环联调全部通过');
}

main().catch((e) => { console.error('❌', e.message); process.exit(1); });
