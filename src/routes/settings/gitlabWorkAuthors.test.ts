import test from 'node:test';
import assert from 'node:assert/strict';

import { parseWorkAuthorsInput } from './gitlabWorkAuthors.ts';

test('纳入作者解析应支持换行与中英文逗号分隔', () => {
  assert.deepEqual(
    parseWorkAuthorsInput('李四\nlisi@corp.com，王五, zhaoliu@x.com'),
    ['李四', 'lisi@corp.com', '王五', 'zhaoliu@x.com'],
  );
});

test('纳入作者解析应去首尾空白并剔除空条目', () => {
  assert.deepEqual(parseWorkAuthorsInput('  李四  \n\n   \n'), ['李四']);
  assert.deepEqual(parseWorkAuthorsInput(''), []);
  assert.deepEqual(parseWorkAuthorsInput('   ，，\n'), []);
});

test('纳入作者解析应按忽略大小写去重并保留首次写法', () => {
  assert.deepEqual(
    parseWorkAuthorsInput('Lisi@Corp.com\nlisi@corp.com\n李四'),
    ['Lisi@Corp.com', '李四'],
  );
});

test('纳入作者解析应保留单条目且不改写内容', () => {
  assert.deepEqual(parseWorkAuthorsInput('李四'), ['李四']);
  assert.deepEqual(parseWorkAuthorsInput('lisi@corp.com'), ['lisi@corp.com']);
});
