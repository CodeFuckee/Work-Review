import test from 'node:test';
import assert from 'node:assert/strict';

import {
  filterProjectOptions,
  normalizeProjectSelection,
  toggleProject,
} from './gitlabProjectPicker.ts';

test('仓库过滤应忽略大小写按子串匹配且空查询返回全部', () => {
  const projects = ['team/App-Backend', 'team/app-frontend', 'docs/site'];
  assert.deepEqual(filterProjectOptions(projects, ''), projects);
  assert.deepEqual(filterProjectOptions(projects, 'APP'), ['team/App-Backend', 'team/app-frontend']);
  assert.deepEqual(filterProjectOptions(projects, 'site'), ['docs/site']);
  assert.deepEqual(filterProjectOptions(projects, '不存在'), []);
});

test('勾选切换应保持原顺序地增删条目', () => {
  const selected = ['a/x', 'b/y'];
  assert.deepEqual(toggleProject(selected, 'c/z'), ['a/x', 'b/y', 'c/z'], '未选中则追加到末尾');
  assert.deepEqual(toggleProject(selected, 'a/x'), ['b/y'], '已选中则移除');
  assert.deepEqual(
    toggleProject(['b/y'], 'a/x'),
    ['b/y', 'a/x'],
    '移除后的列表再次切换应重新追加到末尾',
  );
  assert.deepEqual(
    toggleProject(selected, 'A/X'),
    ['b/y'],
    '忽略大小写识别为已选中并移除，不会产生大小写重复条目',
  );
});

test('保存前规范化应去空白去空并按忽略大小写去重', () => {
  assert.deepEqual(
    normalizeProjectSelection(['  a/x  ', 'A/X', '', '   ', 'b/y']),
    ['a/x', 'b/y'],
  );
  assert.deepEqual(normalizeProjectSelection([]), []);
});

test('保存空选集应得到空数组（统计全部语义）', () => {
  const all = ['a/x', 'b/y'];
  let selected = all;
  for (const path of [...selected]) {
    selected = toggleProject(selected, path);
  }
  assert.deepEqual(normalizeProjectSelection(selected), [], '全不勾 = 空列表 = 统计全部');
});
