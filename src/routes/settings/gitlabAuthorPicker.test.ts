import test from 'node:test';
import assert from 'node:assert/strict';

import {
  filterAuthorOptions,
  mergeWorkAuthorsWithPicked,
  preselectedAuthorEmails,
  toggleAuthor,
  type GitlabAuthorOption,
} from './gitlabAuthorPicker.ts';

function author(name: string, email: string, commits: number): GitlabAuthorOption {
  return { name, email, commits, is_current_user: false };
}

test('作者过滤应按姓名或邮箱子串匹配且空查询返回全部', () => {
  const authors = [
    author('李四', 'lisi@corp.com', 16),
    author('lisi', 'lisi-other@corp.com', 4),
    author('王五', 'wangwu@corp.com', 30),
  ];
  assert.deepEqual(filterAuthorOptions(authors, ''), authors);
  assert.deepEqual(filterAuthorOptions(authors, 'LISI'), [authors[0], authors[1]], '命中姓名与邮箱、忽略大小写');
  assert.deepEqual(filterAuthorOptions(authors, 'wangwu'), [authors[2]]);
  assert.deepEqual(filterAuthorOptions(authors, '不存在'), []);
});

test('勾选切换应忽略大小写识别已选并保持追加顺序', () => {
  const selected = ['a@corp.com', 'b@corp.com'];
  assert.deepEqual(toggleAuthor(selected, 'c@corp.com'), ['a@corp.com', 'b@corp.com', 'c@corp.com']);
  assert.deepEqual(toggleAuthor(selected, 'B@corp.com'), ['a@corp.com'], '忽略大小写移除，不产生大小写重复');
  assert.deepEqual(toggleAuthor(['b@corp.com'], 'a@corp.com'), ['b@corp.com', 'a@corp.com']);
});

test('保存合并应为追加语义：既有条目保留在前且忽略大小写去重', () => {
  assert.deepEqual(
    mergeWorkAuthorsWithPicked(['李四', 'lisi@corp.com'], ['LISI@corp.com', 'wangwu@corp.com']),
    ['李四', 'lisi@corp.com', 'wangwu@corp.com'],
    '已存在的邮箱（含手输）不重复追加，新邮箱追加到末尾',
  );
  assert.deepEqual(
    mergeWorkAuthorsWithPicked([], [' x@corp.com ', '']),
    ['x@corp.com'],
    '条目去首尾空白、丢弃空项',
  );
  assert.deepEqual(mergeWorkAuthorsWithPicked(['手输姓名'], []), ['手输姓名'], '未勾选时不改动名单');
});

test('预勾选应把已写入名单的邮箱（忽略大小写）标记为已选', () => {
  const authors = [
    author('李四', 'lisi@corp.com', 16),
    author('王五', 'wangwu@corp.com', 30),
  ];
  assert.deepEqual(
    preselectedAuthorEmails(authors, ['LISI@corp.com', '手输姓名']),
    ['lisi@corp.com'],
    '只预勾选命中的邮箱，手输姓名不参与',
  );
  assert.deepEqual(preselectedAuthorEmails(authors, []), []);
});
