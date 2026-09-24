import test from 'node:test';
import assert from 'node:assert/strict';

import {
  formatCommitStatsSuffix,
  mergeTimelineRows,
  type FileEventRow,
  type GitlabCommitRow,
  type PlaneWorkItemRow,
} from './timelineGitlab.ts';

function commit(sha: string, committedAt: number): GitlabCommitRow {
  return {
    sha,
    date: '2026-09-17',
    committed_at: committedAt,
    project: 'chenkaidi/work-review',
    branch: 'main',
    short_id: sha.slice(0, 8),
    title: `commit ${sha}`,
    additions: 3,
    deletions: 1,
    author_name: 'cjy66668888',
    author_email: 'a@b.c',
  };
}

function activity(timestamp: number, id: number | null = 1) {
  return {
    id,
    timestamp,
    app_name: 'VS Code',
    window_title: 'edit',
    screenshot_path: '',
    category: 'development',
    duration: 30,
  };
}

test('提交行应与活动行按时间戳交错排序（最新在最上）', () => {
  const rows = mergeTimelineRows(
    [activity(200), activity(100)],
    [commit('c', 150)],
  );
  assert.deepEqual(
    rows.map((row) => row.key),
    [200, 150, 100],
    '降序排列，提交 150 应插在两条活动之间',
  );
  assert.equal(rows[1].kind, 'commit');
  assert.equal(rows[0].kind, 'activity');
});

test('时间戳相同时提交行排在活动行之前且排序稳定', () => {
  const rows = mergeTimelineRows([activity(100)], [commit('c1', 100), commit('c2', 100)]);
  assert.deepEqual(
    rows.map((row) => row.kind),
    ['commit', 'commit', 'activity'],
  );
  assert.deepEqual(
    rows.filter((row): row is Extract<typeof row, { kind: 'commit' }> => row.kind === 'commit').map((row) => row.commit.sha),
    ['c1', 'c2'],
    '同 key 的提交保持输入顺序',
  );
});

test('空输入合并为空列表', () => {
  assert.deepEqual(mergeTimelineRows([], []), []);
});

test('统计后缀应有数值才输出', () => {
  assert.equal(formatCommitStatsSuffix(12, 3), ' (+12/-3)');
  assert.equal(formatCommitStatsSuffix(0, 0), ' (+0/-0)');
  assert.equal(formatCommitStatsSuffix(null, null), '');
});

function fileEvent(occurredAt: number, path = 'src/app.ts'): FileEventRow {
  return {
    path,
    event_type: 'modified',
    size: 1024,
    occurred_at: occurredAt,
    date: '2026-09-17',
  };
}

test('文件事件行应与活动/提交行按时间戳交错，同时间戳时排在提交与活动之后', () => {
  const rows = mergeTimelineRows(
    [activity(200)],
    [commit('c', 200)],
    [fileEvent(200), fileEvent(300, 'src/b.ts')],
  );
  assert.deepEqual(
    rows.map((row) => `${row.kind}:${row.key}`),
    ['fileEvent:300', 'commit:200', 'activity:200', 'fileEvent:200'],
  );
});

test('不传文件事件时保持原有两源合并行为', () => {
  const rows = mergeTimelineRows([activity(100)], [commit('c', 50)]);
  assert.deepEqual(
    rows.map((row) => row.kind),
    ['activity', 'commit'],
  );
});

function planeItem(id: string, updatedAt: number, name = `工作项 ${id}`): PlaneWorkItemRow {
  return {
    plane_item_id: id,
    project_name: 'Alpha',
    name,
    state_name: '进行中',
    updated_at: updatedAt,
    url: null,
  };
}

test('plane 工作项行应与其他来源按时间戳交错，同时间戳时排在最后', () => {
  const rows = mergeTimelineRows(
    [activity(200)],
    [commit('c', 250)],
    [fileEvent(200)],
    [planeItem('wi-1', 200), planeItem('wi-2', 300)],
  );
  assert.deepEqual(
    rows.map((row) => `${row.kind}:${row.key}`),
    ['planeItem:300', 'commit:250', 'activity:200', 'fileEvent:200', 'planeItem:200'],
    '降序排列，同 200 的四类行按 commit/activity/fileEvent/planeItem 排序',
  );
  assert.equal(rows[0].kind, 'planeItem');
});

test('同时间戳的 plane 工作项行保持输入顺序（幂等稳定）', () => {
  const items = [planeItem('wi-1', 100), planeItem('wi-2', 100), planeItem('wi-3', 100)];
  const first = mergeTimelineRows([], [], [], items);
  const second = mergeTimelineRows([], [], [], [...items].reverse());
  const idsOf = (rows: ReturnType<typeof mergeTimelineRows>) =>
    rows
      .filter((row): row is Extract<typeof row, { kind: 'planeItem' }> => row.kind === 'planeItem')
      .map((row) => row.item.plane_item_id);
  assert.deepEqual(idsOf(first), ['wi-1', 'wi-2', 'wi-3'], '同 key 保持输入顺序');
  assert.deepEqual(
    idsOf(second),
    ['wi-3', 'wi-2', 'wi-1'],
    '稳定排序：同 key 行随输入顺序，同输入必得同输出',
  );
});

test('不传 plane 工作项时保持原有三源合并行为', () => {
  const rows = mergeTimelineRows([activity(200)], [commit('c', 200)], [fileEvent(150)]);
  assert.deepEqual(
    rows.map((row) => `${row.kind}:${row.key}`),
    ['commit:200', 'activity:200', 'fileEvent:150'],
  );
});
