# macOS 作业桥接 GitHub Actions（试点）

## 为什么

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

## 怎么再加一个 macOS 作业

```yaml
my-macos-job:
  extends: .macos-bridge
  variables:
    MACOS_SCRIPT: "ci/我的脚本.sh"     # 脚本须自包含；产物放 dist/
  script:
    - bash ci/bridge.sh
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

## 已知取舍（试点阶段）

- **不回传 GitLab commit status**：该 API 在本实例（GitLab 19.4.1）实测需要
  Maintainer 级 token，而 GitHub 侧持有的项目 token 是 Developer 级（最小权限，
  泄露也不能写代码）。代价仅是 MR 提交挂件少一个外链；门禁不受影响
  （靠 bridge.sh 轮询镜像）。后续如需，把 `GL_PAT_131` 换成 Maintainer 级并在
  桥接仓库 PROJECTS 里开 `status: true`。
- GitHub 侧 `timeout-minutes: 30`，GitLab 侧作业 `timeout: 45m`——迁移真实
  编译作业（冷编译可超 1h）时两边都要调大。
- macOS runner 计费：桥接仓库为公开仓库时标准 macOS runner 免费；改私有则按
  10× 费率计分钟（GitHub Free 2000 分钟/月 → 约 200 macOS 分钟）。
