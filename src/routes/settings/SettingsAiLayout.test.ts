import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('AI 设置中的 API 密钥输入应支持显示与隐藏切换', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /let showApiKey = false;/);
  assert.match(source, /\{#if showApiKey\}/);
  assert.match(source, /type="text"/);
  assert.match(source, /type="password"/);
  assert.match(source, /settingsAI\.hideApiKey/);
  assert.match(source, /settingsAI\.showApiKey/);
});

test('联网、语义记忆与长期记忆开关应暴露本地化 switch 语义', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.equal((source.match(/role="switch"/g) || []).length, 3);
  assert.match(source, /aria-label=\{t\('settingsAI\.webAccess\.title'\)\}/);
  assert.match(source, /aria-label=\{t\('settingsAI\.semanticMemory\.title'\)\}/);
  assert.match(source, /aria-label=\{t\('settingsAI\.assistantMemory\.enabled'\)\}/);
  assert.equal((source.match(/aria-checked=\{/g) || []).length, 3);
});

test('日报导出目录应从 AI 设置移到存储设置', async () => {
  const aiSource = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );
  const storageSource = await readFile(
    new URL('./components/SettingsStorage.svelte', import.meta.url),
    'utf8'
  );

  assert.doesNotMatch(aiSource, /日报 Markdown 导出目录/);
  assert.match(storageSource, /settingsStorage\.exportDir/);
  assert.match(storageSource, /pickDailyReportExportDir/);
});

test('模型选择使用 select 下拉并支持手动输入切换', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /invoke<string\[]>\('fetch_models'/);
  assert.match(source, /refreshModels/);
  assert.match(source, /fetchedModels/);
  assert.match(source, /<select/);
  assert.match(source, /settingsAI\.manualModel/);
  assert.match(source, /settingsAI\.refreshModels/);
  assert.match(source, /let showManualInput = false;/);
});

test('刷新模型列表后应给出反馈，且仅在模型为空时才默认回填首项', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /!config\.text_model\.model\?\.trim\(\)/);
  assert.match(source, /settingsAI\.loadedModels/);
});

test('AI 设置应提供超时与思考模式控件', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /settingsAI\.assistantTimeout/);
  assert.match(source, /settingsAI\.reportTimeout/);
  assert.match(source, /settingsAI\.thinkingMode/);
  assert.match(source, /generation_capabilities/);
  assert.match(source, /enable_thinking/);
});

test('已获取模型数量通过 modelsLoaded 变量追踪', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /let modelsLoaded = 0;/);
  assert.match(source, /modelsLoaded = fetchedModels\.length/);
  assert.match(source, /modelsLoaded > 0/);
});

test('select 列表应渲染所有已获取模型并提供手动输入选项', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /#each fetchedModels as model \(model\)/);
  assert.match(source, /__manual__/);
  assert.doesNotMatch(source, /MANUAL_MODEL_VALUE/);
});

test('内置模型指示应在编译期注入后常驻显示并按使用状态切换文案', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /invoke<boolean>\('has_builtin_model'\)/);
  // 指示卡常驻：只要编译期内置了模型就显示，不再要求「正在使用出厂默认配置」
  assert.match(source, /\{#if hasBuiltinModel\}/);
  // 指示卡位于模型分区顶部（提供商网格之前），确保用户一进 AI 设置就能看到
  const hintIndex = source.indexOf("t('settingsAI.builtinModelHint')");
  assert.notEqual(hintIndex, -1, '应使用 builtinModelHint 文案');
  assert.ok(
    hintIndex < source.indexOf("t('settingsAI.provider')"),
    '内置模型指示卡应位于提供商网格之前'
  );
  // 双状态文案：正在使用内置（出厂默认）/ 已配置自定义模型
  assert.match(source, /settingsAI\.builtinModelHint/);
  assert.match(source, /settingsAI\.builtinAvailableHint/);
});

test('选择内置模型供应商时应固定出厂形状并默认 AI 增强', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /function selectBuiltinProvider\(\)/);
  assert.match(source, /config\.text_model\.provider = 'builtin'/);
  assert.match(source, /config\.ai_mode = 'summary';/);
  // 内置选中时隐藏连接字段（出厂默认形状对用户不可见），只留说明与测试入口
  assert.match(source, /\{#if usingBuiltinModel\}/);
  assert.match(source, /\{:else\}/);
  // 网格高亮：builtin 卡片在出厂默认时选中，ollama 卡片此时不选中
  assert.match(source, /provider\.id === 'builtin'\s*\?\s*usingBuiltinModel/);
});

test('内置模型指示文案应覆盖全部四种语言', async () => {
  for (const localeName of ['zh-CN', 'zh-TW', 'en', 'ar']) {
    const localeModule = await import(`../../lib/i18n/locales/${localeName}.ts`);
    const messages = localeModule.default;
    assert.ok(messages.settingsAI.builtinModelHint, `${localeName} 缺少 settingsAI.builtinModelHint`);
    assert.ok(messages.settingsAI.builtinAvailableHint, `${localeName} 缺少 settingsAI.builtinAvailableHint`);
  }
});
