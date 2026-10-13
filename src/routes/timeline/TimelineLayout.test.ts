import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('时间线应渲染编辑部轨道布局与重点卡片容器', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /timeline-editorial-board[\s\S]*timeline-summary-strip/);
  assert.match(source, /timeline-editorial-shell/);
  assert.match(source, /timeline-rail/);
  assert.match(source, /timeline-entry-card-featured/);
  assert.match(source, /timeline-entry-card-compact/);
});

test('时间线应通过显式函数判断重点卡片并读取缩略图', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /function selectFeaturedActivityIds/);
  assert.match(source, /featuredActivityIds = new Set/);
  assert.match(source, /function getTimelineThumbnail/);
  assert.match(source, /getPreferredTimelineAppName/);
  assert.match(source, /shouldPreferTimelineFallbackIcon/);
});

test('时间线应用图标应使用统一安全区并在原生图片失败时回退', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /let failedTimelineIconKeys = new Set<string>\(\)/);
  assert.match(source, /function handleTimelineIconError\(activity\)/);
  assert.match(source, /onerror=\{\(\) => handleTimelineIconError\(activity\)\}/);
  assert.match(source, /timeline-app-icon-has-image/);
  assert.match(
    source,
    /\.timeline-app-icon-has-image\s*\{[^}]*background:\s*transparent;[^}]*box-shadow:\s*none;/
  );
  assert.match(
    source,
    /\.timeline-app-icon-image\s*\{[^}]*width:\s*100%;[^}]*height:\s*100%;[^}]*object-fit:\s*contain;/
  );
  assert.match(
    source,
    /:global\(\.dark\) \.timeline-app-icon-has-image\s*\{[^}]*background:\s*transparent;/
  );
  assert.doesNotMatch(source, /\.timeline-app-icon-image\s*\{[^}]*1\.9rem/);
});

test('时间线仅应在存在有效图片源时使用图片态底板，加载失败后回退普通底板', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const markup = source.slice(source.indexOf('<!-- 时间线列表 -->'), source.indexOf('<style>'));
  const iconResolver = source.slice(
    source.indexOf('function getTimelineIconSrc(activity)'),
    source.indexOf('function handleTimelineIconError(activity)'),
  );

  assert.equal(
    (markup.match(/class:timeline-app-icon-has-image=\{Boolean\(getTimelineIconSrc\(activity\)\)\}/g) || []).length,
    2,
    '重点卡片与紧凑卡片都应按图片源条件添加图片态 class',
  );
  assert.equal(
    (markup.match(/class:timeline-app-icon-has-image=\{Boolean\(getTimelineIconSrc\(selectedActivity\)\)\}/g) || []).length,
    1,
    '详情图标应按图片源条件添加图片态 class',
  );
  assert.doesNotMatch(
    markup,
    /class="[^"]*timeline-app-icon-has-image[^"]*"/,
    '图片态 class 不得静态常驻',
  );
  assert.match(
    iconResolver,
    /if \(failedTimelineIconKeys\.has\(iconKey\)\) \{\s*return null;\s*\}/,
    '图片加载失败后应返回空源并恢复普通图标底板',
  );
});

test('时间线取得有效原生图标后应优先于字母兜底', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const iconResolver = source.slice(
    source.indexOf('function getTimelineIconSrc(activity)'),
    source.indexOf('function handleTimelineIconError(activity)'),
  );

  const nativeIconBranch = iconResolver.indexOf('hasUsableTimelineNativeIcon(base64)');
  const fallbackBranch = iconResolver.indexOf('shouldPreferTimelineFallbackIcon(activity)');

  assert.notEqual(nativeIconBranch, -1);
  assert.notEqual(fallbackBranch, -1);
  assert.ok(nativeIconBranch < fallbackBranch, '有效原生图标必须先于字母兜底返回');
});

test('时间线结构容器与普通控件应使用统一圆角令牌', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const expected = [
    ['timeline-action-confirm-dialog', 'var\\(--radius-lg\\)'],
    ['timeline-entry-card', 'var\\(--radius-md\\)'],
    ['timeline-featured-image', 'var\\(--radius-md\\)'],
    ['timeline-app-icon', 'var\\(--radius-md\\)'],
    ['timeline-load-more-btn', 'var\\(--radius-md\\)'],
    ['timeline-detail-drawer', 'var\\(--radius-lg\\)'],
    ['timeline-detail-preview-frame', 'var\\(--radius-md\\)'],
    ['timeline-category-trigger', 'var\\(--radius-md\\)'],
    ['timeline-category-popover', 'var\\(--radius-md\\)'],
    ['timeline-category-option', 'var\\(--radius-sm\\)'],
  ];

  for (const [className, radius] of expected) {
    assert.match(
      source,
      new RegExp(`\\.${className}\\s*\\{[^}]*border-radius:\\s*${radius};`),
      className,
    );
  }

  assert.doesNotMatch(source, /border-radius:\s*(?:1(?:\.\d+)?rem|0\.8[5-9]rem|0\.9rem)/);
});

test('时间线重点卡片应使用横向标题区与胶囊分类，避免标题和分类互相挤压', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /timeline-entry-meta-featured/);
  assert.match(source, /timeline-entry-heading-featured/);
  assert.match(source, /timeline-entry-category-pill/);
});

test('时间线紧凑卡片应显式定义信息区与标题区的排版区域，避免标题挤占应用信息宽度', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /timeline-entry-card-compact-grid/);
  assert.match(source, /timeline-entry-app-compact/);
  assert.match(source, /timeline-entry-tail-compact/);
});

test('640px 以下时间线应取消独立轨道列，把完整宽度留给活动卡片', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const mobileSource = source.slice(source.indexOf('@media (max-width: 640px)'));

  assert.match(mobileSource, /\.timeline-editorial-shell\s*\{[\s\S]*--timeline-anchor-width:\s*0/);
  assert.match(mobileSource, /\.timeline-rail\s*\{[\s\S]*display:\s*none/);
  assert.match(mobileSource, /\.timeline-entry\s*\{[\s\S]*grid-template-columns:\s*minmax\(0,\s*1fr\)/);
  assert.match(mobileSource, /\.timeline-entry-anchor\s*\{[\s\S]*min-height:\s*0/);
  assert.match(mobileSource, /\.timeline-entry-marker\s*\{[\s\S]*display:\s*none/);
  assert.doesNotMatch(mobileSource, /padding-inline-start:\s*calc\(0\.85rem \+ var\(--timeline-anchor-width\)\)/);
});

test('时间线详情打开时应先显示已有缩略图，并并行请求活动详情与高清图', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /thumbnail:\s*getTimelineThumbnail\(activity\)/);
  assert.match(source, /const freshActivityPromise =/);
  assert.match(source, /const fullImagePromise =/);
  assert.match(source, /Promise\.all\(\[freshActivityPromise,\s*fullImagePromise/);
});

test('活动详情存在缓存缩略图时应优先显示图片，高清图加载状态不得使用完整占位遮挡', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const previewStart = source.indexOf('class="timeline-detail-preview-frame"');
  const previewEnd = source.indexOf('</section>', previewStart);
  const previewSource = source.slice(previewStart, previewEnd);
  const thumbnailBranchIndex = previewSource.indexOf('{#if selectedActivity.thumbnail}');
  const loadingBranchIndex = previewSource.indexOf('{:else if selectedActivity.thumbnailLoading}');

  assert.ok(previewStart >= 0 && previewEnd > previewStart, '应能定位活动详情截图预览区域');
  assert.ok(thumbnailBranchIndex >= 0, '缓存缩略图应作为截图预览的首个条件分支');
  assert.ok(loadingBranchIndex > thumbnailBranchIndex, '仅在没有缓存缩略图时显示完整加载占位');
  assert.match(previewSource, /selectedActivity\.thumbnailLoading[\s\S]*timeline-detail-preview-loading-indicator/);
  assert.doesNotMatch(
    previewSource.slice(thumbnailBranchIndex, loadingBranchIndex),
    /timeline-detail-preview-state/,
    '高清图加载期间不能用完整状态层遮住已有缩略图'
  );
});

test('时间线首屏重点卡片图片应在列表加载阶段预热，减少第一页占位延迟', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /async function preloadTimelineLeadThumbnails/);
  assert.match(source, /await preloadTimelineLeadThumbnails\(preparedActivities\)/);
});

test('时间线实时更新收到新截图后应主动预热缩略图，避免沿用旧展示', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /listen<unknown>\('screenshot-taken'/);
  assert.match(source, /if \(!isTimelineActivity\(event\.payload\)\)/);
  assert.match(source, /if \(newActivity\.screenshot_path\) \{\s*loadThumbnail\(newActivity\.screenshot_path\)/);
});

test('时间线工具栏图标应统一放大且不改变按钮容器', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const toolbarStart = source.indexOf('<div class="page-toolbar">');
  const toolbarEnd = source.indexOf('{#if loading}', toolbarStart);
  const toolbarSource = source.slice(toolbarStart, toolbarEnd);

  assert.ok(toolbarStart >= 0 && toolbarEnd > toolbarStart);
  // MD 迁移后工具栏为 3 个 SMUI 文字/描边按钮（上一天/今天/昨天）+ 3 个 icon-only 裸按钮（删除/刷新/导出）
  assert.equal((toolbarSource.match(/<Button/g) || []).length, 3);
  assert.equal((toolbarSource.match(/class="md-icon-btn"/g) || []).length, 3);
  assert.match(toolbarSource, /<MdIcon name="delete" size=\{20\} \/>/);
  assert.match(toolbarSource, /<MdIcon name="refresh" size=\{20\} \/>/);
  assert.match(toolbarSource, /<MdIcon name="download" size=\{20\} \/>/);
  assert.equal((toolbarSource.match(/aria-label=\{t\(/g) || []).length, 3);
});

test('时间线应使用按钮打开小时摘要右侧抽屉，并保留无障碍状态', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /import HourlySummaryDrawer from '\.\/HourlySummaryDrawer\.svelte'/);
  assert.match(source, /class="page-control-btn timeline-summary-action"/);
  assert.match(source, /aria-haspopup="dialog"/);
  assert.match(source, /aria-expanded=\{showSummaryDrawer\}/);
  assert.match(source, /onclick=\{openSummaryDrawer\}/);
  assert.doesNotMatch(source, /href="#\/timeline\/summary\/\{selectedDate\}"/);
  assert.match(source, /<HourlySummaryDrawer[\s\S]*open=\{showSummaryDrawer\}[\s\S]*date=\{selectedDate\}[\s\S]*summaries=\{hourlySummaries\}/);
});

test('打开小时摘要时应静默刷新，并用请求序号与日期快照防止旧结果覆盖新日期', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /let summaryRefreshRequestId = 0/);
  assert.match(source, /async function refreshHourlySummaries/);
  assert.match(source, /const requestId = \+\+summaryRefreshRequestId/);
  assert.match(source, /const requestDate = selectedDate/);
  assert.match(source, /timelineGateway\.getHourlySummaries\(requestDate\)/);
  assert.doesNotMatch(source, /invoke<unknown>\('get_hourly_summaries'/);
  assert.match(source, /requestId !== summaryRefreshRequestId \|\| requestDate !== selectedDate/);
  assert.match(source, /async function openSummaryDrawer[\s\S]*refreshHourlySummaries\(\)/);
  assert.match(source, /timelineSummary\.refreshFailed/);
});

test('活动详情应改为右侧抽屉，并与小时摘要抽屉互斥且恢复触发按钮焦点', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /let summaryTrigger/);
  assert.match(source, /let detailTrigger/);
  assert.match(source, /bind:this=\{summaryTrigger\}/);
  assert.match(source, /viewActivity\(activity, event\.currentTarget\)/);
  assert.match(source, /class="timeline-detail-overlay[^"]*justify-end/);
  assert.match(source, /import \{ trapFocus \} from '\$lib\/utils\/focusTrap\.ts'/);
  assert.match(source, /<aside\s+class="timeline-detail-drawer"\s+use:trapFocus/);
  assert.match(source, /role="dialog"\s+aria-modal="true"\s+aria-labelledby="timeline-detail-title"/);
  assert.match(source, /async function closeDetail[\s\S]*detailTrigger\?\.focus\(\)/);
  assert.match(source, /async function closeSummaryDrawer[\s\S]*summaryTrigger\?\.focus\(\)/);
  assert.match(source, /async function openSummaryDrawer[\s\S]*closeDetail\(false\)/);
  assert.match(source, /async function viewActivity[\s\S]*closeSummaryDrawer\(false\)/);
});

test('时间线与详情抽屉的深色边界应采用低对比层级，不保留亮白轨道和顶部高光', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /:global\(\.dark\) \.timeline-editorial-board[\s\S]*border-color:\s*rgba\(71, 85, 105, 0\.5\)/);
  assert.match(source, /:global\(\.dark\) \.timeline-detail-drawer[\s\S]*border-color:\s*rgba\(48, 54, 61,/);
  assert.doesNotMatch(source, /rgba\(248, 250, 252, 0\.84\)/);
  assert.doesNotMatch(source, /inset 0 1px 0 rgba\(255, 255, 255, 0\.04\)/);
});


test('活动详情抽屉关闭按钮应使用存在的多语言键名', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /aria-label=\{t\('window\.close'\)\}/);
  assert.doesNotMatch(source, /t\('common\.close'\)/);
});

test('时间线主请求的错误与加载状态只能由当前日期请求提交', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /const requestId = \+\+loadTimelineRequestId;\s*const requestDate = selectedDate;/);
  assert.match(source, /timelineGateway\.getPage\(\{[\s\S]*date: requestDate,[\s\S]*limit: PAGE_SIZE,[\s\S]*offset: 0/);
  assert.match(source, /timelineGateway\.getHourlySummaries\(requestDate\)/);
  assert.doesNotMatch(source, /invoke<unknown>\('get_timeline'/);
  assert.doesNotMatch(source, /invoke<unknown>\('get_hourly_summaries'/);
  assert.match(
    source,
    /catch \(e\) \{\s*if \(requestId !== loadTimelineRequestId \|\| requestDate !== selectedDate\) return;\s*error =/
  );
  assert.match(
    source,
    /finally \{\s*if \(requestId === loadTimelineRequestId && requestDate === selectedDate\) \{\s*loading = false;\s*\}\s*\}/
  );
});

test('加载更多应使用日期与偏移快照，并丢弃日期切换后的旧分页响应', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /let loadMoreRequestId = 0/);
  assert.match(source, /loadMoreRequestId \+= 1;\s*loadingMore = false;/);
  assert.match(source, /const requestId = \+\+loadMoreRequestId;\s*const requestDate = selectedDate;\s*const requestOffset = offset;/);
  assert.match(source, /date: requestDate,\s*limit: PAGE_SIZE,\s*offset: requestOffset/);
  assert.match(source, /if \(requestId !== loadMoreRequestId \|\| requestDate !== selectedDate\) return;/);
  assert.match(source, /offset = requestOffset \+ moreActivities\.length/);
  assert.match(
    source,
    /finally \{\s*if \(requestId === loadMoreRequestId && requestDate === selectedDate\) \{\s*loadingMore = false;\s*\}\s*\}/
  );
});

test('活动详情应按阅读顺序组织截图、活动信息与记录设置，并避免多层卡片', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  const bodyIndex = source.indexOf('class="timeline-detail-body"');
  const heroIndex = source.indexOf('class="timeline-detail-hero"', bodyIndex);
  const previewIndex = source.indexOf('class="timeline-detail-preview"', bodyIndex);
  const metaIndex = source.indexOf('class="timeline-detail-meta"', bodyIndex);
  const settingsIndex = source.indexOf('class="timeline-detail-settings"', bodyIndex);

  assert.ok(bodyIndex >= 0, '应提供详情主体语义容器');
  assert.ok(heroIndex > bodyIndex, '应用身份与时间应位于详情主体顶部');
  assert.ok(previewIndex > heroIndex, '截图应紧随应用身份信息');
  assert.ok(metaIndex > previewIndex, '标题和网址应位于截图之后');
  assert.ok(settingsIndex > metaIndex, '分类和记录策略应收拢到详情底部');
  assert.match(source, /\.timeline-detail-settings\s*\{[\s\S]*border-top:\s*1px solid rgba\(148, 163, 184, 0\.2\)/);
  assert.match(source, /\.timeline-detail-preview-frame\s*\{[\s\S]*background:\s*rgba\(148, 163, 184, 0\.1\)/);
  assert.doesNotMatch(source, /\.timeline-detail-settings\s*\{[^}]*box-shadow:/);
  assert.match(source, /:global\(\.dark\) \.timeline-detail-preview-frame[\s\S]*background:\s*rgba\(48, 54, 61, 0\.38\)/);
  assert.match(source, /@media \(max-width: 640px\)[\s\S]*\.timeline-detail-meta-row\s*\{[\s\S]*grid-template-columns:\s*1fr/);
});

test('640px 及以下活动详情抽屉应全屏展示并移除圆角', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');
  const mobileStart = source.indexOf('@media (max-width: 640px)');
  const mobileSource = source.slice(mobileStart);

  assert.ok(mobileStart >= 0, '应定义 640px 详情抽屉响应式规则');
  assert.match(
    mobileSource,
    /\.timeline-detail-drawer\s*\{[\s\S]*?width:\s*100%;[\s\S]*?height:\s*100vh;[\s\S]*?border-radius:\s*0;/
  );
});

test('时间线工具栏应有日报同款日期快捷按钮（上一天/今天/昨天 + 选中高亮）', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  // 三按钮复用日报既有键，迁移后为 SMUI outlined Button（选中态用主色容器高亮 class 表达）
  assert.match(source, /<Button[\s\S]*?t\('report\.previousDay'\)/);
  assert.match(source, /selectedDate === getLocalDateString\(\) \? ' timeline-quick-date-active' : ''/);
  assert.match(source, /selectedDate === getTimelineYesterdayDateString\(\) \? ' timeline-quick-date-active' : ''/);
  assert.match(source, /t\('report\.today'\)/);
  assert.match(source, /t\('report\.yesterday'\)/);
  // 上一天为相对操作：连点可一直往前翻（selectedDate 前移一天）
  assert.match(source, /selectedDate = shiftIsoDate\(selectedDate, -1\)/);
  // 三按钮位于日期选择器左侧（与日报同序）
  const toolbarStart = source.indexOf('<div class="page-toolbar">');
  const picker = source.indexOf('<LocalizedDatePicker', toolbarStart);
  const prevBtn = source.indexOf("t('report.previousDay')", toolbarStart);
  assert.ok(toolbarStart > 0 && prevBtn > toolbarStart && prevBtn < picker, '快捷按钮应在日期选择器之前');
  // 日期平移复用日报的纯函数模块（昨天 = 今天前移一天）
  assert.match(source, /import \{ shiftIsoDate \} from '\.\.\/report\/reportDateNavigation\.ts'/);
  assert.match(source, /function getTimelineYesterdayDateString/);
});
