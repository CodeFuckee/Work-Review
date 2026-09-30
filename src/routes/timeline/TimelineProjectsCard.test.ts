import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

// 被测功能：时间线顶部「今日项目」归因卡（TimelineProjectsCard.svelte）——
// 纯 Plane 项目清单：自动项来自当日归因聚合，手动项从 Plane 项目列表下拉添加；
// 删除 = 当日屏蔽（removed 标记，归因再算不复活、次日重置）；修改 = 替换为
// 另一项目；未配置 Plane 整卡隐藏；不联动时间线行徽标。

test('今日项目卡应挂载在时间线页顶部（工具栏与统计条之间）', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /import TimelineProjectsCard from '\.\/TimelineProjectsCard\.svelte'/);
  // 挂载点在 page-toolbar 之后、timeline-editorial-board 之前
  const toolbar = source.indexOf('<div class="page-toolbar">');
  const mount = source.indexOf('<TimelineProjectsCard');
  const board = source.indexOf('timeline-editorial-board');
  assert.ok(toolbar > 0 && mount > toolbar && mount < board, '卡片应位于工具栏与看板之间');
  // 传参：选中日期（按日隔离）与当日归因（auto 聚合源）
  assert.match(source, /<TimelineProjectsCard[\s\S]*?date=\{selectedDate\}/);
  assert.match(source, /<TimelineProjectsCard[\s\S]*?attributions=\{planeAttributions\}/);
});

test('今日项目卡应具备 chips 列表与添加/编辑/删除交互结构', async () => {
  const source = await readFile(new URL('./TimelineProjectsCard.svelte', import.meta.url), 'utf8');

  // 项目 chip 列表 + 每项编辑/删除按钮
  assert.match(source, /timeline-projects-card/);
  assert.match(source, /timeline-projects-chip/);
  assert.match(source, /t\('timeline\.projects\.editLabel'\)/);
  assert.match(source, /t\('timeline\.projects\.removeLabel'\)/);
  // 添加按钮 + Plane 项目下拉（编辑复用同款下拉）
  assert.match(source, /t\('timeline\.projects\.addLabel'\)/);
  assert.match(source, /list_plane_projects/);
  assert.match(source, /set_daily_project_adjustment/);
  assert.match(source, /get_daily_project_adjustments/);
  // 最终清单 = 归因聚合(auto) - removed ∪ added
  assert.match(source, /removed/);
  assert.match(source, /added/);
  // 空态文案
  assert.match(source, /t\('timeline\.projects\.empty'\)/);
  // 未配置 Plane（下拉源空且无归因/手动项）时整卡隐藏（cardVisible 守卫）
  assert.match(source, /planeProjects\.length > 0 \|\| todayProjects\.length > 0/);
});

test('今日项目卡文案应在四语言词典中齐备且非空', async () => {
  const keys = ['title', 'addLabel', 'editLabel', 'removeLabel', 'empty'] as const;
  for (const localeName of ['zh-CN', 'en', 'zh-TW', 'ar']) {
    const dictionary = (await import(`./../../lib/i18n/locales/${localeName}.ts`)).default;
    const projects = dictionary.timeline?.projects;
    assert.ok(projects, `${localeName} 应有 timeline.projects 组`);
    for (const key of keys) {
      const value = (projects as Record<string, unknown>)[key];
      assert.equal(
        typeof value === 'string' && value.length > 0,
        true,
        `${localeName} 的 timeline.projects.${key} 应为非空字符串`,
      );
    }
  }
});

test('今日项目卡应有手动触发归因按钮（recompute 事件 + spinner + 失败提示）', async () => {
  const source = await readFile(new URL('./TimelineProjectsCard.svelte', import.meta.url), 'utf8');

  // 卡头部图标按钮：悬停文案 + 计算中 spinner/禁用
  assert.match(source, /t\('timeline\.projects\.recomputeTitle'\)/);
  assert.match(source, /recomputing/);
  assert.match(source, /disabled=\{recomputing\}/);
  assert.match(source, /dispatch\('recompute'\)/);
  // 失败提示行（手动触发必须可见结果，区别于自动链路静默）
  assert.match(source, /t\('timeline\.projects\.recomputeFailed'\)/);
  // 失败提示带自动消失（几秒后清空）
  assert.match(source, /recomputeError/);
});

test('手动归因应由 Timeline 监听 recompute 事件重算当日归因', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /on:recompute=\{manualRecomputeAttributions\}/);
  assert.match(source, /async function manualRecomputeAttributions/);
  // 手动路径复用同一归因加载函数并带成败返回值驱动 spinner
  assert.match(source, /recomputeAttributionsLoading/);
  assert.match(source, /<TimelineProjectsCard[\s\S]*?recomputing=\{recomputeAttributionsLoading\}/);
  assert.match(source, /recomputeError=\{recomputeAttributionsError\}/);
});
