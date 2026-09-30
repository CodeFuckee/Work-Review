import test from 'node:test';
import assert from 'node:assert/strict';
import {
  attachAttribution,
  attributionForTimestamp,
  type AttributionWindowLike,
} from './timelineAttribution.ts';

const windows: AttributionWindowLike[] = [
  { session_start: 100, session_end: 200 },
  { session_start: 300, session_end: 400 },
];

test('时间戳落入某归因窗口内时应返回该窗口（含起止边界）', () => {
  assert.equal(attributionForTimestamp(windows, 150), windows[0]);
  assert.equal(attributionForTimestamp(windows, 100), windows[0]);
  assert.equal(attributionForTimestamp(windows, 200), windows[0]);
  assert.equal(attributionForTimestamp(windows, 400), windows[1]);
});

test('时间戳落在窗口外/间隙/空列表时应返回 null（不归因）', () => {
  assert.equal(attributionForTimestamp(windows, 50), null);
  assert.equal(attributionForTimestamp(windows, 250), null);
  assert.equal(attributionForTimestamp(windows, 450), null);
  assert.equal(attributionForTimestamp([], 150), null);
});

test('多窗口命中时应返回先命中的第一个（与活动行既有约定一致）', () => {
  const overlapping: AttributionWindowLike[] = [
    { session_start: 100, session_end: 300 },
    { session_start: 150, session_end: 250 },
  ];
  assert.equal(attributionForTimestamp(overlapping, 200), overlapping[0]);
});

test('attachAttribution 应按回调时间戳附加归因并保留原行字段', () => {
  const rows = [
    { kind: 'commit' as const, sha: 'a', committed_at: 150 },
    { kind: 'fileEvent' as const, path: 'p', occurred_at: 350 },
    { kind: 'planeItem' as const, id: 'x', updated_at: 50 },
  ];
  const attached = attachAttribution(rows, windows, (row) =>
    row.kind === 'commit' ? row.committed_at : row.kind === 'fileEvent' ? row.occurred_at : row.updated_at,
  );

  assert.equal(attached.length, 3);
  // 原行字段保留
  assert.equal(attached[0].sha, 'a');
  assert.equal(attached[1].path, 'p');
  // 命中窗口的行带归因，未命中的为 null
  assert.equal(attached[0].attribution, windows[0]);
  assert.equal(attached[1].attribution, windows[1]);
  assert.equal(attached[2].attribution, null);
});
