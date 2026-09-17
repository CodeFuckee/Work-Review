import test from 'node:test';
import assert from 'node:assert/strict';

import {
  formatCommitStatsSuffix,
  mergeTimelineRows,
  type GitlabCommitRow,
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
