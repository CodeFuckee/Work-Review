# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### 变更
- **前端迁移 Carbon Design（第一阶段）**：升级 Svelte 5（零源码改动，legacy 模式兼容），引入 carbon-components-svelte 0.113 + carbon-icons-svelte + IBM Plex Sans/Arabic 字体；应用壳层重建为 Carbon UIShell（Header+SideNav+Content，窗口控制/录制/语言/主题并入 Header）；Toast/ConfirmDialog 改用 Carbon 组件（store API 不变）；登录、关于、设置壳（Carbon Tabs）与全部设置面板的手写开关批量迁移 Carbon Toggle/Slider/RadioButtonGroup/TextInput；主题桥接 Carbon 浅色 g10/深色 g90，移除 A/B/C 三套界面风格（Rust 配置字段保留但不再使用）；测试套件相应改写（删除纯视觉断言 10 文件，改写开关/菜单/Plane 图标等断言为 Carbon 语义），725 项测试全绿。助手/日报/时间线/概览四个大页面继续迁移中。

### 修复
- **Linux 登录页连内网服务器地址仍报"无法连接服务器"（v1.2.22 白名单修复在 Linux 无效）**：WebView 页面源在 Linux 是 `tauri://localhost`（wry 注册为 secure），同样是安全上下文，页面 fetch 纯 HTTP 内网地址仍被混合内容拦截；Windows 的 `additionalBrowserArgs` 豁免在 WebKitGTK 不存在，平台各自打补丁不可持续。**根治**：登录/注册请求改由 Rust 侧 `client_http_post_json` 命令（reqwest，15s 超时）发出，完全绕开 WebView 网络栈——任意 http/https 服务器地址全平台可用，不再依赖 CSP/浏览器参数白名单；无 Tauri 运行时的浏览器预览（vite dev/README 截图）自动退回 fetch。临时绕过（本机 `systemd --user` 转发 127.0.0.1:4191→10.0.0.220:4191）可停用。

### 新增
- **CI：macOS 作业桥接 GitHub Actions（试点作业 `macos-bridge-smoke`）**：解决 MacBook runner 随身携带频繁断网导致 macOS 作业无法稳定执行的问题。新增 `ci/bridge.sh`（跑在 linux-x64 shell runner）：把目标脚本 base64 + 上下文 POST 到 GitHub 桥接仓库 `CodeFuckee/gl-macos-bridge` 的 `repository_dispatch`，GitHub 侧 workflow 在托管 `macos-15`（Apple Silicon，一次性 VM）上按 SHA 克隆本仓库、执行脚本、`dist/*` 回传本项目 generic package registry，bridge.sh 轮询终态**镜像为退出码**——流水线门禁/MR 阻断原生生效，GitLab 取消流水线时 trap 同步取消远端 run。试点作业 `macos-bridge-smoke`（stage verify）执行 `ci/macos-smoke.sh` 冒烟（环境信息 + GitLab 回连探测，产物回传验证）；`GH_BRIDGE_TOKEN` CI 变量未配置时自动跳过不影响流水线。凭据分置：GitHub PAT 存 GitLab masked+protected 变量（仅触发 dispatch），GitLab 项目 token（Developer 级 api scope，SHA 克隆实测可用）存 GitHub secret；payload 零凭据、桥接仓库 allowlist 未登记项目拒跑。设计文档 `docs/ci/macos-bridge.md`。后续 `rust-verify-macos`/`build-release-macos` 可套 `.macos-bridge` 模式逐个迁移。
- **登录页服务器地址白名单新增 `10.0.0.220`**：https 源（`useHttpsScheme`）fetch 纯 HTTP 内网地址属混合内容，被渲染层零请求拦截（v1.1.81 已知问题，当时仅放行 `10.37.80.25`）。家庭网络下 10.37.80.25 不可路由、只能用 10.0.0.220 时登录页恒报"无法连接服务器"。CSP `connect-src` 增加 `http://10.0.0.220:*`（全平台生效）；WebView2 `--unsafely-treat-insecure-origin-as-secure` 追加该地址（Windows）。浏览器能打开而应用不行的原因：顶层导航无混合内容/CSP/跨源概念。

### 新增
- **私有 apt 软件源**（Linux/Debian 系 amd64）：`apt update && apt install yue-review` 直装。仓库挂在 GitLab Generic Package Registry 固定地址 `…/packages/generic/apt-repo/stable/`（客户端源地址永不变化），每次发版由新增 `publish-apt-repo` 作业自动重建（Packages 索引 + 含 SHA256 校验和段的 Release + GPG 清签/分离签名，整包先删后传、pool 只留最新版）。关键实测前提：generic registry 下载/上传接受 HTTP Basic 认证（apt 走 `/etc/apt/auth.conf.d`，客户端仅需 read_api PAT）、文件名支持斜杠（拼出 `dists/…` 仓库路径）。签名私钥在受保护 CI 变量 `APT_REPO_GPG_KEY`（本地 `~/.gnupg` 内 ed25519 `FA09…D0D0`，可吊销重发）。配置方法见 `.gitlab-ci.yml` 中 `publish-apt-repo` 作业头注释。
- **时间线「今日项目」卡新增手动归因按钮**：卡标题旁刷新图标按钮，点击只重算当日归因（复用 `plane_attributions_for_date` 按日现算链路，不重拉活动/不动图片缓存，轻量秒级）——徽标与卡的 auto 项随意更新；适合 AI 兜底失败重试、当日持续工作后刷新归因。计算中按钮 spinner + 禁用；归因失败显示轻量错误提示行（4 秒自动消失，区别于自动链路的静默策略——用户主动点击必须知道结果）。`loadPlaneAttributions` 增加成败返回值（自动链路行为不变）。未配置 Plane 时随卡隐藏。i18n 每语言新增 2 键（叶子钉 1_294→1_296）。

### 修复
- **CI：linux 打包加 `--verbose` 抓 linuxdeploy 真实输出（v1.2.20/v1.2.21 三连败无任何工具日志可查）**：tauri-bundler 默认吞掉 linuxdeploy 的 stderr，三版 linux 打包均"运行数百秒后 failed to run linuxdeploy"且日志零细节；verbose 让 bundler 透传工具输出，为对症修复提供证据。
- **「今日项目」卡在归因未出/项目列表拉空时连同手动归因按钮一起消失（手动触发归因最需要的时刻按钮不可达）**：卡可见性原为「Plane 项目列表非空或当日有归因项」——项目列表拉取闪断（无重试）或当日归因尚未算出时整卡隐藏，手动归因按钮随之不可达（v1.2.20 用户实测找不到按钮）。修复：可见性放宽为**配置了 Plane（get_config.plane.url 非空）即显示卡**——空态卡（「当日暂无归因项目」占位）+ 手动归因按钮始终可达；未配置时仍整卡隐藏。附结构断言 2 条 + E2E 新场景 1 条（已配置但拉空且无归因 → 卡可见且按钮可点）。
- **时间线新增「今日项目」归因卡（页面顶部，工具栏与看板之间）**：当日 Plane 项目清单卡——自动项由当日 Plane 归因聚合（distinct 项目名），手动项从 Plane 项目下拉添加（复用归因候选缓存聚合，`list_plane_projects` 命令）；每个项目 chip 可编辑（替换为另一 Plane 项目：删旧加新两次 upsert）与删除（**当日屏蔽**：`removed` 标记后归因再算不复活，次日自然重置）。调整只影响本卡展示，不联动时间线行徽标（行级归因仍由算法按日现算）；未配置 Plane（下拉源空且无归因/手动项）整卡隐藏。数据模型与「归因随用随算不入库」哲学对齐：新表 `daily_project_adjustments(date, project_name, removed, UNIQUE(date, project_name))` 只存人工调整，最终清单 = 归因聚合 − removed ∪ added；三命令 `get_daily_project_adjustments` / `set_daily_project_adjustment` / `list_plane_projects`。新组件 `TimelineProjectsCard.svelte`（自动项蓝色 chip、手动项紫色 chip）；i18n 每语言新增 5 键（叶子钉 1_289→1_294、组件数钉 38→39）。附 Rust 表/纯函数测试 3 条 + 前端结构/文案测试 3 条。
- **时间线工具栏新增日报同款日期快捷按钮（上一天/今天/昨天）**：与日报页完全同构的三按钮插在日期选择器左侧——[上一天]为相对操作（连点可一直往前翻，复用日报的 `shiftIsoDate` 纯函数）、[今天]/[昨天]为跳转并在选中日期命中时高亮（`page-control-btn-active`，互斥切换）；点击直接赋值 `selectedDate` 走既有响应式加载链自动重载数据。复用 `report.previousDay/today/yesterday` 既有 i18n 键（零词典改动、不碰叶子数钉）。附结构断言测试 + E2E 3 条（默认高亮/连点回退/日期切换驱动数据重载）。
- **日报新增「文件修改」小节（与 GitLab 今日提交平级）**：把当日文件监控事件聚合为独立小节（新块 `FILE_EVENTS_TODAY`，默认紧跟「GitLab 今日提交」之后，可在段落钉选/隐藏里配置；AI 编排漂移时重锚回 GitLab 块之后、用户显式钉选时尊重主动布局）。内容为聚合视图不列明细：事件类型统计行（新建/修改/删除/重命名仅显示非零类型 + 去重文件数）+ 完整父目录聚合 Top 5 + 其余目录合并为「其他 N 个目录」，目录路径超 60 字符中间省略；事件超 2000 条注明仅统计最近条数。双路径消费（镜像 GitLab 补充模式，经 `GitlabReportSupplement.file_events` 搭车传入分析器、不扩 Analyzer trait 签名）：独立小节 markdown 经 assemble 注入 + 聚合上下文（单行、控制字符清洗）追加进 AI prompt（AI 增强与本地模板两种模式均注入独立小节）。当日无事件/文件监控未启用时小节为空串整体隐藏；读库失败仅记日志不阻塞日报生成。路径与 AI 上下文做控制字符清洗防 markdown 注入。i18n 每语言新增 1 键（叶子钉 1_288→1_289）。
- **GitLab 提交详情抽屉新增「修改文件」查看**：抽屉打开时后端现拉该提交的文件级 diff（`GitlabClient::fetch_commit_diff`，`GET /projects/{urlencoded}/repository/commits/{sha}/diff`，项目定位用 URL 编码的 path_with_namespace——时间线行未存 project_id；`percent-encoding` 转直接依赖），20 秒超时、结构化失败不 panic、错误串剔除凭据。前端按文件分组展示：路径 + 状态徽标（新增绿/删除红/重命名橙/修改靛）+ 可展开查看 unified diff 染色内容（增行绿底/删行红底/`@@` hunk 紫/上下文灰，暗色主题对应配色）；单文件超 400 行截断提示防 webview 卡顿。会话级缓存（sha → 结果）同提交不重复请求；加载骨架/失败重试/空态三态齐全；抽屉头部新增「在 GitLab 打开」外跳按钮（实例 URL + 项目路径 + sha，凭据无效时仍可跳转但不展示 diff）。i18n 每语言新增 9 键（叶子钉 1_279→1_288）。
- **时间线 GitLab 提交行可点击查看详情**：提交行按钮化（右侧加箭头示意），点击打开右侧详情抽屉——与应用活动抽屉同壳（遮罩/焦点陷阱/Esc 或点遮罩关闭/关闭后焦点交还触发行，两类抽屉互斥打开），头部为 GitLab 提交图标 + 项目名，正文含记录时间与变更统计（+增/-删）双栏、提交信息/分支/提交哈希/作者四行明细（数据全部行内自带，零回库查询）。i18n 每语言新增 5 键（叶子钉 1_274→1_279）。
- **时间线详情抽屉显示 OCR 文本**：点击时间线活动打开的右侧详情抽屉，在「屏幕截图」下方新增「OCR 文本」区块，展示该条记录的 OCR 识别结果（可滚动，保留换行结构；超长内容不撑爆抽屉）。数据直接复用时间线列表/详情接口已返回的 `ocr_text`（无后端改动；采集侧已按隐私策略处理——脱敏应用本就不产生 OCR），null/空白显示「本次记录没有 OCR 结果」占位。i18n 每语言新增 2 键（叶子钉 1_272→1_274）。
- **Plane 工作项归因（从屏幕活动推断正在处理的工作项）**：按 30 分钟间隔把当日活动切分为会话，四层信号打分归因到 Plane 工作项——浏览器 URL 命中工作项 UUID（100 分锁定）、窗口标题/OCR 中的 `IDENTIFIER-编号` 命中（90 分）、GitLab 分支/提交标题命中（85 分）、标题相似度与 OCR 关键词（封顶 30 仅排序）；40 分阈值以上产出归因，未达阈值会话打包一次 AI 语义归因兜底（**OCR 永不出网**：prompt 仅含脱敏窗口标题与候选工作项名称，代码层强制并带单测）。按日回溯、随用随算不入库。展示：时间线活动行紫色徽标显示工作项 key；日报「工作项归因」小节（45 秒预算，超时静默跳过）。Plane 候选池 10 分钟内存缓存；未配置 Plane 时全部静默。
- **Plane 集成（二期：按日同步入库 + 时间线交错 + 日报小节 + 项目过滤）**：镜像 GitLab 提交链路补全复盘闭环。数据库新增 `plane_work_items` 表（`UNIQUE(plane_item_id, synced_date)` 幂等覆盖 + synced_date 索引）与 `upsert_plane_work_items` / `get_plane_work_items_for_date`；命令 `plane_sync_work_items_for_date`（读已保存配置，`enabled`+url/api_key/slug 齐全才执行，项目按 `project_filter` 允许列表过滤，工作项按 updated_at 落在目标本地日期 [当日 00:00, 次日 00:00) 过滤，总预算 20 秒）与 `get_plane_work_items_for_date`；时间线页镜像 GitLab「后台补采→刷新」模式（每日期每会话补采一次、失败静默），`mergeTimelineRows` 扩展第四数据源（planeItem 行，同时间戳排在提交/活动/文件事件之后）；日报新增「Plane 工作项」小节（配置齐全即产出、读当日入库数据不现拉 API、无数据为占位说明，锚定在 GitLab 小节之后、按小时活跃度之前，四语言文案）；`PlaneConfig` 新增 `project_filter`（空 = 全部；忽略大小写精确/前缀匹配，normalize 去空去重）与设置页「项目过滤」textarea；i18n 每语言新增 3 键（叶子钉 1_262→1_265）。
- **Plane 集成（一期：设置 + 连接测试 + 工作项预览）**：设置页新增「Plane」标签（GitLab 之后），可配置私有化部署的 Plane 实例（实例地址 / 个人访问令牌 / 工作区 Slug）并测试连接、拉取近期工作项预览。走官方 REST API（`X-API-Key` 请求头，端点与响应字段依据 developers.plane.so 文档；预览按「列项目 → 聚合前 8 个项目的近期工作项，`order_by=-updated_at` 倒序截 20 条」实现），不做代码合并（用户裁决，2026-09-24）。配套：`AppConfig.plane`（`PlaneConfig { enabled, url, api_key, workspace_slug }`，默认关闭，token 明文本机存储与 GitLab 同策略、url 规范化去尾斜杠）、命令 `plane_test_connection` / `plane_fetch_preview`（结构化返回不 panic、错误串剔除 API Key、reqwest 显式超时 + 禁重定向）、设置组件 SettingsPlane 与四语言 i18n（每语言 20 键）、连接/预览不可达地址回归测试与 url 规范化单测。时间线/日报联动、后台同步、项目过滤为本期（二期）范围。

### 变更
- **时间线 Plane 归因改为双徽标并排（工作项 + 项目各一枚）**：原「紫色工作项徽标 + 旁侧 `· 项目名` 小字」改为紫色工作项徽标（`KEY 工作项名`，保持不变）+ 新增蓝色项目徽标（纯项目名，不带 ID）并排展示，去掉重复小字；四类时间线行（应用活动/提交/文件事件，featured 与 compact 两种布局）统一生效，亮暗主题各自配色（蓝色 rgb(37 99 235) 系），悬停 title 保留「项目 · 工作项」完整信息。不新增组件文件与 i18n 键；附双徽标结构断言测试（新增 1 条），E2E 三条用例断言同步升级（先红后绿，5 条全过）。
- **时间线 GitLab 提交行改用应用活动行同款紧凑栅格（参考活动行观感优化）**：告别自创三列扁平布局（1.75rem 迷你徽标 + 等宽小字项目名 + 独立统计列），换装 2.75rem 图标方块（git 提交线性图形、GitLab 橙色调）+ 粗体项目名 + 「GitLab 提交」分类徽标 pill + 提交信息次要行 + 右侧增删统计，与活动行完全同构（同卡片栅格/间距/悬停浮起）；项目名与提交信息补悬停提示。**文件事件行与 Plane 工作项行一并同构**（旧卡片样式为三行共用，不转换会丢样式）：文件行 = 文件名粗体 + 事件类型徽标（新建/修改/删除/重命名）+ 完整路径次要行 + 大小；Plane 行 = 项目名粗体 + 「Plane 工作项」徽标 + 工作项名次要行 + 状态。删除废弃的 `timeline-entry-card-commit` / `timeline-entry-app-name-commit` / `timeline-commit-badge` 样式；不新增 i18n 键（复用 overview.gitlabTitle 等既有键）。
- **概览页新增「Plane 项目概览」卡片**：按项目聚合所选日期区间内的 Plane 工作项变动（每项目：变动条数 + 最近变动的工作项/状态/时间，进度条以变动最多项目为满格），卡片打开/切日期时自动后台补采（每日期每次会话一次），读本地入库数据零额外 API 请求；沿用设置页项目允许列表；未配置 Plane 时不显示。顺带修复可见性恢复后重建的刷新定时器遗漏文件活动刷新的既有问题。
- **概览页文件活动与 GitLab 提交并排两列（P2）**：两张卡片原先各占整行，现放入分区网格同一行分两列展示（窄屏自动回落单列堆叠）。
- **时间线页新增文件活动展示（P2）**：按所选日期拉取文件监控事件（上限 50 条），以紧凑行（📄 徽标 + 文件名 + 事件类型 + 时间 + 大小）与应用活动/GitLab 提交按时间戳交错排序；未启用文件监控或读取失败时静默为空，不影响时间线首屏。合并逻辑 `mergeTimelineRows` 扩展第三数据源（带交错排序与回退用例）。

### 移除
- **日报移除 Plane 小节（用户裁决：日报暂不记录 Plane，集成另有他用）**：回退二期加入的 PLANE_TODAY 报告块（含分析器参数、锚定函数、多语言标题与占位、相关测试）；时间线交错展示与按日同步入库保留。
- **Telegram Bot（消息通知）功能整体移除**：后端 `telegram_bot.rs`（轮询/命令处理/绑定码）与 `get_telegram_bot_status`、`generate_telegram_bot_bind_code` 命令，`AppConfig` 全部 `telegram_bot_*` 配置字段，前端 TelegramBotPanel 面板与设置页状态轮询/归一化逻辑，四语言 i18n telegram 文案与 README 三语言 Bot 联动段落中的 Telegram 字样一并移除（旧配置文件里的 telegram_bot_* 键会被自动忽略，无需手动清理；飞书/钉钉/企业微信三个 Bot 与共用的 `/devices` `/report` `/generate` 命令不受影响；识别 Telegram 桌面应用的时间线分类关键词与本功能无关，保留）。用户裁决基本不用（2026-09-24；设置入口此前已隐藏）。
- **设置页隐藏 Telegram 通知入口（暂时）**：Telegram Bot 面板从「节点网关 → 消息通知」分组隐藏（`SHOW_TELEGRAM_PANEL = false` 单开关，恢复改回 true 即可）；面板代码与配置字段均保留，仅入口不可见（用户裁决：基本不用，2026-09-24）。
- **桌宠（桌面化身）功能整体移除**：悬浮窗、主动开口提醒、全局键鼠输入联动、跟进对话（followups）、休息提醒、托盘桌宠开关、设置页桌宠分区与全部 avatar 配置字段（旧配置文件里的 avatar 键会被自动忽略，无需手动清理）；README 三语言宣传段、桌宠截图与 BongoCat 第三方署名（THIRD_PARTY_NOTICES.md）一并移除。用户裁决「功能鸡肋」（2026-09-24）。
- **reinstall.sh 移除 GitHub latest 自动回退**：该发布链早已失效，静默装回陈旧版本比直接报错更糟；本地找不到 .deb 时给出 WR_DEB / WR_DEB_URL / GitLab 制品页指引，显式 `WR_DEB_URL` 覆盖仍支持（第九轮评审待确认 #22，用户裁决直接删）。

### 修复
- **时间线 Plane 归因一天只归因到一个工作项（穿插多任务全部丢失）**：会话切分原为纯时间间隔（相邻活动 ≤30 分钟即合并），而活动采集很密集（实测相邻间隔 ≤1 分钟占 90%），穿插多任务的一天几乎不会出现 30 分钟空白——整天被连成一个大会话，每会话只取最高分候选（同分先命中者胜），全天的活动就只归因到一件事。修复：切分信号改为「时间间隔 + 应用切换」——`build_activity_sessions` 在排序遍历时跟踪当前段应用，间隔超限或 `app_name` 变化均切新段（实测真实数据 1 段 → 12 段，编辑器↔浏览器↔IDE 穿插各自成段独立归因）。同应用内连续多任务仍依赖段内 identifier/gitlab 打分与 AI 兜底；人工纠偏反馈闭环（徽标可点击改派、纠偏入库、上下文签名命中自动应用）为二期演进方向，细粒度切分是其可学习性地基。附 2 条回归测试（穿插切分 3 段 / 两任务各产出一条归因）。
- **时间线 Plane 归因徽标缺失（GitLab 提交行/文件事件行不 join + 响应式缺陷致活动行徽标时有时无）**：两层根因。其一，归因时间窗 join 只对应用活动行生效（`planeBadgeFor` 仅在 activity 分支调用），GitLab 提交行与文件事件行即使发生在归因会话窗口内也不显示归属工作项徽标。其二（更深层，Svelte 响应式缺陷）：行内徽标原先在模板里调 `planeBadgeFor(...)` 计算，Svelte 只追踪表达式词法引用的组件变量，函数体内读取的 `planeAttributions` 不被追踪——归因按日现算、晚于行渲染到达，赋值后徽标**不会重渲染出现**，活动行既有徽标同样时有时无（E2E mock 时序下必现）。修复：归因 join 上移到 `timelineRows` 响应式派生层（`attachAttribution`，公共模块 `timelineAttribution.ts`，词法引用 `planeAttributions` 保证后到数据驱动行重算），提交行按 `committed_at`、文件事件行按 `occurred_at`、活动行按 `timestamp` 统一附加归因，模板读 `row.attribution` 渲染同款徽标（不新增 i18n 键）。附公共函数单测 4 条、提交行徽标结构断言、E2E 回归 5 条（tests/e2e/ui/timeline-plane-attribution.spec.ts，mock IPC 无真实后端依赖；报告见 tests/e2e/ui/reports/timeline-plane-attribution-report.md）。
- **更新后 Plane/GitLab 集成配置再次丢失（WebDAV 配置同步启动拉取整包覆盖）**：`apply_synced_remote` 原为整包替换（仅保留本机端口/开机自启等 10 项），启动时按 `synced_at` 拉取远端快照——远端陈旧快照（只在单机配置过 Plane、从未上传过该配置）时间戳较新时，把本机已配置的 Plane 整块抹成默认值（9-26 手工改文件恢复未走 save_config、未打同步戳，本地与远端分叉，升级重启触发拉取即复现）。修复：合并时远端 Plane/GitLab 为「从未配置」的默认空值（视为快照未携带该块配置）则保留本地非空值，远端已有配置仍按最后写入获胜覆盖；用户主动清空走设置页保存不受影响。附 3 条回归测试（远端空值保留本地 plane/gitlab、远端有值正常覆盖）。
- **Plane 连接测试报「解析响应失败」且提示不友好（P2）**：连接测试优先打已废弃的工作区详情端点（部分实例该路由仅接受会话认证，X-API-Key 得 401 会被误判为 Key 无效）；实例返回 2xx 非 JSON（典型：Web 界面子路径的 SPA 兜底 HTML 页）时只报晦涩的 serde 错误。修复：连接测试主路径改为文档端点 `projects/?per_page=1`；2xx 非 JSON 响应给出「命中 Web 页面、检查实例地址」的可行动提示；四语言 urlHint 补充「填 API 根地址、勿带 Web 界面子路径」说明。
- **Plane 实例地址协议门禁（P2，用户裁决）**：默认仅允许 HTTPS 实例地址（API Key 随请求头出网，明文 http 有泄露面）；设置页新增「允许不安全协议（HTTP）」开关，勾选后放行 http（可信内网场景）。测试连接/预览/同步三条命令入口统一校验，缺协议一并拒绝，附命令级与函数级回归测试。
- **概览页残留调试注释文字上屏（P2）**：文件活动卡片后残留一行残缺注释「行布局复用 app-usage-chart 同款网格（…）-->」（`<!--` 开头已丢失，整行按正文渲染上屏）；已删除。同位置上一行未闭合的 `<!--` 注释补齐收尾。
- **文件监控图标改单图标（P2）**：设置侧边栏的文件监控图标原为「文件夹+数据库」两枚 SVG 叠放，视觉杂乱；改为单枚「眼睛」图标表达监控语义（同款描边风格）。
- **设置侧边栏「存储」图标空白（P2）**：图标分支链缺 `storage` 分支——八个标签页只有七个有图标定义，存储页图标渲染为空。补数据库圆柱图标（同款描边风格）。
- **第十轮全量代码评审修复（第二批：前端 TS / CI / 打包脚本 / 文档，9 项，行为保持；完整清单见 `CODE_REVIEW_FINDINGS.md` 第十轮）**：
  - **设置页账号卡片整页显示裸 i18n 键名（P1）**：`settings.accountTitle/accountDesc/accountLoggedInAs/accountNotLoggedIn/accountServerLabel/accountServerHint` 6 个键在四语言 locale 中全部误嵌 `settingsStorage` 分组（与上一轮 8403aea 修复的 fileMonitor 键组误嵌同类回归），组件按 `t('settings.account*')` 取值两级回退均失败，账号卡片标题/描述/登录态/服务器地址标签全部渲染原始键名；四语言同步把 6 键移入 `settings` 组（总叶子数不变）。
  - **README 截图脚本页签索引整体错位（P2）**：桌宠移除删掉设置页第 3 个页签后，capture-readme-pages.ts 仍按旧索引点击——重拍 README 截图时「设置-桌面化身」会实拍隐私页、「设置-隐私」实拍存储页、以此类推整体错一位且无任何报错；按现页签数组修正索引、删除化身条目，并给 `captureSettingsTab` 加索引越界守卫（页签再增删时立刻报错提示同步），mock 配置清理 avatar_*/break_reminder_* 残留字段；契约测试断言同步。
  - **appimage/uninstall.sh 未随改名适配（P2）**：进程探测/桌面项/图标/AppImage glob/数据目录全部只认旧名 Work_Review，新名 Yue_Review 的 AppImage 安装用本脚本卸载会残留；按 deb/uninstall.sh 同款多名字探测改写（Yue_Review/Yue-Review/Work_Review 全兼容，新旧数据目录双清）。
  - **CI：容器打包的 CARGO_HOME 不在 git clean 排除清单（P2）**：daily-package 容器把 CARGO_HOME 挂在 `$CI_PROJECT_DIR/.cargo-home`，不在 GIT_CLEAN_FLAGS 与 version-bump 的 clean 排除清单里，每次发版 crate registry 全量重下；两处补 `-e .cargo-home`。
  - **reinstall.sh 两处收尾（P2）**：`--help` 的 sed 截取范围没跟上第九轮扩充的 Local-deb lookup / 无远程回退说明（补到 2,17p）；静音 oga 临时文件未注册退出清理（Ctrl-C/半途失败会残留 /tmp/wr-silent.*.oga，与 DEB/TMP_FWD 同款 register_cleanup）。
  - **README.zh.md 桌宠残留（P2）**：功能移除时英文/繁体 README 已清，简体漏删——功能列表「桌面化身 Beta」小节与截图区「桌面化身」图片引用（指向已删除的 png）残留；删除。
  - **updater.ts 死映射与死键（P2）**：`RUNTIME_STATUS_PATTERNS` 里 `当前未发现可安装的在线更新 → updater.noInstallAvailable` 的状态消息在 Rust 侧已不存在（多源重写时被删），映射永不可能命中、四语言键同为死键；删除映射与四语言键，i18n 叶子数钉 1_243→1_242。
- **第十轮全量代码评审修复（第一批：Rust 行为保持，29 项；完整问题清单与处置见 `CODE_REVIEW_FINDINGS.md` 第十轮）**：
  - **切换数据目录可误删当前数据（P0，正确性）**：`change_data_dir` 的「同目录/互为父子」两道校验在 `canonicalize` 之前比较字面路径，Windows 大小写差异、8.3 短名、符号链接或含 `..` 的等价形式选中当前数据目录时全部绕过，随后目标目录「清空已有受管条目」会把正在使用的 config.json/截图/OCR 日志（Linux 上含活动中的数据库）整目录删除；对齐 `cleanup_old_data_dir` 的既有口径：先规范化再比较。
  - **崩溃日志目录跟随换目录的修复实际未生效（P1，回归）**：上一轮引入的 `CRASH_REPORT_DATA_DIR` 用 `OnceCell` 存储，`set` 只在首次生效，启动后 `change_data_dir` 成功换目录时的二次更新是静默 no-op——崩溃日志永远写旧目录；改 `RwLock<Option<PathBuf>>`（读取容忍锁中毒）。
  - **OpenAI 兼容流式端点（vLLM 等）工具调用整体被拒（P1，正确性）**：流式装配器对 tool_call 的 id/name 逐片 `push_str` 累积，官方 OpenAI 仅首片携带所以无感，vLLM 等自托管端点每片重复完整值会拼出 `call_xxxcall_xxx`，被 `trusted_event_tool_metadata` 精确匹配当未注册工具拒绝——按 openai-python 官方流式累积语义改为 slot 为空时写入，arguments 保持逐片拼接。
  - **localhost API 三个端点 limit/offset 无钳制（P2，资源）**：`/v1/weekly-review`（经 `load_filtered_activities_in_range` 可全表载入含窗口标题）、`/v1/timeline/` 查询参数直通 DB；在 Tauri/localhost 共用的 inner 层统一钳制（limit ≤10000、offset ≤1000000，同 `/v1/activities` 用户裁决先例）。
  - **S3 SigV4 签名与实际请求路径不一致（P2，正确性）**：canonical_uri 用自造 percent 编码，而实际发送的是 reqwest `Url::parse` 归一化后的路径，`+ ( ) ! = @` 等字符两种形态不同导致 `SignatureDoesNotMatch`——path_prefix 含此类字符的用户上传恒失败；canonical_uri 改用 `parsed.path()`（与线上严格一致），删除自造编码函数。
  - **HTTP 403 状态行写成 `403 OK`（P2）**：localhost_api `reason_phrase` 缺 403 映射且未知状态回落 "OK"，回调签名失败/令牌不匹配等 403 响应状态行错误；补 `403 => "Forbidden"`。
  - **流式错误 body 读取不受 deadline 约束（P2）**：`ensure_stream_status` 对非 2xx 响应 `response.text()` 无超时，半死端点慢速滴漏可无限挂起；包 deadline 剩余时间（另设 10s 上限），超时按空 body 报错（行为同前）。
  - **后台锁中毒恢复统一（P2）**：agent/tools.rs 引用收集缓冲 4 处 `if let Ok`/`unwrap_or(0)` 把 Mutex 中毒吞成引用永久失效零日志；统一 `unwrap_or_else(into_inner)`（纯内存缓冲，恢复安全）。
  - **文件监控事件通道错误静默吞掉（P2）**：core/file_monitor.rs debouncer 的 Err 批次 `unwrap_or_default()` 丢弃事件无痕迹；补 warn（条数+首条）。同批同路径+同类型去重由 O(n²) 线性扫描改 HashSet（移动含数千文件的目录时单批事件上万不再卡 worker）。
  - **AI 网关错误页全文落日志（P2，日志卫生）**：core/analysis/cloud.rs 两处把非 2xx 响应 body（网关大 HTML）整页拼进错误串并 warn 落盘（同文件第九轮已在 src-tauri 侧截 300，core 侧漏网）；补 `chars().take(300)`。`summary.rs decide_block_order` 两条失败路径（AI 调用/JSON 解析）从完全静默改为补 warn 后回退默认顺序。
  - **Windows OCR 失败路径把识别原文落日志（P2，隐私）**：两条失败分支把整段 stdout（屏幕识别文本 JSON）以 warn 明文落盘且未过敏感过滤；只记长度/状态码（同 privacy 日志脱敏口径）。
  - **域名分类热路径每循环堆分配（P2，性能）**：`builtin_domain_category` 在 170 项表循环内每次 `format!(".{suffix}")`；改 `strip_suffix` 免分配等价比较（分类路径逐活动调用）。
  - **配置修复后周期清理仍永久跳过（P2，正确性）**：`storage_cleanup_task` 读的 `config_load_status` 是启动快照且从不再赋值，启动时 Corrupted（fail-safe 跳过清理保护数据）的用户在设置页修复配置后仍永久跳过直到重启；`persist_app_config` 成功落盘后把 fail-safe 状态刷新为 Loaded。
  - **锁屏日志刷屏（P2）**：采集循环锁屏期间每轮（约 5 秒）打一条 info，整夜锁屏上千行；改为仅进入锁屏时打一次。
  - **背景图上传无载荷/解码上限（P2，纵深防御）**：`save_background_image` 对 IPC base64 无长度限制、解码无尺寸限制，解压炸弹可把内存撑到数 GB；载荷钳 20MB + `image::Limits`（8192px/256MB）。
  - **删除自定义分类的 reassign_to 未校验（P2，正确性）**：传入不存在的分类或恰好等于被删 key 时，规则与全部历史活动被批量重定向到无效分类且无报错；校验目标必须存在于内置/自定义分类且 ≠ 被删 key，否则回退 "other" 并 warn。
  - **macOS Dock 图标重载回退按旧名硬编码（P2）**：bundle 内找不到 icon.icns 时回退 `/Applications/Work Review.app/...`，应用更名 Yue-Review 后永不命中且静默；删除回退（找不到即放弃重设）。
  - **`save_hourly_summary` 返回值不可靠（P2，API 卫生）**：UPSERT 走 DO UPDATE 分支时 SQLite 规定 `last_insert_rowid()` 未定义（实为陈旧值）；两处调用方均忽略返回值，签名改 `Result<()>` 并注释陷阱。
  - **测试与死代码清理（P2）**：database.rs 7 处测试收尾只删主库文件（WAL 伴生文件 CI 持续累积）改 `remove_db_files` 三件套；agent 测试库（orchestrator 3 处 + tools 固定名 1 处）加唯一命名与收尾清理；`screen_lock.rs` 只写不读的 `is_locked` 字段与无调用点的 `set_locked` 连同 `#![allow(dead_code)]` 删除；`monitor.rs get_cf_dict_number` 补 `# Safety` 文档；`search_memory` 重复文档注释、`bing_search_html` 与行为矛盾的注释对齐；`generation_params.rs apply_ollama` 生产 `expect` 改防御性跳过（同第九轮 category.rs 口径）。
  - **database.rs 四函数结构重复（P2，可维护性）**：`get_last_activity_by_app`/`get_latest_activity_by_app`/`get_latest_activity_by_app_title`/`get_latest_activity_by_url` 逐字同构（仅过滤列与回溯起点不同）提取 `latest_activity_by_filter` 共享 helper + `today_start_ts`（SQL 形状与参数序不变）。
- **文件监控设置页整页显示裸 i18n 键名（P2）**：`fileMonitor` 键组被误嵌进 `settingsStorage` 分组内部，组件引用的 `settings.fileMonitor.*` 全部找不到，页面各标题/提示显示为原始键名；且缺 `pickFolderTitle` 键（选择文件夹对话框标题同样裸露）。修复：键组移到顶层并改名 `settingsFileMonitor`（与 settingsStorage/settingsAppearance 命名一致），补齐 `pickFolderTitle`，组件引用同步，四语言叶子 1_242→1_243（zh-CN/zh-TW/en/ar 同步）。
- **第九轮待确认批次修复（#6/#7/#11/#15/#23）**：
  - macOS「打开权限设置」`open` spawn 后不收割，每次点击残留僵尸进程直至退出；改后台线程 wait 收割。
  - Telegram 绑定命令走「clone→保存→整体替换」惯用法，未来锁语义演化下会覆盖其他字段并发修改；改原地修改+失败回滚（文件失败则内存保持旧值的语义不变）。
  - 分类回溯两处生产代码 `expect`（DB 主键恒非 NULL、理论不可达）改防御性跳过。
  - Localhost API `/v1/activities` 的 limit 查询参数直通 SQL LIMIT；钳到 ≤10000（同 /v1/reports 的 `.min(100)` 先例）。
  - 删除 `scripts/stage-tauri-key.sh` 及 CI 8 处调用：bash 子进程 export 不回传父 shell，暂存从未实际生效（tauri-cli v2 自行解码原始 base64 值，v1.1.87-90 发版均为佐证）；Windows ps1 版在同一 PowerShell 会话内生效故保留。
- **崩溃日志写错目录（P2，第九轮评审待确认 #2）**：崩溃捕获固定写默认数据目录的 `crashes/`，自定义数据目录的用户按指引去自己的目录找不到日志。修复：启动解析出实际数据目录后记入 OnceCell，崩溃日志优先跟随实际目录；运行期 `change_data_dir` 换目录后同步跟随；极早期（解析前）崩溃回退默认目录，配单元测试。
- **第九轮全量代码评审修复（第二/三批：CI/构建 7 项 + 前端 TS 8 项，行为保持）**：
  - **CI：version-bump 并发推送丢失发版（P2）**：两条 main 流水线全绿并发进入 version 阶段时基于同一 origin/main 推导同一版本号，后推送方 `git push --atomic` 因非 fast-forward 整体失败——自动发版丢失、作业假红需人工重跑；加 `resource_group: version-bump` 跨流水线串行。
  - **CI：version-bump 的 git clean 漏排 `.cache`（P2）**：与全局 GIT_CLEAN_FLAGS 不同步，每次发版把 linux-x64 runner 的 .cache/tauri 打包工具缓存删光，下个 tag 流水线重新经 gh-proxy 全量下载（v1.1.84/86 两次发版失败根因复现路径）；补 `-e .cache`。
  - **CI：Windows 打包暂存的签名私钥不清理（P2，安全）**：`%TEMP%\tauri-signing-key-<pid>.key`（LocalSystem）在常驻 runner 上按次累积；两个打包作业 after_script 末尾清理。
  - **CSP connect-src 三个零消费主机（P2，安全）**：tauri.conf.json 白名单里的 api.github.com/gh-proxy.cn/gh-proxy.com 更新源切换后已无任何引用，徒然放大渲染进程失陷后的出网面；删除。
  - **Linux 安装/卸载脚本全链旧名失配（P2）**：产品改名 Yue_Review/Yue-Review 后，reinstall.sh 本地 deb 扫描永不命中（静默落入失效的 GitHub 回退装回陈旧版）、pgrep/pkill 不再命中运行中进程（full reset 期间不停机）、dpkg/apt 对新包名失明；uninstall.sh 同族；appimage/install.sh 的 AppImage 发现/图标/launcher/desktop/WMClass 改为按检出产物名参数化——全部改双名兼容（yue-review 新名 + work-review 旧名兜底）。
  - **删除死脚本 patch-info-plist.sh（P2）**：全仓无引用且按旧名 find 永不命中，权限描述已由 tauri infoPlist 原生覆盖；linux-builder-22.Dockerfile 头部使用示例仍指导挂载已废弃的宿主缓存卷、PATH 含不存在的 node 目录——对齐现行调用方式。
  - **登录凭证缓存形状校验缺失（P2，正确性）**：`getAuth` 对 localStorage 的合法 JSON 不做形状校验，缺字段/错型的半损坏条目让 `isTokenExpired` 算出 number > undefined 恒 false，坏凭证被判成 allow 直接放行进主界面（admin 同步全挂）；不满足形状一律按无凭证走登录门。
  - **登录/注册请求无超时（P2）**：admin 服务挂起（建连不响应）时按钮永久 pending；fetch 加 15 秒 AbortSignal 超时走既有 network 分支。`setAuth`/`setAdminUrl` 的 localStorage 写入补 try-catch（配额满/隐私模式下不再让登录成功后的流程崩成未处理 rejection）。
  - **分类 store 刷新竞态（P2）**：categoryStore/semanticCategoryStore 的 refresh 无请求序号守卫，并发刷新时旧数据慢响应后到覆盖新响应（UI 显示已删/已改名分类）；补 seq 快照守卫（Ask/Timeline 同款）。
  - **TS 行为保持去重（P2，可维护性）**：`avatarToggle.ts` 三份逐字同构的「改字段→保存→失败回滚」抽 `applyConfigFieldsWithRollback`；`updater.ts` 11 段状态消息正则样板表驱动化（命中顺序不变）；`appVisuals.ts` 精确名 Map 与子串长条件列表两处维护同一批应用名（yue 家族只在一侧）合并为单一有序规则表。
- **第九轮全量代码评审修复（第一批：Rust 行为保持，22 项；完整问题清单与处置见 `CODE_REVIEW_FINDINGS.md` 第九轮）**：
  - **助手会话追加消息无事务（P2，正确性）**：INSERT 消息与 UPDATE 会话 `updated_at` 不包事务，UPDATE 失败/崩溃后消息已落库但会话排序键停留旧值——新消息会话沉底并可能提前落入「保留最近 100 个」清理被连消息一起删；补 `unchecked_transaction`（同表 create/delete 同款口径）。
  - **建库迁移吞掉全部 ALTER 错误（P2，正确性）**：`init_tables` 8 处 `ALTER TABLE ADD COLUMN` 用 `let _ =` 吞错，磁盘满/只读库时列静默缺失、后续对这些列的读写以晦涩错误失败且启动日志零痕迹；新增 `add_column_if_missing` 只忽略 duplicate column，其余错误照常传播。
  - **WebDAV 远端目录被删后上传进程内永久失效（P2，正确性）**：目录缓存只增不失效，PUT 恒 409 且重试只覆盖 5xx/网络错误、补传机制走同一缓存同样失败；PUT 命中 409/404 时失效本次涉及各级目录缓存并重建后重试一次，正常路径不变。
  - **macOS 桌宠输入监听部分注册失败泄漏系统级监听（P2，正确性）**：提前返回路径不撤已成功的 monitor（泄漏在系统侧），且 STARTED 标志复位后的重试再注册一份造成事件重复派发；部分失败先 `removeMonitor:` 再返回。同文件不再对未校验整数 `transmute` 成 `NSEventType`（fieldless enum 非法判别式即 UB），改为按枚举判别式 usize 直接比较（语义不变）；补 4 处 unsafe SAFETY 注释。
  - **企业微信智能机器人退避计数只增不复位（P2，正确性）**：任何一次失败后即使长连接健康运行数小时，下次断线重连仍按累计次数吃满 60 秒封顶退避；健康会话结束后退避从基础值重新起步（连续快速断开仍递增）。
  - **本地 API base_url 对 IPv6 字面量产畸形串（P2，正确性）**：`http://{host}:{port}` 拼接对含冒号 host 产出 `http://::47831`，状态面板地址与 device apiEndpoint 均不可用；IPv6 包方括号，配回归测试。
  - **锁中毒被吞成静默跳过（P2，健壮性）**：main.rs 4 处与 file_monitor_runtime 1 处用 `if let Ok(lock)` 吞 PoisonError——合并/新建路径 OCR 文本写库、远程截图 URL 回写、实体分类内存缓存、轻量模式关窗行为、文件监控事件入库在某次持锁 panic 后从此静默失效无日志；统一 `unwrap_or_else(into_inner)` 恢复口径（与第五轮 feishu 读路径同款）。
  - **飞书 token 缓存写路径仍吞锁中毒（P2，健壮性）**：读路径第五轮已修、写路径漏网——缓存从此永远写不进，每条飞书消息多打一次 tenant_access_token API；写路径同款恢复。
  - **隐私过滤 debug 日志明文记录命中敏感关键词的标题（P2，隐私）**：`crates/core/privacy.rs` 恰在标题命中敏感词时才把标题整条打进 debug 日志（第八轮修的是 src-tauri 侧五处，本处为 core 侧残留）；改只记「命中+长度」。
  - **时间线命令日志明文打印隐私规则清单（P2，隐私）**：`get_timeline` 每次以 info 级打印完整 ignored_apps/excluded_domains（哪些应用/域名被隐藏本身是隐私配置，明文进持久日志）；改只记条数。
  - **MCP/助手会话分页 limit 无上限直通 DB（P2，防御）**：mcp-server `search_activities`/`query_timeline` 与命令层 `list_assistant_conversations` 的 limit 直通 SQL LIMIT（FTS 回退路径内部还放大 12 倍行数）；统一钳 [1, 200]。
  - **agent 收束与确认卡加固（P2，健壮性）**：模型返回 Stop 但正文为空时给兜底文案不再产生空气泡（同 calls_missing/MaxTokens 收束口径）；行动工具确认卡片回显模型参数剥控制字符并限长（同文件 user_memory 预览同款安全立场）；OpenAI/Claude 非流式错误 body 截 300 字符（防网关整页 HTML 进错误串与日志，同 Gemini/流式口径）；fast_answer 记忆检索失败补 warn 不再无声降级。
  - **统计与搜索性能（P2，性能）**：storage 周期清理复用首次统计少付 1-2 遍全目录扫描；`get_daily_stats_with_segments_filtered` 与 `get_hourly_app_breakdown_range_filtered` 数据物化后立即释放连接锁，聚合不再阻塞采集热路径；`search_memory_fallback` 按实参拼确定性 WHERE 让 timestamp 索引可走范围扫描（同 `get_activities_in_range` 已修款）。
  - **健壮性小修（P2）**：`get_hourly_activities` 先 clamp 再 cast（负数 hour 经 u32 回绕恒查 23 点，对齐 `local_hour_end_timestamp` 口径）；记忆块 upsert 注释与实现对齐（仅新内容更长才更新并清空 embedding）。
  - **去重与测试卫生（P2，可维护性）**：`matches_ignored_app_for_stats` 直接委托 privacy 版本（消除逐字重复的第二份匹配实现）；工具规范名查找表整响应构建一次不随 tool_calls 数放大；测试清理统一 drop 连接后删 db/-wal/-shm 三文件（Windows 句柄锁定致删除静默失败、CI 临时目录持续累积）。
- **GitLab 提交行数统计恒为空（P2，服务集成）**：`gitlab_sync_commits_for_date` 拉取提交列表时虽带 `stats=true`，但 GitLab v4 的 commits **列表**接口不返回 stats（该参数被忽略，仅单条详情接口 `/commits/:sha` 返回）——入库的 additions/deletions 全为 NULL，概览统计恒为 0、时间线「+x/-y」从不显示。修复：`fetch_today_commits` 拉完列表后按项目并发（8/批）补拉每条提交详情，总预算 10 秒，超预算或失败的提交保持无统计不影响其余数据（`fetch_commit_stats` 新增；upsert 按 sha 幂等覆盖，重同步即回填）。
- **客户端登录/注册恒报「账号或密码错误」（匿名请求租户过滤器，P1，服务端）**：Admin.Core `ClientAuthService` 的登录/注册查询受 FreeSql 全局租户过滤器影响——匿名请求没有租户上下文，过滤器生成 `TenantId IS NULL`，任何用户都查不到（注册时的默认租户查询、「OA只读」角色/部门查询同受影响，导致注册的用户租户回退为 1 且角色/部门绑定全部落空）。服务端 ClientAuthService 全部查询补 `DisableGlobalFilter(FilterNames.Tenant)`（fork commit 4251f0e9），部署后重新注册测试账号验证登录通过。
- **第八轮全量代码评审修复（行为保持，Rust 批次 + CI/构建批次 + TS 去重批次，共 24 项；另有 14 条涉行为语义列待确认、1 条 .svelte 待过目，见 `CODE_REVIEW_FINDINGS.md` 第八轮）**：
  - **敏感信息过滤正则中文邻接整条漏配（P2，隐私）**：`crates/core/privacy.rs` 四个敏感模式（银行卡/手机号/身份证/邮箱）的 `\b` 是 Unicode 词边界，CJK 汉字属词字符——"手机号13812345678请查收"这类中文紧邻形态整条漏配；而第七轮把采集/桌宠循环日志统一改走的 `filter_text` 正是本函数，脱敏实际失效。改与 ocr.rs 已修款一致：去 `\b` + 匹配后 ASCII 邻位手动校验，命中面只增不减，配中文邻接回归测试。
  - **截图保留天数无上限使清理路径 panic（P1，正确性）**：`storage.rs` 的 `date_naive() - Duration::days(retention_days)` 对 u32 任意值直通（配置同步/手改配置可绕过 UI 写入），超大值越过 `NaiveDate::MIN` 直接 panic——启动清理路径崩即启动失败，周期清理路径 panic 被吞成 warn 后清理永久失效。normalize 钳到 [0, 3650]（0=永久保留语义不变）+ 两处使用点兜底钳制，配回归测试（与已修 parse_temporal_range/storage_limit_mb 同族第三处）。
  - **节点身份并发 ensure 修复不完整（P2，正确性）**：第六轮的 rename 冲突回退只保证磁盘以先落盘者为准，但调用方仍返回自生成的"幽灵" device_id（Windows 回退路径真实可达），且三个并发入口的读-判-写本身无锁（Unix 后写覆盖先写）；补进程内写锁（与 localhost_api token 写锁同款）+ 落盘后重读返回。
  - **数据目录偏好文件非原子写（P2，正确性）**：`data-location.json` 直接 truncate+write，写中途崩溃留半截 JSON，下次启动解析失败静默回退默认数据目录，用户面对"数据全空"；改临时文件（pid+纳秒）+rename 原子写（同 node_gateway 模式）。
  - **实体自动归类 N+1 全表扫描持全局锁（P2，性能）**：`entity_classify_task` 对一批内每个 domain 实体各调一次无 LIMIT 的 14 天浏览器 URL 全量查询且在 AppState 锁内——大库一批最多 12 次全表扫描持全局锁，阻塞采集循环与全部 UI 命令；改整批一次扫描建 domain→ids 映射复用，行为（回溯修正的 id 集合）不变。
  - **debug 级日志明文记录标题/URL（P2，隐私）**：采集循环 4 处（跳过系统窗口/URL 预探测命中/恢复浏览器 URL/落库前刷新）与桌宠循环 1 处（前台切换）debug 日志未过隐私过滤——默认 info 级不落盘，但按排障指引开 RUST_LOG=debug 时邮件主题/URL 明文进日志；落日志前经 `privacy_filter.filter_text` 同款脱敏（热路径仅在 debug 级开启时才加锁过滤）。
  - **分类修正两条查询全列全量载入（P2，性能）**：`get_activities_by_normalized_app_name`/`get_activities_by_domain` 无 LIMIT 全列（含可能多 KB 的 ocr_text）一次性载入；改两段式——先取轻量 (id, app_name/browser_url) 列做精确筛选，再按 id 分块（500/批）取全列，结果集不变。
  - **macOS Vision OCR 脚本路径未转义（P2，健壮性）**：AppleScript 字符串对含 `"`/`\` 的路径（macOS 文件名均合法）编译失败、OCR 静默降级；补转义（与 Windows 侧 PowerShell 转义同口径）。
  - **Rust FFI 块 SAFETY 注释补齐（P2，可维护性）**：monitor/screenshot/idle_detector/screen_lock 七处平台 unsafe 块按第三轮 commands 侧同款补 SAFETY 注释（含提前返回路径的资源释放说明）。
  - **其余行为保持去重/注释修正（P2，可维护性）**：`avatar_engine` 两分支逐字重复的"文档"关键词表抽共享常量；`wecom_aibot` 内联 timeout 表达式改调共享 `report_generate_timeout`（其余 4 bot 均已用）；`monitor.rs` Electron 应用名提取的注释与变量名与代码实际（取 last 段）对齐。
  - **CI：签名私钥出进程参数（P1，安全）**：linux 容器打包 `docker run -e TAURI_SIGNING_PRIVATE_KEY="$TAURI_SIGNING_PRIVATE_KEY"` 把 base64 私钥整段写进 docker 客户端进程 argv，整个打包期间可被同机进程经 /proc 读取（linux runner 同机跑着生产 admin 栈）；改无值 `-e` 形式（docker 从作业环境读取变量值，argv 只留变量名）。version-bump 的 token `-c http.extraHeader`（进 git 进程 argv）改 `GIT_CONFIG_*` 环境变量；nightly-gate 取消 API 与 publish-release 上传循环的 curl `-H "PRIVATE-TOKEN: ..."` 改临时 header 文件 + `--header @file`。
  - **CI：version-bump 推送加固（P2，正确性）**：`git tag` 加 `-f`（runner 工作区跨流水线复用，job 在 tag 创建后、push 前被取消即残留本地 tag，此后同版本重跑恒 "already exists" 失败；版本号由内容推导，强移安全）；推送加 `--atomic`（分支与 tag 一次推送，避免中途失败 tag 先落产生指向 main 之外提交的 tag、触发必红的 tag 流水线）。
  - **CI：六个长作业补作业级 timeout（P2，CI）**：`build-release-windows` 与五个 `package-daily-*` 无作业级 `timeout: 3h`，只能吃本实例不生效的 `default: timeout`（实测死于 3600s）；同 `.rust-verify`/`.tauri-package` 已修款补齐。
  - **CI：version-bump 不再下载上游产物（P2，性能）**：needs 七项默认 `artifacts: true`，把三平台数十 MB 二进制下载进从不消费它们的 bump 作业；全改 `artifacts: false`，契约测试同步。
  - **CI：linux 打包容器 Node 20 → 22（P2，CI）**：builder 镜像装 Node 20 而 `engines: >=22`、其余全部 CI 作业用 Node 22——用户拿到的安装包与 verify 流水线测过的构建出自不同工具链；对齐 22。
  - **CI：release 构建脚本免 O3（P2，性能）**：workspace release profile 补 `[profile.release.build-override] opt-level = 1`，build script/proc-macro 不再按 O3+lto 全力编译，缩短三平台 release 编译时间。
  - **TS：行为保持去重（P2，可维护性）**：`appDisplay.ts` 相邻两函数重复定义的安装器词表抽共享常量；`categories.ts` 两个 store 四处逐字重复的"订阅即退出"快照读取抽 `snapshotStore` 助手（契约测试同步改钉新形态）。

- **打包版登录报「无法连接服务器」（混合内容拦截，P1）**：客户端登录/注册在浏览器里正常、打包应用必失败且 WebView2 连 TCP 都不发起——根因是 Tauri v2 Windows 打包版配置 `useHttpsScheme: true`（页面源为 `https://tauri.localhost`，属安全上下文），从 HTTPS 页面 fetch 纯 IP 的 `http://10.37.80.25:4191` 命中 Chromium 混合内容硬拦截（fetch/XHR 类混合内容直接抛 TypeError，无网络请求；`http://localhost` 因属潜在可信源豁免，故本地 API 不受影响，浏览器预览因源是 http 也不受影响）。修复：主窗口 `additionalBrowserArgs` 在 wry 默认参数之上追加 `--allow-running-insecure-content` 与 `--unsafely-treat-insecure-origin-as-secure=http://10.37.80.25:4191`（CSP `connect-src` 白名单仍然生效兜底）；`auth.ts` 网络异常分支补 console.warn 保留原始错误。UIA 驱动打包应用复现（零 TCP）后修复验证。
- **第七轮全量代码评审修复（行为保持，Rust 安全批次 + Rust 健壮性批次 + TS/CI 批次，共 21 项）**：
  - **云端日报截图路径可越界读任意文件（P1，安全）**：`analysis/cloud.rs` 对 DB 内 `screenshot_path` 直接 `screenshots_dir.join` 后读文件并 base64 外发 AI 端点——绝对路径整体替换/`..` 越界可把 data_dir 外任意文件发给配置的端点（与第一轮已修的 timeline.rs:124 同族，crates/core 因无 `validated_join` 被漏网）；新增 `safe_screenshot_path` 校验，配平台分组回归测试。
  - **"近N天"超大 N 打崩助手请求（P1，正确性）**：`parse_temporal_range` 对 N 无上限，`chrono::Duration::days` 在超大数值上 panic——问题文本经 orchestrator fast_answer 直达此处，"近99999999天"即可让一次助手请求异常中止；N 夹紧到 [0, 3650]，配回归测试。
  - **localhost API 一次瞬态 accept 错误即永久停机（P1，正确性）**：accept 循环对任何错误直接 `?` 传播退出——Windows 上客户端在 accept 前重置连接（WSAECONNRESET，端口探测/健康检查即可触发）会把整个本地 API 打成 stopped，MCP live 上下文/设备面直到重启前全灭；改为瞬态错误记日志+50ms 退避重试（连续 200 次失败仍按异常上报）。
  - **补传任务读取路径未校验（P2，安全）**：`remote_upload_backfill_task` 对 DB 内截图路径只查 `is_absolute` 即 `data_dir.join` 并上传远程存储，`..` 越界可把 data_dir 外文件传上用户网盘；复用 `validated_join`。
  - **采集/桌宠日志明文记录未脱敏窗口标题（P2，隐私）**：采集循环"应用切换"日志在隐私过滤之前、桌宠循环日志完全无过滤，邮件主题/IM 会话名等绕过隐私规则持久进日志文件；两处落日志前经 `privacy_filter.filter_text` 同款脱敏。
  - **localhost token 非原子写+并发竞态（P2，正确性）**：truncate+write 中途崩溃留半截 token 文件且 ensure 只对"缺失或空"自愈（所有客户端持续 401）；文件缺失时两个并发 ensure 各自生成不同 token 先后落盘，先返回方配置的客户端恒 401。改临时文件（pid+纳秒，Unix 保持 0600）+rename 原子写，并以进程内互斥串行化 ensure/rotate（同 node_gateway 第六轮模式）。
  - **macOS 非浏览器应用被查浏览器 URL（P2，正确性）**：macOS `get_browser_url` 缺 `is_browser_app` 门禁（Linux/Windows 入口均有）——名称含 arc/safari 等子串的应用（如 Archive Utility）每轮轮询 spawn osascript 去查 Arc 等真实浏览器的 URL 并错误归属到当前应用名下；入口补门禁。
  - **Firefox 会话文件每轮全量解析（P2，性能）**：recovery.jsonlz4 重会话可达数十 MB，macOS/Linux 每轮轮询（1.5s/0.5s）全量读取+mozlz4 解压+JSON 解析、无任何缓存（Windows 侧有 3s TTL 缓存），持续浏览时是常驻 CPU 热点；按（应用,标题）键加 3s 成功/2s 失败 TTL 缓存，与 Windows 口径一致。
  - **OCR 中文相邻 IP 漏过滤（P2，隐私）**：IP 过滤正则的 `\b` 是 Unicode 词边界，CJK 汉字属词字符——"服务器192.168.1.1"这类中文紧邻形态整条漏配、明文落库流向日报/MCP；改与银行卡号同款的匹配后邻位手动校验，配回归测试。
  - **时段对比负变化量显示 "0s"（P2，正确性）**：trend_comparison 总时长下降时 `format_duration_compact(负数)` 命中 `<=0` 早退返回 "0s"，模型收到 "总时长变化: 0s (-37.5%)" 误以为时长没变；新增 `signed_duration_compact`（负值取绝对值+负号），配回归测试。
  - **删除助手会话非事务（P2，正确性）**：`delete_assistant_conversation` 两条 DELETE 未包事务，第二条失败/崩溃留 0 消息空壳会话（同表 create/append 已是事务先例）；`unchecked_transaction` 包裹。
  - **Telegram bot token 未 trim（P2，正确性）**：守卫按 trim 后判空但原样传给 API，粘贴带首尾空白的 token 通过守卫后 401；入口 trim（与 wecom_aibot 的 id/secret 处理对齐）。
  - **前端死代码清理（P2，可维护性）**：删除 `CacheState.hourlySummaries`（全仓零读零写）与 `updateLastStreaming`（生产零调用，且 findIndex 取"第一条"streaming 消息与注释语义相悖，未来复用并发双流会更新错消息），TypeScriptMigration 契约与测试 fixture 同步。
  - **CI 部署链诊断与缓存修复（P2，CI）**：健康检查失败诊断 `docker logs workreview-admin` 指向不存在的容器名（compose 部署实际为 `yue-review-admin-1`），改 `docker compose logs`；删除 compose 未引用的 `workreview-net` 死步骤并更新头注释为 compose 拓扑；`cargo-llvm-cov` 安装根不在 PATH（预装缺失时每条流水线重复整编），补 `export PATH="$CARGO_HOME/bin:$PATH"`；linux 容器打包 npm 缓存从不被 GIT_CLEAN 排除的 `/build/.npmc` 改为持久缓存 `/build/.npm`（每次发版不再全量重下依赖）；删除 525df65 后无消费者的 ci-caches/linux22 死 mkdir。
  - **CI 长作业被 1 小时超时截断（P1，CI）**：`.gitlab-ci.yml` 顶部虽有 `default: timeout: 3h`，但本项目实例不生效——所有作业实际按项目默认 1 小时超时截断（`failure_reason=server_timeout_running`，两次精确死于 3600s：release 冷编译超线一次、mac 冷缓存 verify 超线一次）。给 `.rust-verify`、`.release-compile`、`.tauri-package`、`tauri-package-windows` 全部补作业级 `timeout: 3h`（作业级优先级最高，兼容旧实例）。
  - 另：`parse_openai_response` 过时的"Ollama 也用这个"注释更正。本轮 12 条涉行为/部署语义条目与 1 条仓库级事项（tauri-updater.key 私钥入库，建议专项轮换）记入 `CODE_REVIEW_FINDINGS.md` 第七轮待确认/仓库级清单。
- **Linux 安装包要求 GLIBC_2.39 被兼容门禁拦截（v1.1.74–78 发布失败根因，CI）**：linux-x64 runner 主机为 Ubuntu 24.04（glibc 2.39），直接构建的 deb/rpm/AppImage 主程序要求 GLIBC_2.39，被第六轮评审加入的 GLIBC 兼容门禁（上限 2.35 = Ubuntu 22.04 基线）连续拦截——门禁按设计工作，根因是构建环境超出基线。修复：Linux 包构建容器化——新增 `scripts/ci/linux-builder-22.Dockerfile`（ubuntu:22.04 + webkit2gtk-4.1 全依赖 + rust 1.97.1 + node 20，基镜像走 daocloud 源规避 docker.io 限流）与容器内打包脚本，`.tauri-package-linux` 作业改为构建器镜像内以 runner 同 UID 构建（`CARGO_HOME` 覆盖到项目内可写目录）、宿主执行 GLIBC 门禁与产物收集；CI 以 fork/上游最新 commit SHA 作克隆 ref 击穿构建缓存。过程中顺带修复：上游新增的根级/子目录 `pnpm-workspace.yaml` 导致子目录 `pnpm install` 报 packages field missing（构建前删除）、vite 8 产物嵌套路径 `dist/src/...` 未归并导致品牌替换与校验全挂（构建后归并到 dist 根）。
- **第六轮全量代码评审修复（行为保持，Rust 批次 + 前端批次 + CI 批次，共 17 项（run.ps1 两小项合并计）+ 1 项源码契约同步）**：
  - **Ollama 非流式响应套用 OpenAI 解析器恒得空答案（P1，正确性）**：`chat_ollama` 请求的是 Ollama 原生 `/api/chat`（顶层是 `message`、无 `choices`），却用 `parse_openai_response` 解析——所有非流式 Ollama 调用（桌宠主动开口、流式中断回退、无事件通道分支）内容恒为空、主动开口永久静默失效。新增 `parse_ollama_response` 按原生格式解析（`message.content`/`message.tool_calls`/`done_reason`，与流式装配器口径一致），配回归测试。
  - **思考型模型流式输出被思维链污染（P2，正确性）**：OpenAI 兼容流式装配器把 `reasoning_content` 增量无条件累积进 `content`——DeepSeek-R1/Qwen3 等模型先发思维链帧再发正文帧时，最终回答=思维链+正文拼接并进会话历史。改为思维链单独累积，仅当正文为空时在 finish 兜底（对齐非流式契约），流式打字增量照发；同步更新前端侧 Rust 源码契约测试。
  - **记忆搜索三个 FTS 检索面从未返回过结果（P1，正确性）**：`search_activities_fts`/`search_hourly_fts`/`search_reports_fts` 都把 FTS5 的 `rank`（bm25 分数，REAL 类型）按 `i64` 读取，遇小数 `FromSql` 失败后被 `filter_map(row.ok())` 静默丢光全部命中行，记忆搜索一直靠 LIKE fallback 兜底（仅 rank 恰为整数的退化情形偶尔工作）。rank 改按 `f64` 读取，补活动/小时摘要/日报三面命中回归。
  - **日报 FTS 检索同日多语言命中返回随机语言（P2，正确性）**：`search_reports_fts` 用 `GROUP BY d.date` 但 `content` 等均为裸列，SQLite 取值未定义——同日存有 zh-CN 与 en 两份日报且双双命中时正文语言随机漂移。附加唯一 `MIN(d.created_at)` 聚合，SQLite 保证裸列取自 created_at 最早那行（结果确定）。
  - **macOS 应用图标转换临时文件落共享 /tmp（P2，加固）**：sips 输出路径 `/tmp/app_icon_{app}_{pid}.png` 完全可预测，多用户机器上可被其他用户预置符号链接写穿；改用户级临时目录+纳秒随机后缀+文件名字符净化。
  - **背景图读取未校验路径（P2，加固）**：`get_background_image` 对 `config.background_image` 直接 `data_dir.join` 后读文件返回 base64——配置同步载荷写入绝对路径/`..` 即可把 data_dir 外任意文件读回前端；复用 `validated_join` 校验。
  - **macOS 浮窗时长被双倍记账（P2，正确性）**：`get_overlay_windows` 用 `dedup_by` 去重同名应用，但该 API 只折叠相邻元素——CGWindowList 按 z 序返回、同应用两个浮动窗口被其他应用隔开时去重失效，同一 tick 内浮窗时长记双倍；改按 app_name 全局保序去重。
  - **设备身份文件首启并发写入竞态（P2，正确性）**：写入用固定临时名 `with_extension("tmp")`——首次启动两个状态查询并发触发时 Unix 上后写者覆盖先写者（两个调用方拿到不同 device_id）、Windows 上 rename 目标已存在直接失败；临时名加 pid+纳秒，rename 冲突时回退采用已落盘身份。
  - **GitLab 提交作者邮箱入库未剥控制字符（P2，加固）**：fetch 侧 `strip_control_chars` 覆盖 title/author_name/project/branch 但漏掉 `author_email`，被劫持的自托管服务端可让入库行携带换行注入 markdown 结构；入库前补同一清洗（对合法邮箱恒等），配回归测试。
  - **阿语界面报告小节编号用中文数字（P2，本地化）**：`sectionNumberPrefix` 只特判 `en`，`ar` locale 落入中文数字分支；改 CJK 白名单（zh-CN/zh-TW），其余语言一律西文数字。
  - **assistant `setMessages` 保留 slice(-40) 静默截断（P2，正确性）**：生产零调用的死入口上残留第三轮从 `appendMessage` 修掉的同类截断——未来任何调用方装入超 40 条的会话即静默丢历史；去掉截断（持久化已有统一兜底）。
  - **图标缓存逐项落盘阻塞主线程（P2，性能）**：每个图标请求完成（含失败分支）都同步执行整包 JSON 序列化+localStorage 写（单项可达上百 KB base64），时间线滚动预热一批图标时卡顿累积；落盘改 100ms 节流（与 store 刷新同款），测试同步为等待首次落盘。
  - **GLIBC 兼容门禁永不生效（P1，CI）**：`check-linux-glibc.sh` 放在 `.tauri-package-linux` 的 `after_script` 里，GitLab 不以 after_script 退出码判定作业成败——门禁 `exit 1` 只打印 ❌ 作业仍绿，不兼容产物照常流入 publish/发布；移入 `script` 末尾（`artifacts: when: always` 仍收集产物便于排查）。
  - **publish-release 补签私钥失败路径残留 /tmp（P2，CI）**：minisign 补签循环里 base64 解码后的私钥临时文件只在成功路径 `rm`，`set -e` 下签名失败即残留共享 runner；补 EXIT trap 兜底。
  - **reinstall.sh 后设 trap 覆盖先设 trap（P2，本地脚本）**：GNOME 扩展段的 `trap 'rm -rf "$TMP_FWD"' EXIT` 整体替换下载段的 `trap 'rm -f "$DEB"' EXIT`——全新机器走下载路径时下载的 .deb 退出不再清理；合并为单一 `register_cleanup`/`on_exit_cleanup` 注册表统一清理。
  - **杂项（tests/e2e）**：`tauri-mock.ts` 删除 `get_ai_providers` 重复键（108 行死值被 116 行 last-wins 覆盖）；`installer-isolation/run.ps1` 下载补 `-f`（404 时不再把 HTML 错误页写进 zip）、超时报错 `$Args` 改 `$ArgList` 修正变量。
- **第五轮全量代码评审修复（行为保持，Rust 批次 + CI 脚本批次）**：
  - **钉钉 webhook 出站重定向绕过 SSRF 白名单（P1，安全）**：回复消息用的 HTTP client 未设重定向策略（reqwest 默认跟随 10 跳），`sessionWebhook` 域名白名单只校验初始 URL——`*.dingtalk.com` 上任一端点 30x 重定向即可把请求发往白名单外地址，307/308 还会原样重发 POST body（即整段日报内容外泄）。出站改用 `redirect(Policy::none())` 专用 client（白名单语义本就是"只许直连钉钉"；正常路径钉钉直接 200，行为不变）。
  - **飞书 token 缓存锁中毒放大为整条回复通道失效（P2）**：`TOKEN_CACHE.lock().ok()?` 把 PoisonError 静默吞成 None——任何一次持锁 panic 后租户 token 永久获取失败、飞书回复全灭且无日志、重启前不可自愈；改为中毒时 `into_inner()` 恢复（缓存仅是加速器，语义不变）。
  - **飞书 message_id 未经编码直接进 URL path（P2，加固）**：事件里的 message_id 若含 `/?#` 等字符可改写回复请求的 path/query 结构；补 path 段百分号编码（飞书真实 om_ id 全在保留集内，编码后恒等），配单测。
  - **实体分类模型调用传输层失败全静默（P2，可诊断性）**：`classify_entities_with_model` 的 client 构建 `.ok()?` 与请求发送 `.ok()?` 均无日志（只有 HTTP 非 2xx 分支有 warn），DNS/连接故障时语义分类静默消失无从排查；两处各补一条 `log::warn`（降级语义不变）。
  - **管理后台品牌化 sed 毁掉 viewport meta（P1，admin-core 镜像）**：`index.html` 的品牌替换用 `content="[^"]*"` 全量匹配——viewport、X-UA-Compatible 的 content 属性一并被刷成品牌文案，移动端布局失配（POC 桌面端测不出）。收窄到 title/keywords/description 三个目标，并加"viewport 必须原样保留"的构建期守卫。
  - **bump-version 对 Cargo.toml 仍全量替换（P2，CI 脚本）**：第四轮修掉 JSON 侧"依赖恰好 pin 成当前版本号被连坐升版"后，`replaceCargoToml` 仍是文件级 split/join 同族残留；改为 `[package]` 段内锚定（段内 version 与全局版本不一致时直接报错），配回归测试。
  - **codesign 脚本私钥残留共享 /tmp（P2，本地脚本）**：`setup-codesign.sh` 生成的签名私钥落 `/tmp/wr_key.pem`，清理只在成功路径末尾——任一步失败即带私钥残留多用户可读目录；加 `umask 077` 收紧新建文件权限 + `trap ... EXIT` 全路径清理。
  - **reinstall.sh 对 DEB 路径 eval 执行（P2，本地脚本）**：`run()` 用 `eval` 执行含 `$DEB` 的安装命令，当前目录被投放 `Work_Review_a;$(…).deb` 这类文件名即以 sudo 权限执行任意命令；被污染路径改走 argv 版 `run_argv`（不经 shell 再解释），其余纯常量命令保持原样。
  - **admin 镜像按裸 SHA clone 必现失败（P1，CI）**：27ba788 把 admin-core 源改为「以最新 commit SHA 为 ref」击穿层缓存，但 `git clone --branch` 只接受分支/标签名，裸 SHA 直接报 `Remote branch not found`——此后每条 main 流水线的 `deploy-admin-core-yx` 全部失败（version-bump 因 `when: always` 照常升版打 tag，掩盖了红流水线）。改为 `git init` + `git fetch --depth 1 origin <SHA>` + `checkout FETCH_HEAD`（本自托管 GitLab 已开 allowAnySHA1InWant，实测可按 SHA 浅取），上游 gitee 未开该配置时回退 fetch master，本地直接构建 Dockerfile 默认参数仍可用。

- **第四轮全量代码评审修复（行为保持，Rust 批次 + CI 脚本批次）**：
  - **Tauri 能力授权裁剪（P1，安全）**：`capabilities/migrated.json` 授予了前端从未使用的能力——fs 插件的读/写/删/改名/建目录（scope 还含 `$RESOURCE/**`，Windows 上即安装目录）与 `shell:allow-execute`（命令 `open`，参数验证器仅要求非空白）。前端既无 `@tauri-apps/plugin-fs` 依赖也从无 `Command.execute` 调用，这些授权只构成"renderer 被攻陷后"的提权面：可写/删安装目录内文件、可借 PATH 解析执行同名 `open` 程序。裁剪为仅保留实际使用的 `shell:allow-open` 与 `fs:default` 基线。
  - **GitLab 提交文本控制字符剥离（P2，安全加固）**：提交标题/项目路径/分支名来自 GitLab API（自托管实例被劫持时即不可信），此前未剥控制字符直接拼进日报 markdown——标题携带 `\n## …` 即可注入伪造小节/列表行。现于拉取构造条目处统一剥离（控制字符折叠为空格），真实 git 提交标题不含控制字符、对正常输入输出不变；配回归测试。
  - **本地 API 请求头扫描 O(n²)（P2，性能）**：`read_request` 每收到一个 chunk 都对整个已积累缓冲从头重扫 `\r\n\r\n`，慢速滴流客户端可低成本放大单连接 CPU 消耗；改为记录已扫偏移增量续扫（保留末 3 字节处理跨 chunk 边界），行为不变。
  - **锁屏探测与本地 API 吞错补日志（P2，可诊断性）**：Linux 锁屏检测的 loginctl/pgrep/dbus-send 三探测全部 `if let Ok` 静默吞错，工具缺失时锁屏检测实际失效且与"未锁屏"不可区分（改为全部无法启动时记一次性告警）；localhost API 运行时异常退出的 `record_runtime_error` 无任何日志，锁中毒时错误彻底无踪迹（开头补 `log::error`）。
  - **版本自增脚本两处（P2，CI 脚本）**：`replaceVersionField` 此前虽已锚定 `"version": "x.y.z"` 键值对，但仍为文件内全量替换——任一依赖恰好 pin 成当前应用版本号仍会被连坐升版（npm ci 失败或依赖被静默改版本），改为只替换根级第一处；`bumpVersion` 逐文件"读→变换→写"无回滚，中途抛错（如 Cargo.lock 缺失）会留下"package.json 已升版、其余四个还停旧版"的半损坏工作区，改为先完成全部变换、最后统一写盘。两条均配回归测试。
  - **发布自检清单断言名统一（P2，CI 脚本）**：更新清单 HTTP 200 但 body 非 JSON（包仓库返回 HTML 错误页）时，"可下载"断言先记 ✅、解析异常又在另一个断言名下记 ❌，真实失败原因被掩盖；拆分出"更新清单为合法 JSON"独立断言，catch 分支断言名统一带上 URL。
- **时间线页 GitLab 提交合并排序方向错误（P2，前端 TS）**：v1.1.58 把提交并入时间线时用了升序合并，而时间线页既有约定是「最新在最上」（`prepareTimelineActivities` 降序）——合并后整页被翻转成最旧在最上，视觉上变成"提交完才轮到截图时间线"。合并排序改为延续页面降序约定（最新在最上），同刻提交行仍在活动行之前；日报末尾的活动时间线表格保持从早到晚升序不受影响。合并排序测试同步改为降序断言。
- **publish-release 误删本版产物导致 v1.1.55 发布失败（P1，CI）**：为防"上一版本产物在共享持久 workspace 残留"加的 `rm -rf target/release/bundle target/*/release/bundle` 放在了 publish-release 脚本里——而该作业的产物由 runner 在**脚本执行前**解压到同一路径，整目录删除把刚解压的本版产物全部删光，`generate-updater-json.mjs` 找不到 darwin-aarch64 产物直接失败（v1.1.55 为该改动到达远端后首个 tag 流水线，v1.1.54 发版时改动尚未推送故幸免）。改为：①按版本定向清残留——`find` 只删文件名不含当前版本号的 bundle 文件（兼容 rpm 连字符命名），保留通用名的 macOS updater 产物（*.app.tar.gz/.sig，本次解压刚覆盖）；②真正堵住原意图防的"新包配旧签名"——macOS `.app.tar.gz.sig` 早于对应 tar.gz 的（mac runner 丢新 sig 时残留的上一版本旧 sig）删除，交给既有补签循环重签。新增契约测试锁定：publish-release 段内禁止整目录盲删、定向清理与旧签名删除必须在位。
- **第三轮全量代码评审修复（行为保持，Rust 批次 + TS/CI 批次）**：
  - **SSRF 护栏漏段（P1，Rust）**：Agent 联网工具的内网护栏 `is_non_public_v4` 不拦 0.0.0.0/8 非全零地址（Linux/BSD 内核把该段目的地按本机处理，`http://0.16.32.48:47831/` 可直达持有工作数据的本机 localhost API），也漏 100.64.0.0/10（CGNAT/Tailscale tailnet）、198.18.0.0/15、组播与 240/4 保留段；补齐判定并新增 9 组回归（含 IPv4-mapped IPv6 形式）。
  - **存储上限 0 值删光全部截图（P1，Rust）**：`storage_limit_mb=0`（配置同步/手改配置可绕过设置页下限直通入库）会让周期清理把上限算成 0 字节、"删到低于上限"的循环永不满足，从最旧到最新逐个删光全部截图目录；`normalize` 阶段钳到 ≥256（与设置页下限对齐），新增回归测试。
  - **桌宠浮动窗时长低估一个数量级（P1，Rust）**：浮动窗口（PiP 画中画）时长按单次轮询间隔（≈1s）记账，且该检测块只在截图发生的轮次执行到（未截图轮次早已 `continue`），macOS 上 PiP 挂 30s 只累计 ≈1s；改为与主活动同口径按"距上次采集的实际间隔"记账。
  - **飞书机器人无法回复任何消息（P1，Rust）**：「回复消息」API 请求体字段名写成 `content_type`，飞书官方必填字段为 `msg_type`——进度提示与最终回复全部被服务端校验拒绝，且进度分支静默吞错；修正字段名。
  - **发布产物"新包配旧签名"残留面（P1，CI）**：publish-release 跑在共享持久 workspace，产物经 artifact 解压获得（只覆盖同名、不删旧文件），bundle 清理此前只加在打包作业上——上一版本的通用名产物 `Yue-Review.app.tar.gz/.sig` 残留，mac runner 再次偶发丢 `.sig` 时补签循环见旧签名非空即跳过，更新清单会变成"新包配旧签名"、客户端验签永久失败；publish-release 作业开头补同款 bundle 清理，产物自检脚本补上头部注释声称却未实现的"清单签名与本地 .sig 逐字比对"，并为自检令牌补齐与同作业 glab/curl 一致的 `CI_JOB_TOKEN` 回退。
  - **长会话发消息即截断可见历史（P2，前端 TS）**：助手 store 的 `appendMessage` 带 `slice(-40)` 上限而 `setConversation` 全量装入不设上限互相矛盾——切到超过 40 条的 SQLite 长会话后发一条消息，内存态即被静默裁到 40 条（旧消息"眼前消失"）且残缺列表被写回 localStorage；改为内存态不截断，localStorage 容量兜底统一收口到持久化层（裁最近 40 条），新增回归测试（`assistantPersist.test.ts`）。
  - **其余修复**：企微 AI 机器人长连接建立无超时（防火墙 DROP 时挂 20~120 秒，包 10s 超时）；localhost API token 掩码按字节切片在手改多字节内容时 panic（改按字符截取 + 回归）；xdotool 窗口几何解析遇到无 `=` 的行整条失败退化为整屏截图（改为跳过）；助手会话新建与超额清理两条 DELETE 未包事务、崩溃留孤儿消息（包同事务）；`get_activities_in_range` 的 `(?1 IS NULL OR …)` 写法令索引失效每次全表扫描（按实参拼确定性条件）；OpenAI 兼容端点漏带 `/v1` 时的候选地址回退对所有 4xx 一律中止、永不尝试 /v1 地址（404/405/410 改为换下一候选）；删除活动记录的截图文件路径未过 `validated_join` 即删除（复用校验）；日报自动导出是三个导出入口中唯一不做敏感目录校验的（补组件校验，失败仅记日志）；macOS 应用图标缓存硬编码共享 `/tmp` 可被同机其他用户污染（改用户级临时目录）；无模型问答不再白付实时上下文采集（按编排器同款 has_model 判定惰性构造）；Windows/macOS FFI 的 unsafe 块补 SAFETY 注释；version-bump 的 `git clean -ffdx` 不再抹掉 runner 增量编译缓存（排除 target/.cargo/.npm）；`generate-updater-json.mjs` 的 CLI 入口判定改 realpathSync（软链/大小写照面时此前会静默退出 0 不产清单）；`bump-version.mjs` 的版本替换改锚定 `"version": "x.y.z"` 键值对（全量替换会误伤恰好同版本号的依赖）；更新签名私钥临时文件在 shell 侧加退出清理（PowerShell 侧保持原样——该文件被后续打包步骤真实消费）。

### 新增
- **本地文件监控（概览页「文件活动」卡片 + 设置页「文件监控」标签）**：设置页新增「文件监控」标签——可启用监控、配置多个递归监听的工作文件夹（支持系统目录选择器）与自定义排除规则（目录段名或 `*` 通配文件名）。Rust 侧基于 notify（debouncer 2 秒防抖，同路径同类型事件合并）监听新建/修改/重命名/删除四类事件，只记录路径、事件类型、时间戳、文件大小四类字段（绝不读取文件内容），入库 `file_events` 新表。概览页新增「文件活动」整行卡片：区间事件总数 + 最近 20 条具体文件事件列表（类型徽标/文件名/目录/时间），随日期区间联动并每 30 秒自动刷新；未启用时整卡隐藏，区间无事件显示占位。v1 第一版不进入日报 AI 分析。
- **概览页新增「GitLab 提交」统计卡片（按项目聚合）**：配置 GitLab 后，概览页底部新增整行卡片，按项目显示所选日期区间内的提交次数与代码行数变更（`+新增` 绿色 / `−删除` 红色），底部合计行汇总总提交数与总行数；项目列表按提交次数倒序，主文案显示路径末段（悬停展示完整路径）。行布局与「应用使用」同款：仓库名 + 下方提交次数，中间为按提交数占比的彩色进度条（以提交最多的项目为满格，配色与应用使用柱色一致），右侧为 ±行数变更。**仓库图标优先取 GitLab 项目头像**（`/api/v4/projects/:id/avatar` 端点，PRIVATE-TOKEN 可取字节；web 上传路由对令牌认证一律 401 不可直接 fetch），Rust 侧按魔数判 MIME 转 data URL（webview CSP img-src 不放行远程图片），随提交补采并发拉取（8/批、预算 8s）并落 localStorage 下次秒出；仓库未自定义头像时保持默认 GitLab 图标。数据口径与时间线/日报一致（`gitlab_commits` 表聚合，additions/deletions 缺失按 0 计），进入概览或切换日期区间时自动刷新，今天每次会话后台补采一次（不阻塞首屏）；未配置 GitLab 时整卡隐藏，区间内无提交时显示占位说明。核心改动：新增 `get_gitlab_project_stats` 聚合命令（SQL GROUP BY，配区间/NULL 统计回归测试）与前端展示纯函数（路径末段/行构建/合计，配单测）。
- **客户端注册/登录（强制登录 + 离线宽限）**：桌面端接入 Admin.Core 账号体系——新增 `/auth` 登录/注册页（双页签：登录=OA 账号+密码；注册=账号/姓名/密码/确认密码/部门（可选），密码至少 6 位，注册成功服务端直接签发 token 自动登录）。启动时无本地凭证强制跳转登录页；token 过期但本地有缓存时进入离线宽限（顶部横幅提示，可继续使用本地功能并一键重新登录）。服务端为 Admin.Core `ClientAuthService`（`/api/admin/client-auth/login|register`，免验证码，密码按用户 PasswordEncryptType 支持 BCrypt/MD5/PasswordHasher 逐类型验证；注册新用户自动绑部门与「OA只读」角色）。客户端 `auth.ts` 负责凭证存储（localStorage）、JWT exp 解析（无 exp 兜底 12 小时）与认证门判定（login/grace/allow 纯函数配单测）；设置页新增「账号」区块（当前用户/服务器地址修改/重新登录/退出登录）；Tauri CSP `connect-src` 放行 `http://10.37.80.25:*`；i18n 四语言补 32 键。浏览器端到端验证：错误横幅、过期横幅、账号预填、守卫重定向均过。
- **管理后台 OA 账号打通（Supabase 部门/员工/凭据同步 + 本地 bcrypt 登录）**：Admin.Core 新增 OA 同步——直连 Supabase Postgres 拉取 `departments`（→ Org）、`auth.users`（bcrypt 哈希）、`user_profiles`/`staff_directory`（OA 账号/姓名/部门/职位），预建 Admin.Core 登录账号（UserName=OA 账号、PasswordEncryptType=Bcrypt）并绑角色（role=admin → 超级管理员，其余 → 新建「OA只读」角色）；OA 用户用相同账号密码登录管理后台，本地 BCrypt 比对验证，运行时不依赖 Supabase。日拉后台任务 + 管理端手动触发端点；compose db 暴露内网 15433 端口；admin 容器加入 supabase_default 网络（直连通道），yue-review db 容器设唯一 hostname 规避 `db` 别名跨网络歧义。
- **CI/CD：部署改用 docker compose（yue-review stack）+ 版本递增规则细化**：`deploy-admin-core-yx` 改为 `docker compose -p yue-review -f admin-core/compose.yaml up -d --wait`（admin + 专用 Postgres 入 stack，替换原 docker run 方式）；version-bump 新增 docs-only 跳过——自最新 tag 以来 `src/`、`src-tauri/`、`crates/` 均无变更（仅文档/CI 等）时不递增版本、不打 tag、不触发三平台打包，作业日志记录「版本保持 v1.1.79，构建计数 v1.1.79+N」后直接结束，省下无代码变更的整轮发版。
- **管理后台上线（Admin.Core 中台，部署于越秀服务器）**：work-review 新增 `admin-core/` 容器化资产与 CI/CD 部署作业（`deploy-admin-core-yx`，main 全绿后自动部署到 tags yx/deploy 的 runner，即 10.37.80.25）。一体化镜像 = .NET 10 Host + admin.ui 静态资源（品牌化为「工作回顾管理后台」），沿 bim5 POC 结论：FreeSql 换 PostgreSQL provider、SPA 回退补丁、pnpm 钉 9.15.9。数据面为独立 Postgres 容器（workreview-db，专用口令）+ 生产配置挂载（`~/workreview/admin-config/`，JWT 密钥独立生成，密钥不进仓库与镜像）。管理界面 http://10.37.80.25:4191（内置种子管理员 admin/123asd，**首登请改密**）。后续在此基座上扩展 Supabase 用户单向同步与 work-review 数据接收 API（见管理后台里程碑 M2）。
- **测试覆盖率提升（第一批：skills-engine / policy）**：skills-engine 从 0% 提升到 94.90%（30 条行为测试：注册表与内置技能契约、执行器管道全程（模板解析/AI 转换包裹/四类透传转换/六种查询变体/三种安全与状态失败门禁）、自适应学习四策略数学与信号过滤、回滚重建、统计增量平均）、core/policy.rs 从 0% 提升到 98.62%（8 条：四类来源的放行/拒绝矩阵、未注册技能只读兜底、审计日志截断与来源分组统计、条目 id 单调）。workspace 行覆盖率 49.82% → 57.21%。全部为行为断言（输入→输出→状态），无凑数用例。
- **流水线作业页显示测试覆盖率**：frontend 作业改用 Node 内置测试覆盖率（`--experimental-test-coverage`，`npm run test:frontend:coverage`），经 GitLab `coverage:` 正则提取「all files」行覆盖率后显示在流水线作业页面；`rust-verify-linux` 的 `cargo test` 升级为 `cargo llvm-cov --workspace --all-targets --summary-only`（同轮完成全量测试与覆盖率汇总，cargo-llvm-cov/llvm-tools 首跑自举后缓存在 runner 持久 HOME），TOTAL 行覆盖率同样经正则上屏。覆盖率单点在 linux 作业产出（mac/windows 的 cargo test 保持全量测试职责，不重复产同一个数字）；本地可用 `npm run test:frontend:coverage` 与 `cargo llvm-cov -p <crate> --summary-only` 复查。
- **时间线加入 GitLab 提交活动**：时间线页与日报末尾的「活动时间线」现在会显示 GitLab 提交行，与应用使用记录按时间交错排序（项目路径作应用列、提交信息作内容列、附 +x/-y 统计，时长列为 —）。新增 `gitlab_commits` 表持久化提交（按 sha 幂等覆盖）：生成日报拉取时自动入库，时间线页打开/切换日期时后台补采（每日期每次会话仅一次，口径与日报一致——token 账号 ∪ 纳入工作的作者、项目允许列表、15 秒超时静默失败），历史日期从入库之日起可回看；核心时间线渲染改为活动+提交合并排序的纯函数并配单测，前端合并逻辑与守卫测试同补。
- **日报新增固定位置的「GitLab 今日提交」独立小节**：配置 GitLab 后，日报目录中新增与其他数据小节平级的「GitLab 今日提交」，位置固定锚定在「网站访问明细」之后、「按小时活跃度」之前（实现为新的 `GITLAB_TODAY` 段落积木：组装后强制重锚到按小时活跃度之前，AI 编排序/钉选不覆盖该锚位；用户显式钉选 GITLAB_TODAY 时尊重钉选置顶）。内容统一为确定性提交列表（每项目 20 条、全局 200 条，超限提示），不再依赖 AI 生成：云端 AI 提示词从"要求 AI 单列小节"改为"小节已由模板生成，AI 分析可自然引用提交内容、但不得单列小节或逐条复述"（AI 失败回退时也不再重复补小节）。**行为变化**：已配置即始终显示小节——无提交时显示「今日无 GitLab 提交记录」占位说明（此前整节省略）；拉取失败时失败说明内嵌在小节内（此前是文末一行备注，且目录不可见）。小节支持日报页段落钉选/隐藏（四语言段落名 GITLAB_TODAY 已配齐）；历史日报不受影响。核心改动下沉 `report_blocks` 块系统与 `gitlab.rs` 纯函数，放置锚位、编号顺延、空占位/失败占位、AI 指令措辞均配单测。
- **GitLab 纳入工作的作者支持从仓库贡献者勾选**：设置 → GitLab 集成的「纳入工作的作者」在保留手输文本框的同时，新增「从仓库选择作者」选择器（与仓库选择器同款弹窗：搜索/全选/清空/保存）。候选来自「项目过滤（允许列表）」选中仓库的贡献者接口（默认分支、按提交数倒序，与日报统计口径一致）；允许列表为空时回退到全部有权限仓库（仓库数超 200 按路径排序取前 200 并提示）。同名多邮箱每人每个邮箱一行，勾选保存只把**邮箱**追加进名单（与后端 author_email 忽略大小写精确匹配对齐，重名不误伤），已写入名单的邮箱自动预勾选、保存为追加语义（不删除已有条目），候选行展示提交数并标记当前账号。后端新增 `gitlab_list_authors` 命令（并发 8 拉取、逐仓库失败跳过仅记日志、整体 25 秒超时结构化失败），聚合与去重逻辑下沉 core 纯函数并配单测；前端选择器纯逻辑（过滤/勾选/合并/预勾选）与设置页接线、四语言文案均配测试。
- **更新链路测试（层次化发布门禁）**：修复“更新后版本号不变”缺陷（发布时安装包选择被 runner 构建目录里累积的旧版产物污染，v1.1.47 起的更新清单全部指向 v1.1.46 安装包，用户每次“更新”都在重装 v1.1.46）过程中新增三层测试，防止同类问题再次溜进发布：
  - **发布产物端到端自检**（`scripts/verify-update-artifacts.mjs`，publish-release 上传完成后运行）：以下载方视角重新拉取更新清单与安装包，校验清单版本一致、各平台下载 URL 与本地上传文件一致、远端安装包 SHA256 与本地一致、签名字段非空；当前为**观察模式**（失败只打 ⚠️ 不阻断，因安装包按版本选择的根因修复尚未落地），根因修复后升级为阻断发布；
  - **生成器不变式单测**（`generate-updater-json.test.ts`，6 用例）：清单 URL 文件名含版本号时必须与发布版本一致（本次事故形态的直接不变式）、签名/notes 透传、必备平台缺失抛错等；
  - **编排契约测试**（`ciUpdateChain.test.ts`）：防自检接线与生成器观察模式被删改；另有一条按版本选择安装包的硬断言以 skip 形式预置，根因修复落地后移除 skip 即生效。

### 变更
- **CI/CD 流水线提速（每次 push 从 1.5-2 小时降到 30 分钟级）**：
  - **三平台完整安装包改为夜间定时产出**（GitLab Schedules，分支 main，每天 02:00）：日常 push 不再产安装包——与自动发版的 Release 资产完全重复，是最大耗时来源；「latest」固定下载链接仍有效（自动指向最近一次夜间成功构建，保留 30 天），正式发版照旧走 tag 发布（资产永久保留）。**夜间流水线带提交门禁**（`nightly-gate`）：自最新发布以来没有新提交时自动取消整条流水线（不重复打包相同产物），有新提交才产出安装包；三个默认启用的打包作业均依赖门禁与各自平台的验证。arm64/macos-x64 可选平台继续由 `PACKAGE_*` 变量控制。

### 修复
- **第二轮全量代码评审修复（CI 编排 / Rust 后端 / 前端，行为保持）**：
  - **夜间产包流水线永不运行（CI）**：安装包改为仅夜间 schedule 产出时 `workflow.rules` 未放行 `schedule`——定时流水线在创建阶段即被过滤，五个 `package-daily-*` 作业永不执行、「latest」固定下载链接随之失效（存量 artifact 30 天过期后 404）；现补上 schedule 放行规则。同步修复两处 macOS 日常包作业 artifacts 配置成对误删（arm64 缺 `name`、x64 缺 `when: always`，与 Windows/Linux 作业不对称）；`publish-release` 的包上传 curl 补齐与 glab 认证同款的 `CI_JOB_TOKEN` 回退（此前令牌缺失时发空 token，全部资产上传 401）；两个 CI 契约测试（`ciArtifacts`/`ciVersionFlow`）从「每次 push 产包 + `files:` 缓存 key」旧设计对齐到「夜间 schedule + 固定 npm 缓存 key」新设计。
  - **语义记忆检索 Top-K 逻辑反转（P1，Rust）**：`search_memory_chunks_semantic` 的淘汰阈值判断用 `top.last()`、超限弹出用 `pop()`，而内部数组按分数升序维护——高分命中被跳过或刚插入即被弹出，实际返回"扫描顺序最早的 limit 条"而非最相似 Top-K；且以升序喂给下游 RRF 融合（rank 0 权重最高给了最差命中），双重劣化语义检索质量。现改为 `top[0]`（当前最差）做门槛、`remove(0)` 淘汰、返回前翻转为降序（最优在前），并修正 `binary_search_by` 比较器方向（原写法与标准约定相反，插入点错乱导致数组不保序），新增回归测试。
  - **桌宠分类缓存对自定义分类不失效（P1，Rust）**：规则缓存签名只哈希 `app_category_rules` 不含 `custom_categories`，用户单独改/删自定义分类后缓存永不刷新、桌宠一直用旧分类（与上一轮已修的"只哈希末条规则"同源残留）；现把自定义分类一并纳入哈希（`CustomCategory` 补 `Hash` 派生）。
  - **时间线导出任意路径写入（P1，Rust）**：`export_timeline_json` 对前端传入的目标路径直接写文件，注释声称"必须以 .json 结尾"但无任何校验、父目录也不校验（与上一轮已修的日报合并导出属同类旁路）；现抽出 `validate_timeline_export_target` 限定 `.json` 后缀（大小写不敏感），父目录复用 `validate_export_dir` 拒绝隐藏/敏感目录（防写入 `.ssh`/`LaunchAgents` 等）。
  - **助手恢复出幽灵"生成中"气泡（P1，前端 TS）**：流式期间助手 store 每 500ms 防抖持久化，应用在生成中途退出/崩溃后，恢复出的消息保留 `streaming:true` 且无任何代码复位，助手页出现永远"生成中"的幽灵气泡（思考光标常驻）；`loadState` 恢复路径现强制 `streaming:false`（`sending`/`sendingRequestId` 本就复位），新增带伪造 localStorage 的恢复路径回归测试。
  - **HTTP 客户端构建失败静默失超时（P2，Rust）**：`GitlabClient` 与云端/本地/AI 三个分析器的 `Client::builder()...build()` 失败时静默回退为无超时客户端（`unwrap_or_default`/`Client::new()`），一旦发生即架空超时保护、把快速失败变成永久挂起；现构建失败直接 `expect` panic 暴露环境问题（TLS 后端不可用时后续请求本就全部失败）。
  - **小修**：GitLab 仓库列表排序改 `sort_by_cached_key`（原每次比较重复 `to_lowercase` 产生 O(n log n) 次字符串分配）；`integration_instruction` 四语言文案里的小节标题改为从 `gitlab_section_title` 注入（单一来源防漂移，输出字节不变）；删除 `agent/tools.rs` `html_to_text` 的死变量 `close`（上一轮清单误报"已修复"，实际未删，本次实际删除）；修正 `avatar_scale` 字段注释与 clamp 实现不符（下限 0.4 而非 0.7）。

### 变更
- **CI/CD 流水线提速（目标：每次 push 从 1.5-2 小时降到 30 分钟级）**：
  - **三平台完整安装包改为夜间定时产出**（GitLab Schedules，分支 main）：日常 push 不再产安装包——与自动发版的 Release 资产完全重复，是最大耗时来源；「latest」固定下载链接仍有效（自动指向最近一次夜间成功构建，保留 30 天），正式发版照旧走 tag 发布（资产永久保留）。arm64/macos-x64 可选平台继续由 `PACKAGE_*` 变量控制。
  - **修复缓存每次发版必然失效的问题**：cache key 此前含 `src-tauri/Cargo.toml`，而 version-bump 每次发版都改写其版本行，导致 key 每条流水线必然漂移、三平台全量冷启动。现 cache 只承担 npm 下载缓存（固定 key `npm-cache-v1`）；Rust 的 registry 与编译产物改由 `GIT_CLEAN_FLAGS` 排除清理、常驻 runner 构建目录，跨流水线增量编译生效（cargo 指纹保证正确性）。注意：runner 构建目录会增长（单机数十 GB），偶发手动清理即可；依赖/工具链大版本调整时手动把 cache key 升版。
  - **发布（tag）流水线裁剪重复验证**：tag 指向的提交与刚在 main 全量验证过的提交仅差版本号，打包作业不再重跑 fmt/check/clippy/test 全套，只做构建打包；release-preflight（版本一致性 + 前端测试）保留。
  - **结构性精简**：删除 verify 作业里独立的 `cargo check`（clippy 为其超集，省一遍全量编译 ×3 平台）；compile 三平台改为按平台 `needs` 链式并行（慢平台不再阻塞其他平台）；删除打包作业里与 tauri `beforeBuildCommand` 重复的显式 vite build。

### 新增
- **AI 日报 GitLab 提交单列小节并按类型归纳**：配置了 GitLab 且当日有提交时，AI 日报现在被要求单列「GitLab 今日提交」小节（与其他小节同级），从提交信息归纳当日代码工作——实现了什么功能、修复了什么问题、进行了哪些重构或配置调整，逐条概括；不再混入进展正文，也不逐字罗列原始 commit message；防编造约束保留（不得虚构列表之外的提交）。本地（无 AI）回退日报的提交清单小节行为不变。

### 修复
- **自动更新"更新后版本号不变"根因修复（配合已上线的更新链路三层测试）**：publish-release 的安装包选择此前用裸 glob（`nsis/*.exe | head -1`）——runner 构建目录跨流水线持久后累积了多个版本的安装包，`ls` 字母序选中了最旧的 v1.1.46 安装包，导致 v1.1.47 起所有版本的更新清单都指向 v1.1.46（用户每次"更新"都在重装 v1.1.46）。现根治：安装包选择、minisign 重签循环、上传与资产链接循环全部按发布版本（`*"$VERSION"*`）过滤；七个打包作业在构建前清理 bundle 目录（消除累积源头）；更新清单版本一致性检查与发布产物自检从观察模式升级为**阻断模式**（自检失败 = 发布失败，坏版本无法流出）。存量 v1.1.52 的 updater.json 已在线上修正并验证（v1.1.46~v1.1.52 客户端下次检查更新即恢复）。
- **自动更新流程不再暴露自建 GitLab 域名**：检查/下载/安装全流程的用户可见文案（状态事件、错误消息、来源标签、结果字段）统一经域名脱敏，自建域名一律替换为中性「更新服务器」；此前连接失败、清单获取失败等错误会原样带出 `https://…` 完整地址。点击「打开下载页」跳转浏览器的功能链接不受影响。
- **全量代码评审后的纯逻辑修复（第二批，前端 TS）**：
  - **Popover 数值选项把显式 0 当缺省**：`popoverPosition` 的 `margin/gap/minHeight/preferredWidth` 用 `Number(x) || DEFAULT` 解析，显式传 0 会被默认值覆盖；新增 `numericOption`（null/undefined/空串/非有限值取默认，0 保留）。
  - **分类颜色非法值产出无效样式**：`hexToRGBA` 对 `#zzzzzz` 之类输入会解析出 `NaN` 产出 `rgba(NaN,…)` 使样式静默失效；现回退默认灰色。
  - **重复实现收敛**：i18n `formatLocalizedTime` 与 `formatLocalizedDate` 实现逐字重复，改为互相委托；`formatDurationLocalized` 英文分支 `compact ? x : x` 两个三元分支完全相同，简化为直接返回；`appVisuals` 的 `electronSvg` 与首字母占位图改用已有的 `encodeSvg` 助手。
  - **windowsToolchain 测试在 Git Bash 会话假失败**：`minimalPathEnv()` 只覆盖 `Path` 拼写，而 Git Bash 环境原变量拼写为 `PATH`，两个键并存导致 PowerShell 仍读到完整旧 PATH、"应该找不到 node"的 2 个用例在本地（Git Bash）恒假失败（CI 的 PowerShell runner 不受影响）；现按不区分大小写统一改写 PATH。
  - 评审中发现的 i18n `interpolate` 链式替换与分类 store 的 subscribe 取快照模式，经核实均为**测试契约固化的预期行为**（`index.test.ts` 断言参数值占位符继续展开、`categories.test.ts` 按源码文本固定快照模式），按行为保持原则未改动。
- **全量代码评审后的安全与正确性修复（第一批，Rust 后端）**：
  - **截图读取路径逃逸（P0）**：`get_screenshot_thumbnail`/`get_screenshot_full` 的路径校验未拒绝 Windows 无盘符根路径（如 `\Windows\x.png`），`Path::join` 会用它替换 data_dir 前缀逃逸出数据目录读取任意图片；现于 `validate_relative_path` 中拒绝 `has_root()`，并新增 `validated_join` 在拼接后二次确认结果仍在 data_dir 内。
  - **日报合并导出绕过目录校验（P0）**：`export_reports_range` 此前对目标路径不校验即可写文件（单篇导出有 `validate_export_dir` 防护，属校验旁路），可写入 `.ssh`/`LaunchAgents` 等敏感位置；现对目标父目录应用同一校验（缺失时先创建再校验）。
  - **Agent 联网工具 SSRF 护栏绕过（P0）**：`ensure_public_http_url` 手写主机解析无法对抗 WHATWG 主机归一，`http://2130706433/`、`http://0x7f000001/`、`http://127.0.0.1./`、`http://[::ffff:127.0.0.1]/` 等形式可绕过内网拦截直连本机/内网；现改用 `reqwest::Url` 规范化解析（IPv6 的 `host_str` 带方括号，剥壳后再判定），IPv4-mapped IPv6 按映射的 IPv4 判定，并补 7 组绕过形态回归测试。
  - **全文检索索引损坏（P1）**：`save_report`/`save_hourly_summary` 用 `INSERT OR REPLACE` 写触发器同步的 FTS5 外部内容表——REPLACE 的隐式删除在默认配置（`recursive_triggers` 关闭）下不触发 AFTER DELETE 触发器，旧词条残留、搜索持续命中过期内容；改为 `ON CONFLICT DO UPDATE` upsert 走 UPDATE 触发器正确维护索引。
  - **插入热路径全表扫描（P1）**：`insert_activity` 每条活动都执行 `total_activities=(SELECT COUNT(*) FROM activities)`，随历史行数线性变慢；改为 `total_activities+1` 自增（重建/删除路径本就重算精确值自愈）。
  - **合并活动静默丢历史 OCR（P1）**：`merge_activity` 读旧 OCR 的错误被 `.ok()` 吞掉后仍继续 UPDATE，瞬时错误即覆盖丢失历史 OCR；现仅"行不存在"按无 OCR 处理，其余错误向上传播。
  - **分类规则缓存永不失效（P1）**：桌宠循环的规则缓存签名只哈希末条规则，改/删非末尾规则后缓存不刷新、分类一直用旧规则；改为对全部规则做内容哈希（`AppCategoryRule` 补 `Hash` 派生）。
  - **空闲阈值改后不生效（P1）**：截图循环仅在启动时读一次 `idle_threshold_minutes`，用户改设置后需重启才生效；现每轮循环随配置热更新（`IdleDetector` 新增 setter）。
  - **GitLab 拉取无超时且失败静默（P1）**：`GitlabClient` 的 reqwest 客户端未设任何超时，GitLab 挂起时日报生成无限阻塞；补 10s 连接/30s 总超时。逐项目拉取提交失败此前完全静默，现记 warn 日志便于排查。
  - **隐私过滤正则笔误（P2）**：邮箱 TLD 字符类 `[A-Z|a-z]` 中的 `|` 为字面量管道符（可匹配 `a@b.c|`），改为 `[A-Za-z]`。
  - **空隐私规则静默全量停记（P2）**：空白规则名/空排除关键词经子串匹配"恒真"命中一切——空规则名会静默停掉全部记录、空关键词让所有窗口永久脱敏；`normalize` 阶段剔除空条目，旧版迁移同样跳过空应用名。
  - **小修**：Claude 端点防重复拼接 `/messages`；云端日报回退斜体标记补闭合 `*`；删除 `urls.dedup()`、`build_fts_query` 重复条件、`node_gateway` 死绑定、`gitlab.rs` 无效果死分支等四处死代码；AI 提示词/本地模板的域名标签此前硬传空映射、忽略用户语义分类改名，现传入真实 overrides；MCP `serverInfo.version` 改用 `env!("CARGO_PKG_VERSION")`；桌宠气泡 `info/success/persistent_info` 构造器去重。

### 新增
- **GitLab 集成新增「从 GitLab 选择仓库」选择器**：项目过滤旁新增按钮（需已填写服务地址与令牌），点击自动拉取令牌有权限的全部仓库（按路径排序），弹窗内支持搜索过滤、逐个勾选、按当前筛选全选与一键清空，实时显示「已选 N / 共 M 个」；保存后勾选结果写回项目过滤列表（与手工编辑完全同一数据）。语义与现有配置一致：不勾选任何仓库 = 统计全部有权限仓库。弹窗支持 Esc 关闭与焦点圈定，四语言文案。
- **GitLab 集成新增「纳入工作的作者」**：同一份工作常以不同 git 身份提交（不同机器的 user.email/user.name、公司/个人邮箱混用），此前日报只统计 token 账号本人的提交、信息不完整。现可在 设置 → GitLab 集成 → 「纳入工作的作者」维护作者名单（姓名或邮箱，一行一个、兼容逗号分隔，自动去首尾空白/空行/忽略大小写去重）；生成日报时这些作者的提交与 token 账号本人取并集纳入（每条名单同时与提交的 author_email/author_name 忽略大小写精确比对，不做子串匹配避免误伤同名前缀）；名单留空 = 仅统计当前账号，行为与旧版完全一致。配置存于 `gitlab.work_authors`（旧配置文件自动兼容），规范化阶段同步去重清洗（四语言文案）。
- **AI 设置新增「内置模型」供应商并设为默认**：供应商网格首位新增「内置模型」卡片（仅编译期注入了内置模型时显示），默认选中；选中后连接细节（端点/模型/密钥）对用户完全隐藏，只保留「测试连接」按钮（真实请求内置模型验证可用性）与说明文案（四语言）。选中内置即自动进入「AI 增强」日报模式；后端 `get_config` 归一化会把出厂默认配置的供应商翻转为 `builtin`、模式翻转为 AI 增强，新增 `AiProvider::Builtin` 类型（OpenAI 兼容协议，配置文件中端点/模型留空）。出厂默认之外的自定义配置行为不变。
- **设置页「内置模型」常驻指示卡 + E2E 测试套件**：AI 设置顶部新增蓝色指示卡，只要编译期注入了内置模型即常驻显示（置于日报模式切换之上，基础模板/AI 增强两个分支均可见），文案按状态切换——出厂默认配置显示「软件已内置 AI 模型，无需配置即可使用 AI 功能；生成内容会发送至内置模型对应的第三方 AI 服务」，已配置自定义模型显示「…当前优先使用你配置的模型」（四语言）。新增 Playwright E2E 套件（`tests/e2e/ui/`，4 用例：双状态卡片、无内置复现、纯 web 降级；`npx playwright test --headed` 人工过目，`E2E_HOLD_MS`/`E2E_SLOW_MS` 控制停留与放慢）。`tauri:dev` 改为包装脚本（`scripts/tauri-dev-with-key.mjs`）：从 git-ignored 的 `src-tauri/.builtin-model-key.local` 读取内置模型 key 并在编译期注入 + touch 入口强制重编（绕开 `option_env!` 不因环境变量变化重编译的坑），本地开发构建与发布版行为一致；key 缺失时照常启动并打印说明。

### 修复
- **自动更新 token 未随发布版编译**：CI 变量误命名为 `UPDATER_GITLAB_TOKEN`，而代码读取的是 `WORK_REVIEW_UPDATER_GITLAB_TOKEN`——v1.1.35~v1.1.37 的安装包全部缺失 token，更新器按「未配置更新源」降级，检查更新永远显示「已是最新」。已将 CI 变量更名为 `WORK_REVIEW_UPDATER_GITLAB_TOKEN`，v1.1.38 起发布版恢复自动更新能力；**v1.1.37 及更早的存量用户需手动安装一次 v1.1.38**（它们的更新器在此期间不工作）。

### 变更
- **应用图标更换为「Yue复盘」新品牌图**：主图标（`src-tauri/icons/icon.png`，macOS/通用，留 macOS 惯例边距）与 Windows 专用图标（`windows-icon.png`，满幅，同时是窗口标题栏图标与 ICO 母图）更换为蓝色圆角「Yue复盘」新图（白底转透明、圆角外透明），`npm run icons:build` 重新派生全部尺寸 PNG / ICO（16–256 共 9 档）/ ICNS 及 `public/` 品牌图；补上 `index.html` 一直引用却不存在的 `public/favicon.png`（64×64）。顺带修复 `scripts/build-icons.ts` 在 Windows 上必然失败的问题：Node 22 出于安全策略禁止 `spawnSync` 无 shell 直接调用 `.cmd`（EINVAL），改为用 `node` 直接调 `@tauri-apps/cli/tauri.js` 入口，跨平台一致。
- **自动更新源从 GitHub 切换到自建 GitLab**：客户端更新端点改为 GitLab 包仓库固定版本直链（`/api/v4/projects/…/packages/generic/yue-review/<版本>/updater.json`），检查流程改为两步——先查 releases API 取最新 tag 做版本比较（GitLab 19 不支持 `latest` 包别名），再对更新清单做可安装性检查（minisign 签名校验不变）；清单不可用时回退提示手动下载，手动下载页指向 `/releases/permalink/latest`。项目保持 private，更新器以编译期注入的只读 Project Access Token（`WORK_REVIEW_UPDATER_GITLAB_TOKEN`，即 CI 变量 `UPDATER_GITLAB_TOKEN`，Guest + read_api、2027-09-10 到期可吊销）以 `PRIVATE-TOKEN` 头完成全部更新请求；未注入该变量的构建（如本地开发）按"未配置更新源"优雅降级。publish-release 新增 `scripts/generate-updater-json.mjs` 生成并上传每个版本的 updater.json（darwin-aarch64 / windows-x86_64 必备，darwin-x86_64 由 `PACKAGE_MACOS_X64` 开启时可选），`tauri.conf.json` 移除 GitHub 端点。注意过渡：v1.1.35 是首个走 GitLab 更新源的版本，存量客户端（≤v1.1.34）内置的仍是 GitHub 端点，需手动升级一次。
- **关于页移除 GitHub 链接与赞助入口**：操作区仅保留「打开数据目录」，赞助弹层（微信/支付宝/Buy Me a Coffee 收款码）连同其状态、Esc 关闭、二维码放大交互与 `.about-support-link` 样式一并删除；四语言的赞助相关 i18n 文案同步清理（词典叶子数 1340 → 1329）。
- **二进制名与便携版更名，彻底消除与旧版的安装冲突**：`mainBinaryName` 由 `Work_Review` 改为 `Yue_Review`——此前 NSIS 安装器按二进制名查杀同名进程，旧版 Work-Review 运行中安装 Yue复盘 会弹「yue-review is running, click ok to kill it」并杀掉旧版进程（E2E 实测复现）。现安装后的主程序为 `Yue_Review.exe`，便携版更名为 `Yue_Review_portable_x64.zip`；CI 便携打包/产物路径/glibc 检查参数同步更新；进程自识别表（categorize/monitor/前端图标与时间线归一化）加入新名并保留旧名兼容。新增安装器隔离 E2E（`tests/e2e/installer-isolation/run.ps1`，Windows）：静默安装/启动并存/静默卸载全链路断言旧进程不被影响，以及独立性契约测试 `appIndependence.test.ts`。
- **应用更名为 Yue-Review（中文名「Yue复盘」），并与旧版实现并存**：productName 与安装包/软件包命名（NSIS setup.exe、dmg、deb、rpm、AppImage 产物名）随英文名更新为 Yue-Review；主窗口标题与侧边栏/关于页品牌改为「Yue复盘」（繁体环境显示「Yue復盤」，英文/阿拉伯环境显示 Yue-Review）。时间线与图标的自身应用识别同步认识新名称，旧名称（Daily-Report、Work Review）的历史记录继续兼容。**为支持与旧版（≤v1.1.29）同时运行**，包标识由 `com.workreview.app` 改为 `com.yuereview.app`，默认数据目录由 `%APPDATA%\work-review` 改为 `%APPDATA%\yue-review`（数据目录偏好文件、OCR 临时目录、MCP Server 配置定位同步迁移）：新旧两版可同时打开、互不干扰、各自独立记录；新版本首启从空数据开始，需要旧历史时可在 设置 → 数据目录 指回旧目录。代价：macOS 需重新授予屏幕录制/辅助功能权限（按标识符记账）；WebView2 界面内缓存（localStorage）重置，业务数据不受影响；旧版自动更新到本版后即归属新标识符，旧安装目录残留一条卸载项可手动清除。二进制名 Work_Review.exe、便携版命名保持不变。另修复 vite dev 监视 target/ 目录导致 cargo 并行构建时 EBUSY 崩溃的问题。

### 修复
- **GitLab Release 资产匿名下载 404**：publish-release 此前用 `glab release create` 直接挂文件，生成的资产链接指向 uploads 路由且不带 `direct_asset_path`——uploads 仅浏览器会话可访问（PRIVATE-TOKEN 无效），项目公开后匿名下载依然 404（v1.1.30 实测）。现改为：产物上传至 Generic Package Registry（`packages/generic/yue-review/<版本>/`），并建立带 `direct_asset_path` 的资产链接，`/releases/<tag>/downloads/<file>` 永久路由对匿名与 PRIVATE-TOKEN 均可用；发布名同步改为 "Yue-Review <tag>"。v1.1.30 存量资产已手工迁移到包仓库并重写全部链接，脚本对重试幂等（release/link 已存在时跳过）。

### 新增
- **内置兜底 AI 模型**：未配置任何文本模型时，日报生成、工作助手问答、桌宠主动 AI、实体分类、MCP 技能等全部文本 AI 功能自动改用内置模型（`deepseek-v4-flash` @ `https://laneai.dev/v1`，OpenAI 兼容），开箱即用；用户一旦在设置中配置自己的模型，始终优先用户配置。API Key 不写入源码与配置文件，由编译期环境变量 `WORK_REVIEW_BUILTIN_MODEL_API_KEY` 注入（需在 GitLab 配置该 CI 变量，未注入时行为与旧版完全一致）；设置页 AI 区块在「未配置且有内置模型」时显示提示横幅（四语言），说明内容将发送至第三方 AI 服务。内置 Key 仅存在于编译产物中，不经过 `get_config`/`save_config` 与本地 API 往返，不会在前端或配置同步中出现。
- **日常安装包产物（可直接下载安装）**：每次推送 main 且 verify/compile 全绿后，新增 5 个 `package-daily-*` job（Windows exe+便携 zip、macOS arm64+x64 dmg、Linux arm64+x64 deb/rpm/AppImage）产出三平台完整安装包并上传为 CI artifact（保留 30 天）；提供固定「latest」下载链接（`/-/jobs/artifacts/main/download?job=package-daily-<os>`，自动指向最近成功构建）。tag 发布版仍按原 Release 流程永久保留安装包。
### 新增
- **CI 版本自增与三平台编译**：每次推送 main 且在 verify（frontend + 三平台校验）+ compile（新增三平台 `cargo build --release` 产物 job：build-release-linux/macos/windows）全部通过后，`version-bump` job 自动将版本末位 +1（逢 100 进位，如 1.2.99 → 1.3.0），同步五处版本源（package.json / package-lock.json / tauri.conf.json / src-tauri/Cargo.toml / Cargo.lock），提交回 main（message 含 `[skip ci]` 防循环）并打 `vX.Y.Z` tag，自动触发 release-preflight → 三平台打包 → glab 发布。需在 GitLab 配置 CI 变量 `GIT_PUSH_TOKEN`（Project Access Token，write_repository + api 权限）；缓存 key 改用 Cargo.toml 避免 bump 后缓存失效。
### 新增
- **GitLab 集成**：设置页新增 GitLab 分组（私有化部署 URL + 个人访问令牌 + 项目白名单 + 日报补充开关 + 连接测试按钮）。生成日报（手动/定时/机器人/AI 助手/本地 API 全链路）时按日报日期拉取该用户在私有化 GitLab 的提交（读 `GET /user` 识别用户，按本机时区过滤"当日"，每项目最多 20 条、全局最多 200 条，超出提示剩余数量；项目允许列表留空 = 全部有权限项目）；AI 模式下提交并入提示词由 AI 整合进日报正文，无法调用 AI 时自动追加独立小节；未配置/无提交静默跳过，拉取失败时日报末尾追加一行备注。

### 修复
- **publish-release 改用 GIT_PUSH_TOKEN 认证 glab**：脚本内 export CI_JOB_TOKEN 后仍 404——本实例（GitLab 19.0.1）对 CI_JOB_TOKEN 访问 Releases API 一律按匿名处理（GET/POST 均 404 Project Not Found，v1.1.26/v1.1.27 两次实测，应为 job token 细粒度权限默认收紧所致）。现改为复用 version-bump 同款项目访问令牌 GIT_PUSH_TOKEN（write_repository + api scope，受保护变量，tag 流水线可读），缺失时回退 CI_JOB_TOKEN（标准实例可用）。
- **publish-release 的 glab 认证改在脚本内 export**：`variables:` 里 `GITLAB_TOKEN: $CI_JOB_TOKEN` 展开为空（CI_JOB_TOKEN 由 runner 在作业启动时注入，服务端变量展开阶段不存在，GitLab 已知限制），glab 以匿名身份调 API，私有项目返回 `404 Project Not Found`（v1.1.26 实测，发生在突破 host/remote 校验问题之后；tag 明明存在却被报 "Tag does not exist" 也是同一匿名 404）。现改为 before_script 里 `export GITLAB_HOST="$CI_SERVER_URL" GITLAB_TOKEN="$CI_JOB_TOKEN"`（脚本环境中 CI_JOB_TOKEN 必然存在，对当前项目有创建 release 权限，release-cli 同款机制），GITLAB_HOST 一并移入脚本统一管理。
- **daily-package 限定了 main push 触发**：package-daily-* 需要受保护变量 TAURI_SIGNING_PRIVATE_KEY(_PASSWORD) 做更新签名，而受保护变量对功能分支/MR 流水线不可见，导致任何 MR 流水线在 daily-package 阶段三平台必然失败（首个 MR 流水线实测）。现将 .package-daily 的规则收紧为仅 main push 运行（tag 仍跳过，发布走 package/publish 阶段），与其注释声明的「每次 push main 产三平台安装包」意图一致，MR 流水线自此可全绿交付。
- **tag 发布 publish-release 阶段 glab 拒绝执行**：runner 检出工作区的 `origin` URL 不带端口（glab 识别为 `home.chenkaidi.top`），与作业变量 `GITLAB_HOST`（`CI_SERVER_URL`，含 `:509` 端口）不一致，`glab release create` 报 "None of the git remotes configured for this repository correspond to the GITLAB_HOST environment variable" 退出，v1.1.19 起 tag 发布无一成功。现发布前先把 `origin` 改写为与 `GITLAB_HOST` 同源的完整地址（`$CI_SERVER_URL/$CI_PROJECT_PATH.git`），并给 `glab release create` 追加 `-R "$CI_PROJECT_PATH"` 显式指定仓库（已本地验证 `-R` 可绕过 remote/host 匹配检查），双保险确保发布链路不再因 host 识别失败。
- **rust-verify-macos clippy 门禁**：修复 703624a 引入的 `&[temp_root.clone()]`（src-tauri/src/commands/system.rs）触发 `cloned_ref_to_slice_refs`（`-D warnings` 下失败），改为 `std::slice::from_ref(&temp_root)`；同时将 `crates/core` 与 `src-tauri` 中 `chunks_exact(_mut)` 常量块写法升级为 `as_chunks(_mut)`，兼容 Rust 1.98+ 新 lint。
- **Linux/macOS runner 的 rustup 定位**：`.rust-node` 新增 `source "$HOME/.cargo/env"` 兜底，解决 rust-verify-linux 的 `rustup: command not found`；仍未找到时输出明确安装指引。
- **Linux CI 去掉 sudo**：rust-verify-linux / tauri-package-linux 此前在 before_script 执行 `sudo apt-get`（shell executor 无 TTY，sudo 等待密码直到 job 超时）；现改为非 sudo 的 `scripts/check-linux-deps.sh` 预检（dpkg -s），系统依赖（libwebkit2gtk-4.1-dev、libgtk-3-dev、libayatana-appindicator3-dev、librsvg2-dev，打包另加 patchelf、libfuse2、rpm、rpm2cpio、cpio、binutils）改为在 Linux runner 上一次性手动安装，缺失时打印安装指引。
- **Windows CI 工具链引导**（rust-verify-windows / tauri-package-windows）：`.powershell-toolchain` 此前只兜底 `C:\Program Files\nodejs` 一个目录，node 由 nvm-windows / scoop / chocolatey 安装或尚未安装时直接报 `node not found` 失败；现提取为 `scripts/windows-toolchain.ps1`，自动探测官方安装、nvm-windows（取最新版本）、scoop、chocolatey 等位置，缺失时列出已查找位置；同时为 npm 12 新增项目级 `.npmrc`（allow-remote）与 package.json `allowScripts`（esbuild），修复 CI `npm ci` 的 EALLOWREMOTE 与 install-scripts 拦截；`check-linux-glibc` 测试在无 bash 的 Windows 环境跳过。
### 变更
- **应用更名为 Daily-Report**：产品名（productName）、主窗口标题、侧边栏/关于页品牌、桌宠场景文案、macOS 权限提示与 i18n 版权行统一改为「Daily-Report」；前端图标/名称归一化同时兼容新旧名称（改名前的历史记录仍可正确识别为应用自身）。打包二进制名（Work_Review.exe）、包标识（com.workreview.app）、数据目录与 CI 产物命名保持不变，避免影响既有安装与升级。
- **安全公告评估关闭**：五条 unsound 类 RustSec 跟踪 Issue（#167-#171：anyhow / event-listener / glib / memmap2 / rand）经 `cargo tree --target all` 依赖路径评估，确认触发条件在本项目均不可达（自有代码不调用受影响 API，相关 crate 均为传递依赖，rand 0.7 仅存在于构建依赖链），全部评估关闭；unsound / 漏洞类公告继续按 Issue 跟踪。

## [1.1.2] - 2026-08-22

### 新增
- **企业微信智能机器人长连接**（#147）：无公网地址时可用 Bot ID + Secret 主动连接 `wss://openws.work.weixin.qq.com`，心跳保活、断线重连、被踢后提示。命令处理仍复用 `/devices` `/report` `/generate`。原 Corp ID 回调模式保留为可选项。
- **助手/日报超时可配置**：设置 → AI 模型可调助手绝对超时（默认 120s）与日报外层超时（默认 300s）。一次助手请求共用一份剩余预算（响应头、流式、回退、429 重试、工具确认、收束）；前端超时会发送取消。日报超时会 abort 后台任务。
- **思考模式与输出预算**：按 Provider 能力映射思考开关 / thinking budget / max output tokens（Ollama `think`、Qwen 流式 `enable_thinking`、SiliconFlow `chat_template_kwargs`、Claude thinking、Gemini thinkingBudget）。不支持的提供商不发送扩展字段。Claude 在带工具时不开启 thinking，避免多轮签名块丢失。
- **WebDAV 常用配置同步**：远程存储为 WebDAV 时可把 AI、分类、外观等设置同步到网盘（最后保存获胜）。本机端口、窗口位置、开机自启、导出目录不覆盖。配置文件含密钥，仅建议同步到自己的网盘。

### 修复
- **Windows/Linux 标题栏**：默认紧凑风格顶距与自定义拖拽条对齐，避免顶部约 5px 被拖拽层盖住；最大化后按钮切换为还原图标。
- **思考型模型兼容**（魔搭 DeepSeek-V4-Flash 等）：AI 助手流式/非流式解析支持 `reasoning_content` 思维链——思考帧实时输出、正文为空时兜底思维链，不再出现"连接测试成功但助手无回复"；`max_tokens` 1600 → 8192 给正文留足额度；连接测试改为校验模型实际返回非空内容（探测额度 16 → 256），消灭 HTTP 200 但无内容的假阳性。
- **模型 API Key 记忆**：设置中切换 AI 服务商后，各家已填写的 API Key / 端点 / 模型名现按服务商持久化保存（新增 `text_model_provider_cache` 配置字段），切回原服务商或重启应用后自动恢复，不再丢失。
- **社区二维码格式**：`docs/group/wechat-group.png` 实为 JPEG 误用 .png 扩展名导致测试失败，已转换为真 PNG。
- **设置页最大化空隙**：设置页板面此前被内容轴限宽居中，Windows 最大化/高分屏下内容两侧出现大片空白（1080p 全屏右侧约 300–500px）；现与概览、时间线一致满宽铺开，宽度由卡片内部网格吸收，AI 对话、日报等阅读型内容仍保留行长上限保证可读性。
- **桌宠设置拥挤**：桌宠配置卡此前 8 个设置块全部垂直堆叠，又窄又长；现"大小/透明度"滑块并排两列、"点击穿透/隐藏身体/休息提醒/主动 AI"开关排成 2×2 网格（休息间隔下拉跟随同列），卡片高度约缩短三分之一。

### 变更
- **企微长连接**：凭证未变且任务仍在跑时不再因保存配置而重连；命令处理移出 WebSocket 循环，避免生成日报时心跳中断。
- **助手超时收束**：从总预算预留收束时间，超时后仍用已有工具结果再问一轮模型。
- **Bot 生成日报超时**：跟随设置里的日报超时，不再写死 120 秒。
- **玻璃拟态视觉风格**：主窗口壳层、通知 Toast、设置卡片改用毛玻璃质感（backdrop-blur + 语义色点缀，借鉴 Alger Music 的设计语言）；「经典实底」风格（ui-style-a）不受影响；Linux（WebKitGTK）因毛玻璃渲染不可靠自动降级为高不透明度实底。
- **死代码清理**：移除生产路径不再使用的助手问题细分类、Ollama 旧列表解析、日报模式归一化 helper、本地分析器未接线的截图描述、小时摘要未使用的 AI prompt 构造，以及从未调用的多窗口采集预留函数；同步删除只覆盖上述死路径的测试。
- **安全审计触发调整**：移除 Security Audit 工作流的每周定时扫描，保留锁文件变更触发与手动运行入口（Actions → Security Audit → Run workflow）。
- **安全审计降噪**：Security Audit 工作流新增忽略 19 条 unmaintained 类 RustSec 公告（gtk-rs GTK3 全家桶、unic-\*、paste、proc-macro-error、fxhash、rustls-pemfile——均为信息性"停止维护"提示，无安全漏洞且无 patched 版本），unsound / 漏洞类公告继续建 Issue 跟踪（保留 #167-#171）；同步关闭已生成的对应机器人 Issue #148-#166。

## [1.1.1] - 2026-08-10

### 新增
- **显式用户记忆**：助手支持由用户创建、编辑、删除和清空长期记忆，并在设置页提供独立管理入口。
- **语义记忆安全重构**：索引、检索与重建流程增加状态校验、并发保护和失败恢复，避免部分写入造成记忆状态失真。
- **即时截图 API**：新增受 Bearer Token 保护的 `POST /v1/screenshots/capture`，可立即截取当前活动界面并返回 JPEG Base64；接口尊重截图隐私开关且不会自动上传。
- **日期快速回退**：日报页增加上一天入口，可连续查看更早记录，无需反复打开历史日期选择器。

### 修复
- **助手阅读位置**：展开历史消息的工具过程或回答依据时，不再强制跳到会话底部；仅在用户原本跟随底部时继续追踪新高度。
- **助手上下文透明度**：恢复“参考记录”的范围与来源说明，改为紧凑信息入口，并补齐简体中文、繁体中文、英文和阿拉伯文。
- **桌宠隐藏模式**（#140）：隐藏桌宠本体时保留通知展开高度，并修复透明区域错误拦截鼠标的问题。
- **Windows 浏览器探测**（#141）：为 UI Automation 浏览器地址读取增加同窗缓存、单飞、超时与熔断，限制长页面或翻译场景下的探测开销。
- **macOS 输入监控**：授权完成后自动重试输入监听，不再必须手动重启才能恢复联动。
- **窗口采集过滤**：排除应用自身的覆盖层窗口，避免记录或截图误采集通知浮层。
- **开发启动链路**：修复 TypeScript PostCSS 配置依赖缺失和 Vite 依赖扫描被错误关闭的问题，恢复 `npm run tauri dev`。
- **应用图标清晰度**：修正浏览器图标获取、HiDPI 尺寸选择和缩放方式，减少模糊与变形。

### 优化
- **TypeScript 工具链**：前端源码、测试与 Node 工具脚本统一迁移为 TypeScript，并将本地开发、CI 与发布环境统一为 Node.js 22+。
- **界面一致性**：统一日期控件、圆角、助手排版和概览字体层级，减少页面之间的视觉割裂。
- **发布可靠性**：Tag 必须与五个版本源及 `origin/main` 一致；扩大 Rust 检查范围，并让缺失安装包、便携包或 Release 附件直接终止发布。
- **Actions 运行时**：官方 GitHub Actions 升级到 Node 24 运行时版本，移除 Node 20 弃用告警。
- **仓库清理**：移除过期实现计划、设计草稿、Python 学习原型与可再生成的 Tauri schema，并收紧相关忽略规则。

## [1.1.0] - 2026-07-31

### 新增
- **内置分类知识库**：约 140 个网站域名 + 70 个应用的确定性分类，浏览器活动按站点内容细分（B 站→娱乐、GitHub→开发），不再一律归"浏览器"。
- **AI 实体分类自动学习**：后台自动归类未识别的应用/域名并回溯修正近 14 天历史，分类随使用自动变准。
- **语义记忆系统**：新增「记忆」能力，可语义搜索看过的屏幕内容，支持本地 Ollama 或 OpenAI 兼容嵌入源；嵌入不可用时降级为关键词检索。
- **6 个新 AI 服务商**：OpenRouter、Groq、xAI Grok、Mistral、LM Studio、自定义 OpenAI 兼容端点。
- **模型/搜索测试按钮**：语义记忆区"测试模型"、助手联网区"测试搜索"，先测通再启用。
- **Agent 能力扩展**：循环放宽到 120s、新增停止按钮、注入实时前台上下文；工具从 6 个扩展到 12 个，并新增 6 个需前端确认的写操作工具（建待办、改分类、暂停/恢复录制等）。
- **会话持久化**：助手历史存入 SQLite（替代 localStorage），支持切换/删除/新对话，旧历史自动导入。
- **时间线双抽屉**：小时摘要和活动详情改为右侧抽屉，集中展示截图、活动信息与分类编辑。
- **网站全量浏览与语义编辑**：概览支持展开全部网站及单域名详情，可直接修改网站语义分类并回填历史。
- **注入防护**：外部内容用边界包裹防提示注入，Gemini Key 改走请求头，系统指令不再拼进 user content。
- **隐藏桌宠本体**（#137）：新增「隐藏桌宠形象」开关，开启后仅保留通知与提醒气泡、隐藏桌宠本体并收缩窗口；适合只想用通知、不需要桌宠形象的场景。

### 修复
- **统计边界与时区**：重叠工作时段按并集计时、加班排除娱乐活动；跨午夜会话按天裁剪；非整小时时区（UTC+5:30 等）的小时图不再归错。
- **统计口径统一**：分类构成条、娱乐占比、洞察句与总投入同源，页面顶层数字不再对不上。
- **隐私过滤加固**：窗口标题入库前过滤邮箱/手机号/卡号；修复银行卡号正则从未生效、卡号原样入库的问题。
- **时长计算修正**：切到忽略应用不再吞掉上一应用时长，切回旧应用不再合并为伪连续区间。
- **存储清理定期执行**：保留天数/容量清理改为每 6 小时后台跑一次，长驻应用也生效。
- **Telegram Bot 断网重连**：按指数退避无限重试，不再连续 3 次失败就永久停机。
- **MCP 日报委托超时**：从 2 秒放宽到 120 秒，未缓存 AI 日报不再委托失败。
- **小时摘要性能**：仅重算缺失或过期的摘要，不再每次查看全量重算并写库。
- **截图补传兜底**：后台每 10 分钟扫描近 72 小时漏传截图自动补传。
- **「测试连接」改测表单当前值**：所见即所测，填完直接点测试不再误报"未配置"。覆盖范围延伸到「测试搜索」「测试模型」——选了免费搜索方案但未点保存就测试，不再误报要求 Tavily API Key。
- **免费搜索（Bing）容错**：`www.bing.com` 在部分网络（尤其国内）会间歇性连接超时，免费搜索现对连接/超时类错误做 2 次指数退避重试（1s、2s），并把总超时从 12s 提到 20s；「测试搜索」与实际搜索共用同一重试逻辑，瞬态抖动不再直接失败。
- **Bot 设备查询 token 安全**：token 改走 Authorization 头，不再暴露在 URL 查询串。
- **常驻网站图标**：灰底缩写升级为真实 favicon（懒加载）+ 字母兜底。
- **常驻网站聚合**：移除全局 Top 10 URL 截断，恢复真实网站总数；同一域名跨浏览器正确合并来源。
- **时间线/概览异步竞态**：日期切换、加载更多和模式切换均加序号校验，旧请求的迟到结果不再覆盖当前页面。
- **助手中文输入法**：IME 组合输入期间按 Enter 不再误发送。
- **助手页状态**：切页面不再误标"已结束"，切回可见生成中气泡。
- **技术错误友好提示**：JS 异常归拢为"加载失败，请稍后重试"（四语言），不再把堆栈怼到用户脸上。
- **日报页修复**：西时区日期显示早一天、缓存订阅未退订泄漏、TTL 从未生效等。
- **助手页 XSS 防护**：补齐 DOMPurify sanitize 并升至 3.4.12，修复上游已披露的 sanitize 绕过。
- **监听器/渲染循环泄漏**：修复 Timeline/Overview 等多处在 await 期间销毁导致的监听器与 rAF+WebGL 永久泄漏。
- **实时时间线崩溃**：修复截图路径为空时抛错致实时更新静默失效。
- **删除自定义分类**：历史活动同步归类到回退分类，不再在时间线/日报中残留死 key。
- **数据清理误删昨天**：修复"保留今天和昨天"实际删掉了昨天记录。
- **IM 机器人**：飞书/钉钉非 2xx 不再误报成功、飞书 token 按 app_id 区分、设备名拼错不再静默回退本机执行。
- **WebDAV**：目录 MKCOL 缓存（不再逐级重发）、Content-Type 改按扩展名推导。
- **窄窗折叠**：<700px 侧栏收为图标 rail，<640px 时间线让出完整宽度。
- **桌宠通知边缘翻转**（#137）：桌宠靠近屏幕右侧时，通知气泡自动改为向左展开，不再因贴右下角而被窗口裁剪。
- **桌宠穿透按钮可点击**（#137）：Windows 此前禁用了智能穿透轮询，导致开启穿透后通知卡片按钮完全点不到；现所有平台统一启用轮询（已有缓存去抖，无高频闪烁），鼠标进入窗口矩形（含按钮区）时临时关闭穿透，按钮恢复正常点击。
- **弹窗焦点陷阱与无障碍**：Tab 循环聚焦、Toast 补 aria-live、全局动画降级、hover-only 操作补键盘可达。
- **暗色模式与 RTL 补齐**：日报、时间线、周期摘要等暗色规则与阿语逻辑属性适配。
- **CI 门禁**：新增 push/PR 触发的测试 + 构建门禁与每周安全审计。
- **日志隐私加固**：URL 采集不再写真实地址到日志，Bot Token 在错误信息中脱敏。
- **关于页版本状态**：设置读取异常不再把真实版本覆盖成硬编码 `1.0.0`，空版本显示为 `--`。
- **开发服务器边界**：Vite 限制 CORS 仅接受 localhost，阻止恶意网页跨源读取本机开发资源。
- **其他**：标准工时输入 clamp 到 1~24、概览时钟节流、DuckDuckGo 选项标注"免费搜索（Bing）"等 UI 细节。

### 优化
- **概览页改版**：从"数据陈列"到"答案呈现"，KPI 四卡带参照系（较上周同日 ±），新增洞察条与「今日节奏」主视觉卡。
- **常驻网站以域名为主角**：与应用使用统一为双层信息排行、时长轨道和总数页脚，"查看全部"在原卡片内展开。
- **时间线工作台重排**：活动流改为重点卡片 + 紧凑条目轨道，首屏与实时截图预热，详情按阅读顺序组织。
- **助手原生聊天画布**：统一页头与会话操作，欢迎态复用品牌图标，快捷问题居中，回答脱离整卡直接排版。
- **日报页升级**：TL;DR 洞察条、参照系 KPI、导出下拉菜单 + 复制全文、生成设置抽屉、宽屏段落粘性目录、生成中骨架屏、昨日日报醒目横幅。
- **日报提示词预设重构**：从下拉框升级为可见的 chip 集合，点击应用、悬停编辑/删除。
- **行式设置系统**：所有设置 tab 升级为行布局，AI 区 checkbox 换成统一滑动开关，隐私页收敛为应用规则与内容过滤两组。
- **设置页响应式**：中等窗口改用可横向滚动的分类导航，凭据网格在小屏自动降为单列。
- **关于页重排**：品牌主体、版本状态、产品原则改为居中低噪声的信息结构，改用留白分组。
- **服务商选择器与连接卡**：从裸下拉框升级为 11 家品牌卡片网格（品牌色首字母头像 + "使用中"标记）。
- **AI 模型页三入口分区**：重组为「模型配置 / 助手联网 / 语义记忆」三个可点击切换的分区。
- **历史会话侧滑抽屉**：从小弹层升级为页内右侧抽屉面板，带"新对话"入口。
- **字号阶梯规范**：根字号回归 16px，日期选择器、助手元信息等不再低于 10px。
- **统计卡与段落锚点**：统计卡顶部 accent 条 + 数值放大，正文 h2 前置渐变竖条，长报告扫读有停靠点。
- **三风格侧栏指示条**：救活三套风格设计过却从未渲染的"当前页指示条"，支持 RTL。
- **撤销独立「记忆」页**：查询统一走助手，索引管理收进 设置 → AI 模型 → 语义记忆。
- **撤销「OS 密钥保险柜」**：删除 keyring 依赖，配置回归明文本地存储。
- **README 三语言与截图更新**：Bot 联动扩展到 Telegram/飞书/钉钉/企业微信；刷新各界面截图与工作流动图。

## [1.0.56] - 2026-07-23
### 新增
- **阿拉伯语支持（RTL）**（#128）：新增阿拉伯语界面翻译 + RTL 布局适配（CSS 逻辑属性替换硬编码方向），后端日报/分类名/活动时间线同步支持阿拉伯语输出。社区贡献 by algethamy。
- **MCP Server 功能增强**：
  - `get_current_context` 通过 localhost API 获取真实前台窗口（主应用运行时），不再仅返回数据库历史记录；回退时标注 `"source": "history"`。
  - `generate_report` 在 AI 增强模式下委托主应用生成 AI 日报，尊重用户的模型配置、工作时段、自定义 prompt 和隐私过滤。
  - 隐私过滤对齐主应用：MCP 查询结果现支持 `Anonymized` 级别脱敏，与主应用的 `PrivacyFilter` 行为一致。
- **接入管理页面重构**：将 997 行单文件拆分为容器 + 7 个子面板（`nodeGateway/`），分 3 组折叠（AI 工具接入 / 消息通知 / 高级）。飞书/钉钉/企业微信统一为泛型 `BotCredentialsPanel` 组件。
- **设计 Token 体系**：建立 `--radius-sm/md/lg`（8/12/20px）+ `--shadow-sm/md/lg` + `--space-section/block` 统一基准，app.css 中 35 处硬编码值迁移到 token 变量。
- **localhost API `/v1/context` 端点**：返回当前前台窗口 + 最近应用列表，供 MCP Server 委托调用。
- **工作助手 Token 流式输出（阶段 2）**：Agent 回答现按 token 逐字流式推送到前端（`StreamEvent::Token`，后端 64 字节/100ms 批量合并降低 IPC 频率），长回答不再"憋到最后一次性蹦出"。四类 provider（OpenAI 兼容/Ollama/Claude/Gemini）各自实现 SSE/NDJSON 增量解析 + 工具调用分片拼装，流式失败自动回退非流式；请求格式转换抽取为流式/非流式共享函数，超时改为逐块空闲 30s + 总时长 120s 双护栏。新增 10 个装配器/批量器单测。
- **工作助手联网工具（阶段 1）**：工具系统从"仅同步本地工具"扩展为支持异步工具（`ToolExecutor::Sync/Async` 双通道，5 个本地工具零改动），新增三个联网工具——`fetch_url`（读网页正文，零依赖）、`web_search`（支持 Tavily / 博查需配 Key，以及 DuckDuckGo / Bing 免费搜索无需 Key）、`query_activities`（按日期查询活动记录，专为"今天做了什么"类问题设计，替代无法匹配相对时间的 FTS 搜索）。移除了 `get_weather`（open-meteo 超时频繁，天气改走 web_search）。默认全关，需在「设置 → AI 模型 → 助手联网能力」显式开启（守住隐私默认关立场）；开启后才把联网配置传入 Agent，问题才会发给搜索服务商/目标网站。含 SSRF 护栏（拦本机/内网/链路本地地址，防模型被诱导抓取本机 localhost API），联网提示注入帮助小模型正确选工具。四语言 i18n + 单测覆盖工具门控、SSRF、HTML 提取、搜索格式化、异步统一入口。

### 修复
- **创意类应用活动时长漏记**：PS/C4D 等创意软件操作时画面几乎不变，但旧逻辑把"键鼠超时 + 画面静止"判定为空闲导致丢时长。反转哈希判定语义：画面静止不再判空闲，仅键鼠 10 分钟硬超时兜底。
- **助手页表格右侧大空白**（#124 回归）：`.assistant-markdown table` 仍为 `display:block`，窄内容表格右侧留白。统一为 `display:table; width:100%`。
- **紧凑数据风格深色模式白边**（#129）：`ui-style-c` 外框浅色 border + 半透明白底在深色模式下显示为白边。补齐 dark 覆盖（border-color + box-shadow + background）。
- **深色模式白色高光线**（#129）：9 个元素（settings-tab-rail、ask-welcome-panel、overview-panel::before 等）的 `inset 0 1px 0 rgba(255,255,255,X)` 在深色下未被覆盖。统一补齐 `box-shadow: none` 或 `display:none`。
- **深色模式原生控件浅色**：新增 `color-scheme: light/dark`，原生 `<select>/<option>` 下拉列表在深色模式下正确渲染。
- **Windows 任务栏不显示应用名**（#129）：窗口标题为空字符串导致任务栏/任务视图无名称。设为 `"Work Review"`。
- **时间线节点未对齐时间轴**（#129）：轴线（absolute + calc）和节点（flex margin:auto）两套独立定位算法差约 14px。统一为共享水平基准。
- **日报活动时间线表格斑马纹误伤**：details 内表格被全局 zebra striping + 表头灰底覆盖。补豁免规则。
- **亮色模式视觉不一致**：StatsCard 文字灰度统一为 slate-500；主卡片圆角极端值收敛（32px→28px）；卡片边框统一为 `border-slate-200/80`。
- **设置页 memoryHint 文案过时**：移除洞察卡片后文案仍提及"自动生成洞察"，改为描述 memory 实际功能。
- **助手流式事件从未到达前端的两处线格式错误**：`StreamEvent::Error(String)` 为内部标签 + newtype 组合，serde 序列化直接失败且被静默吞掉，前端从未收到错误事件（改 struct 变体 `{ error }`）；`Done` 事件的 `tool_labels` 字段以蛇形上线而前端读 `event.toolLabels`（枚举顶层 rename_all 不作用于字段），一直靠命令返回值双通道掩盖（补字段级 camelCase）。新增线格式契约测试锁定。
- **停止记录后两处指示器仍显示绿色脉冲**（#131）：概览右上角"实时"圆点只判断是否"今天"模式，时间轴"活动记录"圆点更是硬编码绿色脉冲——两处都没响应录制状态。新增 `recordingStore`（参照 `assistantStore` 模式），App.svelte 在 `get_recording_state` 初始化和 `recording-state-changed` 事件中写入 store，概览/时间轴读取 store 判断圆点颜色：录制中（绿+脉冲）/ 已停止（灰）。新增 7 个单测。
- **火山引擎（豆包）连接测试 URL 拼接错误**：连接测试的 `/v1` 回退逻辑假设所有 OpenAI 兼容 API 的端点都以 `/v1` 结尾，但火山引擎用 `/api/v3`、智谱用 `/api/paas/v4`——这些已含版本段，不应再追加 `/v1`（会拼出 `/api/v3/v1/chat/completions` 这种不存在的端点，用户报错 `os error 10061` 根因）。修复 `openai_compatible_chat_completion_urls` 和 `fetch_openai_compatible_models`：识别任何版本号段（`/v1`～`/v9`）后不再回退。新增 3 个测试覆盖火山引擎 v3 / 智谱 v4 / DeepSeek 无版本号三种场景。
- **助手页配了模型却默认用基础模板**（#133）：用户在设置页配置好 AI 模型并测试通过后，助手页的模型选择器仍默认停在"基础模板"而非已配置的模型——用户不知道要手动切换下拉框，导致 AI 回答"我目前无法使用 AI 模型进行分析"。新增 `hasUserSelectedModel` 标志：首次打开助手页且用户从未手动操作过模型选择器时，如果存在已配置的模型档案，自动选中第一个；一旦用户手动切换（含切回基础模板），就尊重用户选择不再覆盖。新增 3 个单测。
- **基础模板模式下身份/能力类问答答非所问**："你是谁"被路由到 FallbackPath，机械回答"我目前无法使用 AI 模型进行分析"。根因是 `route_query` 的闲聊触发词列表太窄（只有"你好/hello/帮助"），身份类问题（"你是谁/你是什么/who are you"）和能力类问题（"你能干什么/介绍下自己"）全部漏网。统一修复：扩展 DirectPath 触发词覆盖身份/能力问答；`direct_answer` 新增身份模板（介绍工作助手身份 + 提示当前模式）和能力模板；`fallback_answer` 文案改为引导性而非机械报错（"这个问题我需要 AI 模型才能回答好"）。新增 5 个测试。
- **基础模板模式大幅扩展工作查询关键词覆盖**：之前 `work_signals` 列表只收了某个说法的单一表达（如"做了什么"但不收"干了什么"，"应用"但不收"软件"），导致大量常见工作查询问法（"忙不忙""摸鱼了吗""数据怎么样""开会开了多久""在干嘛"等 15+ 种）落到 FallbackPath 给引导文案。按同义词家族扩展：做/干/搞、忙、数据/情况/概览、专注/产出/摸鱼/下班、app/软件/程序、多久/小时、开会/会议、小结/汇总、干嘛/干什么。同时 `fast_answer` 在无时间词时默认查"今天"（而非全量 10000 条），让"小结""数据如何"等无时间词查询更聚焦。新增 1 个测试覆盖 15 种扩展问法。
- **日报提示词预设编辑**（#130）：日报页面的提示词预设下拉菜单新增编辑按钮（铅笔图标），用户可以直接修改已有预设的名称和内容，而不必删除后重建。编辑保存逻辑区分"新建"与"更新"两种场景。
- **配置文件原子写入与故障安全启动**：配置加载/保存改为原子操作（临时文件 + rename + fsync），写入前自动备份上一版本（`.bak`），临时文件通过 RAII 在所有路径（含错误/panic）自动清理。启动时若主配置损坏，从备份恢复或以故障安全默认值（关闭所有采集/权限/远端）启动，保留损坏现场供排查。支持 4 种状态（Loaded / Missing / RecoveredFromBackup / Corrupted），故障状态下跳过权限初始化和自动写回。Unix 临时文件权限 0600 防符号链接劫持。新增覆盖四态组合、原子覆盖写、备份生成、父目录 sync 失败等场景的测试。
- **MCP Server localhost 请求认证**：MCP Server 对 localhost API 的所有请求现在携带 Bearer token 认证（从 `localhost_api_token.txt` 读取）。fail-closed 设计——token 缺失或格式非法时不发请求，降级到本地模板/历史数据。每次请求重读 token 文件，支持主应用轮换后即时生效，无需重启 MCP。2 秒超时，请求失败不阻塞 MCP 主流程。
- **Bot 日报生成 API 对齐**：IM Bot（Telegram/飞书/企业微信/钉钉）触发的日报生成改为走标准 localhost API（`POST /v1/reports/generate`，JSON body），替代旧的查询字符串拼接。鉴权从 URL token 升级为 Bearer header（token 不再暴露在 URL/访问日志里）。服务端同时保留 GET（query）和 POST（JSON body）两条路径，旧版 bot 仍可兼容。

### 优化
- **日报页面视觉重设计**：去玻璃拟态 + 斜纹纹理，改极简现代风（统一 token、轻阴影、大留白、统计卡重做、表格去重阴影 + zebra、活动时间线折叠样式现代化）。
- **助手多轮对话携带工具历史轨迹**：历史 payload 此前只发 `{role, content}`，丢掉了工具步骤信息，导致下一轮模型重走上一轮失败的工具（例如上轮 `search_memory` 0 条后下轮又先调 `search_memory`）。现在 assistant 消息若含已完成工具步骤，会以紧凑内联摘要追加到 content 尾部再发后端。摘要按工具类型区分，避免数字误导：
  - 失败工具：`web_search↯`（最强"别再试"信号）
  - `search_memory`：`search_memory→0条`（references 仅它贡献，hits 有真实语义）
  - 其他成功工具：`query_activities✓` / `fetch_url✓`（不写数字，因为它们的 hits 恒为 0——`collect_references` 全项目仅 search_memory 调用一次）
  - 示例：`[工具：search_memory→0条 | query_activities✓ | web_search↯]`
  - system prompt 增加声明，告知模型这种方括号片段是机器摘要、不要复述给用户（4 个 locale 各自翻译，加在 `ask.rs` 的 `build_assistant_system_prompt` 末尾——该函数是生产路径 `chat_work_assistant` 唯一调用的 prompt 源；之前曾误加在 `executor.rs` 的 `DEFAULT_SYSTEM_PROMPT`，但生产路径 `unwrap_or` 永远走不到，是死代码，已修正）。
  - 同时：后端 `StepResult` 事件增加 `ok: bool` 字段区分真正失败 vs 成功但 0 引用；前端跳过 streaming 或失败的半截轮次；补齐端到端“防重走失败路径”、旧事件三态兼容和各 locale system prompt 契约测试。
- **助手流式请求隔离与可靠投递**：每次回答按 assistant 消息 ID 定点更新，并以 `sendingRequestId` 绑定发送占用；页面卸载后可立即开始新请求，旧请求的迟到事件和 finally 不会污染或释放新请求。Token 增量仍允许在背压时丢帧，`StepStart`、`StepResult`、`Done` 则通过内部 ACK 等待 Tauri Channel 实际发送成功；桥接失败会向 Agent 传播并取消已在途的模型/工具 Future，且不会误走 FastPath。
- **工具计数测试断言修复**：移除 `get_weather` 后基础工具数从 5 变 6，但 `test_openai_tools_format_is_valid` 的断言未同步更新，已修正。

## [1.0.55] - 2026-07-09
### 修复
- **右键 WebView 原生菜单**（#126）：应用壳层全局拦截 `contextmenu`，避免桌面应用内弹出浏览器默认右键菜单；新增回归测试覆盖监听与清理逻辑。
- **概览指定日期选择框**（#119 / #127）：日期选择器浮层已改为挂载到 `body` 并使用 fixed 定位，避免被概览容器裁剪；同步修复日期范围前后切换口径。
- **AI 助手时间上下文**（#122）：向助手提示词注入当前日期与时间，避免回答中出现过期或错误时间。
- **桌宠多屏位置**（#120）：记录并恢复桌宠所在显示器位置，减少多屏环境下重启或拖拽后的漂移。
- **设置页更新开关**：自动检查更新开关现在会立即持久化，避免切换后未保存导致下次启动状态不同步。
- **启动与 Linux 发布链路**：修复开机自启、Linux release 工作流与相关打包校验问题。
- **构建 CSS 警告**：避免 Tailwind 把 Markdown 表格分隔符正则误识别为任意属性 class，生产构建不再输出 CSS minify syntax warning；新增构建警告守护测试。

### 优化
- **日报页面**：重做日报视觉结构，收紧 Markdown 表格布局，避免表格右侧留下大块空白。
- **工作助手**：优化模型选择器尺寸、Markdown 渲染与流式输出节流，提升长回复阅读和输入体验。
- **设置页本地 API**：本地 API 地址在未启用或本机地址场景下跳过无意义自动测试，减少误报。

## [1.0.54] - 2026-07-02
### 修复
- **日报区块隐藏后无法恢复**（#109）：`apply_preferences` 生成日报时不再过滤 hidden 区块，隐藏完全交给前端显示层；在「管理段落」恢复后立即可见，无需重新生成。
- **托盘菜单英文模式下中文残留**（#115）：config 新增 `locale` 字段，托盘菜单文案三语映射（zh-CN/en/zh-TW），切换语言时实时刷新托盘。
- **助手回答不跟随提问语言**（#116）：AI 增强（Agent）/ 闲聊（Direct）/ 无模型（Fast）三条路径现在都按用户提问语言回答，英文提问英文答、中文提问中文答，不管工作记录里存的是什么语言。
- **FastPath 分类名中文残留**：`fast_answer` 分类分布按提问语言分流，英文模式不再夹中文分类名。
- **更新器兜底泄漏**：未识别的后端中文状态串不再原样返回，避免英文模式泄漏。
- **Linux 编译失败**（1.0.53 遗留）：修复 `commands.rs` 拆分后 `avatar.rs` 的 `include_str!` 路径深一层、`system.rs` 跨模块引用 `gnome_avatar_extension_*` 私有函数未补 `use`、`avatar.rs` 缺 `PathBuf` import。
- **托盘命令编译**：`set_app_locale` 改私有（避免 `#[tauri::command] pub` 宏重导出冲突 E0255）+ `AppHandle` 所有权 clone。
- **日报生成闭包语法**：修复 `apply_preferences` 闭包参数多余 `>`。

### 优化
- **静默驻留 GPU 占用**（#118）：开机自启 + 静默驻留 + 轻量模式下，主窗口隐藏时暂停所有 CSS 动画（`animation-play-state: paused`），WebView2 GPU 占用从 7-30% 降到接近 0。
- **概览日期栏**：删除日期选择栏下方的分隔横线（消除"内容没加载"误解）+ 收紧高度。日历式选择控件实测后保留原 text input。
- **桌宠设置文案**：精简桌宠相关描述（两轮 + 主动 AI 提醒优化），数据说明新增"会消耗少量 token"提示。

## [1.0.53] - 2026-06-28
### 新增
- **删除活动记录**：时间线支持按单条 / 按日期 / 按时间段 / 按应用四种粒度删除活动记录，连带删除对应截图与 OCR 文本；删除后自动失效受影响日期的日报与小时摘要缓存，统计与日报保持一致（下次查看时重新生成）。
- **界面视觉风格 A/B/C**：新增 Quiet Pro / Soft Layers / Compact Data 三套界面风格，每套带预览，经 `ui_visual_style` 配置持久化（Beta）。

### 修复
- **英文模式下中文残留**（#111）：将绕过 i18n 的硬编码 UI 文案（窗口按钮、AI 测试报错、桌宠 aria-label、图表图例等）提取为三语 key 并对齐；助手 starter prompt 改为按 locale 切换（含动态数据翻译）；新增 `I18nEnglishLeak` 回归测试守住 title/aria-label/placeholder 不含中文与三语 key 对齐。
- **自定义分类在日报时间分配里丢失**（#109）：`normalize_category_key` 此前把任何非内置分类折叠到 "other"，导致日报时间分配看不到自定义分类；改为聚合时保留自定义 key。
- **时间线「周期摘要」按钮日期跳到今天**（#110）：现在携带当前查看日期；同时修复从 `location.hash` 读取 `?date`（hash router 下 `location.search` 为空的潜在 bug，也影响桌宠继续）。
- **Linux KDE Plasma Wayland 启动崩溃**（#113）：上游 webkit2gtk/GTK 在 Wayland 下的协议错误（`Error 71 dispatching to Wayland display`，Tauri #10702），在 GTK 初始化前注入 `WEBKIT_DISABLE_DMABUF_RENDERER=1` 规避；README 文档化该问题与 `GDK_BACKEND=x11` 兜底。
- **响应式**：日报 Markdown 表格横向滚动、长 URL/文本换行不再撑破列；node gateway 设备表单与概览浏览器卡片在窄窗口下正确 truncate。
- **更新器镜像失效**：原 `ghproxy.cn` + `ghp.ci` 返回 HTTP 000，换为 `gh-proxy.cn` + `gh-proxy.com`（验证 HTTP 200）。
- **崩溃无记录**：启动安装 panic hook，崩溃信息（版本/消息/位置/栈）写入 `<data_dir>/crashes/crash-<ts>.log`（此前静默退出无法诊断）。
- **AiTransform model 绑定**（曾被误改为 `_model`）恢复。
- **自定义分类名在多语言下被误翻译**：自定义分类（非系统内置）现在保留用户定义的名字，不再被 `translateCategoryLabel` 误覆盖；系统分类仍按语言翻译。影响时间线、概览、每小时图表的分类名显示。
- **Linux 构建失败**（`commands.rs` 拆分遗留）：avatar.rs 移到 `commands/` 子目录后，`include_str!` 的相对路径深一层未更新（`../../` → `../../../`）；system.rs 跨模块调用 avatar.rs 的 `gnome_avatar_extension_*` 私有函数未补 `use`，且 avatar.rs 缺 `PathBuf` import。修复后 Linux CI 编译通过。

### 优化
- **桌宠 GPU 占用**：移除 `.scene-svg` 的 `will-change`（曾强制整场景常驻 GPU 合成层）+ 降低 drop-shadow 模糊半径 18→12px（视觉近乎不变，减少逐帧 filter 开销）。
- **语言切换展示**：当前语言改用深色背景标记（去掉 ✓）；README 当前语言加粗且不再自链接，「中文」改为「简体中文」以消歧义。
- **代码质量**：移除约 981 行死代码（16 个未用函数及测试 helper）；6 处重复的隐私过滤函数收敛到 `core/privacy.rs` 单一来源；`commands.rs`（8517 行）按领域拆为 14 个子模块（行为不变，注册路径不变）；workspace 0 warning。
- **每小时图表时长格式 locale 化**：坐标轴刻度与图例时长改用 `formatDurationLocalized` 跟随当前语言（此前硬编码 `Xh/Xm/Xs`）。
- **README 界面预览扩充**：三语 README 新增工作流 GIF 与各界面截图（概览 / 时间线 / 时间线详情 / 日报 / 助手 / 接入管理 / 小时总结 / 各设置页），覆盖完整工作流与配置面；配套截图脚本 `capture-readme-hourly-summary.mjs` 与守护测试。

## [1.0.52] - 2026-06-23
### 新增
- **AI 编排日报区块**：日报改为 AI 依据当日工作模式决定区块顺序，每区块支持固定（pin）/隐藏（hide），用户偏好始终覆盖 AI 顺序；AI 排序缓存到配置避免重复 LLM 调用；区块按顺序动态编号（一、二、…）。
- **主动式桌宠**：由模型自定开口节奏/内容/情绪（`next_check_minutes`），跟进卡片裁剪，卡片或气泡显示期间窗口拖拽兜底。
- **动态开场提示**：配置 AI 模型后，工作助手依工作上下文动态生成开场提示（新增 `generate_text_with_model` 命令）。
- **小时活跃度多模式**：概览支持按周 / 今日 / 日期范围查看每小时应用分布。
- **Windows 便携版**：release 资产新增 Windows portable zip（#106）。

### 修复
- **统计口径不一致**（根因）：隐私过滤前移到聚合入口 `get_daily_stats_with_segments_filtered`，总时长 / 应用 / 分类 / 小时分布 / 网站统计从此同口径（概览、MCP、日报生成、旧报重渲染统一走前置过滤，替换各处后置过滤）。
- **每小时分布可能超过 60 分钟**：跨小时活动改为按小时切片、重叠区间只计一次（归最新观测），小时桶不再溢出。
- **切应用时长丢失**：上一应用无历史记录时也创建轻量回补记录，不再丢弃 `duration_delta`。
- **MCP 读接口未过滤隐私**：timeline / memory / sessions / current_context / weekly 资源统一应用"忽略应用 / 排除域名"过滤。
- **MCP notifications/initialized 返回无效空响应**（#107）。
- **Linux CI apt 抖动**：azure 镜像换回 archive.ubuntu.com 主源，`apt-get update` 加 180s 超时 + 3 次退避重试（曾导致 v1.0.51 一次 6 小时的取消）。

### 优化
- **统一明暗主题色彩**：浅色文字从 8 级 slate 收敛到 4 级；深色全面迁移到 GitHub Dark 调色板（`#0d1117/#161b22/#21262d/#30363d/#484f58`），`text-white`→`#e6edf3`，所有阴影补 `dark:` 覆盖；移除设置保存坞渐变、关于页更新横幅居中、移除更新 toast。
- **语言切换展示**：语言按钮显示语言全称（不再只是 ZH/EN/TW 短码）、选中项加 ✓、顺序统一为 ZH/EN/TW。
- **界面细节**：滚动条与卡片圆角细化、小时活跃度图改 24 列网格对齐（首尾刻度不再溢出）、无数据时摘要显示占位、深色白底高光清理、设置 chip 形状、侧栏导航激活态（#108）。
- **README 重写**：三语言（EN/ZH/TW）引言与特性文案重写更准确，新增 Localhost API 措辞守护测试，群组资源压缩。

## [1.0.51] - 2026-06-18
### 修复
- 活动采集误判（根因）：`normalize_electron_app_name` 的浏览器标题推断缺少进程名守卫，对所有进程生效。当窗口标题含浏览器关键词（如 PyCharm 打开 `test_edge_cloud.py`，文件名含 "edge"）时，明确进程名被强制改写成浏览器名（PyCharm → Microsoft Edge），导致 `app_name` 与 `executable_path` 不一致落库——这正是"浏览器活动却显示编译器图标"的源头。现仅在 Electron / Helper 类通用进程上才用标题推断，明确进程名直接保留。
- Safari/WebKit 内部 helper 误识别为独立浏览器：`com.apple.SafariPlatformSupport.Helper`、`com.apple.WebKit.Networking` 等 Safari/WebKit 的瞬态内部 XPC service 被采集为独立 app，且 bundle id 含 "safari" 被 `is_browser_app` 判为浏览器，于是以原始 bundle id 作为独立"浏览器"条目显示。`normalize_display_app_name` 现将这类内部 helper 统一归并到 Safari。
- 应用图标错配（显示层）：当 `executable_path` 与 `app_name` 不一致时，不再错误显示编译器图标。`executable_path` 解析出的 bundle 现需通过名称匹配校验（`macos_score_app_bundle_name > 0`）才获最高优先级，不匹配时回退名称评分兜底。
- 图标缓存陈旧：macOS 磁盘缓存目录与前端 localStorage 同步升级到 v2，失效历史脏缓存（曾因上述错配把错误的编译器图标持久化），确保修复对存量用户立即生效。
- 基础模板（无模型模式）现完整覆盖工作查询：含时间词（今天/本周/最近…）或工作词（做了什么/总结/效率/复盘…）的查询都走统计模板，不再漏到「无法分析」兜底；仅明显非工作领域（天气/股票…）才兜底。
- 助手 `ask.emptyResponse` i18n 缺失（Ask.svelte 引用但三语无此 key，曾 fallback 到字面 key）现补全三语。
### 优化
- 工作助手路由简化为「相信模型」：移除复杂意图 / 时间段 / 工作关键词等硬编码分类规则，有模型时统一交给 Agent，由模型自行判断问题是否工作相关、是否调用工作记录工具。非工作问题（如「今天天气怎么样」）不再因含「今天」被误判为工作查询而返回工作统计；无模型时走模板兜底。
- 助手界面优化：品牌色 indigo 一致化（发送按钮渐变、用户气泡、步骤指示器、欢迎页光晕、入口卡片 hover）；新消息/步骤/错误进入动效与按钮按下反馈；助手气泡 padding 打架修复。
### 文档
- README 改为英文主页（中文版移至 `README.zh.md`）；三语补齐 Localhost API 文档（认证 / 接口列表 / 示例）；界面预览图按语言对应 `Introduction_en/zh/tw` 目录；语言切换链接统一为 English · 中文 · 繁體中文。
- 新增多语言 release notes 机制（参考 cc-switch）：中文主体取自 CHANGELOG，英文 / 繁体作为独立文件放在 `docs/release-notes/`，发布时 CI 自动在 Release body 顶部生成语言切换链接。
- 新增 MIT LICENSE 文件 + `package.json` license 字段 + README 链接（此前仓库无 LICENSE，法律上等于「保留所有权利」，与开源意图矛盾）。
- README 顶部 + 关于页加「个人使用 · 数据本地」定位声明，弱化「员工监控」误解。

## [1.0.50] - 2026-06-14

### 新增
- 工作助手流式输出（阶段 1）：基于 Tauri `ipc::Channel` 统一事件流，回答过程实时展示工具调用步骤（"记忆检索 → 找到 N 条 → 统计聚合…"），并把命中记录作为"引用"折叠展示，让结论可追溯。命令仍返回完整 `AssistantAnswer`（双通道冗余，前端注释掉 `onEvent` 即 100% 回退到老路径）

### 修复
- 工作助手隐私：Agent 工具链与 FastPath 现在遵守"忽略应用 / 排除域名"设置，避免把被屏蔽应用的窗口标题作为工具结果发给云端 LLM（违背"本地优先"承诺）
- AI 工具链空转：provider 返回 ToolCall 停止原因却无实际 tool_calls 时立即终止，不再空耗最多 8 轮 API 配额与 30s 等待
- OCR 隐私：PaddleOCR 解析失败的错误信息不再包含原始 OCR 文本（可能含密码 / 私信），避免进入日志
- OCR 管道死锁：OCR 与监控外部命令改为后台排空 stdout/stderr 管道，密集文本 / 大图输出不再因管道缓冲（64KB）写满而必现超时
- OCR 文本清理：单字中文乱码按字符数过滤（原按字节，3 字节单字漏过）；`is_valid_char` 白名单补齐代码 / URL 符号与日韩文，不再破坏屏幕上的代码与 URL
- 空闲检测串扰：用户恢复活动后不再因屏幕与离开时相似被快速重新误判为空闲（`reset` 同步清除哈希基线）
- 跨午夜加班统计：跨午夜工作段（如夜班 22:00→次日 06:00）不再把班内活动误计为加班
- 浏览器分类一致性：Tabbit 在 `categorize_app` 与 `is_browser_app` 两处均识别为浏览器
- 桌宠跟进遮蔽：被"稍后提醒"的高分项目不再占据唯一推荐位、压制次高分可提醒项目
- 桌宠气泡测试：对齐 #101 收窄后的气泡宽度规格，2 个过时测试与残留注释同步更新
- Live2D：渲染循环在组件销毁时正确取消（修复 RAF 泄漏）；动作 / 表情资源加载改为 `await`，idle 动作不再因竞态丢失

### 安全
- Gemini API Key 改用 `x-goog-api-key` 请求头传递，不再出现在 URL（避免网络错误 / 代理日志泄露密钥）
- MCP `resources/read` 现在和 `tools/call` 一样经过权限策略校验与脱敏，AI 编码工具无法再绕过 PolicyEnforcer 读取未脱敏的 OCR 文本 / 窗口标题 / 截图路径
- Localhost API token 校验改为常量时间比较，与飞书 / 钉钉 / 企微签名校验的安全基线一致
- 钉钉回调签名时间窗口从 ±1 小时收紧到 ±5 分钟（与企微一致），降低请求被截获后的重放风险

### 优化
- Cloud 模式日报：AI 调用失败时回退到基础统计模板，与 Summary / Local 模式行为一致，不再整条命令失败

## [1.0.49] - 2026-06-10

### 新增
- 工作助手 Agentic 架构（Stage 1-6）：五层结构 Tools → Model → Executor → Memory → Orchestrator，支持多轮工具调用、自动降级和路由分流
- 5 个 Agent 工具：search_memory、analyze_intents、aggregate_stats、category_search、trend_comparison
- 统一 Model 层：OpenAI 兼容 / Claude / Gemini 三种 provider 的多轮 tool-calling 格式转换
- Agent 循环：最大 8 轮迭代 + 30 秒超时双重保护
- Orchestrator 四路径路由：Direct / Fast / Agent / Fallback，Agent 失败自动降级
- 加班时长统计：秒级精度，统计面板可见
- 桌宠鼠标穿透选项：透明区穿透、本体可交互，可开关
- 日报系统提示词可编辑：支持自定义 override + 一键还原默认
- 浏览器识别扩展：内置 Tabbit 支持
- 自动导出重试机制：导出失败与报告生成解耦，支持补导出

### 修复
- 浏览器 URL 合并准确性：SPA 页面导航后 URL 正确刷新；URL 采集失败时不再误合并不同标签页
- 桌宠弹窗溢出：缩小最大宽度以适配紧凑窗口
- 趋势对比分类排序：修正排序比较器的下标错误
- 测试编译恢复：DailyStats 和 merge_activity 签名变更后测试调用同步更新
- 仓库 URL 同步：所有链接从 Work_Review 更新为 Work-Review

### 优化
- 复杂意图关键词扩展（最多/最少/排名/占比/效率）
- FastPath 改用 get_activities_in_range 替代纯 FTS 查询
- 数据库连接改为 Arc<Mutex> 支持异步安全 Clone

## [1.0.48] - 2026-05-26

### 新增
- 企业微信（WeCom）群机器人集成：支持通过 Webhook 推送日报、工作提醒，配置入口在接入管理页
- 钉钉（DingTalk）群机器人集成：支持自定义关键词和签名密钥，接入管理页可独立启停
- Telegram Bot 绑定码机制：用户可在 Bot 中输入绑定码关联设备，替代直接 chat_id 配置
- MCP Server 路径一键复制：接入管理页新增数据库路径和配置路径的复制按钮
- 存储上限可配置：设置 > 存储页可直接调整存储上限（256 MB - 100 GB）
- Localhost API 新增截图 URL 字段：时间线和统计接口返回截图的完整访问路径

### 安全
- MCP Server 脱敏强化：tool 返回值递归清理 screenshot_path、ocr_text，window_title 截断至 40 字符
- MCP Server 启用开关：配置中 mcp_server_enabled 为 false 时 binary 直接退出，确保开关为真开关
- MCP Server 权限门控：所有 tool 调用统一经过 PolicyEnforcer 权限检查

### 优化
- 提取 bot_common 公共模块：Telegram、飞书、钉钉、企业微信 Bot 共享签名验证、密钥安全读写等基础能力
- 放宽 localhost API loopback 限制：允许非 loopback 地址访问（适配局域网场景）
- 清理 Rust 编译警告，lib/ 目录加入 gitignore

## [1.0.47] - 2026-05-20

### 新增
- 远程截图存储：支持 S3/MinIO 和 WebDAV 上传截图，上传成功后回写 URL 到数据库（#88, @lumia1998）
- 时间线详情弹窗隐私规则快捷设置：三段式按钮 + 确认弹窗，支持完全记录/仅统计时长/完全忽略
- About 页面自动检查更新开关：可关闭启动时自动更新检查
- Localhost API 文档写入 README（#83）

### 安全
- Telegram Bot 新增 chat_id 白名单：未授权会话拒绝响应（#86）
- 飞书 Bot 强制要求 verification_token：未配置时拒绝所有请求（#86）
- localhost API 限制 loopback 地址：禁止绑定 0.0.0.0 暴露到局域网（#86）
- Tauri webview 添加 CSP 安全策略（#86）
- shell:allow-execute 添加 args 校验（#86）
- config.json 写入时设置 0o600 权限（#86）
- 更新 endpoint 顺序调整：GitHub 直连优先，代理作为 fallback（#86）

### 优化
- 隐私规则按钮从合并样式改为带间距的独立卡片
- 三语 i18n 补全（zh-CN / en / zh-TW）

## [1.0.46] - 2026-05-09

### 修复
- 修复所有 AI 提供商（豆包、OpenAI、Claude、Gemini、Ollama 等）的 API 地址双重拼接问题：用户填写完整 URL 时不再导致 404。
- 模型列表下拉菜单中"手动输入"选项移至顶部。
- 修复浏览器/应用已打开时启动软件导致 URL 丢失和分类显示"未知"的问题：首次轮询现在会主动探测浏览器 URL。

## [1.0.45] - 2026-05-08

### 新增
- 提示词预设模板：支持保存、切换、删除多套日报提示词，下拉菜单显示当前选中预设名称，删除前需确认。
- 实时统计卡片：日报页面上方展示当日工作时长、活跃应用等实时数据，替代嵌入模板的过时统计。
- 本地 API 新增 9 个端点：今日统计、总览、按日统计、最近应用、分类概览、分类列表、语义分类、小时摘要、存储信息。
- 本地 API 新增按小时应用分布、周回顾、时间线端点；报告生成接口由 POST 改为 GET 保持一致性。
- 可配置空闲判定阈值（默认 5 分钟，范围 1-60），新增工作时段开关（适合居家办公场景）。
- 隐私规则支持批量多选操作（#81）。
- 截图保留新增"永久保留"选项（#79）。
- CI 新增 Linux ARM64 (aarch64) deb 构建。

### 优化
- 提示词预设下拉菜单重新设计为卡片式布局：选中项显示勾选标记，删除按钮 hover 时才显示。
- API 文档示例页面重设计为卡片式布局：GET 绿色 / POST 琥珀色方法标签，深色终端风格 curl 代码块。
- 日报统计修复：使用 report_blocks 实时替换机制解决统计数据过时问题（#80）。
- 应用名称识别从约 60 个扩展到 130+，覆盖更多系统工具和常见桌面应用（#79）。
- 设置页按视觉分区重新组织：工作时段、报告配置、系统行为。

## [1.0.44] - 2026-04-30

### 优化
- 移除未使用的前端依赖 `chart.js` 和 `@tauri-apps/plugin-fs`，减小安装包体积。
- 移除未使用的 Rust 依赖 `toml`，收窄 `tokio` features 从 `full` 到按需启用（rt-multi-thread、macros、sync、time、net、fs、io-util）。
- 排除 Live2D SDK 构建产物进入 dist，通过 Vite `closeBundle` 插件自动清理。
- 修复 i18n 区域文件拆分后多个测试文件的路径引用错误。

## [1.0.43] - 2026-04-30

### 新增
- 新增自适应截图分辨率：自动按屏幕宽度控制截图最大宽度，并在设置页提供「自动 / 固定宽度」模式切换与宽度滑杆（#44）。
- 工作助手升级为 FTS5 全文检索：活动记录、小时摘要和日报均可进入记忆搜索；意图识别扩展到时间统计、对比、列表和自由问答等 8 类。
- 工作助手新增动态提示词构建、对话上下文连续性、实体抽取、日内活跃洞察、异常提示、进度条式基础模板和后续行动建议。
- 日报支持段落级编辑并保存回本地已保存日报，编辑后的内容可继续用于回看与导出。
- 设置页新增日报 Markdown 自动导出开关：配置导出目录后，可单独控制自动生成日报是否同步导出文件。
- 日报生成状态进入全局缓存，跨页面切换时能保持更一致的生成中反馈。

### 优化
- 助手和日报提示词改用连续工作 Session 作为时间线输入，减少碎片活动直接进入 AI 上下文带来的噪声。
- 浏览器未识别页面现在保留原始 URL 并归入「未识别页面」，避免浏览器总时长与域名明细不一致；域名总时长会在 URL 截断前完成统计。
- 浏览器窗口标题会清理内存警告、浏览器名、标签计数等内部信息，减少无效上下文进入时间线、日报和助手分析。
- 记忆搜索在活动记录之外补充小时摘要和日报 fallback，提高历史问答命中率。
- 设置页视觉统一为新的卡片行与磨砂舞台结构，常规、存储、AI 等区域减少重复分隔线和冗长提示。
- 日报页头部和模式信息进一步收敛，移除无用的报告元信息代码。
- 桌面化身继续以原版标准 PNG / BongoCat 预设作为正式展示；键盘专注、极简办公等预设资源保留在注册表中，并移除静态预设中的彩色热点覆盖层，避免看起来像调试图层。
- 专注会话倒计时现在会随时间正常刷新。
- README 中文 / 繁体 / 英文同步更新桌面化身定位、自然语言时间范围示例和社区二维码布局。
- 自然语言时间范围说明从“最近 3 天”调整为“最近 N 天”，与实际支持任意天数的解析能力保持一致。

### 修复
- 修复 OpenAI 兼容接口在 DeepSeek 等提供商上因默认流式返回导致日报解析失败的问题：请求中显式关闭 stream。
- 修复云端分析截图路径被重复拼接 `screenshots/screenshots/...` 的问题。
- 修复自适应截图遇到零高度图片时可能 panic 的问题。
- 修复清理旧活动时只删除截图文件、不删除数据库记录的问题。
- 修复重复活动清理缺少事务保护，异常中断时可能造成数据不一致的问题。
- 修复生成日报并发标记存在 TOCTOU 竞态的问题，并改用 RAII guard 保证 panic 或失败后也能释放生成中状态。
- 修复迁移数据目录时历史数据库可能被目标目录清理流程删除的问题：现在按「清理目标目录 -> 复制托管文件 -> 最后备份数据库」执行，并在迁移后重建 FTS 索引。
- 修复删除历史活动时无法利用索引的问题：按时间戳范围删除，避免对每行调用 `date()`。
- 修复隐私规则默认值、域名后缀匹配、`excluded_apps` 到 `app_rules` 自动迁移、域名提取重复实现等问题。
- 修复隐私应用匹配过宽的问题，避免短词或双向 contains 误伤无关应用。
- 修复日报缓存 key 与 locale 不一致、TTL 类型错误、自动日报缓存失效 key 不一致等导致的重复请求或旧数据问题。
- 修复快速切换日报日期、时间线日期和详情时旧异步响应覆盖新状态的问题。
- 修复助手调用后端长时间无响应时前端无限转圈的问题，新增超时保护。
- 修复 URL 截断按字节切片导致中文等多字节字符 panic 的问题。
- 修复飞书 token 在长任务结束前可能过期、Telegram 命令解析失败时静默无响应的问题。
- 修复时间线加载更多后的排序与归一化不一致、空列表访问首项崩溃等问题。
- 修复节点身份文件写入非原子操作导致崩溃时文件损坏的问题。
- 修复侧边栏初始路由为空时可能崩溃的问题。
- 修复桌宠分类时没有传入用户自定义分类、输入监听失败后无法重试、分类覆盖短词误匹配等问题。
- 修复截图合并路径中缺失 id、启动窗口 unwrap、MCP Server mutex poisoned 后 panic 等稳定性问题。
- 修复截图哈希位移溢出、MCP Server locale 与分段统计、缓存活动去重、App 启动 pending cleanup、设置 dirty flag、Linux Wayland 环境变量恢复顺序等问题。
- 修复自动生成日报在设置了导出目录后总是导出 Markdown 的问题，现在会遵循新增的自动导出开关。
- 修复已保存日报缺少内容更新入口的问题，段落编辑保存后会写回本地报告记录。

### 清理
- 移除已由 Rust 实现替代的 Python Bot 目录与示例配置。
- 移除旧的 `src-tauri/src/analysis` 重复实现、未使用的前端组件 / 工具函数、过期桌宠 PNG 资源、无用 CSS 与旧 superpowers 计划文档。
- 移除未启用的 Live2D 运行时依赖与发布资源，1.0.43 仍以当前 PNG / BongoCat 桌面化身链路作为正式展示。
- 清理 core、skills-engine、前端 i18n、JS 导出和导入中的死代码，降低后续维护成本。

## [1.0.42] - 2026-04-28

### 新增
- 支持多段工作时间设置（最多 8 段），休息时间不计入工作时长，设置 > 常规可直接管理（#71）。
- 概览页浏览器域名支持语义分类标记（如标记为「休息娱乐」则不计入工作时长），修改后自动回填历史记录（#72）。
- 网站语义分类现直接映射到基础分类，影响工作/休息时长统计。
- 时间线时段摘要支持展开全文 / 收起，不再截断较长内容。
- 接入管理页新增 MCP Server 开关与配置参考，自动填入数据库和配置路径。
- README（中文 / 繁体 / 英文）新增 MCP Server 配置说明、接入管理截图、Bot 联动 Beta 标注。
- 设置页标题栏支持 sticky 吸顶，滚动到底部填写 AI 模型配置时保存按钮始终可见（#74）。
- 日报自动生成时间现可在设置中自定义（#73）。

### 优化
- 接入管理页布局重构：Bot 集成（Telegram / 飞书）和 MCP Server 各自归入大卡片，层级更清晰。
- 设置侧栏 Beta 标签缩小并禁止换行。
- 按小时活跃度摘要卡片文本居中显示。
- 编译优化 profile 提升至 workspace 根 Cargo.toml。
- 本地 Mac 打包脚本仅生成 .app，跳过 updater 产物。
- 浏览器 URL 去重缓存增加 256 条上限，防止长期运行内存持续增长。
- 桌宠继续提醒 snooze 记录在每次操作时自动清理已过期条目。
- 前端按日期缓存增加 7 天淘汰策略，避免浏览历史页时内存累积。
- 图标缓存 store 更新改为 100ms 合并批量写入，避免逐个图标触发全量对象拷贝。
- 助手会话 localStorage 持久化增加 500ms 防抖，减少主线程阻塞。
- 桌宠后台循环分类规则改用脏签名检测，仅在配置变更时重新 clone，降低热路径分配压力。

### 修复
- 修复日报自动生成时间未适配多段工作时间的问题：现取最后一段结束时间。
- 修复域名语义修改后历史记录基础分类未同步更新的问题。
- 修复测试文件路径指向旧 src-tauri 的问题（适配 core crate 提取）。
- 修复 macOS CI 构建产物路径因 CARGO_TARGET_DIR 导致 src-tauri 路径翻倍、后续步骤找不到产物的致命问题。
- 移除无效的 macOS re-sign 步骤（DMG 和 updater 已在 Build 阶段封包，后续重签不影响交付物）。

## [1.0.41] - 2026-04-20

### 新增
- 继续提醒卡片补充完整 tooltip 与 aria 文案，桌宠窗口随卡片内容动态调整尺寸。
- 桌宠提醒与设置页 UX 改进。

### 修复
- 修复 Windows 11「开机自启 + 静默驻留」仍弹出主窗口的问题：自启动命令现正确携带 `--hidden` 参数，静默模式与启动参数保持同步。
- 修复桌宠展开后窗口超出可视区域的问题：展开时按新尺寸钳回屏幕范围内；若展开 RPC 失败则自动回滚窗口尺寸。
- 修复自启动切换失败时 UI 状态失同步的问题（#69）。
- 修复桌宠权限校验与社区文档相关问题。

## [1.0.40] - 2026-04-18

### 新增
- 为 GNOME Wayland 提供仓库内桌宠输入扩展，桌宠现在可以通过 D-Bus 获取最近一次按键与鼠标状态。
- 设置页支持自动安装 GNOME 桌宠扩展，并显示安装、启用以及“需要重新登录 GNOME Shell”状态。

### 修复
- 修复 macOS 在部分前台窗口场景下更容易选错时间线截图目标屏幕的问题：前台窗口位置与尺寸现直接参与截图目标判定。
- 修复同一时间线活动在后续合并时详情截图被新截图覆盖的问题：合并记录时保留首张归档截图路径，避免历史截图漂移。
- 修复时间线合并路径中仅用于 OCR 的新截图在任务完成后仍残留的问题：OCR 完成后会清理临时截图，减少无效归档。
- 修复时间线详情页“高清图”实际返回缩略图的问题：现直接返回归档原始 JPG，截图内容判断更准确。
- 修复 Linux X11 下桌宠缺少全局键鼠监听的问题，恢复完整键盘与鼠标联动。
- 修复 KDE / Hyprland Wayland 下桌宠输入能力不透明的问题，当前会按实际能力标记为“仅鼠标移动”。
- 修复 GNOME Wayland 下桌宠长期缺少可用输入链的问题，当前已支持鼠标联动与最近一次键盘按下联动。
- 修复 GNOME 扩展安装后状态不透明的问题：当扩展已启用但当前 Shell 尚未加载最新版本时，会明确提示重新登录后再试。

## [1.0.39] - 2026-04-16

### 新增
- 引入 BongoCat 风格的桌面化身资源与预设注册能力，`原版标准` 预设已接入当前桌宠链路。
- 补充第三方资源归属说明：README 与 `THIRD_PARTY_NOTICES.md` 现明确标注 BongoCat 上游项目与 MIT 许可来源。

### 修复
- 修复 Windows 下桌面化身无法稳定接收键盘与鼠标输入的问题，恢复 `原版标准` 预设的基础联动能力。
- 修复 Windows 在“全部窗口最小化 / 回到桌面 / 系统瞬态窗口”场景下仍将空闲时长续记到上一应用的问题，减少待机时间被误算为工作时长。
- 修复 Windows “启动时静默驻留”仍弹出主窗口的问题，校正自启动命令参数与隐藏启动行为。
- 修复 macOS LaunchAgent 自启动缺少隐藏参数的问题，静默启动会显式携带 `--autostart --hidden`。
- 下线尚未收口的 `键盘专注` 与 `极简办公` 预设，历史配置会自动回退到 `原版标准`，避免出现未完成交互带来的误导。
- 修复时间线实时更新时缩略图预热不及时的问题。

### 说明
- `桌面化身 Beta` 仍处于持续完善阶段，本次版本只将已收口的预设与交互链路纳入正式说明，未完成部分会继续迭代。

## [1.0.38] - 2026-04-14

### 新增
- 所有 AI 提供商（OpenAI、DeepSeek、SiliconFlow、Qwen、Zhipu、Moonshot、Doubao、MiniMax、Gemini、Claude）支持获取模型列表：点击刷新按钮即可下拉选择可用模型，不再需要手动输入模型名称。
- 时间线页面支持自定义应用分类：用户可创建自定义分类（名称 + 颜色 + 图标）并分配给任意应用，也可删除自定义分类（预设分类不可删除）。
- 概览页面支持自定义网站语义分类：用户可在修改域名分类时新建自定义语义分类，也可删除自定义分类（13 个内置分类不可删除），删除后相关域名回退到"未知活动"。

### 修复
- 修复 Windows 自定义窗口控件（最小化/最大化/关闭）外层包裹盒子的问题：去除容器边框和背景，按钮直接贴合窗口右上角。
- 修复时间线中时间圆点与时段摘要圆点距离中线不一致的问题：改为绝对定位，两种圆点统一紧贴轨道线。
- 修复 Windows 截图时屏幕边框闪烁黄色隐私指示器的问题：优先使用 GDI BitBlt（静默无感），失败时才降级到 Windows Graphics Capture API。
- 修复 macOS 截图时屏幕边框闪烁黄色隐私指示器的问题：优先使用 screenshots crate 的 CGDisplay 截图（静默无感），失败时才降级到 screencapture CLI。

## [1.0.37] - 2026-04-13

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.37] - 2026-04-13

### 修复
- 修复 Windows 自定义窗口控件（最小化/最大化/关闭）视觉悬空问题：按钮已从浮动毛玻璃胶囊改为扁平集成风格，更贴合系统原生观感。
- 修复时间线与时段摘要中时间轴样式不统一的问题：标记尺寸、阴影、轨道渐变和暗色模式已全部对齐。
- 修复 Linux KDE Wayland 下使用 sudo 运行时截图全黑的问题：启动时自动从原用户进程恢复 Wayland/DBus 环境变量，sudo 和非 sudo 均可正常截图和追踪窗口。
- 修复拖拽 PDF 等文件到窗口时应用被替换为文件预览器的问题：已全局拦截文件拖放导航。
- 修复 Windows 开机自启动"静默驻留"仍然弹出窗口的问题：增加诊断日志和 single_instance 二次自启动防护。
- 修复 Timeline 每次截图事件过度清除全部缓存的问题：改为仅清除概览缓存，保留日报和其他日期的时间线缓存。
- 修复自动日报生成无并发保护导致重复 AI 调用的问题：增加互斥标志防止生成未完成时重复触发。
- 修复数据目录迁移时 raw copy 导致 SQLite 数据库损坏的问题：恢复使用安全的 backup_to() 接口（含 WAL checkpoint）。
- 修复 Settings 存储回调缺少错误处理导致 unhandled rejection 的问题：补齐 try/catch。
- 修复 config.json 中 screenshot_interval 设为 0 或 1 导致持续截图、CPU 飙高的问题：增加 5-600 秒范围校验。
- 修复合并路径 OCR 哈希去重使用 static 变量导致跨活动污染的问题：改为循环局部 Arc 共享。
- 修复云截图采样超过 5 张上限导致额外 AI token 消耗的问题：增加硬性 break 上限。
- 修复待办提取标题包含 "TODO:" 等前缀的问题：改用正确的正则捕获组。
- 修复意图分类 fallback 分数（80）高于具体匹配（52-70）的问题：修正基础分叠加逻辑。
- 修复内存搜索全表扫描导致大数据量时变慢的问题：增加 LIKE WHERE 条件做初步过滤。
- 修复简体中文界面下助手引用类型显示原始 key 名的问题：补齐 zh-CN ask.referenceTypes 翻译。

## [1.0.36] - 2026-04-10

### 修复
- 修复日报页更容易让用户误以为“当前配置就是当前这份日报实际生成方式”的问题：页面现会区分当前日报的真实生成方式与当前配置，并在 AI 未真正生效时显示友好的回退原因。
- 修复 AI 增强日报在请求失败或返回空内容时前端只能看到“基础模板”、却看不到原因的问题：当前会将回退原因随日报一并保存并展示，便于判断是旧日报、配置不可用还是请求失败。
- 修复日报成稿区视觉重心偏左的问题：正文与生成时间现会作为同一条阅读栏整体居中，减少右侧大面积留白带来的不平衡感。
- 修复侧边栏激活态高亮条与导航图标重叠的问题：高亮条已移动到图标区外侧，减少激活项视觉冲突。
- 修复时间线重点卡片与详情首屏截图出现明显占位等待的问题：列表现会优先预热首屏缩略图，点开详情时也会先显示已有缩略图。
- 修复本地摘要模型在较慢推理环境下更容易超时回退模板的问题：本地 / 本机 OpenAI 兼容端点的日报摘要超时已延长。
- 修复时间线中部分应用名称与兜底图标不稳定的问题：当前会优先显示更友好的应用名，并对安装器与原始进程名使用更稳的兜底图标。

### 优化
- 优化时段摘要的信息层次：在主摘要之外补充副摘要、节奏标签和主应用数量，减少“只有一句话”的单薄感。
- 优化桌面化身 Beta 的状态提醒细粒度：同一工作母状态下的细分上下文切换现可触发更贴合场景的状态气泡。
- 优化关于页技术栈胶囊、更新状态提示和侧边栏英文副标题的窄宽度表现，减少裁切和拥挤。

## [1.0.35] - 2026-04-07

### 新增
- 新增核心工作流 `A 包` 界面升级：侧边栏、概览、日报、助手、设置、关于页与时间线时段摘要页现统一接入编辑部式视觉系统，并补充对应的布局与表面一致性测试。
- 新增应用显示兜底资源：当系统应用拿不到原生图标时，`Discover / Mail / System Authentication` 现会使用明确的 SVG 兜底图标，而不再直接退回双色字母块。

### 修复
- 修复概览页应用使用模块中部分系统应用仍显示原始进程名与错误字母图标的问题：当前会归一化 `discover / mail / 邮件 / coreautha` 等名称，并在应用聚合时保留最近可用的执行路径，减少图标命中丢失。
- 修复主界面壳层与关键页面存在“卡片套卡片”导致视觉层级过厚的问题：当前卡片感已收敛到必要主分区，避免 sidebar / main / 页面内容重复叠壳。
- 修复主界面录制状态、配置变更与轻量模式关闭行为的前后端同步问题：录制状态变更、配置回推与窗口关闭策略现保持一致，减少侧边栏状态和窗口行为错位。
- 修复 `tauri dev` 与前端 dev server 地址不一致带来的本地调试与 CI 偶发问题：当前已统一固定到 `127.0.0.1`，减少桌面端加载开发服务器失败的情况。
- 修复跨平台条件编译与运行时配置边界不稳的问题：补齐 Linux 会话识别、截图配置与窗口行为相关的守卫，减少不同平台下测试与运行结果不一致。

### 优化
- 优化概览页信息结构：顶部统计恢复为真正独立的四张卡片，网站访问、应用使用与按小时活跃度保持清晰分区，同时去掉不必要的额外包裹层。
- 优化时间线与时段摘要阅读体验：时间线摘要已并入主卡头部，时段摘要页改为阶段带状布局，重点信息更集中。
- 优化日报、助手、设置与关于页的视觉统一性：页面标题、章节面板、工具区与阅读区的节奏更一致，减少从一个页面切到另一个页面时的风格断裂。

## [1.0.34] - 2026-04-05

### 新增
- 扩展 Linux Wayland 活动窗口 provider：补齐 `GNOME / KDE Plasma / Sway / Hyprland` 的活动窗口来源，并为未知 Wayland 会话增加自动 provider 探测链。
- 新增 Linux 桌面环境识别：当前会区分 `X11 / Wayland` 以及 `GNOME / KDE Plasma / Sway / Hyprland / Unknown`，供活动窗口采集、截图裁剪和关于页状态展示复用。
- 新增 Linux 兼容性状态可视化：关于页现会显示当前会话类型、桌面环境、命中的活动窗口 provider 以及 Linux Browser URL 能力等级，方便直接确认当前桌面的支持状态。
- 新增 Linux OCR 补救链路：在 Tesseract 结果不稳定或质量不足时，现可回退到 PaddleOCR worker 并结合区域补救流程，减少 Linux OCR 长文本、低质量文本和局部漏识别的问题。

### 修复
- 修复 Linux 下 Wayland 活动窗口长期无法回流主记录链路的问题：当前主流 Wayland 会话可恢复 `app_name / window_title / window_bounds`，并继续沿用现有录制、分类、时间线与统计链路。
- 修复 Linux 截图链路长期只保留整桌面截图的问题：`ActiveWindow` 模式下现会实际消费 `window_bounds`，优先按活动窗口所在显示器裁剪，拿不到显示器拓扑时回退为按窗口区域裁剪。
- 修复 Linux 下浏览器 URL 长期固定缺失的问题：Firefox / Zen / LibreWolf / Waterfox 现优先走 sessionstore 恢复，Chromium 系补齐标题提取与最近记录兜底，不再一律回退为空值。
- 修复 Linux 构建输出里 `AUTOSTART_LAUNCH_ARG` 在非 Windows 目标下的未使用告警：当前已限定为 Windows 条件编译常量，减少无效警告噪音。
- 修复 Ollama 模型列表容易混入嵌入模型、无文本生成能力模型的问题：当前会过滤嵌入模型，并在模型探测失败时做更稳健的名称与能力兜底，减少设置页里看到不可用模型的情况。
- 修复桌宠窗口交互防护失效的问题：当前会继续拦截右键菜单和打印快捷键，避免桌宠窗口弹出难以关闭的原生界面。
- 修复开机自启动自定义启动参数在非 Windows 目标上的作用范围过宽问题：当前已明确将自定义 launcher 逻辑限定在 Windows 分支，减少跨平台误用风险。

### 优化
- 优化 Linux browser URL 恢复策略：Linux 下现改为跨桌面统一入口，Firefox family 优先走 sessionstore，其他浏览器再回退到标题提取，减少未来继续补 provider 时重复维护 URL 链路。
- 优化 Linux Wayland provider 策略：由桌面环境单一路径改为按环境优先级排序的 provider 探测链，未知 Wayland 会话也会自动尝试已接入 provider，而不再直接报不支持。
- 优化关于页 Linux 兼容性提示：当前会显示命中的活动窗口 provider 与 Linux Browser URL 能力等级，便于排查不同桌面环境下的支持状态。
- 优化 README 中英繁三套 Linux 说明：平台标识、依赖清单、Wayland provider 范围与 Linux Browser URL 能力边界现已与当前实现对齐，减少“文档仍写 X11、代码已支持 Wayland”的错位。
- 优化 OCR 工作流稳定性：PaddleOCR 现改为 worker 复用模式，并在低质量首轮结果下增加区域补救与自动重试，降低频繁拉起 Python 进程带来的抖动。

## [1.0.33] - 2026-04-03

### 修复
- 修复 macOS 下腾讯视频图标缺失的问题：应用图标兜底匹配现支持 `腾讯视频 / QQLive / Tencent Video` 别名，减少显示名与 `.app` 包名不一致导致的空图标。
- 修复工作时长统计更容易把娱乐活动并入有效工时的问题：工作时长汇总现排除 `entertainment` 类活动，减少娱乐类场景抬高工时。
- 修复 macOS 下前台应用偶尔只识别成通用 Electron / Helper 进程名的问题：现结合 bundle 元数据还原真实应用名，降低时间线、分类规则与统计聚合误判。
- 修复按小时活跃度图表内联选中反馈不够明确的问题：当前选中时段的反馈样式与文案更直接，减少“已切换但不够明显”的困惑。
- 修复按小时活跃度柱状图选中气泡只显示小时段、不显示累计时长的问题：当前选中时会同时展示小时区间和该时段时长。
- 修复 macOS 截图在部分系统环境下主链路不可用时容易失败的问题：新增 `screencapture` 回退，降低无法截屏导致的记录中断。

### 优化
- 优化 macOS 应用图标兜底匹配的扩展方式：常见显示名别名现集中维护，后续补充同类应用时不需要继续改动整条图标解析链路。

## [1.0.32] - 2026-04-01

### 新增
- 新增概览页多时间视角：支持今日、本周、指定日期与日期段切换，并补充跟随当前语言的紧凑日期选择器。
- 新增概览图表交互：应用使用与按小时活跃度现均支持横向图 / 柱状图切换，`24 小时活动分布` 的柱子可直接点击查看该时段活跃时长。

### 修复
- 修复活跃时长更容易误算的问题：现在对未锁屏的长时间无输入场景采用“3 分钟进入疑似空闲 + 20 分钟强制切断”策略，并将脱敏记录与浮窗 / PiP 记录一并纳入空闲剔除，减少午休、离岗或后台动态页面导致的幽灵活跃时长。
- 修复按小时活跃度与时段摘要在重叠活动、跨小时和跨零点场景下更容易重复累计或显示不一致的问题：统计现按真实覆盖时长计算，并消除卡片显示“1分钟”而文本显示“0分钟”的矛盾。
- 修复时间线中未保存截图的活动仍显示截图加载失败的问题，现改为明确提示“本次记录未保存截图”。

### 优化
- 优化概览页布局与文案切换：日期范围、副标题、空状态与图表说明现会随今日 / 单日 / 日期段 / 本周视角动态变化，多语言文案同步补齐。
- 优化应用使用柱状图展示：改为共享坐标系的统一图表布局，并保持时长标签单行显示，提升多应用对比可读性。
- 优化日报页头部布局，减少英文长标题和日期信息挤压。

## [1.0.31] - 2026-03-31

### 新增
- 新增休息提醒：启用桌宠后，可按 30 / 45 / 50 / 60 / 90 / 120 分钟间隔提示起身活动，并支持直接在桌宠气泡上关闭本次提醒。
- 新增开机自启动启动方式：启用自启动后，可选择“显示主界面”或“静默驻留”，减少系统启动时对当前桌面的打断。

### 修复
- 修复 macOS 环境下浏览器页面 URL 错误识别的问题：优化 AppleScript 提取规则，对 Value、Description、Title、Name 等候选来源做权重评分，并过滤可疑的纯主机名假链接，提高当前页面 URL 识别精度。
- 修复浏览器同标题页面更容易漏记导航变化的问题：在切换判定前增加 URL 预探测，并在浏览器标题或地址变化时使用更短的截图冷却时间，减少页面切换记录延迟。

### 优化
- 优化项目文档入口：新增繁体版 `README.tw.md`，并同步更新中 / 英 / 繁三套 README 的语言切换链接与介绍图，方便不同语言用户查看最新说明。

## [1.0.30] - 2026-03-30

### 新增
- 新增全局多语言支持：应用壳层、概览、时间线、日报、设置、关于页、桌宠提示与更新提示现统一接入简体中文、英文与繁体中文文案体系。
- 新增按语言隔离的日报存储与读取：同一天可分别保存不同语言的日报版本，切换语言后会自动切换到对应语言内容。

### 修复
- 修复日报页顶部在英文与不同窗口宽度下更容易被压缩变形的问题，恢复为与中繁版一致的横向展开布局。
- 修复语言切换后仍复用旧语言日报的问题：当目标语言日报缺失时，会自动补生成当前语言版本。
- 修复语言菜单文案被裁切的问题，切换菜单现改为左对齐展开，并按“英文缩写 + 语言名称”展示完整文案。
- 修复网站访问明细在英文日报中仍显示中文语义分类的问题，域名语义标签现会按当前语言输出。

## [1.0.29] - 2026-03-30

### 新增
- 新增 README 中英文双文档结构：`README.md` 顶部可直接切换到英文版 `README.en.md`，便于海外用户快速理解安装、能力边界与使用方式。
- 新增网站语义分类规则第一版：补充常见 AI 工具、在线文档、任务管理、视频站点与社交娱乐平台的默认语义识别，并支持在概览页网站访问弹层内按域名手动修改分类。
- 新增概览页按小时活跃度模块：可直接查看全天 24 小时活跃分布，并将高峰时段摘要同步写入基础模板与 AI 增强日报。
- 新增 Ollama 模型列表刷新能力：AI 设置页可直接拉取本机已安装模型，并保留手动输入模型名称的回退入口。
- 新增稀宇科技 MiniMax 提供商：支持以 OpenAI 兼容方式接入 MiniMax 文本模型，默认预置官方 API 地址与推荐模型名。

### 修复
- 修复轻量模式下关闭主界面仍只会隐藏窗口的问题：现在点击关闭按钮会真正释放主 Webview，同时继续保留后台记录与托盘，再次打开时按需重建主界面。
- 修复网站访问只显示裸域名、不够直观的问题：现在域名分类可按规则或手动覆盖，并会按域名回填历史记录，同时在日报中以“域名（语义分类）”的形式体现。
- 修复 Ollama 模型选择在刷新后容易显示空白、旧模型混入下拉和手动输入不同步的问题：下拉列表现仅展示 Ollama 实际返回的模型，未命中的手动值会单独提示。
- 修复在线更新误判问题：现在只有当前平台存在可安装更新包时才会进入在线更新，遇到“有新版本但暂未准备好安装包”时不再直接报错。
- 修复自动检查更新在失败后仍提前写入最后检查时间的问题，避免一次网络异常后长时间不再自动重试。

### 优化
- 优化 macOS 后台轮询策略：主监控、桌宠活跃态/空闲态以及锁屏检测现按平台差异降频，减少后台空转与无效唤醒。
- 优化浏览器 URL 补全时机：当快路径未拿到地址时，会在真正需要落库前再补做一次窗口级解析，减少浏览器页面统计断裂。
- 优化概览页网站语义分类编辑体验：域名语义分类现改为固定中文分类集下拉选择，减少手填歧义并提升历史回填一致性。

## [1.0.28] - 2026-03-29

### 新增
- 新增日报增强能力：支持在日报页为 `AI 增强` 模式填写附加提示词，并支持将日报以 Markdown 形式手动导出或按天自动落盘到自定义目录。
- 新增截图范围模式：可在“活动窗口所在屏幕”和“整桌面拼接截图”之间切换，默认按活动窗口所在显示器截图。
- 新增旧数据目录清理入口：完成数据目录迁移后，可一键清理旧目录中的应用托管数据。

### 修复
- 修复 Windows 11 更改数据目录时更容易崩溃的问题：迁移流程改为“复制普通数据 + 数据库备份导出”，避免直接拷贝正在占用的数据库文件。
- 修复多屏幕环境下截图容易落到主屏的问题：macOS 与 Windows 现都会优先按活动窗口所在显示器截图。
- 修复时间线列表在实时合并时更容易保留旧截图、旧标题的问题，前端现改为以活动 `id` 为准做增量替换。
- 修复日报页显示的生成模式与实际内容不一致的问题：当本次实际上回退到基础模板时，页面会按真实生成方式展示。
- 修复关于页在线更新在部分网络环境下更容易失败的问题：更新源调整为优先直连 GitHub，并提高连接与请求超时阈值。

### 优化
- 优化存储设置布局：将截图保留、日报导出、数据目录迁移与清理拆成独立卡片，减少设置页拥挤感。
- 优化 AI 设置页密钥输入体验：支持显示/隐藏 API 密钥，减少排查配置时的来回修改成本。
- 优化 Linux Release 流程：CI 现会同时产出 `.deb` 与 `.AppImage`，并补齐产物校验与命名归一。
- 优化 README 与发布说明：同步当前平台支持、日报导出、多屏截图模式和 Linux 使用边界，减少文档与实际功能不一致的问题。

## [1.0.27] - 2026-03-28

### 新增
- 新增轻量模式：关闭主界面后可释放主 Webview，仅保留后台记录与托盘；再次打开主界面时按需重建，降低后台界面常驻带来的资源占用。
- 新增时间线内的应用默认分类修改能力：可直接在活动详情中调整应用基础分类，并同步回填该应用的历史记录。
- 新增 Linux (X11) 平台支持：补齐活动窗口检测、空闲检测、截图、OCR 与锁屏检测的基础链路，可在基于 X11 的 Linux 桌面环境中运行。

### 修复
- 修复 Firefox / Zen 系浏览器页面识别不稳定的问题：补强 URL 采集、标题回查与结果去重，减少只能看到浏览器总时长、误记为 `about:newtab` 或只显示地址栏页的情况。
- 修复浏览器 URL 在时间线、概览和日志中的中文百分号编码显示不友好的问题，改为更可读的展示形式。
- 修复托盘录制状态与主界面左上角状态不同步的问题，并补齐配置变更后的前端状态回推。
- 修复手动修改应用分类后桌宠动作仍沿用旧分类判断的问题，现会立即按新的分类规则刷新状态。
- 修复主窗口关闭重建、从桌宠唤起等场景下窗口更容易停留在原空间或原显示器的问题，主窗口会尽量跟随触发来源所在显示器与活跃空间。

### 优化
- 优化应用图标获取策略，减少对固定应用名或固定路径的依赖，提升更多应用的图标命中率。
- 优化分类管理入口，移除设置页里的独立分类页，将分类调整集中到时间线详情，减少重复入口。
- 优化应用基础分类与语义分类联动后的历史回写链路，为后续继续补齐分类命中率打下基础。
- 补充 Linux 依赖与能力边界说明，当前 Linux 侧仍以窗口级记录为主，尚未支持浏览器 URL 级追踪。

## [1.0.26] - 2026-03-28

### 新增
- 新增活动语义分类层：在基础应用分类之外补充 `编码开发`、`任务规划`、`资料调研`、`会议沟通` 等中文语义标签，并将语义分类结果与置信度写入活动记录。
- 新增桌宠状态细分：浏览器场景现可进一步识别为 `文档中`、`调研中`、`排期中`、`编码中`，并补充 `演示中`、`学习中`、`播客中` 等上下文标签。
- 新增桌宠位置记忆：拖动桌宠后会自动保存位置，下次启动优先恢复到上次放置的位置。

### 修复
- 修复桌宠动作过于单一的问题：同一状态下不再长期保持单一姿态，现会轮换不同主动作并补充更自然的停顿与节拍变化。
- 修复桌宠在会议、视频、音乐、生成中等场景下表现不够准确的问题：补齐更多上下文对应的表情、爪子动作、尾巴节奏与过渡动作。
- 修复桌宠相关测试与构建链路中的过时断言，避免新动作系统接入后被旧规则误判为异常。
- 修复 `v1.0.25` 发布 tag 指向错误提交，导致 GitHub Release 未能提取对应的 `CHANGELOG.md` 内容。
- 修复 `v1.0.25` 发布产物与 updater 元数据仍残留 `1.0.24` 版本号的问题，重新对齐 `package.json`、`Cargo.toml`、`tauri.conf.json` 与锁文件中的版本声明。
- 修复 Firefox / Zen 浏览器的网站访问采集不稳定问题：新增对 Firefox 系浏览器 `sessionstore` 的解析，并在标签页选择状态滞后时结合窗口标题回查当前页面，减少只记录浏览器总时长、拿不到具体页面或误记为 `about:newtab` 的情况。
- 修复浏览器 URL 在概览、时间线和命令输出中显示乱码的问题：对中文等非 ASCII 百分号编码片段改为可读化展示，同时保留会影响 URL 语义的 ASCII 编码字符，避免 `q=%E5...` 这类长串编码直接暴露给用户。

### 优化
- 优化浏览器场景归类准确度：GitHub Pull Request、仓库文件页、Linear/Jira 看板、文档站和搜索结果页现会优先归入更贴合的语义类别，减少误判为泛浏览行为。
- 优化桌宠动作编排：将固定节拍循环改为按状态动态调整节拍时长，并加入动作池与非均匀序列，降低机械感。
- 优化 macOS 应用图标匹配策略：由硬编码路径改为结合应用包扫描、名称归一化与可执行路径回溯的动态匹配，提升 Zen、`antigravity_tools` 等应用的图标命中率。
- 优化浏览器 URL 采集日志输出：相同结果不再高频重复打印，降低调试日志噪音。
- 重新发布 `1.0.26`，确保 GitHub Actions 基于包含版本变更与更新说明的正确提交生成 Release 正文、安装包命名和 updater 清单。

## [1.0.25] - 2026-03-28

### 修复
- 该版本发布时 tag 指向错误提交，导致 Release 正文、安装包命名和 updater 元数据未正确反映本次更新内容；对应功能变更已在 `1.0.26` 中重新发布。

## [1.0.24] - 2026-03-27

### 新增
- 新增 `桌面化身 Beta`：提供独立桌宠窗口，以更轻量的状态表达当前工作节奏，并支持双击回到主界面。
- 新增桌宠外观设置：可在「外观」中单独控制桌面化身的开启状态、尺寸和猫体透明度，且透明度仅作用于猫本身，不影响背景图。

### 修复
- 修复桌宠在状态切换、显示反馈和视觉裁切上的一系列问题：补齐状态气泡、纯透明窗体、尾巴可见性、耳内色块越界和阅读态表情过呆等细节。
- 修复桌宠配置保存后不能立即同步到桌面窗口的问题：尺寸、透明度和生成中状态现会即时刷新。

### 优化
- 重构桌宠状态检测链路：将桌宠轮询从主截图循环中拆出，改为独立轻量循环，并关闭快路径中的浏览器 URL 采集，降低状态切换延迟感。
- 优化录制与统计链路的稳定性与精度：延续最近一轮录制循环重构成果，进一步减少前台窗口切换、空闲判断与统计归属中的阻塞与偏差。
- 优化助手交互体验：补强短追问承接、回答意图承接和多轮对话的上下文连续性，减少“继续”“展开说说这个”“那依据呢”一类追问的误判。
- 优化桌宠视觉风格：从工具化提示改为更接近陪伴型桌宠的表达，补充状态气泡、表情细节与更克制的动态节奏。

## [1.0.23] - 2026-03-26

### 优化
- 清理构建脚本与配置冗余，统一前端构建入口为标准 Vite 命令，消除本地与 CI 构建行为不一致的隐患。
- 版本号同步更新，确保 package.json、Cargo.toml 与 tauri.conf.json 三处版本一致。

## [1.0.22] - 2026-03-26

### 修复
- 修复 Windows 下时间线、概览和网站访问中的应用图标容易错位、混用或缺失的问题，图标读取现优先绑定真实可执行路径。
- 修复 macOS 状态栏与 Dock 图标切换时的显示不稳定问题，补充独立状态栏模板图标和 Dock 图标刷新逻辑。

### 优化
- 优化 Windows 图标缓存键与应用聚合逻辑，减少同名不同路径应用被错误复用图标的情况。
- 优化图标构建脚本与关于页文案细节，减少发布资源分叉和界面换行问题。

## [1.0.21] - 2026-03-25

### 修复
- 修复 Windows 下浏览器归类不稳定导致概览网站访问统计异常的问题。
- 修复时间线、概览与应用使用图标表现不一致的问题，统一图标获取与缓存链路。
- 修复关于页和设置页本地数据目录显示异常的问题，Windows 下不再直接展示 `\\?\` / `\??\` 前缀。

### 优化
- 优化 Windows 图标提取兜底策略，补充安装目录、注册表和快捷方式回查，提升常见应用图标命中率。
- 更新 README 介绍与界面截图展示。

## [1.0.19] - 2026-03-24

### 修复
- 修复在线更新在部分更新源仅能获取 `updater.json`、但安装包下载失败后直接中断的问题：应用内更新现会按更新源依次检查并自动切换后备源重试，减少因为单个代理异常导致的升级失败。
- 修复 `v1.0.18` 发布产物命名与 `updater.json` 不一致导致的在线更新失败问题：自动更新清单、安装包和签名现在会统一对齐，避免清单指向不存在的下载地址。

### 优化
- 统一应用构建产物命名为 `Work_Review`，不再生成带空格或 `.` 的安装包、应用包和更新文件名，降低 GitHub Release 资产命名漂移带来的风险。
- 优化 Release workflow 与 tag 发布链路，统一发布说明中的安装路径和产物名称，确保 GitHub Actions 在发版时生成稳定一致的更新资产。

## [1.0.18] - 2026-03-23

### 新增
- 新增数据存储位置设置，支持选择新的本地存储目录并迁移现有数据，减少手动搬迁成本。

### 修复
- 修复助手页面进入后默认停留在旧历史位置的问题，现会优先展示最新消息并滚动到底部。
- 修复助手模型选择状态丢失的问题，切换页面后仍可保持上次使用的模式。
- 修复基础模板回答内容过于固定的问题，现会根据提问类型返回更贴合的问题结果。
- 修复助手模型选择器交互异常的问题，恢复为稳定的原生下拉选择。
- 修复概览页统计刷新感知偏慢的问题，减少后台计划执行时的界面延迟感。
- 修复监控、OCR 与外部命令调用在异常情况下可能长时间阻塞的问题，补充超时回收与兜底处理。

### 优化
- 优化侧边栏宽度、页面留白与卡片节奏，减少空洞感，提升主内容区的聚焦度。
- 优化关于页布局，改回更克制的居中单卡结构，减少信息噪音和视觉割裂。
- 优化助手页排版密度与字体比例，提升阅读对称性与连续对话的浏览流畅度。
- 优化设置页存储区域结构与状态反馈，迁移流程更直观。
- 清理前端无效代码与部分冗余逻辑，降低维护负担并减少潜在内存泄露风险。

## [1.0.17] - 2026-03-22

### 新增
- 工作助手智能问答升级：新增工作智能分析模块，支持 Session 聚合、意图识别、周报复盘和待办提取
- 时间范围自动提取：问"今天做了什么""上周总结""最近3天"等自然语言时，自动识别并限定检索范围，不再搜全部历史
- 问答页面重构为气泡式对话 UI，支持多轮连续对话
- 新增模型选择器，支持在"基础模板"和"AI 增强"之间切换
- 新增确认对话框组件和助手状态管理

### 修复
- 修复"基础模板"模式误走 AI 的问题：选择基础模板时不再 fallback 到默认模型，直接使用结构化模板回答
- 修复 Ollama 丢失 system prompt 的问题：将 `/api/generate` 改为 `/api/chat`，正确传递系统提示词
- 修复历史消息关键词污染当前问题工具检测的问题：改为两阶段检测，当前问题优先匹配，仅在无命中时才参考历史上下文

### 优化
- Fallback 答案不再一刀切 dump 全部数据，而是根据触发的工具类型（记忆/待办/Session/复盘）生成对应结构的回答
- AI prompt 优化：明确告知模型数据时间范围、精简历史上下文（最近4轮，每条截断200字）、强化"直接回答问题"指令
- CI/CD release workflow 优化

## [1.0.16] - 2026-03-20

### 修复
- 修复 Windows 浏览器网站访问统计经常出现 `0 站点 / 0 页面` 的问题：浏览器 URL 获取现会扫描更多 UI Automation 候选控件，并在短时间采集失败时复用同窗口最近一次成功识别的 URL，减少 Chrome 等浏览器地址识别丢失导致的统计断裂。
- 修复浏览器名称不统一导致统计和图标命中不稳定的问题：`Chrome / Google Chrome`、`Edge / msedge` 等常见别名现会统一归一，避免同一浏览器被拆成多条统计记录。
- 修复 Windows 应用图标大量缺失的问题：图标提取链路现支持更多进程别名、路径名和窗口标题匹配，减少显示名与真实进程名不一致时取不到图标的情况。

### 优化
- 优化网站访问统计的近似精度：后台轮询间隔由 `5s` 调整为 `3s`，降低页面切换时长统计的基础误差。
- 优化应用图标回退体验：当前端拿不到原生应用图标时，现会显示稳定的字母徽标，避免 Windows 下大量空白占位。

## [1.0.15] - 2026-03-20

### 修复
- 修复应用名称重复显示的问题：`work-review / Work Review` 等别名现在会在概览统计、最近应用和运行中应用列表中统一归并，避免同一应用被拆成多条记录。
- 修复前端图标缓存长期运行时可能持续增长的问题：全局应用图标缓存现已加入 LRU 上限，降低长时间使用后的内存累积风险。
- 修复多页面布局密度与对齐不一致的问题：统一概览、时间线、日报、设置、关于五个页面的标题区、卡片内边距、控件高度、提示区和空状态留白，减少视觉偏移和局部过松问题。

### 优化
- 优化侧边栏视觉层级：重新梳理品牌区、导航项、底部工具区的纵向节奏，微调图标、文字、选中态蓝条和字重关系，使整体重心更稳定。
- 优化应用图标与品牌资源：新增 Windows 高 DPI 图标生成链路，重建 `icon.ico` 多尺寸资源，补齐 `20x20`、`40x40` 等关键尺寸，改善 Windows 下任务栏、开始菜单和快捷方式的清晰度。
- 优化页面图标与品牌映射：补充应用默认图标回退和品牌资源映射，减少部分应用取不到图标时的空白或风格不统一问题。
- 更新项目文档：同步 README、Windows 平台说明与当前实现，明确 Windows 打包产物、OCR 现状和现有功能结构。

## [1.0.14] - 2026-03-20

### 修复
- 修复应用内更新链路不稳定的问题：更新流程改为先校验更新元数据，再执行下载、安装与重启，降低更新过程中断和误判的概率。
- 修复自动更新发布不完整的问题：发布流程现会严格校验 macOS 与 Windows 必需产物及签名，缺少任一平台文件时直接终止发布。
- 修复工作结束后日报缓存未及时失效的问题：当天自动生成日报后会立即清理对应缓存，避免页面继续显示旧内容。
- 修复运行中修改工作结束时间后不立即生效的问题：应用启动后的定时任务现会同步最新配置，无需重启即可按新时间执行。

### 优化
- 优化更新体验：应用启动后支持按间隔自动检查新版本，并在已有最新版时静默跳过，减少无效检查。
- 优化全局提示反馈：统一使用非阻塞提示消息展示更新、设置和清理结果，减少系统弹框打断操作。
- 优化设置页加载性能：配置、模型提供商和存储统计改为并行加载，缩短设置页首屏等待时间。

## [1.0.13] - 2026-03-19

### 修复
- 修复应用内自动更新流程：点击更新后可直接自动检查、下载、安装并重启，不再在下载阶段因错误的事件监听调用而中断。
- 修复更新源超时回退问题：将更新超时控制下沉到插件层，并调整为代理源优先，提升国内网络环境下的更新成功率。
- 修复 `latest.json` 缺失导致的自动更新失败：当原生更新源不可用时，Windows 现会自动回退到 GitHub Release 检查并下载安装包，继续保持一键更新覆盖旧版本。
- 修复时间线与浏览器使用时长统计的口径偏差：浏览器 URL 识别与合并逻辑改为准确优先，避免标签页和窗口之间串时长。
- 修复历史日报入口感知弱的问题：日报页补充今天、昨天和日期切换入口，历史日期切换后会立即加载对应日报内容。
- 修复 Windows 打包编译错误：调整 UI Automation 地址栏候选值的借用与匹配逻辑，消除 `monitor.rs` 中的所有权冲突。

### 优化
- 清理测试构建产物与临时输出，避免无关文件进入版本库。

## [1.0.12] - 2026-03-18

### 修复
- 修复了因为国内 GitHub 连接不稳定导致“检查网络更新超时”的问题。加入自动降级到代理加速镜像源的配置。
- 修复了浏览器标签页统计和累计时间错误：现在能够正确追踪同一程序内的标签页URL切换，精确计算时间归属，避免旧标签页的时长误累加问题。

### 新增
- 支持查询历史日期的日报：界面顶端现已提供日期选择器，可自由翻查并生成过往每一天的历史日报汇总。

## [1.0.11] - 2026-03-17

### 新增
- 自动更新支持：
  - 应用内「关于」页面新增检查更新功能（并带有网络重试和抗挂起机制）
  - 支持检查并拉取 GitHub Releases 最新版本并展现下载进度提示，支持静默重启覆盖

### 修复
- 修复「数据保留天数」设置保存不生效的 Bug
  - 数据存储管理配置现在可以正确同时同步更新 截图保留天数 及 元数据保留天数

## [1.0.10] - 2026-03-17

### 修复
- 修复主界面右侧标题区域高度与左侧边栏 Logo 区域视觉不对齐问题
  - 去除 App 全局和右侧内容区顶部因为适配红绿灯预留的过多高度（`pt`）间距，使整个界面布局极简化、视觉更紧凑对齐
- 修复 `AI 增强` 模式下的错误配置保存问题
  - 若 AI 模型未通过联通性测试强行提交时，系统将阻拦保存并将生成模式自动重置为 `基础模板`

### 优化
- 优化 AI 日报设置在 `基础模板` 下的展示逻辑，彻底隐藏无关的模型配置表单，进一步保持界面清爽

## [1.0.9] - 2026-03-17

### 修复
- 修复因高频截屏引发的严重 Frontend / Backend 内存泄漏（导致进程占用达 9GB+ 的问题）
  - 前端 `Timeline.svelte` 给 Base64 图片数据的缓存（原图/缩略图/App图标）强制增加 LRU 数量上限，并在日期切换时主动销毁旧缓存实例
  - 后端对异步并行的 OCR 切片解析任务加入 `Semaphore(2)` 保护防堆积限流
- 修复 AI 增强配置中可能因为连接前置守卫未挂载导致的按钮点击无效问题

## [1.0.8] - 2026-03-16

### 修复
- 修复 Windows 编译错误：适配 uiautomation 0.24.4 API 变更
- 修复 macOS 上 Dock 等系统进程被统计使用时长的问题
- 修复应用悬浮工具栏导致后台时长虚高的问题

### 优化
- Windows 浏览器 URL 获取改用原生 UIAutomation COM 接口
- 工作时间设置支持分钟级精度（如 8:30-17:30）
- 浮动窗口检测阈值优化，减少误统计

## [1.0.7] - 2026-03-16

### 修复
- 修复 macOS 睡眠/Power Nap 唤醒时产生幽灵活动记录的问题
  - 新增 CGSessionCopyCurrentDictionary 锁屏检测（系统级 API，最可靠）
  - 解决合盖睡眠期间 WPS Office 等应用被错误记录使用时长
- 修复时段摘要中卡片显示"1分钟"但文本显示"0分钟"的矛盾
  - 后端摘要文本时长改为向上取整，与前端 Math.round() 对齐

## [1.0.6] - 2026-03-15

### 新增
- 视频画中画（PiP）小窗使用时间统计
  - 通过 macOS `CGWindowListCopyWindowInfo` 检测浮动窗口（layer > 0）
  - 自动识别腾讯视频、爱奇艺、QuickTime Player 等应用的 PiP 小窗
  - 浮动窗口时长独立累计，不与前台应用重复计时
  - 自动排除系统进程（Dock、控制中心等）和过小窗口
  - 受隐私过滤规则约束，支持脱敏和跳过
- 自定义背景图片功能
  - 设置页新增「外观」标签，支持上传自定义背景图
  - 可调节背景不透明度和模糊程度，实时预览
  - 图片自动压缩（最大 1920px 宽，JPEG 格式）
- 设置页新增「AI 模型」独立标签页

### 修复
- 修复 README 中 Releases 下载链接指向错误仓库名的问题

### 优化
- 设置页 UI 精简：标题栏和按钮样式统一，去除多余渐变和阴影
- 概览页、时间线、报告等页面样式微调
- 侧边栏布局和交互优化
- 关于页按钮和技术栈标签样式统一

## [1.0.5] - 2026-03-13

### 新增
- 开机自启动：设置页新增「开机自启动」开关，支持 macOS（LaunchAgent）和 Windows（注册表）
  - 集成 `tauri-plugin-autostart` 官方插件
  - 初始化时自动查询系统实际状态并同步

### 修复
- 修复侧边栏左下角版本号与关于页不一致的问题
  - 所有版本号展示统一通过 Tauri `getVersion()` API 动态读取
  - 版本唯一来源为 `tauri.conf.json`，不再硬编码

### 优化
- AI 模型设置 UI 重构
  - 模式选择从大卡片改为紧凑分段按钮
  - 表单从混合网格统一为单列布局
  - 测试按钮与提供商选择同行，操作更直觉
  - 非 AI 模式时折叠配置面板，减少干扰

## [1.0.4] - 2026-03-13

### 新增
- 多 AI 提供商支持：新增 Claude、Gemini API，统一 AI 调用接口
- macOS 构建支持 ApplicationServices 框架

### 修复
- 修复小时统计 24 点溢出问题
- 修复 macOS 编译错误（`AppError` 导入缺失）

### 优化
- 截图兼容性和监控模块稳定性优化
- 数据库查询与存储逻辑优化
- 存储管理清理策略改进

## [1.0.3] - 2026-03-12

### 新增
- 智能空闲检测：自动解决"应用挂着不用但时长继续累加"的问题
  - 两阶段检测机制：
    - 第一阶段：键鼠活动检测（Windows: `GetLastInputInfo`，macOS: `CGEventSourceSecondsSinceLastEventType`）
    - 第二阶段：屏幕内容变化检测（截图哈希比对）
  - 智能判断逻辑：
    - 有键鼠操作 → 活跃，正常计时
    - 无键鼠操作但屏幕有变化 → 活跃（终端运行程序、视频播放等场景）
    - 无键鼠操作且屏幕连续 3 次无变化 → 空闲，暂停计时
  - 跨平台支持：Windows 和 macOS 双平台完整实现
  - 无需配置：固定 3 分钟阈值，自动工作，用户无感知

### 修复
- 修复 Windows 10 截图和应用统计不工作的问题
  - 新增 GDI BitBlt 截图备用方案，当 Windows Graphics Capture API 失败时自动降级
  - Windows 11 优先使用 WGC API（高性能），Windows 10 自动切换到 GDI 方案（兼容性优先）
  - 解决了 Win10 与 Win11 截图 API 不兼容导致的记录缺失问题

### 优化
- 新增国产浏览器 URL 获取支持：360 安全/极速浏览器、QQ 浏览器、搜狗浏览器、2345 浏览器、猎豹浏览器、傲游浏览器等
- Windows 窗口控制按钮改为原生风格：右上角方形按钮（最小化/最大化/关闭），不再使用 macOS 风格的圆形按钮

## [1.0.2] - 2026-03-11

### 修复
- 修复 Windows 10 不被记录的问题
  - 改用 `PROCESS_QUERY_LIMITED_INFORMATION` 替代高权限标志，覆盖 UAC 保护进程和 Microsoft Store 应用
  - 增加 `QueryFullProcessImageNameW` 和窗口标题推断两级备用方案，基本消除 Unknown 进程记录
  - 新增 `winbase` feature 支持
- 修复活动总时长严重偏低的问题（Unknown 进程时长全部堆积到同一条记录）

### 优化
- 概览页布局优化：网站访问和应用使用区块始终渲染，消除加载时的 layout shift
- 关于页删除无意义的"数据安全保障"说明卡片
- 侧边栏 Logo 区域视觉优化（图标和间距收紧）

## [1.0.1] - 2026-03-11


### 修复
- 修复 Windows 10 启动后概况数据全为 0 的问题
  - 锁屏检测：使用 OpenInputDesktop 替代不可靠的 GetForegroundWindow/quser 判断
  - 截屏兼容：DrawBorderSettings::WithoutBorder 在 Win10 不支持时自动降级到 WithBorder
  - 窗口获取：GetForegroundWindow 返回 null 时降级为 Desktop，避免跳过轮询

### 优化
- 修复 Rust 后端编译警告
- 修复前端 A11y 可访问性警告
- Clippy 代码风格优化

## [1.0.0] 

### 核心功能
- 智能工作追踪：自动记录应用使用情况
- 活动合并：同一应用连续使用自动合并为单条记录
- 浏览器 URL 追踪：记录访问的网站
- 时间线视图：可视化查看每日工作轨迹
- AI 日报生成：支持本地 Ollama 和云端 API

### 隐私保护
- 应用规则：跳过、模糊或正常记录指定应用
- 敏感关键词过滤：自动过滤包含敏感词的活动
- 域名黑名单：不记录指定网站
- 锁屏检测：锁屏时自动暂停记录

### 平台支持
- macOS 10.13+：屏幕录制权限检测
- Windows 10+：完整功能支持

### 技术规格
- 后端：Rust + Tauri 2
- 前端：Svelte 4 + TailwindCSS
- 数据库：SQLite
