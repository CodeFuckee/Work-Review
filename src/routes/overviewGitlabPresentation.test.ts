import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildGitlabProjectRows,
  gitlabProjectShortName,
  sumGitlabProjectRows,
} from './overviewGitlabPresentation.ts';

test('gitlabProjectShortName 取路径末段，无分隔符时原样返回', () => {
  assert.equal(gitlabProjectShortName('group/alpha'), 'alpha');
  assert.equal(gitlabProjectShortName('a/b/c/depth'), 'depth');
  assert.equal(gitlabProjectShortName('plain-repo'), 'plain-repo');
  assert.equal(gitlabProjectShortName('  spaced  '), 'spaced');
  assert.equal(gitlabProjectShortName('trailing/'), 'trailing');
});

test('buildGitlabProjectRows 保留输入顺序并按最多提交数计算进度条宽度', () => {
  const rows = buildGitlabProjectRows([
    { project: 'group/alpha', commits: 3, additions: 120, deletions: 30 },
    { project: 'group/beta', commits: 1, additions: 0, deletions: 0 },
  ]);
  assert.deepEqual(
    rows.map((row) => [row.shortName, row.commits, row.additions, row.deletions]),
    [
      ['alpha', 3, 120, 30],
      ['beta', 1, 0, 0],
    ],
  );
  assert.equal(rows[0].project, 'group/alpha', '完整路径应保留供 tooltip');
  assert.equal(rows[0].barPercent, 100, '提交最多的项目应为满格');
  assert.equal(rows[1].barPercent, Math.max((1 / 3) * 100, 2));
  assert.deepEqual(buildGitlabProjectRows([]), [], '空输入应返回空列表');
});

test('sumGitlabProjectRows 汇总提交次数与行数变更', () => {
  const totals = sumGitlabProjectRows([
    { project: 'a', shortName: 'a', commits: 3, additions: 120, deletions: 30, barPercent: 100 },
    { project: 'b', shortName: 'b', commits: 2, additions: 7, deletions: 90, barPercent: 66.7 },
  ]);
  assert.deepEqual(totals, { commits: 5, additions: 127, deletions: 120 });
  assert.deepEqual(sumGitlabProjectRows([]), { commits: 0, additions: 0, deletions: 0 });
});
