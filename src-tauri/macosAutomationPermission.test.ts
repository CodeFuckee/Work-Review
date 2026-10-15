import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

// 被测问题：macOS 每次启动都弹「"yue-review"想控制此Mac并访问你的数据」，
// 用户授权后仍反复弹出。根因两层：
// ① entitlements 把 helper 专用的 com.apple.security.inherit 烧进主应用签名，
//    且携带对非沙箱应用无意义的沙箱设备 key——异常 entitlements 干扰 TCC 对
//    code identity 的校验，授权记录无法持久化；
// ② 采集链路每秒向 System Events 发 Apple Events（前台窗口采集用 osascript），
//    这是权限需求本身的来源；签名身份不稳定（CI 无 signingIdentity 证书）时
//    TCC 记录失配 → 每次启动重弹。

function stripComments(text: string): string {
  // 只对「代码本身」断言：注释允许保留历史原因说明（解释为什么不用某 API），
  // 不然连防回归说明自己都会触发 doesNotMatch
  return text
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/^\s*\/\/.*$/gm, '');
}

test('主应用 entitlements 不得包含 helper 专用的 inherit 权限', async () => {
  const raw = await readFile(new URL('./entitlements.plist', import.meta.url), 'utf8');
  const source = stripComments(raw);
  // com.apple.security.inherit 只允许出现在 XPC/helper 子二进制的 entitlements 中；
  // 出现在主应用 entitlements 属配置错误（TCC 校验异常的直接嫌疑）
  assert.doesNotMatch(
    source,
    /com\.apple\.security\.inherit/,
    '主应用 entitlements 不应包含 com.apple.security.inherit（helper 专用 key）',
  );
});

test('entitlements 不应使用沙箱设备 key 表达屏幕录制能力', async () => {
  const source = await readFile(new URL('./entitlements.plist', import.meta.url), 'utf8');
  // macOS 屏幕录制是 TCC 运行时权限（由 Info.plist 的 NSScreenCaptureUsageDescription
  // 描述），com.apple.security.device.screen-capture 是 App Sandbox 的设备能力
  // entitlement——非沙箱应用携带它没有意义，属于配置噪音
  assert.doesNotMatch(
    source,
    /com\.apple\.security\.device\.screen-capture/,
    '非沙箱应用不应携带沙箱设备能力 key（屏幕录制走 TCC + Info.plist 描述）',
  );
});

test('macOS 前台窗口采集不得依赖 System Events Apple Events', async () => {
  // 前台窗口采集是每秒必发的路径——它用 osascript tell System Events 是
  // automation 弹窗的需求源。应改用原生 CGWindowList/proc_pidpath（仅依赖
  // 已授权的屏幕录制权限，零 Apple Events）。
  const monitorSource = await readFile(new URL('./src/monitor.rs', import.meta.url), 'utf8');
  // 三个平台有同名函数，切 macOS cfg 块内的那一份（Windows 实现在前）
  const macosAnchor = monitorSource.indexOf(
    '#[cfg(target_os = "macos")]\nfn get_active_window_with_options',
  );
  assert.notEqual(macosAnchor, -1, 'macOS 前台采集函数应存在');
  const fnStart = monitorSource.indexOf('fn get_active_window_with_options', macosAnchor);
  const fnSource = stripComments(
    monitorSource.slice(fnStart, monitorSource.indexOf('\n#[cfg', fnStart)),
  );
  assert.doesNotMatch(
    fnSource,
    /osascript|System Events/,
    '前台窗口采集（每秒热路径）不应走 osascript/System Events',
  );
  // 新实现应使用 CoreGraphics 窗口列表（kCGWindowName 依赖已授权的屏幕录制权限）
  assert.match(
    fnSource,
    /CGWindowListCopyWindowInfo|get_active_window_native/,
    '前台窗口采集应使用原生 CGWindowList 实现',
  );
});

test('macOS 运行中应用列表不得依赖 System Events Apple Events', async () => {
  // 设置页 onMount 无条件调用 get_running_apps，其 macOS 实现用 osascript
  // tell "System Events" 列运行应用——v1.2.29 热路径清理后仅存的 System Events
  // Apple Events 需求源，是「想控制此 Mac」弹窗在设置页复发的直接原因。
  // 应改用原生 NSWorkspace 枚举（activationPolicy == regular 等价于
  // System Events 的 "background only is false"，零 Apple Events）。
  const statsSource = await readFile(new URL('./src/commands/stats.rs', import.meta.url), 'utf8');
  const macosAnchor = statsSource.indexOf(
    '#[cfg(target_os = "macos")]\nfn get_running_apps_impl',
  );
  assert.notEqual(macosAnchor, -1, 'macOS get_running_apps_impl 应存在');
  const fnStart = statsSource.indexOf('fn get_running_apps_impl', macosAnchor);
  const fnSource = stripComments(
    statsSource.slice(fnStart, statsSource.indexOf('\n#[cfg', fnStart)),
  );
  assert.doesNotMatch(
    fnSource,
    /osascript|System Events/,
    '运行中应用列表不应走 osascript/System Events（automation 弹窗需求源）',
  );
  assert.match(
    fnSource,
    /NSWorkspace|runningApplications/,
    '运行中应用列表应使用原生 NSWorkspace 枚举',
  );
});

test('CI 打包脚本应支持稳定签名身份并在缺证书时显式回退 ad-hoc', async () => {
  // ad-hoc 签名的 cdhash 每次构建都变，TCC 授权随每次发版失效——用户即使授权过，
  // 更新一版后权限弹窗照样重弹（弹窗复发的放大器）。打包脚本应支持把自签代码
  // 签名证书（桥接 secrets 注入）导入临时 keychain 做稳定签名；未注入时必须
  // 显式回退 ad-hoc 并告警，而不是静默出包。
  const packageScript = await readFile(new URL('../ci/macos/package.sh', import.meta.url), 'utf8');
  assert.match(
    packageScript,
    /MACOS_SIGNING_P12/,
    '打包脚本应读取 MACOS_SIGNING_P12（稳定签名证书，base64 p12）',
  );
  assert.match(
    packageScript,
    /signingIdentity/,
    '打包脚本应通过 --config 覆盖 bundle.macOS.signingIdentity',
  );
  assert.match(
    packageScript,
    /codesign --verify/,
    '配置证书后应对产物做签名完整性自检（codesign --verify）',
  );
  assert.match(
    packageScript,
    /回退 ad-hoc/,
    '未配置证书时应显式告警回退 ad-hoc（授权无法跨版本持久）',
  );
});
