import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

async function readSection(startMarker: string, endMarker: string): Promise<string> {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const start = source.indexOf(startMarker);
  const end = source.indexOf(endMarker, start + 1);
  assert.ok(start > 0, `${startMarker} 应存在`);
  assert.ok(end > start, `${endMarker} 应存在于 ${startMarker} 之后`);
  return source.slice(start, end);
}

const readCommitSection = () =>
  readSection("row.kind === 'commit'", "row.kind === 'fileEvent'");
const readFileEventSection = () =>
  readSection("row.kind === 'fileEvent'", "row.kind === 'planeItem'");
const readPlaneSection = () =>
  readSection("row.kind === 'planeItem'", '{@const activity = row.activity}');

test('GitLab 提交行应复用应用活动行的紧凑栅格结构（图标+粗体名+徽标 / 标题 / 右侧统计）', async () => {
  const section = await readCommitSection();

  // 与活动行同款卡片栅格：'app app' / 'title meta'
  assert.match(section, /timeline-entry-card-compact timeline-entry-card-compact-grid/);
  // 项目名占用应用名槽位（粗体 0.98rem，非等宽小字）
  assert.match(section, /timeline-entry-app-name/);
  assert.doesNotMatch(section, /timeline-entry-app-name-commit/);
  // 提交信息占用窗口标题槽位
  assert.match(section, /timeline-entry-title-compact/);
  // 统计占用时长/箭头的右侧槽位
  assert.match(section, /timeline-entry-tail-compact/);
});

test('GitLab 提交行图标应为应用图标方块 + GitLab 提交图形，徽标复用「GitLab 提交」文案', async () => {
  const section = await readCommitSection();

  // 与活动行同款 2.75rem 图标方块（GitLab 橙色调），内含 git 提交 SVG 图形
  assert.match(section, /timeline-app-icon timeline-app-icon-gitlab/);
  assert.match(section, /<circle/);
  assert.match(section, /<line/);
  // 分类徽标位显示「GitLab 提交」（复用 overview.gitlabTitle，不新增键）
  assert.match(section, /overview\.gitlabTitle/);
  assert.match(section, /timeline-entry-category-pill/);
});

test('项目名与提交信息应带悬停提示，长文本截断时可看全称', async () => {
  const section = await readCommitSection();

  assert.match(section, /title=\{row\.commit\.project\}/);
  assert.match(section, /title=\{row\.commit\.title\}/);
});

test('文件事件行应同款紧凑栅格：文件名加粗 + 事件类型徽标 + 路径次要行 + 大小统计', async () => {
  const section = await readFileEventSection();

  assert.match(section, /timeline-entry-card-compact timeline-entry-card-compact-grid/);
  assert.match(section, /timeline-app-icon timeline-app-icon-fileevent/);
  assert.match(section, /timeline-entry-app-name/);
  assert.match(section, /timeline-entry-category-pill/);
  assert.match(section, /fileEventBadgeKey\(row\.event\.event_type\)/);
  assert.match(section, /title=\{row\.event\.path\}/);
  assert.match(section, /timeline-entry-tail-compact/);
});

test('Plane 工作项行应同款紧凑栅格：项目名加粗 + Plane 徽标 + 工作项名次要行 + 状态', async () => {
  const section = await readPlaneSection();

  assert.match(section, /timeline-entry-card-compact timeline-entry-card-compact-grid/);
  assert.match(section, /timeline-app-icon timeline-app-icon-plane/);
  assert.match(section, /timeline-entry-app-name/);
  assert.match(section, /timeline\.planeItemBadge/);
  assert.match(section, /timeline-entry-category-pill/);
  assert.match(section, /timeline-entry-title-compact/);
  assert.match(section, /timeline-entry-tail-compact/);
});

test('旧扁平提交卡样式应整体移除，仅保留仍被引用的公共样式', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  // 旧三列扁平卡片样式（含暗色变体）与迷你徽标、等宽项目名不再出现
  assert.doesNotMatch(source, /timeline-entry-card-commit/);
  assert.doesNotMatch(source, /timeline-entry-app-name-commit/);
  assert.doesNotMatch(source, /timeline-commit-badge/);
  // 公共样式仍保留：橙色时间锚点、统计字号
  assert.match(source, /timeline-entry-marker-commit/);
  assert.match(source, /timeline-commit-stats/);
});

test('三类外部行的新图标方块均应有暗色主题配色', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /:global\(\.dark\) \.timeline-app-icon-gitlab/);
  assert.match(source, /:global\(\.dark\) \.timeline-app-icon-fileevent/);
  assert.match(source, /:global\(\.dark\) \.timeline-app-icon-plane/);
});
