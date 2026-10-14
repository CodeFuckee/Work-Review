import test from 'node:test';
import assert from 'node:assert/strict';

// 多机工作记录同步纯函数测试：块切分、seq 恢复、日报 LWW 判定、拉取游标单调前进。
// 与服务端的交互（HTTP/加密/invoke）不在纯函数层，E2E 与联调覆盖。

interface ActivityLike {
  uid: string;
  timestamp: number;
}

test('工作记录同步模块应导出全部纯函数与常量', async () => {
  const mod = await import('./workSync.ts');
  assert.equal(typeof mod.splitIntoBlocks, 'function');
  assert.equal(typeof mod.resolveNextSeq, 'function');
  assert.equal(typeof mod.shouldPushReport, 'function');
  assert.equal(typeof mod.mergePullCursor, 'function');
  assert.equal(typeof mod.groupBlocksByMachine, 'function');
  assert.ok(mod.WORK_SYNC_BLOCK_SIZE >= 1);
  assert.ok(mod.WORK_SYNC_INTERVAL_MS >= 30_000);
});

test('splitIntoBlocks 空输入返回空且不产生空块', async () => {
  const { splitIntoBlocks } = await import('./workSync.ts');
  assert.deepEqual(splitIntoBlocks([], 200), []);
});

test('splitIntoBlocks 按块大小切分并保留顺序与余数块', async () => {
  const { splitIntoBlocks } = await import('./workSync.ts');
  const items: ActivityLike[] = Array.from({ length: 450 }, (_, i) => ({
    uid: `u${i}`,
    timestamp: i,
  }));
  const blocks = splitIntoBlocks(items, 200);
  assert.equal(blocks.length, 3, '450 条按 200 切应为 3 块');
  assert.equal(blocks[0].length, 200);
  assert.equal(blocks[1].length, 200);
  assert.equal(blocks[2].length, 50, '余数块保留剩余条目');
  assert.equal(blocks[0][0].uid, 'u0', '块内保持原顺序');
  assert.equal(blocks[2][49].uid, 'u449');
});

test('splitIntoBlocks 恰好整除时不产生空尾块', async () => {
  const { splitIntoBlocks } = await import('./workSync.ts');
  const items: ActivityLike[] = Array.from({ length: 400 }, (_, i) => ({
    uid: `u${i}`,
    timestamp: i,
  }));
  const blocks = splitIntoBlocks(items, 200);
  assert.equal(blocks.length, 2);
  assert.equal(blocks[1].length, 200);
});

test('resolveNextSeq 以本地与云端最大 seq 为准递增', async () => {
  const { resolveNextSeq } = await import('./workSync.ts');
  // 正常：本地游标更大
  assert.equal(resolveNextSeq(5, { machA: 3, machB: 9 }, 'machA'), 6);
  // 状态文件丢失后从服务端 latest 恢复
  assert.equal(resolveNextSeq(0, { machA: 12 }, 'machA'), 13);
  // 双方都为空（首次推送）
  assert.equal(resolveNextSeq(0, {}, 'machA'), 1);
  // 云端比本地新（另一处客户端用同一 machineId 推过——理论不该发生，取大者防 seq 冲突）
  assert.equal(resolveNextSeq(4, { machA: 9 }, 'machA'), 10);
});

test('shouldPushReport 本地更新才推送且时间相等跳过', async () => {
  const { shouldPushReport } = await import('./workSync.ts');
  assert.equal(shouldPushReport(1000, null), true, '云端无版本（从未推过）应推送');
  assert.equal(shouldPushReport(2000, 1000), true, '本地更新应推送');
  assert.equal(shouldPushReport(1000, 2000), false, '云端更新应跳过（改走拉取）');
  assert.equal(shouldPushReport(1000, 1000), false, '相等视为已同步，跳过推送');
});

test('mergePullCursor 只前进不后退', async () => {
  const { mergePullCursor } = await import('./workSync.ts');
  assert.equal(mergePullCursor(5, 9), 9);
  assert.equal(mergePullCursor(9, 5), 9, '服务端乱序/旧响应不得回退游标');
  assert.equal(mergePullCursor(0, 0), 0);
});

test('groupBlocksByMachine 按机器聚合且保持 seq 顺序', async () => {
  const { groupBlocksByMachine } = await import('./workSync.ts');
  const blocks = [
    { machineId: 'machB', seq: 2, cipher: 'c2', iv: 'i2' },
    { machineId: 'machA', seq: 1, cipher: 'c1', iv: 'i1' },
    { machineId: 'machB', seq: 5, cipher: 'c5', iv: 'i5' },
    { machineId: 'machB', seq: 4, cipher: 'c4', iv: 'i4' },
  ];
  const grouped = groupBlocksByMachine(blocks);
  assert.deepEqual(Object.keys(grouped).sort(), ['machA', 'machB']);
  assert.deepEqual(
    grouped.machB.map((b) => b.seq),
    [2, 4, 5],
    '同机块按 seq 升序（属主写入顺序，幂等收敛依赖该顺序）',
  );
  assert.deepEqual(
    grouped.machA.map((b) => b.seq),
    [1],
  );
});
