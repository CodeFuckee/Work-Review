import test from 'node:test';
import assert from 'node:assert/strict';
import { access, readFile } from 'node:fs/promises';

const typescriptStoreUrl = new URL('./categories.ts', import.meta.url);

test('分类 Store 应迁移到 TypeScript 且消费者使用新入口', async () => {
  await access(typescriptStoreUrl);

  const timelineSource = await readFile(
    new URL('../../routes/timeline/Timeline.svelte', import.meta.url),
    'utf8'
  );
  const overviewSource = await readFile(new URL('../../routes/Overview.svelte', import.meta.url), 'utf8');

  assert.match(timelineSource, /stores\/categories\.ts['"]/);
  assert.match(overviewSource, /stores\/categories\.ts['"]/);
});

test('分类 store 应对已知系统分类 key 使用当前语言翻译而不是后端中文名称', async () => {
  const source = await readFile(typescriptStoreUrl, 'utf8');

  assert.match(source, /translateCategoryLabel\(found\.key\)/);
  assert.match(source, /translatedCategoryName !== found\.key/);
  assert.match(source, /name: isKnownSystemCategory\s*\?\s*translatedCategoryName\s*:\s*\(found\.name \|\| translatedCategoryName\)/);
  assert.doesNotMatch(
    source,
    /name: found\.name \|\| translateCategoryLabel\(found\.key\)/,
    '不能直接优先使用后端返回的中文内置分类名，否则英文时间线 chip 会继续显示中文'
  );
});

test('语义分类 store 应对已知语义分类使用当前语言翻译而不是后端中文名称', async () => {
  const source = await readFile(typescriptStoreUrl, 'utf8');

  assert.match(source, /translatedSemanticCategoryName = translateSemanticCategoryLabel\(found\.key\)/);
  assert.match(source, /isKnownSemanticCategory = found\.is_system \|\| translatedSemanticCategoryName !== found\.key/);
  assert.match(
    source,
    /return isKnownSemanticCategory\s*\?\s*translatedSemanticCategoryName\s*:\s*\(found\.name \|\| translatedSemanticCategoryName\)/
  );
  assert.doesNotMatch(
    source,
    /return found\.name \|\| translateSemanticCategoryLabel\(found\.key\)/,
    '不能直接优先使用后端返回的中文语义分类名，否则英文网站分类会继续显示中文'
  );
});

test('分类 Store 应保留刷新容错、同步快照读取和颜色回退行为', async () => {
  const source = await readFile(typescriptStoreUrl, 'utf8');

  assert.match(source, /try\s*{[\s\S]*invoke<CategoryInfo\[]>\('get_categories'\)[\s\S]*}\s*catch\s*\(e\)/);
  assert.match(source, /try\s*{[\s\S]*invoke<SemanticCategoryInfo\[]>\('get_semantic_categories'\)[\s\S]*}\s*catch\s*\(e\)/);
  // 同步快照读取统一走 snapshotStore 助手：两个 store ×（条目查询 + 全量列表）= 4 处
  assert.equal((source.match(/snapshotStore\(subscribe\)/g) || []).length, 4);
  assert.match(source, /function snapshotStore<T>\(subscribe: Readable<T>\['subscribe'\]\): T/);
  assert.match(
    source,
    /if \(!hex \|\| !hex\.startsWith\('#'\) \|\| hex\.length < 7\) return `rgba\(100, 116, 139, \$\{alpha\}\)`;/
  );
});
