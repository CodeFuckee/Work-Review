import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('时间线详情抽屉应展示 OCR 文本区块，置于截图预览之后', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  // 区块标题与空态占位走 i18n
  assert.match(source, /timeline\.detail\.ocr\b/);
  assert.match(source, /timeline\.detail\.ocrEmpty/);
  // 展示数据来自选中活动的 ocr_text，经 getOcrDisplayText 归一
  assert.match(source, /getOcrDisplayText\(selectedActivity\.ocr_text\)/);
  // OCR 区块必须出现在截图预览区块之后（同屏幕内容语义分组）
  const previewIndex = source.indexOf('timeline-detail-preview"');
  const ocrIndex = source.indexOf('timeline-detail-ocr"');
  assert.ok(previewIndex > 0, '截图预览区块应存在');
  assert.ok(ocrIndex > previewIndex, 'OCR 区块应在截图预览之后');
});

test('OCR 文本块应可滚动，避免超长屏幕内容撑爆抽屉', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  assert.match(source, /\.timeline-detail-ocr-text\s*\{[\s\S]*?max-height:[^;]+;/);
  assert.match(source, /\.timeline-detail-ocr-text\s*\{[\s\S]*?overflow-y:\s*auto;/);
  // 长字符串换行：pre-wrap 保留 OCR 换行结构，break-word 防止长串横向溢出
  assert.match(source, /\.timeline-detail-ocr-text\s*\{[\s\S]*?white-space:\s*pre-wrap;/);
  assert.match(source, /\.timeline-detail-ocr-text\s*\{[\s\S]*?overflow-wrap|word-break/);
  // 暗色主题需有对应配色
  assert.match(source, /:global\(\.dark\) \.timeline-detail-ocr-text/);
});

test('空 OCR 占位复用详情抽屉既有的空态样式', async () => {
  const source = await readFile(new URL('./Timeline.svelte', import.meta.url), 'utf8');

  const ocrIndex = source.indexOf('timeline-detail-ocr"');
  const sectionEnd = source.indexOf('</section>', ocrIndex);
  const ocrSection = source.slice(ocrIndex, sectionEnd);
  assert.match(ocrSection, /timeline\.detail\.ocrEmpty/);
  assert.match(ocrSection, /timeline-detail-preview-state/);
});
