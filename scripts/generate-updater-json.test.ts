import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  assertPlatformVersionConsistency,
  buildUpdaterManifest,
} from './generate-updater-json.mjs';

// 测试夹具：临时目录里造一份产物 + 签名 + notes
function makeFixture({ withSignature = true } = {}) {
  const dir = mkdtempSync(join(tmpdir(), 'updater-json-test-'));
  const artifact = join(dir, 'Yue-Review_1.1.51_x64-setup.exe');
  writeFileSync(artifact, 'installer-bytes');
  if (withSignature) {
    writeFileSync(`${artifact}.sig`, 'signature-of-1.1.51\n');
  }
  const notes = join(dir, 'notes.md');
  writeFileSync(notes, '# release notes');
  return { dir, artifact, notes };
}

test('buildUpdaterManifest 应构建含版本/平台/签名/notes 的清单并归一化版本号', () => {
  const { artifact } = makeFixture();
  const manifest = buildUpdaterManifest({
    version: 'v1.1.51',
    urlBase: 'https://example.test/api/v4/projects/1/packages/generic/yue-review/1.1.51/',
    platforms: [
      ['windows-x86_64', artifact],
      ['darwin-aarch64', artifact],
    ],
    notes: '# release notes',
  });
  assert.equal(manifest.version, '1.1.51', '应去除 v 前缀');
  assert.equal(manifest.notes, '# release notes');
  assert.match(manifest.pub_date, /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/);
  const win = manifest.platforms['windows-x86_64'];
  assert.equal(
    win.url,
    'https://example.test/api/v4/projects/1/packages/generic/yue-review/1.1.51/Yue-Review_1.1.51_x64-setup.exe',
  );
  assert.equal(win.signature, 'signature-of-1.1.51', '签名应去除换行后透传');
});

test('buildUpdaterManifest 缺必备平台产物时应抛错', () => {
  assert.throws(
    () => buildUpdaterManifest({
      version: '1.1.51',
      urlBase: 'https://example.test/',
      platforms: [['windows-x86_64', '']], // darwin-aarch64 完全缺失
      notes: '',
    }),
    /缺少 windows-x86_64 更新产物|更新清单缺少必备平台/,
  );
});

test('buildUpdaterManifest 可选平台缺产物时应跳过且缺签名时抛错', () => {
  const { dir, artifact } = makeFixture();
  // 可选平台 darwin-x86_64 无产物 → 跳过
  const skipped = buildUpdaterManifest({
    version: '1.1.51',
    urlBase: 'https://example.test/',
    platforms: [
      ['windows-x86_64', artifact],
      ['darwin-aarch64', artifact],
      ['darwin-x86_64', ''],
    ],
    notes: '',
  });
  assert.equal(skipped.platforms['darwin-x86_64'], undefined);
  // 可选平台有产物但缺签名 → 抛错
  const artifactNoSig = join(dir, 'Yue-Review_1.1.51_x64-setup-nosig.exe');
  writeFileSync(artifactNoSig, 'bytes');
  assert.throws(
    () => buildUpdaterManifest({
      version: '1.1.51',
      urlBase: 'https://example.test/',
      platforms: [
        ['windows-x86_64', artifact],
        ['darwin-aarch64', artifact],
        ['darwin-x86_64', artifactNoSig],
      ],
      notes: '',
    }),
    /缺少 darwin-x86_64 更新签名/,
  );
});

test('版本一致性不变式：URL 文件名版本与发布版本不一致应抛错（2026-09-16 事故形态）', () => {
  // 本次事故形态：发布 1.1.51，清单 URL 却指向累积的 1.1.46 安装包
  const broken = {
    version: '1.1.51',
    notes: '',
    pub_date: '2026-09-16T04:31:37Z',
    platforms: {
      'windows-x86_64': {
        url: 'https://example.test/yue-review/1.1.51/Yue-Review_1.1.46_x64-setup.exe',
        signature: 'sig',
      },
    },
  };
  assert.throws(
    () => assertPlatformVersionConsistency(broken, '1.1.51'),
    /1\.1\.46.*不一致|不一致.*1\.1\.46/,
    'URL 文件名版本与发布版本不一致必须被拒绝',
  );
});

test('版本一致性不变式：一致版本与通用文件名应放行', () => {
  const ok = {
    version: '1.1.51',
    notes: '',
    pub_date: '2026-09-16T04:31:37Z',
    platforms: {
      'windows-x86_64': {
        url: 'https://example.test/yue-review/1.1.51/Yue-Review_1.1.51_x64-setup.exe',
        signature: 'sig',
      },
      'darwin-aarch64': {
        // macOS tar.gz 为覆盖式通用文件名，不含版本号，无法比对 → 放行
        url: 'https://example.test/yue-review/1.1.51/Yue-Review.app.tar.gz',
        signature: 'sig',
      },
    },
  };
  assert.doesNotThrow(() => assertPlatformVersionConsistency(ok, '1.1.51'));
});

test('版本一致性不变式：清单 version 字段与期望不符应抛错', () => {
  const mismatched = {
    version: '1.1.46',
    notes: '',
    pub_date: '2026-09-16T04:31:37Z',
    platforms: {},
  };
  assert.throws(
    () => assertPlatformVersionConsistency(mismatched, '1.1.51'),
    /与发布版本 1\.1\.51 不一致/,
  );
});
