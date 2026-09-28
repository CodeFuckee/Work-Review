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
