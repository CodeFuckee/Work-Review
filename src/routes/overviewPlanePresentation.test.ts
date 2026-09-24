import test from 'node:test';
import assert from 'node:assert/strict';

import { buildPlaneProjectOverview } from './overviewPlanePresentation.ts';

test('buildPlaneProjectOverview 空输入应返回空列表', () => {
  assert.deepEqual(buildPlaneProjectOverview([]), []);
});

test('buildPlaneProjectOverview 按项目分组并统计当日变动条数', () => {
  const rows = buildPlaneProjectOverview([
    { project_name: 'Alpha', name: '修复登录崩溃', state_name: '进行中', updated_at: 1_760_000_100 },
    { project_name: 'Beta', name: '整理需求文档', state_name: '待办', updated_at: 1_760_000_050 },
    { project_name: 'Alpha', name: '优化首页加载', state_name: '已完成', updated_at: 1_760_000_200 },
    { project_name: 'Alpha', name: '补充单元测试', state_name: '进行中', updated_at: 1_760_000_150 },
  ]);
  assert.deepEqual(
    rows.map((row) => [row.project, row.count]),
    [
      ['Alpha', 3],
      ['Beta', 1],
    ],
  );
});

test('buildPlaneProjectOverview latest 应取组内 updated_at 最大的条目', () => {
  const rows = buildPlaneProjectOverview([
    { project_name: 'Alpha', name: '修复登录崩溃', state_name: '进行中', updated_at: 1_760_000_100 },
    { project_name: 'Alpha', name: '优化首页加载', state_name: '已完成', updated_at: 1_760_000_200 },
    { project_name: 'Alpha', name: '补充单元测试', state_name: '进行中', updated_at: 1_760_000_150 },
  ]);
  assert.equal(rows.length, 1);
  const alpha = rows[0];
  assert.ok(alpha);
  assert.equal(alpha.latestName, '优化首页加载');
  assert.equal(alpha.latestState, '已完成');
  assert.equal(alpha.latestUpdatedAt, 1_760_000_200);
});

test('buildPlaneProjectOverview 乱序输入仍保持首次出现顺序', () => {
  const rows = buildPlaneProjectOverview([
    { project_name: 'Gamma', name: '任务一', state_name: '进行中', updated_at: 500 },
    { project_name: 'Alpha', name: '任务二', state_name: '待办', updated_at: 900 },
    { project_name: 'Gamma', name: '任务三', state_name: '已完成', updated_at: 700 },
  ]);
  assert.deepEqual(
    rows.map((row) => row.project),
    ['Gamma', 'Alpha'],
    '应保持首次出现顺序，不做名称排序',
  );
  assert.equal(rows[0]?.latestName, '任务三');
  assert.equal(rows[1]?.latestName, '任务二');
});

test('buildPlaneProjectOverview updated_at 并列时保留先出现的条目为 latest', () => {
  const rows = buildPlaneProjectOverview([
    { project_name: 'Alpha', name: '先出现', state_name: '进行中', updated_at: 1_000 },
    { project_name: 'Alpha', name: '后出现', state_name: '待办', updated_at: 1_000 },
  ]);
  assert.equal(rows.length, 1);
  assert.equal(rows[0]?.count, 2);
  assert.equal(rows[0]?.latestName, '先出现');
  assert.equal(rows[0]?.latestState, '进行中');
});
