// 多机工作记录同步引擎（TS 侧编排 + 纯函数）。
// 设计契约（grilling 结论，见知识库 25-projects/work-review/multi-machine-record-sync-grilling.md）：
// - 服务端只存密文（沿用配置同步的 PBKDF2+AES-GCM 端侧加密），合并全部在客户端；
// - 引擎放 TS：JWT 与派生密钥在 WebView 侧，HTTP 走 Rust 透传命令（client_http_post_json）；
// - 每轮三件事：推本地新/脏活动块 → 拉对方块解密入库（uid 幂等 + 远控影子标记）→
//   日报双向 LWW；
// - 日报生成前必须先拉一轮（「生成前强制拉取」），否则同日日报缺对方数据。

import { invoke } from '@tauri-apps/api/core';
import { getAdminUrl, getAuth } from '../stores/auth.ts';
import { decryptPayload, encryptPayload, getStoredSyncKey } from './configSync';

/** 单块活动条数上限：配合 5 分钟轮询单块密文 ~0.7MB，服务端请求体无压力 */
export const WORK_SYNC_BLOCK_SIZE = 200;

/** 常驻轮询间隔：5 分钟（公司机下班关机场景下，最后一块由退出补推兜底） */
export const WORK_SYNC_INTERVAL_MS = 5 * 60 * 1000;

/** 日报推送回看窗口：只同步最近 N 天的日报，避免历史全量重推 */
export const REPORT_PUSH_WINDOW_DAYS = 14;

/** Rust 侧导出批次返回结构（commands::work_sync::WorkSyncExportBatch） */
export interface WorkSyncExportBatch {
  activities: SyncActivityPayload[];
  ids: number[];
  max_id: number;
  has_more: boolean;
}

/** 与 Rust work_sync::SyncActivityPayload 对齐（字段已是剥离本地路径后的净载荷） */
export interface SyncActivityPayload {
  uid: string;
  timestamp: number;
  app_name: string;
  window_title: string;
  ocr_text: string | null;
  category: string;
  duration: number;
  browser_url: string | null;
  semantic_category: string | null;
  semantic_confidence: number | null;
}

/** 云端活动块（密文） */
export interface SyncBlock {
  machineId: string;
  seq: number;
  cipher: string;
  iv: string;
}

/** 云端日报块（密文） */
export interface CloudReportBlock {
  date: string;
  locale: string;
  cipher: string;
  iv: string;
  machineId: string;
  updatedAt: number;
}

/** 按 block size 切块；空输入返回空数组，恰整除不产生空尾块 */
export function splitIntoBlocks<T>(items: T[], blockSize: number): T[][] {
  const blocks: T[][] = [];
  for (let i = 0; i < items.length; i += blockSize) {
    blocks.push(items.slice(i, i + blockSize));
  }
  return blocks;
}

/** 推送 seq：本地游标与服务端 latestSeq 取大者 + 1（状态文件丢失后可恢复） */
export function resolveNextSeq(
  localLastSeq: number,
  latestSeqByMachine: Record<string, number>,
  machineId: string,
): number {
  const cloud = latestSeqByMachine[machineId] ?? 0;
  return Math.max(localLastSeq, cloud) + 1;
}

/** 日报 LWW：本地 updated_at 严格大于云端版本才推（相等视为已同步） */
export function shouldPushReport(localUpdatedAt: number, cloudUpdatedAt: number | null): boolean {
  if (cloudUpdatedAt === null) return true;
  return localUpdatedAt > cloudUpdatedAt;
}

/** 拉取游标只前进：服务端乱序/旧响应不得回退本地游标 */
export function mergePullCursor(current: number, incoming: number): number {
  return Math.max(current, incoming);
}

/** 把 pull-blocks 返回的块按机器聚合，组内按 seq 升序（属主写入顺序，
    uid UPSERT 收敛依赖该顺序：后块覆盖前块） */
export function groupBlocksByMachine<T extends { machineId: string; seq: number }>(
  blocks: T[],
): Record<string, T[]> {
  const grouped: Record<string, T[]> = {};
  for (const block of blocks) {
    (grouped[block.machineId] ??= []).push(block);
  }
  for (const key of Object.keys(grouped)) {
    grouped[key].sort((a, b) => a.seq - b.seq);
  }
  return grouped;
}

// ── 以下为编排层（依赖 Tauri invoke / 网络），不做单测，E2E 与联调覆盖 ──────

function deliverError(result: { delivered: boolean; status: number; body: unknown; error?: string }): unknown {
  if (!result.delivered) {
    throw new Error(result.error || '网络请求失败');
  }
  if (result.status < 200 || result.status >= 300) {
    throw new Error(`服务端返回 ${result.status}`);
  }
  return result.body;
}

/** Admin.Core 统一包装 { success, code, msg, data }：取 data，非 2xx 抛错（401 交由调用方识别） */
function unwrapData(body: unknown): { data: unknown; status: number } {
  const status = (body as { status?: number }).status ?? 0;
  const wrapper = body as { success?: boolean; data?: unknown };
  if (wrapper && typeof wrapper === 'object' && 'data' in wrapper) {
    return { data: wrapper.data, status };
  }
  return { data: body, status };
}

async function postJson(adminUrl: string, path: string, token: string, payload: unknown): Promise<{ data: unknown; status: number }> {
  const result = await invoke<{
    delivered: boolean;
    status: number;
    body: unknown;
    error?: string;
  }>('client_http_post_json', {
    url: `${adminUrl}${path}`,
    body: payload,
    authToken: token,
  });
  const body = deliverError(result) as Record<string, unknown>;
  return unwrapData({ ...body, status: result.status });
}

/** 同步轮次日志（设置卡状态展示用） */
export interface WorkSyncRunSummary {
  pushedBlocks: number;
  importedActivities: number;
  pushedReports: number;
  pulledReports: number;
}

/**
 * 执行一轮完整同步（推活动块 → 拉活动块 → 日报双向）。
 * 任一阶段失败抛错，由调用方决定是否中断（生成前拉取场景失败不阻塞生成）。
 */
export async function runWorkSyncRound(
  adminUrl: string,
  token: string,
  syncKey: CryptoKey,
  options: { pullOnly?: boolean; machineName?: string } = {},
): Promise<WorkSyncRunSummary> {
  const summary: WorkSyncRunSummary = {
    pushedBlocks: 0,
    importedActivities: 0,
    pushedReports: 0,
    pulledReports: 0,
  };

  const wsState = await invoke<WorkSyncState>('work_sync_get_state');
  const machineId = wsState.machine_id;
  if (!machineId) throw new Error('本机机器身份缺失');

  // ── 1. 推送本地活动块 ──
  if (!options.pullOnly) {
    for (let guard = 0; guard < 50; guard += 1) {
      const batch = await invoke<WorkSyncExportBatch>('work_sync_export_batch');
      if (batch.activities.length === 0) break;
      const latest = await postJson(adminUrl, '/api/admin/activities-sync/pull-activities', token, {
        cursors: {},
      });
      const latestSeqByMachine =
        (latest.data as { latestSeqByMachine?: Record<string, number> })?.latestSeqByMachine ?? {};
      let seq = resolveNextSeq(wsState.last_pushed_seq, latestSeqByMachine, machineId);
      const blocks = splitIntoBlocks(batch.activities, WORK_SYNC_BLOCK_SIZE).map((chunk) => ({
        seq: seq++,
        chunk,
      }));
      for (const { seq: blockSeq, chunk } of blocks) {
        const plain = JSON.stringify(chunk);
        const { cipher, iv } = await encryptPayload(syncKey, plain);
        await postJson(adminUrl, '/api/admin/activities-sync/push-activities', token, {
          machineId,
          blocks: [{ seq: blockSeq, cipher, iv }],
        });
        summary.pushedBlocks += 1;
      }
      await invoke('work_sync_advance_cursors', {
        update: { pushedSeq: seq - 1 },
      });
      await invoke('work_sync_mark_synced', { ids: batch.ids, maxId: batch.max_id });
      if (!batch.has_more) break;
    }
  }

  // ── 2. 拉取远端活动块 ──
  const pullResult = await postJson(adminUrl, '/api/admin/activities-sync/pull-activities', token, {
    cursors: wsState.pull_cursors ?? {},
  });
  const pullData = (pullResult.data ?? {}) as {
    blocks?: SyncBlock[];
    latestSeqByMachine?: Record<string, number>;
  };
  const grouped = groupBlocksByMachine(pullData.blocks ?? []);
  const newCursors: Record<string, number> = { ...(wsState.pull_cursors ?? {}) };
  for (const [remoteMachine, blocks] of Object.entries(grouped)) {
    if (remoteMachine === machineId) continue; // 自己的块不回灌（重装恢复场景 v1 不做）
    let maxSeq = newCursors[remoteMachine] ?? 0;
    for (const block of blocks) {
      const plain = await decryptPayload(syncKey, block.cipher, block.iv);
      const rows = JSON.parse(plain) as SyncActivityPayload[];
      const outcome = await invoke<{ inserted: number; updated: number }>(
        'work_sync_import_batch',
        { machineId: remoteMachine, activities: rows },
      );
      summary.importedActivities += outcome.inserted;
      maxSeq = mergePullCursor(maxSeq, block.seq);
    }
    newCursors[remoteMachine] = maxSeq;
  }
  // 未出现的机器游标保持不变；latestSeqByMachine 用于把游标追平（块可能被过期清理）
  for (const [remoteMachine, latestSeq] of Object.entries(pullData.latestSeqByMachine ?? {})) {
    if (remoteMachine !== machineId) {
      newCursors[remoteMachine] = mergePullCursor(newCursors[remoteMachine] ?? 0, latestSeq);
    }
  }
  await invoke('work_sync_advance_cursors', { update: { pullCursors: newCursors } });

  // ── 3. 日报双向 LWW ──
  if (!options.pullOnly) {
    summary.pushedReports = await pushLocalReports(
      adminUrl,
      token,
      syncKey,
      machineId,
      options.machineName ?? '',
    );
  }
  summary.pulledReports = await pullCloudReports(
    adminUrl,
    token,
    syncKey,
    wsState.last_report_pull_time ?? 0,
  );

  return summary;
}

interface WorkSyncState {
  machine_id: string;
  last_pushed_activity_id: number;
  last_pushed_seq: number;
  pull_cursors: Record<string, number>;
  last_report_pull_time: number;
}

interface DailyReportMeta {
  date: string;
  locale: string;
  content: string;
  ai_mode: string;
  model_name: string | null;
  fallback_reason: string | null;
  created_at: number;
  updated_at: number;
  origin_machine: string | null;
}

function localDateStamp(offsetDays: number): string {
  const d = new Date();
  d.setDate(d.getDate() - offsetDays);
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const dd = String(d.getDate()).padStart(2, '0');
  return `${d.getFullYear()}-${mm}-${dd}`;
}

/** 推送本地最近 N 天日报到云端（服务端 LWW 拒旧） */
async function pushLocalReports(
  adminUrl: string,
  token: string,
  syncKey: CryptoKey,
  machineId: string,
  machineName: string,
): Promise<number> {
  const start = localDateStamp(REPORT_PUSH_WINDOW_DAYS - 1);
  const end = localDateStamp(0);
  const reports = await invoke<DailyReportMeta[] | null>('get_reports_in_range_for_sync', {
    startDate: start,
    endDate: end,
    locale: null,
  });
  // 未注册命令（E2E mock）/异常环境可能返回 null：同步是尽力而为，空处理
  const reportList = Array.isArray(reports) ? reports : [];
  let pushed = 0;
  for (const report of reportList) {
    const plain = JSON.stringify({
      content: report.content,
      ai_mode: report.ai_mode,
      model_name: report.model_name,
      fallback_reason: report.fallback_reason,
      created_at: report.created_at,
    });
    const { cipher, iv } = await encryptPayload(syncKey, plain);
    const result = await postJson(adminUrl, '/api/admin/activities-sync/save-daily-report', token, {
      date: report.date,
      locale: report.locale,
      cipher,
      iv,
      machineId,
      machineName,
      updatedAt: report.updated_at,
    });
    const data = result.data as { accepted?: boolean };
    if (data?.accepted) pushed += 1;
  }
  return pushed;
}

/** 拉取云端日报更新并落地（apply_remote_report 覆盖本地行） */
async function pullCloudReports(
  adminUrl: string,
  token: string,
  syncKey: CryptoKey,
  sinceTime: number,
): Promise<number> {
  const result = await postJson(adminUrl, '/api/admin/activities-sync/pull-daily-reports', token, {
    sinceUpdatedAt: sinceTime,
  });
  const data = (result.data ?? {}) as {
    reports?: CloudReportBlock[];
    latestUpdatedAt?: number;
  };
  let applied = 0;
  let maxTime = sinceTime;
  for (const block of data.reports ?? []) {
    const plain = await decryptPayload(syncKey, block.cipher, block.iv);
    const payload = JSON.parse(plain) as Omit<DailyReportMeta, 'date' | 'locale' | 'updated_at' | 'origin_machine'>;
    await invoke('apply_remote_report', {
      report: {
        date: block.date,
        locale: block.locale,
        content: payload.content,
        ai_mode: payload.ai_mode,
        model_name: payload.model_name,
        fallback_reason: payload.fallback_reason,
        created_at: payload.created_at,
        updated_at: 0,
        origin_machine: block.machineId,
      },
      updatedAt: block.updatedAt,
      originMachine: block.machineId,
    });
    applied += 1;
    maxTime = mergePullCursor(maxTime, block.updatedAt);
  }
  if (data.latestUpdatedAt) {
    maxTime = mergePullCursor(maxTime, data.latestUpdatedAt);
  }
  if (maxTime > sinceTime) {
    await invoke('work_sync_advance_cursors', { update: { reportPullTime: maxTime } });
  }
  return applied;
}

/** 供调用方取同步密钥（未登录/密钥未派生返回 null） */
export async function requireSyncKey(): Promise<CryptoKey | null> {
  return getStoredSyncKey();
}

/** 用本地存储的凭据执行一轮同步；未启用/未登录/无密钥返回 null（调用方静默跳过）。
    401 等错误向上抛，由调用方决定提示方式（生成前拉取场景吞掉不阻塞） */
export async function syncWithStoredCredentials(
  enabled: boolean,
  machineName: string,
  pullOnly = false,
): Promise<WorkSyncRunSummary | null> {
  if (!enabled) return null;
  const auth = getAuth();
  const adminUrl = getAdminUrl();
  if (!auth?.token || !adminUrl) return null;
  const syncKey = await getStoredSyncKey();
  if (!syncKey) return null;
  return runWorkSyncRound(adminUrl, auth.token, syncKey, { pullOnly, machineName });
}

/** 日报生成前强制拉取（grilling 结论）：另一台机器的活动必须先进库，
    日报才能覆盖全天。开关从配置读取，任何失败静默吞掉——拉取失败绝不阻塞生成。 */
export async function pullBeforeGenerate(): Promise<void> {
  try {
    const cfg = await invoke<Record<string, unknown>>('get_config');
    const enabled = cfg.work_sync_enabled === true;
    const machineName = typeof cfg.machine_name === 'string' ? cfg.machine_name : '';
    await syncWithStoredCredentials(enabled, machineName, true);
  } catch {
    // 离线宽限：本地功能照常，同步等下一轮
  }
}
