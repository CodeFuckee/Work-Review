import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const COMMIT_DETAIL_KEYS = [
  'commitStats',
  'commitMessage',
  'commitBranch',
  'commitHash',
  'commitAuthor',
] as const;

const COMMIT_FILES_KEYS = [
  'commitFiles',
  'commitFilesEmpty',
  'commitFilesFailed',
  'commitFileAdded',
  'commitFileModified',
  'commitFileDeleted',
  'commitFileRenamed',
  'commitDiffTruncated',
  'openInGitlab',
] as const;

test('GitLab 提交行应可点击打开详情（按钮化 + viewCommit + 右侧箭头示意）', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const start = source.indexOf("row.kind === 'commit'");
  const end = source.indexOf("row.kind === 'fileEvent'");
  assert.ok(start > 0 && end > start, 'commit 行分支应存在');
  const section = source.slice(start, end);

  assert.match(section, /<button[\s\S]*?class="timeline-entry timeline-entry-commit"/);
  assert.match(section, /viewCommit\(row\.commit, event\.currentTarget\)/);
  assert.match(section, /timeline-entry-arrow/);
});

test('提交详情抽屉应与应用活动抽屉同壳：遮罩/焦点陷阱/关闭按钮/aria', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const start = source.indexOf('{#if selectedCommit}');
  assert.ok(start > 0, '提交详情抽屉应存在');
  const section = source.slice(start, start + 6000);

  assert.match(section, /timeline-detail-overlay/);
  assert.match(section, /use:trapFocus/);
  assert.match(section, /role="dialog"/);
  assert.match(section, /aria-modal="true"/);
  assert.match(section, /bind:this=\{commitDetailCloseButton\}/);
  assert.match(section, /closeCommitDetail\(\)/);
  // 标题 = 项目名，副标题 = 「GitLab 提交」
  assert.match(section, /selectedCommit\.project/);
  assert.match(section, /overview\.gitlabTitle/);
});

test('提交详情抽屉应展示提交特有字段：时间/变更统计/提交信息/分支/哈希/作者', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const start = source.indexOf('{#if selectedCommit}');
  const section = source.slice(start, start + 9000);

  assert.match(section, /selectedCommit\.committed_at/);
  assert.match(section, /timeline\.detail\.commitStats/);
  assert.match(section, /timeline\.detail\.commitMessage/);
  assert.match(section, /selectedCommit\.title/);
  assert.match(section, /timeline\.detail\.commitBranch/);
  assert.match(section, /selectedCommit\.branch/);
  assert.match(section, /timeline\.detail\.commitHash/);
  assert.match(section, /selectedCommit\.short_id/);
  assert.match(section, /timeline\.detail\.commitAuthor/);
  assert.match(section, /selectedCommit\.author_name/);
});

test('两类抽屉应互斥打开，关闭时焦点交还给触发行', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  // 打开提交抽屉先关活动抽屉；打开活动抽屉先关提交抽屉
  const viewCommitStart = source.indexOf('async function viewCommit');
  const viewCommitBody = source.slice(
    viewCommitStart,
    source.indexOf('}', source.indexOf('commitDetailCloseButton?.focus()'))
  );
  assert.match(viewCommitBody, /selectedActivity = null/);
  assert.match(viewCommitBody, /commitDetailTrigger = trigger/);

  const viewActivityStart = source.indexOf('async function viewActivity');
  const viewActivityBody = source.slice(viewActivityStart, viewActivityStart + 900);
  assert.match(viewActivityBody, /selectedCommit = null/);

  // 关闭提交抽屉恢复触发行焦点（与应用抽屉 closeDetail 同款约定）
  const closeCommit = source.slice(
    source.indexOf('async function closeCommitDetail'),
    source.indexOf('async function closeCommitDetail') + 500
  );
  assert.match(closeCommit, /restoreFocus/);
  assert.match(closeCommit, /commitDetailTrigger\?\.focus\(\)/);
});

test('提交详情抽屉应包含「修改文件」区块：加载 + 可展开 diff + 状态徽标 + 截断', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const start = source.indexOf('{#if selectedCommit}');
  const section = source.slice(start, start + 18000);

  assert.match(section, /timeline\.detail\.commitFiles/);
  // 每文件可展开（展开状态来自 expandedDiffFiles.has；aria-expanded 反映展开态）
  assert.match(section, /expandedDiffFiles\.has\(/);
  assert.match(section, /aria-expanded=\{expanded\}/);
  // diff 染色行渲染走 parseDiffLines
  assert.match(section, /parseDiffLines\(/);
  // 加载中、失败重试、空态三种状态（失败文案在 script 侧组装）
  assert.match(section, /commitDiffLoading/);
  assert.match(section, /retryCommitDiff/);
  assert.match(source, /loadCommitDiff/);
  assert.match(source, /timeline\.detail\.commitFilesFailed/);
  assert.match(section, /timeline\.detail\.commitFilesEmpty/);
  // 超长 diff 截断提示
  assert.match(section, /timeline\.detail\.commitDiffTruncated/);
});

test('提交详情抽屉头部应有「在 GitLab 打开」外跳按钮', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const start = source.indexOf('{#if selectedCommit}');
  const section = source.slice(start, start + 8000);

  assert.match(section, /timeline\.detail\.openInGitlab/);
  assert.match(section, /commitWebUrl\(selectedCommit\)/);
});

test('打开提交抽屉时应触发 diff 加载，同 sha 会话内命中缓存不重复请求', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  // 会话级缓存 Map（sha → 拉取结果）
  assert.match(source, /commitDiffCache/);
  // viewCommit 打开后拉取 diff
  const viewCommitStart = source.indexOf('async function viewCommit');
  const viewCommitBody = source.slice(viewCommitStart, viewCommitStart + 700);
  assert.match(viewCommitBody, /loadCommitDiff/);
});

test('GitLab 提交行应显示 Plane 归因徽标（提交时间落入归因窗口即 join）', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const start = source.indexOf("row.kind === 'commit'");
  const end = source.indexOf("row.kind === 'fileEvent'");
  assert.ok(start > 0 && end > start, 'commit 行分支应存在');
  const section = source.slice(start, end);

  // 提交行徽标读派生层附加的 row.attribution（归因 join 在 timelineRows 派生层完成，
  // 归因按日现算晚于行渲染到达，行内调函数 Svelte 追踪不到依赖、徽标不会出现）
  assert.match(section, /row\.attribution/);
  assert.match(section, /timeline-plane-badge/);
  // join 统一在派生层：timelineRows 赋值语句应词法引用 planeAttributions
  const derivation = source.slice(source.indexOf('$: timelineRows'), source.indexOf('$: timelineRows') + 400);
  assert.match(derivation, /planeAttributions/);
  assert.match(derivation, /attachAttribution/);
});

test('时间线归因应为双徽标并排：紫色工作项徽标 + 蓝色项目徽标（纯项目名）', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const start = source.indexOf("row.kind === 'commit'");
  const end = source.indexOf("row.kind === 'fileEvent'");
  const section = source.slice(start, end);

  // 项目徽标：独立样式类 + 直接显示项目名称（不带 · 前缀小字，避免与工作项徽标旁重复）
  assert.match(section, /timeline-project-badge/);
  assert.match(section, /\{row\.attribution\.project_name\}/);
  assert.doesNotMatch(section, /· \{row\.attribution\.project_name\}/);
  // 双徽标并存：工作项徽标仍在
  assert.match(section, /timeline-plane-badge/);

  // 全部四类行渲染点统一双徽标（commit/fileEvent/activity-featured/activity-compact）
  const badgeSpans = source.match(/timeline-project-badge"/g) ?? [];
  assert.ok(badgeSpans.length >= 4, `项目徽标应出现在 4 处渲染点，实际 ${badgeSpans.length}`);

  // 蓝色项目徽标样式：亮暗两套配色，圆角沿用 radius 令牌
  const style = source.slice(source.indexOf('.timeline-project-badge {'));
  assert.match(style, /rgb\(37 99 235\)/);
  assert.match(style, /:global\(\.dark\) \.timeline-project-badge/);
  assert.match(style, /var\(--radius-md\)/);
});

test('修改文件与外跳文案应在四语言词典中齐备且非空', async () => {
  const localeNames = ['zh-CN', 'en', 'zh-TW', 'ar'] as const;
  for (const localeName of localeNames) {
    const dictionary = (await import(`./../../lib/i18n/locales/${localeName}.ts`)).default;
    const detail = dictionary.timeline?.detail;
    assert.ok(detail, `${localeName} 应有 timeline.detail 组`);
    for (const key of COMMIT_FILES_KEYS) {
      const value = (detail as Record<string, unknown>)[key];
      assert.equal(
        typeof value === 'string' && value.length > 0,
        true,
        `${localeName} 的 timeline.detail.${key} 应为非空字符串`
      );
    }
  }
});

test('提交详情文案应在四语言词典中齐备且非空', async () => {
  const localeNames = ['zh-CN', 'en', 'zh-TW', 'ar'] as const;
  for (const localeName of localeNames) {
    const dictionary = (await import(`./../../lib/i18n/locales/${localeName}.ts`)).default;
    const detail = dictionary.timeline?.detail;
    assert.ok(detail, `${localeName} 应有 timeline.detail 组`);
    for (const key of COMMIT_DETAIL_KEYS) {
      const value = (detail as Record<string, unknown>)[key];
      assert.equal(
        typeof value === 'string' && value.length > 0,
        true,
        `${localeName} 的 timeline.detail.${key} 应为非空字符串`
      );
    }
  }
});
