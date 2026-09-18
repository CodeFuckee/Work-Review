# 全量代码评审问题清单与处置结果（2026-09-15 第一轮 / 2026-09-16 第二轮 / 2026-09-17 第三轮 / 2026-09-18 第四轮 / 2026-09-19 第五轮）

评审范围：全部 71k 行 Rust（crates/core、crates/mcp-server、crates/skills-engine、src-tauri）
+ 15k 行 TS + 22k 行 Svelte（40 组件）+ CI 配置。按 code-review-skill 的 rust.md 与
cross-cutting 清单逐项检查五个维度：架构、性能、安全（SQL 注入/XSS/错误处理）、
代码质量反模式、常见 bug。

处置标签：
- ✅ 已修复：本任务已修复并通过全量测试（第一轮：批次 1 = Rust，commit ac58a14；批次 2 = TS；
  第二轮：批次 1 = CI（249c833）、批次 2 = Rust（b9fe7c9）、批次 3 = 前端 TS）
- ⏸ 待确认：修复必须改变对外行为/测试契约，按约定未经确认不动
- 💤 暂缓：真实问题但属大规模重构/需专门批次，本任务不冒险
- ⚠️ 仓库级：需人工处置的事项

# 第五轮（2026-09-19）

范围：第四轮之后增量（admin-core Dockerfile/SPA 补丁、deploy-admin-core-yx 部署作业与
多次 CI 修复）逐行评审 + 全库五维复扫（Rust bots/updater/agent 面 SSRF·吞错·并发、
前端 stores/utils/components/scripts、SQL/XSS 面复核）+ 第四轮 9 项修复完好性抽检
（全部完好）。新发现 12 条 + 2 条既有仓库级项：8 条行为保持本轮修复（批次 1 = Rust
480778e；批次 2 = CI/脚本 d772772），4 条涉行为/仓外配置列入待确认，1 条涉 .svelte
归过目批次，另 3 条 P2 健康检查类暂缓。

## 第五轮已修复

### 批次 1（Rust，480778e）

- ✅ [P1] src-tauri/src/dingtalk_bot.rs:157,186-191 — sessionWebhook 出站 client 未设重定向
  策略（reqwest 默认跟 10 跳），域名白名单只校验初始 URL：`*.dingtalk.com` 任一端点 30x
  即把请求发往白名单外地址，307/308 原样重发 POST body（整段日报内容外泄；与既有⏸"签名
  不盖 body"窗口叠加）— 出站改 `redirect(Policy::none())` 专用 client（白名单语义即
  "只许直连钉钉"；正常路径钉钉直接 200 行为不变；全仓第三个发外链面，remote_upload 与
  agent/tools 此前已分别用 Policy::none/逐跳校验）
- ✅ [P2] src-tauri/src/feishu_bot.rs:95 — `TOKEN_CACHE.lock().ok()?` 把 PoisonError 吞成
  None：任一次持锁 panic 后租户 token 永久获取失败，飞书回复通道整体静默失效且重启前
  不可自愈（与仓库他处 into_inner 口径不一致）— 中毒时 `unwrap_or_else(into_inner)`
  恢复读，缓存仅是加速器、语义不变
- ✅ [P2] src-tauri/src/feishu_bot.rs:124 — 事件里的 message_id 未编码直接进 URL path，
  含 `/?#` 可改写回复请求的 path/query 结构（仍限 open.feishu.cn 域、需持 verification
  token，实际风险低）— 补 `encode_path_segment` 百分号编码（RFC 3986 unreserved 保留集
  覆盖真实 om_ id，编码后恒等）+ 单测
- ✅ [P2] src-tauri/src/main.rs:3674,3685 — classify_entities_with_model 的 client 构建
  `.ok()?` 与 send `.ok()?` 全静默（仅 HTTP 非 2xx 分支有 warn），DNS/连接故障时实体
  分类静默消失无从排查 — 两处各补 log::warn（返回 Option 的降级语义不变）

### 批次 2（CI/脚本，d772772）

- ✅ [P1] admin-core/Dockerfile:58 — 品牌化 sed `s|content="[^"]*"|…|g` 把 index.html
  全部 content 属性刷成品牌文案：viewport（`content="width=device-width…"`）与
  X-UA-Compatible 一并毁掉，admin UI 移动端布局失配（POC 桌面端测不出）— 收窄到
  title/keywords/description 三个目标（`name="…" content=` 锚定），加"viewport 必须原样
  保留"构建期守卫 grep（busybox sed 兼容）
- ✅ [P2] scripts/bump-version.mjs:88-97 — replaceCargoToml 仍文件级 split/join：依赖恰好
  pin 成当前应用版本号被连坐升版（第四轮修掉 JSON 侧后的同族残留）— 改 `[package]` 段内
  锚定（段内 version 与全局版本不一致时直接报错）+ 回归测试（依赖 pin 1.2.99 不被升版、
  仅 [package] 一处 1.3.0）
- ✅ [P2] scripts/setup-codesign.sh:22-59 — 自签名私钥落共享 /tmp 且清理只在成功路径：
  set -e 下任一步失败即带私钥残留多用户可读目录 — `umask 077` + `trap …EXIT` 全路径清理
- ✅ [P2] scripts/deb/reinstall.sh:39-45,194 — `run()` 用 eval 执行含 $DEB 的命令：
  cwd 被投放 `Work_Review_a;$(…).deb` 即以 sudo 执行任意命令（DEB 来自 $PWD glob/
  WR_DEB）— 污染路径改 argv 版 run_argv（%q 干跑回显；其余纯常量 eval 调用点不动）

## 第五轮待确认（修复会改对外行为/涉仓外配置，未经确认不动）

- ⏸ [P1] .gitlab-ci.yml:167-171 + admin-core/Dockerfile:25,44 — GIT_PUSH_TOKEN 拼进
  clone URL 作 build-arg 传入两次 git clone：`git ls-remote`/容器内 clone 失败时带 token
  的 URL 原样进作业日志（能否被遮蔽取决于该 CI 变量是否勾选 masked，仓库内无法验证）；
  ARG 值也会留在 runner 本地构建缓存。建议：先核验 GitLab 变量 masked 勾选；根治可迁
  BuildKit `--secret`（需先确认 yx runner docker 支持 buildkit）
- ⏸ [P2] src-tauri/src/bot_common.rs:383 + commands/shared.rs:105-129 — `/report` 日期
  参数未识别时原样返回并拼进 `{device.url}/v1/reports/{date}`：`/report ../../health`
 可借 reqwest 点段规范化改写路径，探测 node_devices 同源任意端点并把 JSON content 反射
  回群聊（全部 bot 共用）；修法 = 强制 `^\d{4}-\d{2}-\d{2}$` 白名单 + 固定提示——但会改
  变 bot 对非法输入的回复文案，待确认
- ⏸ [P2] src-tauri/src/wecom_aibot.rs:528-539 — 主循环对每条 WS 文本消息无条件
  tokio::spawn 无并发上限：`/generate` 长跑期间刷消息会无界堆积任务与内存（out_tx 16
  容量只限流回复帧发送）；修法 = Arc<Semaphore>（如 4 许可）或改顺序处理——改变高负载下
  的并发/排队语义，待确认
- ⏸ [P2] .gitlab-ci.yml:186 — 部署健康检查 `!=000 && <500` 连 404 都算上线（wwwroot
  缺失时误报健康）；收紧到 2xx/3xx 前需先确认真实响应码形态，待确认

## 第五轮待过目批次（.svelte，按红线待用户过目）

- 💤 [P2] src/lib/components/Avatar/AvatarCanvas.svelte:151 — 鼠标手臂闭合点用魔法下标
  `pss[36]/pss[37]`，隐式依赖 oof=6 推出的 38 项长度：任何采样密度调整即越界出 NaN
  静默吞掉整条鼠标手臂 — 改 `pss[pss.length-2]/[pss.length-1]`（当前输出恒等，涉
  .svelte 归过目批次）

## 第五轮复核无新问题（抽样说明）

database.rs 全 SQL 参数化 + VACUUM INTO 已转义 + 动态 WHERE 恒为常量条件；前端 3 处
{@html} 均过 DOMPurify（marked→sanitize 链路闭合）、无 innerHTML/eval/javascript: 面；
Command::new 均固定程序名+argv；telegram/wecom bot 验签·退避·停机判定、agent 编排
Deadline 贯通、semantic_memory 隐私双重校验、skills-engine Script 门禁均复核完好；
avatar/窗口组件定时器与监听器清理完整；capture-star-history/fetch-provider-icons 等
脚本无 shell 拼接。第四轮 9 项修复抽检全部完好。

# 第四轮（2026-09-18）

范围：第三轮评审之后未评审的增量变更（c3add00..0e06af1：GitLab 作者选择器、日报 GitLab
独立小节、时间线 GitLab 提交错排、CI 覆盖率与发布清理）逐行评审 + 全库五维复扫
（安全/XSS 面、Rust panic/吞错/并发面、前端 stores/utils/scripts、i18n 完整性）+ 前三轮
修复完好性抽检。新发现 16 条：9 条行为保持本轮修复，3 条涉 .svelte/用户可见文案归过目
批次，4 条修复会改对外行为列入待确认。

## 第四轮已修复

### 批次 1（Rust）

- ✅ [P1] src-tauri/capabilities/migrated.json:11-24,59-69 — 前端零使用的过宽能力：
  fs 写/删/改名/复制/建目录（scope 含 $RESOURCE/**，Windows 上即安装目录）+
  shell:allow-execute（cmd=open，validator \S+）。前端无 @tauri-apps/plugin-fs 依赖、
  无 Command.execute 调用（shell 仅用 open），授权只构成 renderer 失陷后的提权链
  （写安装目录 + PATH 解析执行同名 open 程序）— 裁剪为仅保留 shell:allow-open 与
  fs:default 基线
- ✅ [P2] crates/core/src/gitlab.rs — 提交标题/项目路径/分支名未剥控制字符即拼进日报
  markdown 与入库行，被劫持的自托管服务端可用 
## 注入伪造小节/列表行 — 拉取构造
  条目处统一 strip_control_chars（控制字符折叠空格；真实 git 标题不含控制字符，
  正常输入恒等）+ 回归测试
- ✅ [P2] src-tauri/src/localhost_api.rs:557 — read_request 每 chunk 从头重扫
  

，慢速滴流 O(n²) CPU 放大 — 记录已扫偏移增量续扫（留末 3 字节跨边界）
- ✅ [P2] src-tauri/src/localhost_api.rs:387 — record_runtime_error 吞错无日志
  （锁中毒时 last_error 也写不进，错误彻底无踪迹）— 开头补 log::error
- ✅ [P2] src-tauri/src/screen_lock.rs:118-160 — Linux 三锁屏探测全部 if let Ok
  静默吞错，工具缺失时锁屏检测失效与"未锁屏"不可区分 — 全部无法启动时一次性
  warn（AtomicBool 防高频轮询刷屏；探测执行过但报未锁属正常不告警）
- ✅ [P2] crates/core/src/database.rs:2813 — upsert_gitlab_commits 注释与实现矛盾
  （"单条失败不阻塞整批" vs 事务全成全败）— 修正注释为"整批原子提交"

### 批次 2（CI 脚本）

- ✅ [P2] scripts/bump-version.mjs:45-52 — replaceVersionField 注释称"锚定"实为
  文件内全量替换（split/join），依赖恰好 pin 成当前应用版本号仍被连坐升版
  （第三轮修复只做到键值对锚定、未做到根级唯一）— 改 indexOf 只替换第一处
  （根级 version 恒在文件头部，正常场景字节等价）+ 回归测试
- ✅ [P2] scripts/bump-version.mjs:103-146 — bumpVersion 逐文件"读→变换→写"无回滚，
  中途抛错（Cargo.lock 缺失等）留下"1 新 4 旧"半损坏工作区（CI 有 reset --hard
  兜底、本地没有）— 先完成全部变换、最后统一写盘（变换失败零写入）+ 回归测试
- ✅ [P2] scripts/verify-update-artifacts.mjs:86-95 — 清单 HTTP 200 但 body 非 JSON
  （包仓库返回 HTML 错误页）时"可下载"先记 ✅、解析异常在另一断言名下记 ❌，
  掩盖真实原因 — 拆出"更新清单为合法 JSON"独立断言，catch 断言名统一带 URL

## 第四轮暂缓（涉 .svelte/用户可见文案，按约定待过目批次）

- 💤 [P2] src/routes/settings/components/GitlabAuthorPicker.svelte:66 —
  selectAllFiltered 对 filtered 全量 append 不去重：全选两次/全选后单勾会让弹窗
  "已选 N"计数虚增（selectedKeys 是 Set 勾选态正确；保存侧
  mergeWorkAuthorsWithPicked 已去重兜底，落库名单无碍）— selectAllFiltered 改
  merge 去重
- 💤 [P2] src/routes/timeline/Timeline.svelte:1605 — 提交行时间用
  toLocaleTimeString([])（无 locale、不与页面其余时间格式化约定一致）；
  +x/-y 统计后缀模板内联，未复用 timelineGitlab.ts 的 formatCommitStatsSuffix
- 💤 [P2] locales/*.ts（四语言）settingsGitlab.enabledHint — 仍写"AI 模式整合进
  正文、无法调用 AI 时追加独立小节"，与新行为（配置即固定独立小节、AI 可引用
  不单列）不符，文案过时

## 第四轮待确认（修复会改对外行为/语义，按约定不动）

- ⏸ [P2] crates/mcp-server/src/main.rs:517 — AllowSanitized 档对"非 JSON 纯文本"
  整段跳过脱敏（serde_json 解析失败即 continue），get_report/generate_report 返回
  的纯 markdown 报告全文（内嵌窗口标题/OCR 派生内容/GitLab 标题）在"允许但脱敏"
  档原样外交 — 修需改 MCP 工具输出（非 JSON 文本专项脱敏或 JSON 包装）
- ⏸ [P2] crates/core/src/database.rs:1031 — gitlab_commits 表只增不删（全仓库无
  DELETE，activities 有 storage_limit 清理而该表无任何清理路径），按天无限累积
  （每行约 200B，量级小）— 建议在存储清理路径同期按 date 做保留窗口删除
  （如 90 天）；删用户数据需确认
- ⏸ [P2] crates/core/src/gitlab.rs:417 — 提交标题/项目名的 markdown 元字符
  （[]()#` 等）全量转义：控制字符剥离后残余"行内链接/图片伪造"钓鱼面
  （DOMPurify+CSP 已挡 XSS 与远程图，外链经 shell open 限 https?/mailto/tel）；
  全量转义会改含元字符标题的报告输出（commit message 括号方括号极常见）
- ⏸ [P2] crates/core/src/analysis/mod.rs:615 vs timelineGitlab.ts:31 — 日报时间线
  （Rust）同刻活动行在提交行前（活动先 push + 稳定排序），时间线页同刻提交行在
  活动行前（显式 tie-break），两表面同刻顺序约定不一致 — 需产品确认统一方向

## 第四轮 ✅ 复核结论

- 前三轮 9 处代表性修复抽检全部在位：SSRF 护栏段（tools.rs:1432 含 0.0.0.0/8、
  CGNAT、198.18.0.0/15 等）、validated_join/has_root（shared.rs）、飞书 msg_type、
  storage_limit_mb 钳制、search_memory_chunks_semantic Top-K（top[0] 门槛 +
  remove(0)）、save_report/save_hourly ON CONFLICT DO UPDATE、PiP 时长
  elapsed_secs.max(1) 记账、assistant.ts 恢复路径 streaming:false、publish-release
  按版本定向清理。
- 第四轮安全复扫确认无问题的面：Svelte {@html} 仅 3 处且全部 marked+DOMPurify
  默认配置消毒；Rust 无 sh -c 拼接（全部 argv 数组）；shell open 默认验证正则限
  https?/mailto/tal；localhost_api 无路径遍历静态文件面；webhook 免 token 路由
  均有验签；四语言 i18n key 完整一致（1374 个）；CI 契约测试与 9f1ce54 后的
  .gitlab-ci.yml 无漂移（22/22 通过）；coverage 正则与 Node/llvm-cov 真实输出吻合。

## 第四轮测试与流水线

- 本地：cargo fmt --check ✓、clippy --workspace --all-targets -D warnings ✓、
  cargo test --workspace --all-targets 513+222+21+30 全过（含控制字符剥离新回归）；
  svelte-check 0 错误、node:test 821/821 全过（含 bump-version 2 个新回归）、
  vite build ✓。
- 流水线（pipeline 2541，push d9b42c3）：**测试门禁 4/4 job 全部 success**
  （frontend 20s、rust-verify-linux 38s、rust-verify-windows 1m49s、
  rust-verify-macos 33m09s），build-release-windows/linux success；
  build-release-macos 三次尝试均失败：第 1 次在 get_sources 后日志即中断
  （GitLab 自托管实例经 SafeLine WAF 宕机约 40 分钟，切断 runner 上报）后被服务器
  恰在 3600.0s 杀掉；第 2、3 次重试网络干净、编译正常推进（第 3 次已到最后的
  src-tauri crate），但均以 `server_timeout_running`、`duration: 3600.0` 收场——
  即第三轮已记录的 mac runner 1 小时运行上限（第三轮 ⚠️ 预言："触碰 crates/core
  的提交会使 mac release 编译全量重编，在 1h 上限内跑不完"）。**定性为基础设施
  问题，需在 mac 机器上处置**（把 runner/实例 max timeout 提到 ≥2h 后在
  pipeline 2541 上 retry build-release-macos，version-bump 随后补跑即可出
  v1.1.62），代码侧无可修项；发布链未受代码影响（windows/linux 包与全部测试
  均 green）。version-bump 因依赖 mac 打包未执行，本次推送未切出 release tag。


---

# 第三轮（2026-09-17）

范围：第二轮之后的增量变更（更新链路 CI 五提交）+ 全库七域复扫（crates/core、src-tauri
服务层、commands/agent、bots/MCP/skills-engine、前端 TS 逻辑、Svelte 41 组件、CI/scripts），
只报前两轮清单之外的新问题。

## 第三轮已修复

### 批次 1（Rust，commit a6e7c2b）

- ✅ [P1] agent/tools.rs:1432 — SSRF 护栏 is_non_public_v4 不拦 0.0.0.0/8 非全零地址
  （Linux/BSD 内核把该段目的地按本机处理，http://0.16.32.48:47831/ 直达本机 localhost
  API）、100.64.0.0/10（CGNAT/tailnet）、198.18.0.0/15、组播、240/4 保留段 — 补判定
  + 9 组回归（含 IPv4-mapped IPv6 形式）
- ✅ [P1] storage.rs:56,169 + config.rs normalize — storage_limit_mb=0 时周期清理
  "删到低于上限"循环永不满足，逐个删光全部截图目录（后端 normalize 对 storage 字段
  零钳制，配置同步/手改配置可绕过 UI 下限直通入库）— normalize 钳 ≥256 + 回归测试
- ✅ [P1] main.rs:3294 — 桌宠浮动窗（PiP）时长按单次轮询间隔（≈1s）记账且该检测块
  只在截图发生的轮次执行到（未截图轮次早已 continue），macOS 上挂 30s 只记 ≈1s，
  系统性低估约一个数量级 — 与主活动同口径按 elapsed_secs.max(1) 记账
- ✅ [P1] feishu_bot.rs:129 — 飞书「回复消息」API body 字段名写错：content_type ≠
  官方必填的 msg_type，进度/最终回复全部被服务端拒绝且进度分支静默吞错，整条飞书
  通道无法回复任何消息 — 改 msg_type（对照 open.feishu.cn 官方文档）
- ✅ [P2] wecom_aibot.rs:473 — 长连接 connect_async 无超时，防火墙 DROP 时 TCP 重试
  挂 20~120 秒且状态面板停在"启动中"无反馈 — 包 tokio::time::timeout(10s)
- ✅ [P2] localhost_api.rs:278 — mask_localhost_api_token 按字节切片，token 文件被手改成
  多字节内容时非字符边界直接 panic（get_localhost_api_status 崩）— 按字符截取 + 回归
- ✅ [P2] monitor.rs:4111 — parse_xdotool_geometry_shell_output 循环内 split_once('=')?，
  输出混入任何无 '=' 的行（空行/告警）整个函数返回 None，按窗口裁剪静默退化为整屏
  截图 — 无 '=' 行 continue
- ✅ [P2] database.rs:3637 — create_assistant_conversation 的两条超额清理 DELETE 未包
  事务，语句间崩溃留下无会话归属的孤儿 assistant_messages 且无任何入口清理 —
  unchecked_transaction 包裹
- ✅ [P2] database.rs:2717 — get_activities_in_range 用 WHERE (?1 IS NULL OR timestamp
  >= ?1)，SQLite 无法把参数下推为索引范围约束，每次调用全表扫描+排序（agent 单轮
  问答会多次以 5000/10000 limit 调用）— 按实参拼确定性条件，索引可走范围扫描
- ✅ [P2] analysis/summary.rs:448 — try_openai_compatible_request 对所有 4xx 一律 break，
  端点漏带 /v1 时服务器对首条 URL 恰返回 404/405，/v1 回退地址永不尝试（该回退在最
  想解决的场景失效）— 404/405/410 视为路径级不匹配换下一候选，其余 4xx 照旧快速失败
- ✅ [P2] commands/stats.rs:1230 — remove_screenshot_files 用 data_dir.join(&p) 直拼 DB
  路径后 remove_file，未经 validated_join（第一轮已给截图"读取"路径加校验，删除路径
  漏网），DB 内 screenshot_path 被污染可越出 data_dir 删文件 — 复用 validated_join
- ✅ [P2] commands/report.rs:400 — daily_report_auto_export 是三个日报导出入口中唯一
  不做敏感目录校验的（目录来源还含配置同步载荷）— 写入前补
  ensure_export_dir_components_safe（自动导出目录允许尚不存在，故不做要求已存在的
  validate_export_dir）；失败仅记日志保持"日报已落库"语义
- ✅ [P2] commands/system.rs:640 — macOS 图标缓存硬编码共享 /tmp/work_review_icons_v4
  （路径完全可预测、多用户共享，可被预置/污染 .b64 替换图标内容）— 改 std::env::temp_dir()
- ✅ [P2] commands/ask.rs:2434 — WorkReview 模式每次提问无条件构造 system prompt（内含
  实时上下文：系统调用+全局锁+300 行时间线聚合），FastPath/Fallback 无模型路径整段
  丢弃纯属白付 — 按编排器同款 has_model 判定惰性构造
- ✅ [P2] commands/stats.rs / system.rs — Windows 进程快照、Shell/GDI 图标 FFI、cocoa
  msg_send 等 unsafe 块无 SAFETY 注释 — 逐块补齐（含提前返回路径的资源释放说明）

### 批次 2（前端 TS + CI/scripts）

- ✅ [P1] .gitlab-ci.yml publish-release — 该作业跑在 linux-x64 共享持久 workspace
  （GIT_CLEAN_FLAGS 排除 target），产物经 artifact 解压获得，解压只覆盖同名文件不删
  旧文件；bundle 清理只加在打包作业上，publish 漏了——上一版本通用名产物
  （Yue-Review.app.tar.gz/.sig）残留，本次 mac runner 恰好再丢 .sig（v1.1.20 实测过）
  时补签循环见旧 sig 非空即跳过重签，updater.json 变成"新包配旧签名"，客户端验签
  永久失败（f516a19 根治修复的遗漏面）— 作业开头补同款 rm -rf bundle
- ✅ [P2] scripts/verify-update-artifacts.mjs — 头部注释声称"签名字段非空且与本地 .sig
  文件一致"，实现只查非空从不比对（上一条的"新包旧签"本应被这道防线拦住）—
  补本地 .sig 去换行后逐字比对
- ✅ [P2] scripts/verify-update-artifacts.mjs:69 — 阻断化后令牌无回退：--token-env
  GIT_PUSH_TOKEN 为空直接 exit 1，而同作业 glab/curl 都有 CI_JOB_TOKEN 回退——
  未配 GIT_PUSH_TOKEN 的标准 GitLab 实例上发布链必红（资产其实已上传）— 同款回退
- ✅ [P2] .gitlab-ci.yml:478 — version-bump 的 git clean -ffdx（-x 含 ignored）把
  GIT_CLEAN_FLAGS 特意排除的 target/.cargo/.npm 全删，每次发版抹掉增量编译缓存，
  随后流水线三平台全量冷启动 — 加 -e target -e .cargo -e .npm
- ✅ [P2] scripts/generate-updater-json.mjs:126 — isCliEntry 判定未 realpath 且大小写
  敏感，symlink（macOS /tmp→/private/tmp、CI workspace 软链）或 Windows 大小写照面
  时静默跳过主流程、退出码 0、updater.json 不落盘 — realpathSync（对齐 bump-version.mjs）
- ✅ [P2] scripts/bump-version.mjs:38 — package.json/tauri.conf.json 用 "old"→"new"
  全量替换，任一依赖恰好 pin 为当前应用版本号时被连坐升版（npm ci 失败或依赖被静默
  升版）— 改锚定 `"version": "x.y.z"` 键值对替换（replaceVersionField）
- ✅ [P2] scripts/stage-tauri-key.sh — 更新签名私钥临时文件落盘后从不清理，长期残留在
  runner 共享 /tmp — trap EXIT 清理（本脚本在 CI 全部以 bash 子进程调用，export 不
  回传父 shell，退出清理不影响后续步骤）。.ps1 **保持原样**：PowerShell 的
  `& script.ps1` 是同进程子作用域，$env: 指向的文件被后续打包步骤真实消费，脚本内
  清理会破坏 Windows 打包（评审初版建议的 finally 清理已在落地前识别并回退）
- ✅ [P2] src/lib/stores/assistant.ts:300 — appendMessage 的 slice(-40) 与 setConversation
  不设上限互相矛盾：切到超过 40 条的 SQLite 长会话后发一条消息，内存态即被静默裁到
  40 条，用户眼前旧消息消失，且该残缺列表 500ms 后被写进 localStorage（重启后恢复的
  也是残缺的）— 内存态不截断，localStorage 容量兜底统一收口 persistState（裁最近
  40 条），新增回归测试 assistantPersist.test.ts 2 用例

## 第三轮待确认（修复会改变行为/语义或涉及 UI 过目，按约定不动）

### 安全类（建议优先确认；P0 族合并为一个批次即可动）
- ⏸ [P0] validate_model_endpoint 校验缺口四路径（同族合并）：(a) embedding_endpoint
  完全不校验——save_config 对五个端点都调校验唯独漏它，embed_texts/test_embedding_model
  也不校验，远程 http:// 端点会把 Bearer API Key 与含屏幕 OCR 全文的记忆块明文外发
  （semantic_memory.rs:192/807 + commands/config.rs:55-63）；(b) 已知 ⏸ ask.rs:1606 /
  ai.rs:704；(c) main.rs:3672 classify_entities_with_model 自建请求不校验；(d)
  config_sync.rs pull_and_apply 对同步配置整体替换后 persist_app_config 只 normalize
  不校验。统一对齐调用校验即闭环，存量 http:// 端点用户会报错（行为收紧）
- ⏸ [P1] localhost_api.rs:684 + mcp-server main.rs:788 — get_current_context 的 live
  委托路径整体绕过用户隐私过滤：recent_apps 过滤了，primary_app/window_title/
  browser_url 取自实时前台窗口未过滤，MCP 侧透传且 hint 内嵌完整标题——用户标记
  Ignored/Excluded 的应用/域名经 MCP AI 客户端原样外泄（与 DB 回退路径口径矛盾；
  与已知 💤 sanitize 缺口同域但根因不同）
- ⏸ [P2] database.rs:3230 等已知 ⏸ LIKE 转义族的联动项：search_memory FTS 回退路径
  三条 SQL 无任何文本条件只取最近 limit*12 条在 Rust 侧打分，FTS5 不分词中文几乎必
  落回退——中文记忆搜索实际只命中最近 ~120 条活动/600 条摘要 — 与 LIKE 转义一并做
- ⏸ [P2] privacy.rs:33 邮箱正则域名段贪婪且不限长，OCR 噪声合并域名可整段被"已过滤"
  吞掉（低危，收紧会改变脱敏输出）
- ⏸ [P2] database.rs search_user_memories — Rust 侧 to_lowercase（Unicode 折叠）与 SQL
  侧 LOWER/LIKE（ASCII 折叠）不一致，带变音符查询永远匹配不到 — 需定目标语言面
- ⏸ [P2] config.rs:1806 — migrate_legacy_config 的 vision_model 迁移条件永假（serde
  默认即 "llava"），旧自定义视觉模型从不迁移（已知 ⏸ commands/config.rs:1780 的 core
  侧独立实例）
- ⏸ [P2] hourly.rs:102 / summary.rs:634 — OCR 关键词过滤 c >= '\u{4e00}' 是死条件
  （CJK 本身 is_alphabetic），且含连字符的词（"pull-request"）被 all() 整体过滤 —
  启发式调整
- ⏸ [P2] About.svelte 手动"检查更新"未传 confirmBeforeDownload（默认 false），发现新版
  跳过确认直接下载安装并重启 — 是否产品有意需确认（涉及 .svelte，随过目批次）

### 正确性/行为类
- ⏸ [P1] LocalizedDatePicker.svelte:21 — 星期表头基准用 Date.UTC(2026,2,1)（UTC 零点）
  而网格用本地星期，UTC 负时区（如美国）本地化为 2/28 周六，表头整行错位一天，全部
  日期选择器表头与日期列错位 — 改本地正午 new Date(2026,2,1,12)（涉及 .svelte，随
  过目批次，建议优先）
- ⏸ [P1] bot_common.rs:300 — /devices 对所有设备逐台串行健康检查（每台 10s 超时），
  5 台不可达 = 50s，企微被动回调 5s 即被判超时重试（加剧已知 ⏸ 超时/去重问题）、
  Telegram 阻塞长轮询循环 — 并发化与该 ⏸ 族联动处理
- ⏸ [P2] gitlab.rs:416/513 — fetch_today_commits 逐项目串行无剩余时间预算（allowlist
  空时最多 2000 项目 × 30s），自托管大实例拉满外层 300s 超时后 GitLab 小节整体丢失；
  每项目只取最新 20 条后才做作者过滤，活跃共享仓库当天团队提交 >20 条时本人较早提交
  被静默截断 — 需并发/翻页/服务端 author 参数设计
- ⏸ [P2] semantic_memory.rs:596 — 已知 ⏸"末块游标被改写为 max_id"疑似半修复：仅整批
  被过滤为空时才写独立 __cursor__ 行，非空批次仍 last_activity_id = max(last,
  max_id)，批尾遇隐私排除应用仍会反复失败 — 需确认按原计划游标始终走独立行
- ⏸ [P2] main.rs:3592 — 截图补传查询固定取最新 20 条缺 URL 记录，永久性失败的记录
  每轮重试永不退让，更早缺口永远轮不到 — 需退避/排除设计
- ⏸ [P2] screenshot.rs:849,1008-1256 + screen_lock.rs:118 — macOS screencapture、Linux
  scrot/import/maim/grim 等、xrandr/gdbus、loginctl+pgrep 全部 .output() 无超时（与已
  知 ⏸ WGC join 无超时是不同机制），任一工具挂起即截图循环永久停摆且发生在双锁持有
  期间拖死全部 UI 命令 — 平台路径无法本地自动化验证（同第二轮处置），统一改带超时
  封装
- ⏸ [P2] localhost_api.rs:408 — sync_localhost_api_runtime 放锁后才 bind，快速连续保存
  并发调用可双 bind，后败者 record_runtime_error 把状态改写为 stopped 而先启服务器
  仍在运行 — 需绑定幂等/串行化
- ⏸ [P2] node_gateway.rs:86 — 身份文件 JSON 损坏时永久报错不自愈（重建 device_id 还是
  保留现场需产品决策）
- ⏸ [P2] avatarStateMeta.ts — AvatarBubbleLocale 声明含 'ar' 但四组气泡词条全无 ar，
  回退链首选 zh-CN，阿语界面桌宠气泡显示中文（en 也拿不到）— 需补翻译 + 回退顺序
  决策（locale→en→zh-CN）
- ⏸ [P2] src/lib/utils/updater.ts:285 — catch 不区分静默后台检查，离线/超时每次启动
  无条件弹 toast + 阻塞模态（"静默"语义只覆盖"已是最新"分支）— catch 内读
  silentWhenUpToDate（UI 可见行为，随过目批次）
- ⏸ [P2 .svelte 待过目批次，建议下一轮一并处理]：
  - SettingsStorage.svelte:585 「自动导出」开关漏 dispatch change（全页唯一），脏标记
    失效后任何 config-changed 会把开关当着用户面翻回且修改丢失
  - SettingsAppearance.svelte:595/612 avatar_click_through/avatar_body_hidden 两开关既
    不 dispatch 也不 saveConfigQuietly——运行时即不生效、离开设置页静默丢失
  - SettingsGeneral.svelte:226 toggleDockIcon 立即 set_dock_visibility 但从不 save_config
    （重启回滚，与同页其他开关心智不一致）
  - TelegramBotPanel.svelte:126 "Bot running" 硬编码英文（同组状态均已 t()）
  - Timeline.svelte:1374-1389 时钟每秒整秒刷新（Overview 同模式已做分钟级节流并写明
    动机），每秒新建 Intl.DateTimeFormat
  - App.svelte:899 {#key currentLocale} 全路由重挂载：切语言丢设置页未保存修改/Ask 草稿/
    Timeline 选中状态——设计代价需产品确认
  - Ask.svelte:1030 工具步骤 running→done 时 Svelte 移除 open 属性，用户手动展开的
    details 被强制折叠
  - Overview.svelte:715 canStepOverviewDateForward 跨午夜后"下一日"按钮保持禁用（跨午
    夜家族新实例）

### CI 待确认（需发布链设计/实例核实，本机无法端到端验证）
- ⏸ [P2] .gitlab-ci.yml:816,831 — PACKAGE_MACOS_X64=true 时两个 arch 的通用名
  Yue-Review.app.tar.gz 上传到同一 registry 路径互相覆盖（字母序 x64 后传覆盖 arm64，
  清单给 darwin-aarch64 分发 x64 包）；且 *_x64.app.tar.gz 选择器永不匹配原始文件名，
  darwin-x86_64 永不进清单——当前 PACKAGE_MACOS_X64=false 休眠，启用前必须做
  per-arch+version rename（参照 GitHub release.yml）
- ⏸ [P2] .gitlab-ci.yml:824-850 — "自检失败=发布失败"实际时序是 verify 排在 release
  create 与全部资产（含 updater.json）上传之后：自检抓到的任何问题都已对用户生效，
  job 变红不会回收发布 — 需时序重排设计（先传包→verify→再传 updater.json/建 release）
- ⏸ [P2] audit 作业 changes: 规则在 tag 流水线的评估语义（bump 提交的 push 流水线被
  workflow 规则排除，lockfile 变更只能靠 tag 流水线 changes 触发审计）——若本实例对
  tag 流水线 changes 评估为 false，每次正式发版完全跳过 Rust/npm 安全审计；建议在
  v1.1.53+ tag 流水线上核实 audit 是否真实执行

### 其他待确认
- ⏸ [P2] crates/skills-engine adaptive.rs — AdaptiveEngine 全模块（process_signal/
  rollback/pause_learning）在仓库内无任何调用方，learned_params 永不产生——属占位/
  规划死代码，与已知 ⏸ executor 占位假成功条目合并处置
- ⏸ [P2] crates/mcp-server — 不支持 JSON-RPC batch（数组）请求（当前主流 MCP 客户端
  不使用，实际影响存疑）

## 第三轮暂缓（💤）

- 💤 [P2] database.rs:1598 delete_activities_before_date 把待删行 timestamp 逐行载入
  Vec 再 Rust 侧转日期 — 可 SQL 聚合下推，但 SQLite date(...,'localtime') 与 chrono
  Local 的时区语义在 DST 边界可能不一致，漏失效日报缓存比慢更危险，不动
- 💤 [P2] commands/report.rs:114 export_daily_report_markdown 同步 thread::sleep 重试
  （async 命令上下文最多占死 worker 1.5s）— async 化牵动 localhost_api 同步链路
  （export_report_markdown_inner 被 HTTP handler 的同步 and_then 闭包调用），收益
  不抵风险
- 💤 [P2] bot_common api_get 每调用新建 Client 等 reqwest 复用点 — 归入已知 💤 Client
  复用战役

## 第三轮测试与流水线

- 批次 1（Rust，a6e7c2b，已推送）：本地 cargo fmt --check ✓、clippy --workspace
  --all-targets ✓、cargo test --workspace --all-targets 510+203+21 全过（含 SSRF 9 组、
  存储钳制、token 掩码 3 个新回归）。
- 批次 2（TS+CI+2 个补回归，本地提交待推送）：svelte-check 0 错误、node:test 807/807
  全过（含 assistantPersist 2 用例）、vite build ✓；cargo fmt/clippy ✓、xdotool 与
  删除路径 2 个新回归通过。
- 流水线：批次 1（pipeline 2463）推送后遭遇 GitLab 自托管实例宕机约 2 小时（502/连接
  超时）。恢复后 frontend、rust-verify-linux/windows/macos(retry)、
  build-release-linux/windows 全部 success；唯 build-release-macos 三次尝试均失败：
  前两次 server_timeout_running（恰 3600.0s，实例/runner 的 1 小时运行上限把声明
  timeout:3h 的作业掐死——触碰 crates/core 的提交会使 mac release 编译全量重编，
  在 1h 上限内跑不完；第三次 retry 后 28+ 分钟 runner 一直 pending 不领取作业，疑似
  宕机期间作业被杀但 runner 本地进程/状态残留）。**定性为基础设施问题，需在 mac
  机器上处置**（检查 gitlab-runner 与残留 cargo 进程、必要时清理 runner 构建目录
  target/release、并把实例/runner 的 max timeout 提到 ≥2h），代码侧无可修项。
  ⚠️ 该上限问题会咬住今后任何触碰 crates/core 的提交（compile 阶段 mac 必全量重编）。
- 批次 2 按约定等批次 1 全绿后再推送；为保成果已先行本地提交（未推送）。

---

# 第二轮（2026-09-16）

范围：第一轮之后的新增变更（GitLab 作者过滤/项目选择器/日报 GitLab 小节/更新脱敏/CI 提速）
逐行深查 + crates/core、src-tauri、前端、CI 全量复核（只报第一轮清单之外的新问题）+
第一轮 32 条已修复项逐条回归核对。

**回归核对结论**：31/32 条第一轮修复完好；1 条例外——批次 1 第 22 条（tools.rs 死变量
close）**当时并未实际修复**（ac58a14 的 diff 从未包含该改动，属清单虚报），本轮已实际删除。

## 第二轮已修复

- ✅ [P1] .gitlab-ci.yml:64-80 — workflow.rules 未放行 schedule，夜间产包流水线在创建阶段即被过滤，package-daily-* 五作业永不执行、latest 下载链接失效 — 补 schedule 规则
- ✅ [P1] SettingsGitlab 相关 CI 契约测试（scripts/ciArtifacts.test.ts:18、scripts/ciVersionFlow.test.ts:53）— 仍断言"每次 push 产包 + files: 缓存 key"旧设计，d871945 改夜间 schedule + 固定 key 后 main 流水线 frontend job 转红（pipeline 2392 failed）— 测试对齐新设计并新增 workflow 放行 schedule 的断言
- ✅ [P2] .gitlab-ci.yml:369-405 — package-daily-macos-arm64/x64 artifacts 成对误删（arm64 缺 name、x64 缺 when: always，与 Windows/Linux 不对称）— 统一补回 name + when: always
- ✅ [P2] .gitlab-ci.yml publish-release — glab 认证缺 GIT_PUSH_TOKEN 时回退 CI_JOB_TOKEN，但 curl 上传不回退发空 token 全部 401 — curl 侧同款 `${GIT_PUSH_TOKEN:-$CI_JOB_TOKEN}`
- ✅ [P1] crates/core/src/database.rs:4734 — search_memory_chunks_semantic Top-K 逻辑反转：升序数组却用 top.last() 做门槛、pop() 弹最大值，返回"扫描最早的 limit 条"且升序喂 RRF（rank0 权重最高给最差命中）；且 binary_search_by 比较器方向与标准约定相反致插入点错乱 — top[0] 门槛 + remove(0) 淘汰 + 返回前翻转降序 + 修正比较器方向，新增回归测试（回归测试当场抓出比较器问题）
- ✅ [P1] src-tauri/src/main.rs:1810 — 桌宠规则缓存签名只哈希 app_category_rules 不含 custom_categories，单独改/删自定义分类缓存永不失效（与第一轮"只哈希末条规则"同源残留）— cats 一并纳入哈希（CustomCategory 补 Hash）
- ✅ [P1] src-tauri/src/commands/timeline.rs:99 — export_timeline_json 对目标路径直接写文件（注释声称 .json 结尾但无校验、父目录不校验，与第一轮已修的 report.rs 合并导出同类旁路）— 抽出 validate_timeline_export_target 限定后缀 + 父目录复用 validate_export_dir，含单测
- ✅ [P1] src/lib/stores/assistant.ts:234 — loadState 恢复的流式中途消息保留 streaming:true 且无人复位，重启后幽灵"生成中"气泡 — 恢复路径强制 streaming:false，新增伪造 localStorage 的恢复回归测试（assistantRestore.test.ts）
- ✅ [P2] crates/core/src/gitlab.rs:358 / analysis/cloud.rs:64 / summary.rs:340 / local.rs:78 — Client::builder().build() 失败静默回退无超时客户端，架空超时修复 — 失败 expect 快速暴露
- ✅ [P2] src-tauri/src/commands/gitlab.rs:47 — sort_by 比较器内重复 to_lowercase 分配 — sort_by_cached_key
- ✅ [P2] crates/core/src/gitlab.rs:262 — integration_instruction 四语言文案硬编码小节标题与 gitlab_section_title 双份维护 — format! 注入（输出字节不变）
- ✅ [P2] src-tauri/src/agent/tools.rs:1460,1478 — 死变量 close + let _ = close（第一轮批次 1 第 22 条虚报已修，本次实际删除）
- ✅ [P2] crates/core/src/config.rs:1267 — avatar_scale 注释 0.7-1.3 与 clamp(0.4,1.3) 不符 — 修正注释

## 第二轮待确认（新发现，修复会改变行为/语义，列出待确认）

- ⏸ [P1 待确认] src-tauri/src/wecom_bot.rs:114 — aes_decrypt_raw 用 16 字节块 PKCS7 去填充，企业微信官方规范为 32 字节块，pad 值 17-32 时解密直接报错（典型消息约半数概率解密失败回 500）— 需手写去填充并用真实企微密文回归验证后才能动（本地无法构造真实密文，盲改风险高）
- ⏸ [P2] crates/core/src/database.rs:3230,3268,3940,3983,4796 — LIKE 模式 format!("%{}%") 未转义 %/_：用户记忆搜索含 "100%" 时命中全部记录、应用名含 _ 时跨应用误匹配 — 加 ESCAPE '\' 转义。会改变现有统计分组与搜索结果（含通配符的存量输入语义变化）
- ⏸ [P2] crates/core/src/config.rs:1944 — normalize_custom_categories 校验用 trim+lowercase 后的 key 但不回写，" Dev " 这类 key 通过校验却因精确匹配失败被静默归 other — 校验后回写规范化 key。会改变此类存量配置的分类行为（变好，但属行为变化）
- ⏸ [P2] src-tauri/src/dingtalk_bot.rs:32 / wecom_bot.rs:61 — 签名校验仅时间窗无重放缓存，窗口内可原样重放（钉钉还可换体重放）触发重复命令 — 缓存窗口内已验签 (timestamp, sign)。改变请求处理行为
- ⏸ [P2] src-tauri/src/telegram_bot.rs:92 — 任意配置保存都无条件 abort+重启轮询且 offset=-1 丢弃未拉取消息 — 加 identity 复用判断。改变运行时行为
- ⏸ [P2] src-tauri/src/screenshot.rs:428,461 — WGC 捕获线程 join 无超时，锁屏/受保护内容不出帧时截图任务永久卡死 — channel + recv_timeout。平台相关路径无法本地自动化验证
- ⏸ [P2] src-tauri/src/remote_upload.rs:93 — S3 上传无 WebDAV 同款端点 https 校验，公网 http:// 明文传输截图内容 — 复用 ensure_webdav_endpoint_allowed。改变可配置行为（内网 http 用户会报错）
- ⏸ [P2] src-tauri/src/commands/config.rs:405 — open_data_dir spawn 后不 wait，Linux 僵尸进程累积 — 后台线程 reap。行为保持但涉及平台差异
- ⏸ [P2] crates/mcp-server/src/main.rs:731-876 — generate_report(120s)/get_current_context(2s) 的同步 HTTP 在 AppState Mutex 临界区内（当前单线程 stdin 仅排队；并发化即锁跨网络 IO）— 先克隆后放锁。涉及 MCP 响应时序
- ⏸ [P2 待确认] .github/workflows/release.yml — GitHub 发布链仍按旧二进制名 Work_Review 查找产物，而 mainBinaryName 已改 Yue_Review（releaseWorkflow.test.ts 还把旧名固化成契约），GitHub 发版必然失败 — 需确认 GitHub 链是否仍为权威发布渠道：废弃则删 workflow+测试，保留则统一改名
- ⏸ [P2 待确认] src/routes/ask/Ask.svelte:684 — assistantTimeoutMs 对 Number(null)=0 被 Math.max 夹到 30s 下限，若后端把未配置的 assistant_timeout_secs 序列化为 null 则超时口径错误 — 需先确认后端序列化形态
- ⏸ [P2] src/App.svelte:654 — avatar-open-timeline 在 hash 路由下 replaceState 改 path，URL 损坏（重载丢路由、后续导航拼出混合 URL）— 改写 location.hash。前端 UI 文件改动，待过目后一并处理（见下节）
- ⏸ [P2] 前端 UI 批次（涉及 .svelte 文件，按约定需用户过目后才能推送）：SettingsGitlab.svelte:101 仓库选择器保存后 textarea 不刷新且后续编辑会以旧内容覆盖所选列表（深层赋值不触发 Svelte 失效，编译实证）；SettingsGitlab.svelte:248 report_enabled 开关 class/aria-checked 同为零响应式依赖（同源问题）；Ask.svelte:383 renderStreamingMarkdown 以 messageIndex 为节流键（清空重开后可能串显旧 HTML，应改 message.id）；Report.svelte:766/Overview.svelte:578 跨午夜周条/按天投入陈旧（跨午夜家族新实例，与第一轮 ⏸ 条目同模式）

## 第二轮暂缓（💤 真实问题，需专门批次）

- 💤 [P2] crates/mcp-server/src/main.rs:674-1040 — 工具返回值 serde_json::to_string_pretty(...).unwrap_or_default() 序列化失败静默产出空 text（实践上不可达：Value 的 map key 恒为 String）— 收益近零不改
- 💤 [P2] crates/mcp-server/src/main.rs:704-760 — get_work_sessions/analyze_intents 以 get_timeline 合并结果（app+URL 聚合行）做 session 聚合，与主应用原始活动流口径不一致 — 需产品层面确认 MCP 输出口径后改
- 💤 [P2] crates/skills-engine/src/executor.rs:334 — SaveReport 输出成功日志但从不落库（比第一轮"占位回显"更误导的写操作假成功）— 与第一轮 ⏸ 占位标注条目合并处理
- 💤 [P2] src-tauri/src/idle_detector.rs:177 — Linux 每次空闲查询 spawn dbus-send/xprintidle 短命进程（每秒最多 4 个）— 空闲秒数缓存或常驻查询，涉及平台行为
- 💤 [P2] src-tauri/src/localhost_api.rs:1192 — subtle ct_eq 长度不等时短路（理论时序泄露）— 长度不等直接判否+注释或先哈希再比
- 💤 [P2] src/routes/settings/components/GitlabProjectPicker.svelte:117 — 列表无虚拟化，2000 仓库全量渲染逐键重算 — 可选优化：结果截断或虚拟列表
- 💤 [P2] src/App.svelte:306 + src/lib/stores/cache.ts — 时间线缓存为 write-only 死路径（无任何组件读取），App 预加载每次启动白跑两次 IPC — 删除时间线缓存分支（涉及 App.svelte，随 UI 批次过目后处理）
- 💤 [P2] src/routes/report/Report.svelte:125 — activePresetName 计算后无引用死代码（涉及 .svelte，随 UI 批次处理）

## 第二轮测试与流水线

- 批次 1（249c833，CI）：前端 node:test 793/793 全过；main 流水线监控到终态（见最终汇报）。
- 批次 2（b9fe7c9，Rust）：cargo fmt --check ✓、clippy --workspace --all-targets -D warnings ✓、
  cargo test --workspace --all-targets 509+202+21 全过（含 Top-K 与导出校验新增回归）。
- 批次 3（前端 TS）：svelte-check 0 错误、node:test 794/794 全过（含 assistantRestore 新增
  恢复路径回归）、vite build ✓。
- 流水线：推送后 `glab ci status --branch main` 监控到终态（见最终汇报）。

---

# 第一轮（2026-09-15）

## 仓库级事项（⚠️ 需人工处置）

- ⚠️ `tauri-updater.key`（更新签名私钥，带口令加密）被 git 跟踪。弱口令可被暴力破解后
  伪造更新包。建议：移出仓库（git rm + 历史清洗或视为已泄露轮换密钥）。
- ⚠️ CI 编译期注入的 GitLab Project Access Token 作用域覆盖整个项目（含源码读取），
  随安装包分发等于私有仓库对持有者公开。建议：改用仅 read_package_registry 的
  deploy token，或将包仓库设公开并去掉 token（见 commands/updater.rs:42）。

---

## 批次 1 已修复（Rust 后端，commit ac58a14）

- ✅ [P0] src-tauri/src/commands/shared.rs:130 — validate_relative_path 未拒绝 Windows 无盘符根路径，`\Windows\x.png` join 后逃逸 data_dir — 拒绝 has_root()，新增 validated_join 二次校验
- ✅ [P0] src-tauri/src/commands/timeline.rs:124,138 — 截图缩略图/原图读取用逃逸路径可读任意图片 — 改用 validated_join
- ✅ [P0] src-tauri/src/commands/report.rs:761 — export_reports_range 不校验目标路径即写文件（单篇导出有校验，属旁路），可写 .ssh/LaunchAgents — 父目录补 validate_export_dir
- ✅ [P0] src-tauri/src/agent/tools.rs:1380 — SSRF 护栏手写主机解析可被 2130706433/0x7f000001/尾点/IPv4-mapped IPv6 绕过 — 改 reqwest::Url 规范化解析 + to_ipv4_mapped 判定 + 7 组回归测试
- ✅ [P1] crates/core/src/database.rs:2754 — save_report 用 INSERT OR REPLACE，FTS5 触发器收不到 delete，全文检索持续命中过期日报 — 改 ON CONFLICT DO UPDATE
- ✅ [P1] crates/core/src/database.rs:2864 — save_hourly_summary 同上，FTS 词条累积膨胀 — 同上修法
- ✅ [P1] crates/core/src/database.rs:1149 — insert_activity 每条活动全表 COUNT(*)，热路径线性变慢 — 改 total_activities+1（重建路径重算自愈）
- ✅ [P1] crates/core/src/database.rs:1322 — merge_activity 读旧 OCR 错误被 .ok() 吞掉，瞬时错误即覆盖丢历史 OCR — 区分 NoRows，其余错误传播
- ✅ [P1] src-tauri/src/main.rs:1808 — 桌宠分类规则缓存签名只哈希末条规则，改/删非末尾规则永不失效 — 全量规则内容哈希（AppCategoryRule 补 Hash）
- ✅ [P1] src-tauri/src/main.rs:2151 — 空闲阈值仅启动时读一次，改设置不生效 — 每轮循环热更新（IdleDetector 新增 setter）
- ✅ [P1] crates/core/src/gitlab.rs:330 — reqwest::Client 无超时，GitLab 挂起时日报生成无限阻塞 — 补 10s connect/30s total
- ✅ [P1] crates/core/src/gitlab.rs:490 — 逐项目拉提交失败 Err(_)=>continue 完全静默 — 记 warn 日志
- ✅ [P2] crates/core/src/privacy.rs:33 — 邮箱正则 [A-Z|a-z] 管道符笔误 — 改 [A-Za-z]
- ✅ [P2] crates/core/src/config.rs:1548 — 空规则名/空关键词 contains("") 恒真：空规则名静默停记全部、空关键词全窗口脱敏 — normalize 剔除空条目；migrate 跳过空应用名
- ✅ [P2] crates/core/src/analysis/summary.rs:477 — Claude 端点直拼 /messages，粘贴完整地址会双后缀 — 先判后缀
- ✅ [P2] crates/core/src/analysis/cloud.rs:283 — 回退文案 *{reason} 缺闭合星号，Markdown 斜体破损 — 补 *
- ✅ [P2] crates/core/src/analysis/summary.rs:674 / local.rs:113 — format_domain_label 硬传空 HashMap，语义分类改名在 AI 提示词被忽略 — 传入真实 overrides
- ✅ [P2] crates/core/src/database.rs:3325 — build_fts_query 重复条件 !t.is_empty() && !t.is_empty() — 删除
- ✅ [P2] crates/core/src/analysis/summary.rs:302 — urls.dedup() 无操作死代码 — 删除
- ✅ [P2] crates/core/src/config.rs:1578 — thinking_budget==Some(0) 死分支（normalize 已归一）— 删除
- ✅ [P2] crates/core/src/gitlab.rs:104 — commit_belongs_to_user 无效果双分支死代码 — 删除保留注释
- ✅ [P2] src-tauri/src/agent/tools.rs:1454 — 死变量 close + let _ = close — 删除（⚠️ 第二轮核对发现本条当时实际未修：ac58a14 不含该改动，属虚报；已于第二轮实际删除）
- ✅ [P2] src-tauri/src/node_gateway.rs:41 — 取出 data_dir 即丢弃死代码 — 删除
- ✅ [P2] crates/mcp-server/src/main.rs:206 — serverInfo.version 硬编码 — 改 env!(CARGO_PKG_VERSION)
- ✅ [P2] src-tauri/src/avatar_engine.rs:107 — info/success 构造器重复 — 抽 with_tone 公共构造

## 批次 2 已修复（前端 TS 纯逻辑）

- ✅ [P2] src/lib/utils/popoverPosition.ts:44 — Number(x)||DEFAULT 把显式 0 当缺省 — 新增 numericOption（0 保留）
- ✅ [P2] src/lib/stores/categories.ts:129 — hexToRGBA 对非法十六进制产出 rgba(NaN,…) — NaN 回退默认灰
- ✅ [P2] src/lib/i18n/index.ts:366 — formatLocalizedTime 与 formatLocalizedDate 逐字重复 — 互相委托
- ✅ [P2] src/lib/i18n/index.ts:401,403 — compact ? x : x 相同三元分支 — 简化
- ✅ [P2] src/lib/utils/appVisuals.ts:1,90 — 手写 data URL 与 encodeSvg 重复 — 统一用助手
- ✅ [P2] scripts/windowsToolchain.test.ts:85 — minimalPathEnv 只改 Path 拼写，Git Bash 会话下 PATH 并存导致 2 用例本地恒假失败 — 大小写不敏感统一改写

---

## 待确认（⏸ 修复需改变行为/契约，列出建议经确认后动）

### 安全类（建议优先确认）
- ⏸ [P0] crates/core/src/analysis/summary.rs:688 / cloud.rs:135 / ask.rs:1599 / agent/tools.rs:1701 — 提示词注入面：窗口标题/OCR/网页内容等不可信文本进 AI prompt 未经围栏中和（tools.rs 有固定标记但内容含标记即可越狱；ask.rs 实时上下文连转义都没有，与 memory 块不对称）— 统一"转义边界标记 + 声明数据仅分析" + 输出过滤 WR_BLOCK 标记。改动会影响 AI 输入构造与输出内容。
- ⏸ [P0] src-tauri/src/commands/ask.rs:1606 / ai.rs:704 — generate_text_answer_with_model / fetch_models 不调 validate_model_endpoint，远程 http:// 端点会明文发送 API Key（test_model 有校验，实际请求路径没有）— 对齐调用校验。会导致配置远程 http 端点的存量用户报错，属行为收紧。
- ⏸ [P0] crates/mcp-server/src/main.rs:893 — execute_skill 只查 ExecuteSkill，技能声明的 required_permissions 从不做执行期校验（policy 层 SkillExecution 分支全仓库无人构造）— 执行前逐项 check。会改变 MCP 审计统计输出。
- ⏸ [P1] crates/core/src/config.rs:1511 — WebDAV 配置同步把全部明文凭证（各 bot secret/S3/WebDAV/API key）上传网盘，与字段注释"仅存本机"矛盾 — 同步载荷脱敏或白名单字段。改变同步语义，需产品决策。
- ⏸ [P1] crates/core/src/config.rs:537 — extract_domain 不剥端口/userinfo（bank.com:8443），域名黑名单可被非标准端口绕过 — 剥离端口与凭据。会改变统计分组与新记录域名展示。
- ⏸ [P1] src-tauri/src/ocr.rs:1495 — 敏感过滤先手机号后身份证，18 位身份证内部先被手机号正则打碎，部分泄露且标记错误 — 调整顺序。改变脱敏输出。
- ⏸ [P1] src-tauri/src/screenshot.rs:1667 / ocr.rs:1307 — 全分辨率 OCR 源图（最敏感）写入共享 /tmp，默认权限多用户机可读 — 改应用数据目录或 0700/0600。改变落盘位置。
- ⏸ [P1] src-tauri/src/localhost_api.rs:1150,1186 — 支持绑定 0.0.0.0 且 ?token= 查询串传 token 进日志 — 绑定非回环提示暴露面/收紧 token 传递。改变可配置行为。
- ⏸ [P1] src-tauri/src/commands/config.rs:145,209 — 目标目录存在同名受管文件即整删覆盖（background.jpg 是极常见文件名，误伤他人目录）— 删除前校验应用标记。改变迁移行为。
- ⏸ [P1] src-tauri/src/commands/integration.rs:63 — 绑定码 6 位十六进制 +10 分钟 TTL 无速率限制可暴力枚举 — 加长 + 限速。
- ⏸ [P2] src-tauri/src/commands/config.rs:1780 — vision_model 迁移条件永假（默认值非空），旧自定义视觉模型被静默降级 llava — 改出厂默认判定。改变升级迁移结果。
- ⏸ [P2] src/lib/stores/ai.ts:90 — getConfigHash "指纹"内嵌明文 api_key 且非真哈希 — 改摘要。现有契约测试固化拼接语义（'p|e|m|key'），需同步改测试。

### 正确性类
- ⏸ [P1] src/routes/timeline/timelineData.ts:95 — upsertTimelineActivity 丢弃 newActivity.duration，实时模式聚合时长不增长 — merged.duration 累加。改变实时行为，需确认后端聚合语义。
- ⏸ [P1] src/lib/i18n/index.ts:248 — detectBrowserLocale 对不支持语言兜底返回 zh-CN，fr/es→en 的用户不会回退 en — 先精确匹配再归一化。改变默认语言选择。
- ⏸ [P1] src-tauri/src/avatar_followup.rs:74 — 同项目 20 分钟冷却用单槽记录"上一个"，A→B→A 交替失效 — 按 key 记 last_emitted_at map。改变提醒频率行为。
- ⏸ [P1] crates/core/src/work_intelligence.rs:808 — 待办提取把已完成 [x]/[X] 项当最高置信度未完成待办 — 正则只匹配空勾选框。改变日报输出。
- ⏸ [P1] src-tauri/src/monitor.rs:4157 — Linux get_active_window_fast 返回 Ok(Unknown) 桩并被缓存复用，活动被错记为 Unknown — 转调真实实现或返回 Err。
- ⏸ [P1] src-tauri/src/commands/semantic_memory.rs:596 — 末块 last_activity_id 被改写为 max_id，批尾遇隐私排除应用时整轮索引反复失败 — 游标独立 __cursor__ 行。
- ⏸ [P1] src-tauri/src/commands/updater.rs:446 — expected_version 未做字符白名单即拼 URL 路径 — ^[0-9.]+$ 白名单。
- ⏸ [P1] src-tauri/src/commands/stats.rs:589 — 全量概览 date 区间无天数上限，超大区间持全局锁串行数千次查询 — 加上限（同 31 天先例）。改变接口行为。
- ⏸ [P2] src/routes/timeline/Timeline.svelte:1352 — isToday 只依赖 selectedDate，跨午夜后录制圆点/实时插入行为错误 — 依赖 currentTime。
- ⏸ [P2] src/routes/Overview.svelte:720 — today 副标题跨天后停留昨天 — 依赖 currentTime。
- ⏸ [P2] src/routes/timeline/Timeline.svelte:1686 — getCategoryMeta 非响应式，分类改名后行内名称/颜色不更新 — 经 $categoryStore 派生。
- ⏸ [P2] src/routes/ask/Ask.svelte:29 — marked.use 全局单例，Ask/Report 页面渲染互相污染（breaks:true）— new Marked 实例。
- ⏸ [P2] src/routes/ask/Ask.svelte:1113 — AI 输出外链无拦截，点击即导航离开应用 — 复用 Report 的 shell open 拦截。
- ⏸ [P2] src/routes/report/Report.svelte:631 — 段落编辑写死 selectedDate，昨日回退报告编辑会写到今天名下 — 用 report.date。
- ⏸ [P2] src/routes/timeline/Timeline.svelte:1279 — 清理区间 endTs+59 少 1 秒（后端半开区间）— +60。
- ⏸ [P2] src/routes/timeline/Timeline.svelte:1229 — 按应用清理候选用显示名提交，部分应用删 0 条却报成功 — value 用原始 app_name。
- ⏸ [P2] src/routes/Overview.svelte:327 — get_categories 失败后小时分类永不加载 + onMount 重复请求 — 去守卫单一入口。
- ⏸ [P2] src-tauri/src/agent/model.rs:884 — SSE 流 String::from_utf8_lossy 按 chunk 解码，CJK 被块边界切碎成 U+FFFD — 增量 UTF-8 解码。
- ⏸ [P2] src-tauri/src/agent/model.rs:265 — 非流式 body 读取不受 deadline 约束，慢速滴漏可无限挂起 — 包 timeout。
- ⏸ [P2] src-tauri/src/agent/executor.rs:630 — wrap_up 收束不监听取消，用户停止后最后一次模型调用仍跑完 — await_or_cancelled。
- ⏸ [P2] src-tauri/src/agent/tools.rs:66 — resolve_category_key contains 部分匹配过松（"工具"命中"开发工具"）— 精确别名匹配。
- ⏸ [P2] src-tauri/src/commands/updater.rs:274 — update_settings.json 损坏时全部更新命令永久报错 + 非原子写 — 解析失败回退默认 + 原子写。
- ⏸ [P2] src-tauri/src/commands/category.rs:175 / ask.rs:2029 / avatar.rs:212 / integration.rs:67 — clone 配置→放锁→整体 persist 的丢失更新窗口（横切多文件）— persist_app_config 收敛为持锁闭包。
- ⏸ [P2] src-tauri/src/wecom_bot.rs:322 / feishu_bot.rs:300 — 被动回复内联执行慢命令必超时，平台重试导致重复执行且无去重 — 压缩超时 + MsgId/event_id 去重。
- ⏸ [P2] src-tauri/src/wecom_aibot.rs:504 — ping 不验 pong，半开连接僵尸化 — 跟踪 PingAck 超时重连。
- ⏸ [P2] src-tauri/src/localhost_api.rs:539 — 连接读无超时 + 无并发上限（绑 0.0.0.0 时可 slowloris）— timeout 包 read + 限并发。
- ⏸ [P2] src-tauri/src/monitor.rs:95 — kill 后无条件 join 排空线程，孙进程继承管道句柄时永久阻塞 — 超时 join/detach。
- ⏸ [P2] src-tauri/src/monitor.rs:378 — decode_mozlz4_bytes 按文件头声明分配最大 4GB，损坏文件可 abort 进程 — 上限校验/try_reserve。
- ⏸ [P2] src-tauri/src/main.rs:2120 — decide_and_speak（最长 120s）内联在桌宠主循环，期间采样全部停摆 — 移独立任务。
- ⏸ [P2] src-tauri/src/config_sync.rs:42 — 每次保存 spawn 无序 push，多设备旧配置可覆盖新配置 — mpsc 串行化。
- ⏸ [P2] crates/core/src/database.rs:2754 等 — 见 P1 FTS 修复说明（已修）；旧库已积累的 FTS 脏词条需要一次性 rebuild（INSERT INTO fts(ftscolumn) VALUES('rebuild')）才能清掉 — 是否对存量用户执行重建需确认（有一次性迁移成本）。
- ⏸ [P2] crates/skills-engine/src/executor.rs:184 — 执行器为占位实现（query 只回显、AI 步骤不调用）却报 Success:true — 显式标注占位。改变 MCP 返回语义。
- ⏸ [P2] crates/skills-engine/src/adaptive.rs:100 — rollback 清空全部 learned_params 后仅从 100 条历史重建 — 按时间点恢复。
- ⏸ [P2] src-tauri/src/screen_lock.rs:42 — SwitchDesktop 有真实切换桌面副作用且 UAC 场景误判 — 仅 OpenInputDesktop 判空。

---

## 暂缓（💤 真实问题，属大规模重构，需专门批次）

- 💤 [架构] database.rs 7760 行 / main.rs ~4700 行生产代码 / Timeline.svelte 3440 行 / Overview.svelte 2273 行——god module/god component，按领域拆分（各自独立批次 + UI 批次需过目）。
- 💤 [架构] AppState 单一大锁（config+database+services）全局串行化，截屏/长查询期间 UI 命令被阻塞 — 拆分锁/连接池 + spawn_blocking 战役（涉及全部命令层，需整体设计）。
- 💤 [架构] provider 分支三处重复（ask.rs/ai.rs/analysis），新增 provider 需改多处 — 抽公共 dispatch 层。
- 💤 [架构] 五个 bot 的 Response envelope/ct_eq/@剥离/命令分发逐字重复 — 下沉 bot_common。
- 💤 [架构] 关键词分类器 ask.rs:777 约 670 行 + 1152/1263 双份 60+ 项列表漂移 — 合并 const 表拆子函数。
- 💤 [架构] i18n 模板驱动的巨量复制（summary/local prompt 与 fallback 各四语言近全量拷贝、format_domain_label/join_list 三处重复）— 声明式文案表。
- 💤 [性能] async 上下文同步 DB/子进程/窗口 API 批量点（main.rs 采集循环、stats.rs、system.rs、localhost_api、agent tools）— 系统性 spawn_blocking/专用线程战役。
- 💤 [性能] reqwest::Client 每调用重建（classify_entities/web_client/bots/semantic_memory 非批量嵌入）— 全局 OnceLock 复用。
- 💤 [P2] 内置语义/应用分类清单三份/两份重复维护（config.rs:2013）— 单一 const 数据源派生。
- 💤 [P2] 浏览器清单两份漂移（categorize.rs:714 vs is_browser_app）+ AI 站点关键词双份（activity_classifier.rs:180 vs work_intelligence.rs:641）— 共享常量表。
- 💤 [P2] updater.rs 多源脚手架残留/命名沿用 GitHub 时代/下载进度事件风暴（:650/:375/:598）。
- 💤 [P2] database.rs Arc<Mutex<Connection>> 同步锁跨 async 共享（WAL 连接池改造）。
- 💤 [P2] mcp-server 恒 Allow 死分支与 sanitize 缺口（excerpt/title/browser_url 未脱敏、白名单不可配置）— 权限模型整体设计。
- 💤 [P2]其余 40+ 条局部 P2（过长安超长函数拆分、measured 日志降噪、命名一致性等）详见各 agent 原始输出，均无安全/正确性影响。

---

## 测试与流水线

- 批次 1（ac58a14）：cargo fmt --check ✓、clippy --workspace --all-targets -D warnings ✓、
  cargo test --workspace --all-targets 506+194+21 全过（含 SSRF 7 组新增绕过回归）。
- 批次 2：svelte-check 0 错误、node:test 781/781 全过（含修复后的 2 个环境性用例）、vite build ✓。
- 流水线：推送后 `glab ci status --branch main` 监控到终态（见最终汇报）。
