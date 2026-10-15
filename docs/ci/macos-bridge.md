# macOS 作业桥接 GitHub Actions（试点）

> 2026-10-10 起桥接已**多平台化**：同一座桥按 payload 的 `kind`（`BRIDGE_KIND`，
> `macos|linux`）路由——macOS 作业用 `extends: .macos-bridge`，Linux 作业用
> `extends: .linux-bridge`（Ubuntu runner；打包类可经 allowlist 路由 `ubuntu-22.04`
> GLIBC 基线档）。产物包名按平台分（`macos-bridge` / `linux-bridge`）。完整方案见
> ZCode workspace 的 `gitlab-linux-jobs-on-github-actions.md`。

macOS 作业此前只能跑在那台 `tags: macos` 的 MacBook runner 上——设备随身携带、
频繁断网，作业时长随之失败/排队。GitHub Actions 提供托管的 Apple Silicon macOS
runner（一次性 VM，跑完即毁），用它承接「只能在 macOS 上执行」的作业。

## 架构（一段话版）

作业改写在 **Linux shell runner**（linux-x64）上跑 `ci/bridge.sh`：它把目标脚本
（base64）+ 上下文（SHA/分支/白名单 CI 变量）POST 到 GitHub 桥接仓库
`CodeFuckee/gl-macos-bridge` 的 `repository_dispatch`；GitHub 侧 workflow
（`runs-on: macos-15`）按 SHA 克隆本仓库、执行脚本、把 `dist/*` 回传到本项目
generic package registry；bridge.sh 轮询该 run 终态并**镜像为退出码**——
流水线门禁、MR 阻断、后续 stage 依赖全部照常工作。取消流水线时 bridge.sh
的 trap 会同步取消 GitHub run。

```
GitLab main push → 流水线
  └─ macos-bridge-smoke（linux-x64）── dispatch ──▶ GitHub Actions macos-15
        │ bridge.sh 轮询 ◀──────────── 执行 ci/macos-smoke.sh，dist/ 回传
        └─ 镜像 success/failure 为本作业退出码
```

## 依赖的三样东西

| 什么 | 在哪 | 说明 |
| --- | --- | --- |
| `GH_BRIDGE_TOKEN` | GitLab 项目 CI 变量（masked+protected） | GitHub PAT，对桥接仓库 contents:write（触发 dispatch 用） |
| `GL_PAT_131` | GitHub 桥接仓库 Actions secret | 本项目 project access token（Developer 级、api scope），GitHub 侧克隆源码 + 上传产物用 |
| `CodeFuckee/gl-macos-bridge` | GitHub | 桥接仓库：workflow + 项目注册表（allowlist）；公开仓库 macOS runner 免费 |

`GH_BRIDGE_TOKEN` 未配置时 `macos-bridge-smoke` 自动跳过（`when: never`），
流水线不受影响。

## 迁移状态（2026-10-10，macOS 作业已全部脱离 MacBook）

| 作业 | 执行体 | 状态 |
| --- | --- | --- |
| `macos-bridge-smoke` | `ci/macos-smoke.sh` | 桥接（常驻冒烟 + registry 回环守护） |
| `rust-verify-macos` | `ci/macos/verify.sh` | 桥接（实测 ~7 分钟） |
| `build-release-macos` | `ci/macos/release-build.sh` | 桥接（实测 ~15 分钟，产物还原真实二进制名 `work-review`） |
| `tauri-package-macos-arm64` | `ci/macos/package.sh`（triple 透传） | 桥接（实测 ~21 分钟；签名/产物结构已 dry-run 验证） |
| `tauri-package-macos-x64` | `ci/macos/package.sh`（交叉编译） | 桥接（随 `PACKAGE_MACOS_X64` 启用，当前关） |
| `package-daily-macos-arm64` | `ci/macos/package.sh` | 桥接（夜间 schedule；latest 下载链接依赖的 artifacts 已保留） |
| `package-daily-macos-x64` | `ci/macos/package.sh` | 桥接（随 `PACKAGE_MACOS_X64` 启用，当前关） |

四个打包作业共用 `ci/macos/package.sh`，目标三元组经 `TAURI_TARGET_TRIPLE` 透传切换。

重型作业语义：

- **超时链**：GitHub `timeout-minutes: 175m` < GitLab 轮询 `GH_POLL_TIMEOUT_MIN: 170` < GitLab 作业 `timeout: 3h`。改任何一侧都要同步检查另外两侧。
- **冷构建代价**：一次性 VM 无跨流水线增量，每次 push 的 verify 约 1.5–2.5h（此前 MacBook 靠持久构建目录增量）。公开仓库 macOS runner 免费，代价是时间不是钱。
- **产物还原**：macOS 脚本把产物按仓库相对路径镜像进 `dist/`（如 `dist/target/release/Yue_Review`），桥接作业（`MACOS_FETCH_ARTIFACTS: "true"`）从 package registry 拉回、strip `dist/` 解包，下游 artifacts/publish 的 glob 零改动。
- **变量透传**：`MACOS_ENV_PASSTHROUGH: "RUST_VERSION RUSTFLAGS"` 按名整值透传（值可含空格）。**密钥绝不进 payload**：Tauri 签名私钥放在桥接仓库 secrets `GL131_TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)`（值取自本项目同名变量，改名需两侧同步），workflow 注入 macOS 环境后 tauri-cli v2.12 直接读取。
- **透明度**：GitHub Actions run 页面即 macOS 侧完整日志；桥接作业日志里有每 15s 的轮询轨迹与最终 run 链接。

## 怎么再加一个 macOS 作业

```yaml
my-macos-job:
  extends: .macos-bridge
  stage: compile                        # 模板默认 verify，按需覆盖
  variables:
    MACOS_SCRIPT: "ci/macos/我的脚本.sh"  # 脚本须自包含；产物放 dist/
    MACOS_ENV_PASSTHROUGH: "RUST_VERSION RUSTFLAGS"  # 按名透传非敏感变量
    GH_POLL_TIMEOUT_MIN: "170"          # < 本作业 timeout，> GitHub 侧执行时长
  timeout: 3h
  rules: [...]
```

脚本可用的注入变量（白名单）：`CI_PROJECT_PATH/NAME`、`CI_COMMIT_REF_NAME/SHA`、
`CI_PIPELINE_IID`、`CI_JOB_NAME`、`CI_SERVER_HOST`、`CI_PROJECT_DIR`（=检出目录）。
额外变量经作业 `variables: MACOS_EXTRA_ENV: "K=V K2=V2"` 传递（值不含空格）。
**注意**：GitLab 的 `services:`/`cache:`/`artifacts:` 语义不跨桥接；产物统一
`dist/` → package registry（包名 `macos-bridge`，版本 = correlation_id
`gl<项目ID>-p<流水线ID>j<作业ID>`，下载走
`GET /api/v4/projects/131/packages/generic/macos-bridge/<corr>/<corr>.tgz`，
凭据用 read_api 级 PAT）。

## 安全边界

- payload 只含脚本与元数据，**不含任何凭据**（视为半公开数据）。
- 凭据分置：GitHub PAT 只存 GitLab masked 变量；GitLab token 只存 GitHub secret。
- 桥接仓库 allowlist 未登记的项目一律拒跑（防伪造 dispatch）。
- 详细威胁模型见桥接仓库 `README.md`。

## 稳定签名身份（2026-10-15 起，解决 TCC 授权每版失效）

ad-hoc（`-`）签名的 cdhash 每次构建都变，macOS TCC 按签名身份记录授权——
更新一版授权即失效，权限弹窗（「想控制此 Mac」「想控制 Chrome」…）每版重弹。
配置**自签代码签名证书**后 TCC 按证书 designated requirement 记录授权，跨版本持久：

1. **生成证书**（Mac 上一次性）：`P12_PASSWORD='…' ./scripts/ci/generate-macos-signing-cert.sh`
   → 产出 `workreview-signing.p12` + 同名 `.b64`（单行 base64），并自动导入本机钥匙串。
2. **配桥接仓库 secrets**（`CodeFuckee/gl-macos-bridge`）：
   - `GL131_MACOS_SIGNING_P12` = `.b64` 文件内容；
   - `GL131_MACOS_SIGNING_P12_PASSWORD` = 导出密码。
3. **桥接仓库 workflow** 给打包作业 env 注入（与 `GL131_TAURI_SIGNING_*` 同款映射）：
   `MACOS_SIGNING_P12` / `MACOS_SIGNING_P12_PASSWORD`。
4. `ci/macos/package.sh` 检测到 `MACOS_SIGNING_P12` 后自动：导入一次性 VM 临时
   keychain → `--config` 覆盖 `bundle.macOS.signingIdentity` 为证书 SHA-1 →
   产物自检 `codesign --verify` + Authority 非空。未配置时显式告警回退 ad-hoc，出包不受影响。

注意：切换签名身份的那一版需要重新授权一次（TCC 身份变更），此后跨版本持久。

## 已知取舍

- **不回传 GitLab commit status**：该 API 在本实例（GitLab 19.4.1）实测需要
  Maintainer 级 token，而 GitHub 侧持有的项目 token 是 Developer 级（最小权限，
  泄露也不能写代码）。代价仅是 MR 提交挂件少一个外链；门禁不受影响
  （靠 bridge.sh 轮询镜像）。后续如需，把 `GL_PAT_131` 换成 Maintainer 级并在
  桥接仓库 PROJECTS 里开 `status: true`。
- macOS runner 计费：桥接仓库为公开仓库时标准 macOS runner 免费；改私有则按
  10× 费率计分钟（GitHub Free 2000 分钟/月 → 约 200 macOS 分钟）。
- 桥接作业占用 linux-x64 runner 槽位整段时长（多为 sleep 轮询，CPU/内存开销可忽略）。
