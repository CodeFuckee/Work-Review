# 全量代码评审问题清单与处置结果（2026-09-15 第一轮 / 2026-09-16 第二轮 / 2026-09-17 第三轮 / 2026-09-18 第四轮 / 2026-09-19 第五轮 / 2026-09-20 第六轮）

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

# 全量代码评审问题清单与处置结果（2026-09-15 第一轮 / 2026-09-16 第二轮 / 2026-09-17 第三轮 / 2026-09-18 第四轮 / 2026-09-19 第五轮 / 2026-09-20 第六轮 / 2026-09-21 第七轮）

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

# 第九轮（2026-09-23）

## 第九轮裁决执行（2026-09-24，用户逐条拍板）

- **待确认 #1（桌宠）→ 整体移除**：用户裁决「桌宠功能鸡肋，直接去掉」，经四问确认边界后执行全家桶删除：Rust 侧 avatar_engine/followup/input/proactive 四模块 + commands/avatar.rs + main.rs 桌宠循环/托盘开关/nudge 全家 + AppState avatar 字段 + config.rs 全部 avatar_* 与 break_reminder_* 字段 + capabilities avatar 窗口 + scripts/gnome-shell 桌宠专用扩展；前端侧 Avatar 组件/路由/设置分区/i18n 156 键×4 语言 + break_reminder 残留。GitLab 项目头像（gitlab.rs）与 focused-window-dbus 窗口跟踪扩展保留。原待确认 #1（桌宠阈值不读配置）随功能移除消解；.svelte 待过目两条 Avatar 项随之消解。
- **待确认 #17（auth.ts 内网 admin 默认值）→ 有意保留**：用户确认为内网部署的刻意默认值，不改代码。
- **待确认 #22（reinstall.sh GitHub 回退）→ 已删**：GitHub latest 回退整段删除，本地无 deb 时报错并给出 WR_DEB/WR_DEB_URL/GitLab 制品页指引；WR_DEB_URL 显式覆盖保留。

范围：十分片并行复扫（main+storage+采集 / localhost+网关+上传+桌宠输入 / agent+五bot /
commands 20 文件 / database.rs 8.4k 行单文件深审 / core 其余+mcp-server+skills-engine /
前端 stores+utils+i18n / 12 组件 / 路由页 28k 行 / CI+构建+打包+admin-core+capabilities），
与八轮台账逐条去重。新发现 44 条（全部 P2——P0/P1 已被前八轮清完），否决/降级分片上报
若干（ocr 测试内锁、deb 包名未经实机核实的断言等收进待确认）。分三批修复 37 处行为保持项：
批次 1 = Rust（22 项，commit 0ab38a4）、批次 2 = CI/构建（7 项）、批次 3 = 前端 TS（8 项）；
12 条涉行为语义/需产品决策列待确认；7 条 .svelte 待过目；linux 打包供应链 pin+sha256
列暂缓。

## 第九轮已修复

### 批次 1（Rust，22 项，commit 0ab38a4）

- ✅ [P2] crates/core/src/database.rs:4047 — append_assistant_message 的 INSERT 消息与
  UPDATE 会话 updated_at 未包事务：UPDATE 失败/崩溃后消息已落库但会话排序键停留旧值，
  新消息会话沉底并可能提前落入「保留最近 100 个」清理被连删（第七轮台账「append 已有事务」
  系笔误）— unchecked_transaction 包裹（同表 create/delete 口径）
- ✅ [P2] crates/core/src/database.rs:866,905-919,1017 — init_tables 8 处 ALTER TABLE
  ADD COLUMN 用 `let _ =` 吞掉一切错误：磁盘满/只读库时列静默缺失、后续读写以晦涩错误
  失败且启动日志零痕迹 — 新增 add_column_if_missing 只忽略 duplicate column，其余传播
- ✅ [P2] crates/core/src/database.rs:3818,3858,3882 — search_memory 三个 FTS 通道
  `if let Ok` 吞掉全部错误（FTS 损坏/rebuild 中间态无从排查）— 失败分支补 log::warn
- ✅ [P2] crates/core/src/database.rs:5188 — search_memory_fallback 的
  `(?1 IS NULL OR timestamp >= ?1)` 参数不可下推为索引范围约束，带旧日期区间时 DESC
  扫描先跳过全部新行 — 按 get_activities_in_range:2799 已修款拼确定性 WHERE
- ✅ [P2] crates/core/src/database.rs:2013,2204 / 2522,2738 — 两个统计聚合函数数据物化
  后仍持连接锁做纯内存聚合（逐活动小时桶切分/逐小时去重），阻塞采集热路径 insert_activity
  — drop(conn) 上移到物化完成处（get_hourly_app_breakdown_range_filtered 净省 220 行持锁）
- ✅ [P2] crates/core/src/database.rs:4807 — upsert_memory_chunk 注释称「内容变化时清空
  embedding」，实现是仅 LENGTH(excluded) > LENGTH 才更新（第四轮 upsert_gitlab_commits
  注释矛盾同族）— 注释对齐实现
- ✅ [P2] crates/core/src/database.rs:3245 — get_hourly_activities 的 `(hour as u32).min(23)`
  把负数回绕成巨大 u32 再收成 23（hour=-1 查 23 点）— 先 clamp(0,23) 再 cast
- ✅ [P2] crates/core/src/database.rs:5352+ — 测试清理 49 处只删 db 主文件：WAL 模式下
  -wal/-shm 任何平台都不在清理范围、Windows 上连接存活时主文件删除静默失败，CI 临时目录
  持续累积 — drop(db) 后 remove_db_files 统一删三文件
- ✅ [P2] crates/core/src/database.rs:64 — matches_ignored_app_for_stats 与
  privacy::matches_ignored_app 逐字重复（双份实现单边演化风险）— 委托 privacy 版本
- ✅ [P2] crates/core/src/privacy.rs:138 — debug 日志恰在窗口标题命中敏感关键词时把标题
  整条明文落盘（第八轮修的是 src-tauri 侧五处，本处是 core 侧残留；RUST_LOG=debug 排障时
  最敏感的标题恰是唯一进日志的标题）— 只记「命中+长度」
- ✅ [P2] crates/mcp-server/src/main.rs:693,668 — search_activities/query_timeline 的
  limit 直通 DB LIMIT（FTS 回退路径内部放大 12 倍行数），失控客户端可全量载入 —
  钳 [1,200]
- ✅ [P2] src-tauri/src/storage.rs:53-61,166 — cleanup() 单次执行 2-3 遍全量递归扫描 +
  全路径物化（超限判断/内部重算/末尾重统计），数万文件时每次周期清理白付 1-2 遍 —
  首次统计传入 cleanup_oldest_until_under_limit 复用，末尾仅在发生过删除时重统计
- ✅ [P2] src-tauri/src/main.rs:2997,3192,3234,4021,4358 + file_monitor_runtime.rs:33 —
  6 处 `if let Ok(lock)`/`.lock().ok()` 把 Mutex 中毒吞成静默跳过：合并/新建路径 OCR 写库、
  远程截图 URL 回写、实体分类内存缓存、轻量模式关窗、文件事件入库在某次持锁 panic 后从此
  失效无日志（4358 还会翻转关窗行为）— 统一 unwrap_or_else(into_inner)（第五轮同款）
- ✅ [P2] src-tauri/src/remote_upload.rs:296-305,384 — ENSURED_WEBDAV_DIRS 只增不失效：
  远端目录被外部删除后 MKCOL 永不再发、PUT 恒 409（send_with_one_retry 只覆盖 5xx/网络
  错误），补传机制走同一缓存同样失败——上传进程内永久失效且注释声称的兜底兜不住 — PUT
  命中 409/404 时失效本次涉及各级目录缓存、重建后重试一次
- ✅ [P2] src-tauri/src/avatar_input.rs:1555 — macOS 全局监听部分注册失败提前 return：
  非 nil 的 monitor 不发 removeMonitor: 泄漏在系统侧，STARTED 复位后的重试再注册一份
  （事件重复派发）— 部分失败先成对 removeMonitor 再返回
- ✅ [P2] src-tauri/src/avatar_input.rs:1503,1526 — transmute 未校验 usize 到 fieldless
  enum NSEventType（非合法判别式即时 UB，契约全靠 mask 巧合）— 改按枚举判别式 usize
  直接比较（语义不变），mouse_group_from_event_type 换 *_raw 版本
- ✅ [P2] src-tauri/src/avatar_input.rs:893,925,967,1422 — 4 处 unsafe 块无 SAFETY 注释
  （第三/八轮同款补注漏网面：LL 钩子不阻塞约束、X11 per-thread Display、主线程要求）—
  逐块补注
- ✅ [P2] src-tauri/src/localhost_api.rs:384,523,717 — base_url/apiEndpoint/日志 用
  `http://{host}:{port}` 拼接，host 为 IPv6 字面量时产出 `http://::47831` 畸形串（UI 监听
  地址是自由文本、单测显式放行 "::"）— 新增 http_base_url 对含冒号 host 包方括号，
  配回归测试
- ✅ [P2] src-tauri/src/wecom_aibot.rs:416-435 — 重连退避计数只增从不复位（对照 telegram
  成功后清零）：任何一次失败后即使长连接健康运行数小时，下次断线仍按累计次数吃满 60s
  封顶退避 — 健康会话结束（Ok 分支）退避重新计
- ✅ [P2] src-tauri/src/agent/tools.rs:407 — 行动工具确认摘要对 create_todo/set_app_category
  的模型参数原文回显：无长度上限、无控制字符清洗，换行/伪系统文案可污染确认卡片（同文件
  user_memory 预览已按「模型参数不可信」加固，同一边界修了一半）— 剥控制字符 +
  truncate_chars(80)
- ✅ [P2] src-tauri/src/agent/executor.rs:448 — trusted_event_tool_metadata 每个工具调用
  整表重序列化注册表（14-18 个工具含完整 JSON Schema）再线性查找 — canonical_tool_labels
  整响应预构建一次，trusted_event_tool_metadata_from 表内精确匹配（含注入名拒绝语义不变）
- ✅ [P2] src-tauri/src/agent/executor.rs:410 — Stop 终态 content.unwrap_or_default()：
  模型返回 Stop 且正文为空（中转网关截断）时用户收到空气泡（其余三条终态路径均有兜底，
  唯独主路径没有）— 空正文回退固定提示语
- ✅ [P2] src-tauri/src/feishu_bot.rs:118 — TOKEN_CACHE 写路径仍 `if let Ok`（读路径第五轮
  已修）：任一次持锁 panic 后缓存永久写不进，每条飞书消息多打一次 tenant_access_token API
  — 写路径同款恢复
- ✅ [P2] src-tauri/src/agent/model.rs:261,478 — OpenAI/Claude 非流式错误路径把完整响应
  body 拼进 AppError（网关大 HTML 错误页整页进错误串并经 warn 落日志；Gemini :617 与流式
  :953 均已截 300，同文件口径不一）— 两处补 chars().take(300)
- ✅ [P2] src-tauri/src/agent/orchestrator.rs:575 — fast_answer 对 search_memory 用
  unwrap_or_default() 吞 DB 错误且无日志：FTS/库故障时「相关记录」小节静默消失无从排查 —
  Err 分支补 log::warn 后再取默认
- ✅ [P2] src-tauri/src/commands/timeline.rs:25 — get_timeline_inner 以 info 级打印完整
  ignored_apps/excluded_domains 清单：哪些应用/域名被隐藏本身是隐私配置，每次打开时间线/
  每次 localhost /v1/timeline 调用都明文进持久日志 — 只记条数
- ✅ [P2] src-tauri/src/commands/assistant_history.rs:27 — list_assistant_conversations 的
  limit 无上限直通 SQL LIMIT（同仓 user_memory clamp 1..500、file_monitor clamp 1..200
  均已钳）— clamp(1,200)

### 批次 2（CI/构建，7 项）

- ✅ [P2] .gitlab-ci.yml:545 — version-bump 无 resource_group：两条 main 流水线全绿并发
  进入 version 阶段时基于同一 origin/main 推导同一版本号，后推送方 push --atomic 因非
  fast-forward 整体失败——自动发版丢失、作业假红 — 加 resource_group: version-bump
  （后到方重取 main 自然跳过/递增）
- ✅ [P2] .gitlab-ci.yml:589 — version-bump 的 `git clean -ffdx` 排除清单漏 `.cache`
  （与 :51 GIT_CLEAN_FLAGS 不同步）：每次发版把 linux-x64 runner 的 .cache/tauri 打包工具
  缓存删光，下个 tag 流水线重新走 gh-proxy 全量下载（v1.1.84/86 两次发版失败根因）— 补
  `-e .cache`
- ✅ [P2] .gitlab-ci.yml:393,827 — Windows 两个打包作业的 after_script 不清理暂存私钥：
  `%TEMP%\tauri-signing-key-<pid>.key`（LocalSystem）在常驻 runner 上按次累积 —
  after_script 末尾 Remove-Item 清理（打包已消费完）
- ✅ [P2] src-tauri/tauri.conf.json:111 — CSP connect-src 含 api.github.com / gh-proxy.cn /
  gh-proxy.com 三个零消费主机（更新源已切自建 GitLab，全仓无引用）：渲染进程失陷后的出网
  白名单被无谓放大 — 删除三项
- ✅ [P2] scripts/deb/reinstall.sh — 全链按旧名 Work_Review 匹配：本地 `Work_Review_*.deb`
  扫描永不命中→静默落入已知失效的 GitHub latest 回退装回陈旧版；pgrep/pkill -f Work_Review
  不再命中运行中的 Yue_Review（full reset 期间进程不停止）；dpkg -s work-review 对新包
  失明 — 全部改 yue-review/Yue_Review/Yue-Review 与旧名双探测双清理（含用户级
  desktop/图标/数据目录双路径）
- ✅ [P2] scripts/deb/uninstall.sh:43-62 + scripts/appimage/install.sh:64-151 — 同族旧名
  失配 — 双名探测；install.sh 的 AppImage 发现/内部图标/launcher/desktop/WMClass 改为
  按检出产物名参数化（Work_Review/Yue_Review/Yue-Review 全兼容）
- ✅ [P2] src-tauri/scripts/patch-info-plist.sh — 死脚本（全仓无引用）且按旧名 find 永不
  命中静默 exit 0，其注入的两条权限描述已由 tauri.conf infoPlist 原生覆盖 — 删除
- ✅ [P2] scripts/ci/linux-builder-22.Dockerfile:13-17,43 — 头部使用示例仍指导挂载已废弃的
  宿主 cargo/rustup 缓存卷（525df65 起废弃）；PATH 含不存在的 /usr/local/node/bin
  （NodeSource 装于 /usr/bin）— 注释对齐现行调用方式、删死 PATH 项

### 批次 3（前端 TS，8 项）

- ✅ [P2] src/lib/stores/auth.ts:39 — getAuth 的 JSON.parse 只 try-catch 不做形状校验：
  localStorage 存了合法 JSON 但缺字段/错型的条目（如 {"token":"x"} 无 expiresAt）时
  isTokenExpired 算出 number > undefined = false，resolveAuthGateFromStorage 把坏凭证判成
  allow 直接放行进主界面（admin 同步全挂）— 校验 token 非空 string + expiresAt 有限
  number，不满足一律走登录门
- ✅ [P2] src/lib/stores/auth.ts:106 — postClientAuth 的 fetch 无超时：admin 服务挂起
  （建连但不响应）时登录/注册按钮永久 pending — AbortSignal.timeout(15s)，超时走既有
  network 分支
- ✅ [P2] src/lib/stores/auth.ts:46,77 — setAuth/setAdminUrl 的 localStorage.setItem 无
  try-catch：配额满/隐私模式抛错发生在登录成功之后，变未处理 rejection、用户看到「登录
  失败」但服务端已发 token — 捕获后 warn 照常返回
- ✅ [P2] src/lib/stores/categories.ts:54-61,95-102 — 两个分类 store 的 refresh 无请求序号
  守卫：编辑后的 refresh 与其他组件 onMount 的 refresh 并发时，旧数据慢响应后到覆盖新响应
  （Ask/Timeline/SettingsAI 均已用 requestId 守卫，此处漏网）— 模块级 seq 快照守卫
- ✅ [P2] src/lib/utils/avatarToggle.ts — toggleAvatarSetting/updateAvatarScaleSetting/
  updateAvatarOpacitySetting 三份逐字同构的「改字段→save→catch 回滚再 rethrow」模板 —
  抽 applyConfigFieldsWithRollback 泛型助手
- ✅ [P2] src/lib/utils/updater.ts:88-151 — localizeRuntimeStatusMessage 11 段
  match+if-return 样板 — 表驱动 RUNTIME_STATUS_PATTERNS（顺序敏感保持原命中顺序）
- ✅ [P2] src/lib/utils/appVisuals.ts — fallbackIconMap 精确键与 getFallbackAppIcon 的
  includes 长条件列表两处维护同一批应用名（yue 家族只存在于子串侧，增名漏一处即漂移）—
  合并 FALLBACK_ICON_RULES 有序表（exact 组整体在前保持原两阶段语义）

## 第九轮待确认（修复会改对外行为/语义或需产品决策，未经确认不动）

- ♻️ [P2] src-tauri/src/main.rs:1721 — 桌宠循环 IdleDetector 硬编码 3 分钟空闲阈值（原待确认）— **2026-09-24 用户裁决桌宠整体移除，随功能删除消解，见「第九轮裁决执行」**
- ✅→♻️ [P2] src-tauri/src/main.rs:3634 — 崩溃日志写 default_data_dir().join("crashes") 而非实际数据目录（原待确认）— **2026-09-24 用户裁决修复**：resolve 后存 OnceCell，崩溃日志优先跟随实际目录，change_data_dir 换目录后同步跟随，极早期崩溃回退默认目录；配回归测试
- ⏸ [P2] src-tauri/src/commands/memory.rs:22 — synthesize_insights_inner 用无隐私过滤的
  get_daily_stats_with_segments（命令层其余统计面全走 _filtered + ignored_apps）：
  被隐藏应用的时长会汇入「高峰时段/娱乐占比/办公时长」洞察落库展示 — 改过滤口径后新生成
  洞察数字会变（main.rs:1867/2133 有同款无过滤调用）
- ⏸ [P2] src-tauri/src/commands/stats.rs:765 — get_hourly_app_breakdown_inner 对
  date_from/date_to 无天数上限，底层单条 SQL 无 LIMIT 全列载入（含 ocr_text），持 AppState
  锁放大：传 10 年区间一次载入全部活动 OCR 进内存 — 加 31/62 天上限会使超大区间从慢速
  返回变报错（归既有 ⏸ stats.rs:589 同批决策；SELECT 收窄可与上限解耦后做）
- ⏸ [P2] src-tauri/src/commands/timeline.rs:49 — get_activity 按 id 直读单条活动（含完整
  ocr_text）无隐私过滤：列表面过滤 Ignored 应用/排除域名、详情面不过滤，IPC 面可按 id
  穷举读被隐藏应用 OCR（UI 只用已过滤列表 id、localhost 未暴露，实际可达性低）— 补过滤
  会改 IPC 返回契约，需先确认「详情信任列表」是否有意契约
- ✅→♻️ [P2] src-tauri/src/commands/system.rs:231 — open_permission_settings 的 macOS `open` spawn 不收割（原待确认）— **2026-09-24 用户裁决修复：后台线程 wait 收割**
- ✅→♻️ [P2] src-tauri/src/telegram_bot.rs:581 — handle_bind_command 的 clone→保存→整体替换惯用法（原待确认）— **2026-09-24 用户裁决修复：原地修改 state.config + 保存失败回滚，锁内落盘与 persist_app_config 口径一致**
- ⏸ [P2] src-tauri/src/agent/model.rs:1259 — ClaudeStreamAssembler 的 BTreeMap 按未校验
  content-block index 无界扩容：与已知 ⏸ model.rs:986（OpenAI partial_calls 极大 index）
  同根因同威胁模型，台账只登记了 OpenAI 侧 — 随该 ⏸ 同批定案
- ⏸ [P2] src-tauri/src/agent/executor.rs:442 — 单次模型响应可携带任意多条 tool_calls 全部
  串行执行：deadline/取消只在每轮开始检查，一轮内 N 个联网工具（各 15-20s）可远超 120s
  总预算 — 是否给单响应工具数设上限属行为变更
- ⏸ [P2] src-tauri/src/agent/tools.rs:1409 — ensure_public_http_url 放行单标签主机名
  （http://nas/ 经 search suffix 可达内网）：注释声明的豁免范围是「DNS 解析到内网 IP」，
  单标签内网名是否算已豁免范畴需定口径
- ⏸ [P2] crates/core/src/privacy.rs:100,105 — 密码/API-Key 两个敏感模式要求
  `[\s:：=]+` 分隔符，中文 OCR 粘连形态（"密码abc123"）整条漏配（第八轮修的 \b 漏配、
  第一轮修的管道符笔误之外的第三种漏配形态）— 分隔符放宽为可选会扩大脱敏命中面
  （"token"/"secret" 后紧跟任意非空白即被吞），误伤容忍度需确认
- ⏸ [P2] crates/core/src/config.rs:637 — metadata_retention_days 是死配置：注释宣称超过
  后删除数据库记录，全仓无任何消费者（用户以为有 30 天元数据保留实为永不清）— 补实现
  （涉删除语义）还是删字段（改配置面+Settings.svelte）需产品意图确认
- ⏸ [P2] crates/core/src/analysis/mod.rs:136 + local.rs:77 + cloud.rs:81 —
  create_analyzer 只把 generation_timeout 传给 SummaryAnalyzer，Local 硬编码 120s、Cloud
  硬编码 60s，report_generation_timeout_secs 配置对这两个模式完全不生效（Cloud 全流程
  可达 360s 与外层超时口径脱节）— 修后超时语义随配置伸缩，可观察
- ✅→♻️ [P2] src-tauri/src/localhost_api.rs:1073 — /v1/activities 的 limit 查询参数无钳制（原待确认）— **2026-09-24 用户裁决修复：钳 ≤10000（同 /v1/reports 先例）**
- ✅→♻️ [P2] category.rs:104,150 — 生产代码 expect（原待确认，avatar.rs 侧已随桌宠移除消失）— **2026-09-24 用户裁决修复：防御性跳过（let-else continue）**
- ⏸→✂️ [P2] telegram_bot.rs:92,220 / wecom_aibot.rs:84,496 — 轮询/长连接任务 panic 无哨兵 —
  **telegram 侧随功能整体移除消解（2026-09-24 用户裁决）；wecom_aibot 侧仍待定**
- ✅→♻️ [P2] scripts/ci/linux-package-inner.sh:31 — 发布链打包工具供应链（原待确认）—
  **2026-09-24 用户裁决修复（commit b4bc02e）**：5 个工具钉 github 直连真源计算的 sha256
  白名单（镜像交叉验证 8/8 一致），plugin-gtk/gstreamer 的 raw URL 钉死 commit
  （dda522bc/2a2e6749）；缓存命中同样全量校验；校验不过即 exit 1 拒绝打包（含打包器自行
  重试路径），上游升版流程写入脚本注释。fetch_tool 三路径（缓存命中/损坏重下/白名单
  不匹配 fatal）本机实测通过
- ⏸ [P2] .gitlab-ci.yml:1024 — publish-release 端到端自检只传 windows-x86_64 与
  darwin-aarch64：启用 PACKAGE_MACOS_X64 后 darwin-x86_64 清单项的远端 SHA/签名一致性
  永不在自检范围（第七轮 ⏸ x64 通用名互覆盖之外的同域盲区）— 启用该开关前需补第三个
  --platform
- ⏸ [P2] admin-core/Dockerfile:82 — 运行阶段无 USER 指令，容器以 root 运行且持 LAN 可达
  端口与生产凭据挂载 — 加 USER $APP_UID 属部署语义变更：挂载目录属主/读权限需同步调整
  （现网首启可能需手动 chown 一次），需与部署窗口协调
- ✅→♻️ [P2] scripts/deb/reinstall.sh:194 — 本地 deb 未命中时的回退是 GitHub latest API（原待确认）— **2026-09-24 用户裁决直接删：回退整段移除，无本地 deb 时报错并给指引，WR_DEB_URL 显式覆盖保留**
- ✅→♻️ [P2] scripts/stage-tauri-key.sh — 疑似 no-op（原待确认）— **2026-09-24 用户裁决修复：实锤后删除——bash 子进程 export 不回传父 shell、tauri-cli v2 自行解码原始值、v1.1.87-90 发版佐证；脚本与 CI 8 处调用全删，ps1 版（Windows 会话内生效）保留**
- ⏸→✔️ [P2] src/lib/stores/auth.ts:54 — 默认 admin URL 硬编码内网 IP `http://10.37.80.25:4191` — **2026-09-24 用户确认为内网部署的刻意默认值，保留不动**
- ⏸ [P2] src-tauri/src/avatar_input.rs:1419 — macOS current_cursor_ratio 遍历 NSScreen 只取
  联合包围盒未逐屏 clamp：多屏负坐标排列下是否产生可感知画布偏移需 macOS 实机验证

## 第九轮待过目批次（.svelte，按红线待用户过目，本轮未动）

- [P2] src/routes/ask/Ask.svelte:703,712,725 — 双击/双回车重复提交：防重守卫读 `sending`
  （store），beginSending 在首个 await（ensureConversation IPC 往返）之后才调用，新会话
  首条消息在途期间再按发送可产生双份 user 消息与两次并发 chat_work_assistant — 首个
  await 前同步置 submitting 标志
- [P2] src/routes/settings/components/SettingsAppearance.svelte:404,411 — 背景显示强度/
  模糊滑块 on:input 每个输入事件全量 saveConfigQuietly 落盘（拖拽一次几十次并发 IPC），
  同文件桌宠缩放/透明度已有 120ms 防抖先例 — 复用防抖
- [P2] src/routes/Overview.svelte:1563-1567 — visibilitychange 恢复可见时重建的 30s 定时器
  丢了初建版的 refreshFileEvents 分支（复制粘贴漂移）：隐藏→恢复一次后文件活动卡片停止
  自动刷新 — 两处合并同一 startIntervals()
- [P2] src/routes/about/About.svelte:64-70 — checkForUpdates 的 runUpdateFlow 无
  try/finally：reject 一次 isCheckingUpdate 永久 true，「检查更新」按钮锁死到重进页面
- [P2] src/routes/report/Report.svelte:309-311 — 生成成功但 get_saved_report 返回 null 时
  伪造 {content:'', created_at:now} 空壳报告渲染并写乐观缓存，且后台刷新仅在 savedReport
  非空时覆盖——空壳会话内长期滞留 — 置 null 走空态不落缓存（后端是否真可达需核实）
- [P2] src/routes/settings/components/SettingsFileMonitor.svelte:46,60-65,118 —
  `fileMonitor().folders = …` 深层赋值不触发 Svelte 失效：所选目录不回显、继续手输会以旧
  文本覆盖刚选列表（第二轮 SettingsGitlab.svelte:101 编译实证同族新实例）— 局部变量后
  整体重赋值 config.file_monitor
- [P2] src/routes/settings/components/AssistantMemoryManager.svelte:196,216 — 删除/清空
  记忆用裸 window.confirm，绕过全局 ConfirmDialog 模态层级与暗色样式 — 改 stores/confirm
- 重构机会（待过目）：Report.svelte:824-905 与 Overview 的 computePeakWindow/
  formatSignedCompactDuration 逐字重复可抽共享 .ts；设置页 clamp+Number||fallback 模式
  三文件十余处可抽 clampNumberInput；密钥可见性切换块五份可抽 SecretInput 组件；
  Ask.svelte:128,1204 modelSelectEl 死绑定可删；Ask:361-379 与 Report:511-514 各自实现的
  marked+DOMPurify 渲染可抽共享工具（与已知 ⏸ marked 单例互污染一并设计）；
  AvatarCanvas.svelte:511-686 四个热点渲染块同构 SVG 模板约 175 行可合并；
  AvatarFollowupCard.svelte:91-117 三个按钮 class 串逐字相同可 each 消除；
  AppUsageChart.svelte:33 注释「前 8」与 DEFAULT_COUNT=6 矛盾（历轮同类按可修 P2 处理，
  本条随过目批次走）；AvatarCanvas.svelte:342-343 两条响应式语句表达式逐字相同可合一

## 第九轮暂缓（💤）

- 💤 database.rs 连接锁错误映射 86 处逐字重复 — 抽 lock_conn() 助手：纯机械但面广，
  建议与 init_tables 按域拆分同批做
- 💤 get_daily_stats_with_segments_filtered（1913-2361 约 450 行）与浏览器 page_hint 推导
  双份逐字重复（2034-2076 vs 2609-2643）— 拆三段子函数 + derive_browser_page 共享 helper
- 💤 summary.rs build_ai_prompt override 注入把同一组统计字段序列复制 8 处、分类关键词
  三份松同步清单 — 归既有 💤 四语言/分类清单战役
- 💤 gitlab.rs enrich_commit_stats 与 fetch_avatar_data_urls 的「分批 spawn(8)+预算+收割」
  框架逐字同构 — 抽 batched_with_budget 助手（低频发版路径，收益/风险比一般）
- 💤 Timeline(3556 行)/Overview(2597 行)/Report(1746 行) 巨型组件拆分 — 已知条目行号
  持续增长，建议尽快排期专门批次
- 💤 updater.ts 中文散文正则反解后端状态消息 — 长期方向是后端改发结构化 stage 码
  （涉 Tauri 事件契约变更）

## 第九轮测试与流水线

- 批次 1（Rust）：cargo +1.97.1 fmt/check(0 warning)/clippy(0 warning)/test 全绿
  （519+236+21+30=806 通过），npm run test:frontend 833 通过（Rust 改动可能波及的前端
  契约测试全过）；main 流水线全绿：
  <https://home.chenkaidi.top:509/chenkaidi/work-review/-/pipelines/2875>
  （途中 GitLab 服务器宕机 ~1.5h 打断 rust-verify-macos，重试后 build-release-macos 以
  3h04m 完成——作业级 `timeout: 3h`（第八轮补）首次验证生效，runner 12 mac 全量冷编译
  需 >1h）
- 批次 2/3：npm run check(0 错误)/test:frontend(833)/build 全绿 + bash -n 三个改动脚本；
  与 docs 同流水线全绿：
  <https://home.chenkaidi.top:509/chenkaidi/work-review/-/pipelines/2881>
  （resource_group version-bump / clean -e .cache / 私钥清理等本批 CI 改动即在此条
  流水线生效验证）
# 第十轮（2026-09-24）

范围：十分片并行复扫（main+storage+采集 / localhost+网关+bot / agent 七文件 /
commands 19 文件+capabilities+注册表一致性 / database.rs 8.5k 行深审 /
core 其余+mcp-server+skills-engine / stores+utils+i18n+App / 路由页四组件 /
设置页+auth+about+组件库 / CI+脚本+打包+admin-core+e2e），与九轮台账逐条去重。
新发现按台账条目口径 74 条：已修复 36 条（38 处，P0 1 / P1 8 / P2 其余），分两批：
批次 1 = Rust 29 项（commit 9ec77bf，流水线 <https://home.chenkaidi.top:509/chenkaidi/work-review/-/pipelines/2926> 全绿）；
批次 2 = TS/CI/脚本/文档 9 项（流水线 2930 全绿）。.svelte 项 16 条按红线全部列待过目未动；
行为语义/需产品决策项 19 条列待确认；3 条列暂缓。

## 第十轮已修复

### 批次 1（Rust，29 项，commit 9ec77bf）

- ✅ [P0] src-tauri/src/commands/config.rs:244 — change_data_dir 的「同目录/互为父子」
  两道校验在 canonicalize 之前比较未规范化字面路径：Windows 大小写差异/8.3 短名/符号链接/
  「..」等价形式选中当前数据目录时全部绕过，随后 ensure_target_dir_ready 整删该目录受管
  条目（config.json/截图/OCR，Linux 含活动中的数据库）= 数据销毁 — canonicalize 提前到
  比较之前（对齐同文件 cleanup_old_data_dir 既有正确口径）
- ✅ [P1] src-tauri/src/main.rs:49-54 — 崩溃日志目录 CRASH_REPORT_DATA_DIR 用 OnceCell，
  set 只在首次生效：上一轮修复（03aa42e「换目录后同步跟随」）实际未生效，change_data_dir
  后的二次 set 是静默 no-op — 改 RwLock<Option<PathBuf>>（崩溃路径读取容忍锁中毒）
- ✅ [P1] src-tauri/src/agent/model.rs:998 — OpenAI 兼容流式装配器对 tool_call id/name
  逐片 push_str：官方 OpenAI 仅首片携带所以无感，vLLM 等自托管端点每片重复完整值拼成
  "call_xxxcall_xxx"，被 trusted_event_tool_metadata 精确匹配当未注册工具拒绝（该类端点
  工具调用整体失效）— slot 为空时写入（openai-python 官方流式累积语义），arguments 保持逐片拼接
- ✅ [P2] src-tauri/src/commands/memory.rs:133 + timeline.rs:20 — /v1/weekly-review 的
  limit（`unwrap_or(5000)` 只兜默认）可传 4294967295 经 load_filtered_activities_in_range
  全表载入含窗口标题；/v1/timeline/ 的 limit/offset 同族直通 DB — 在 Tauri/localhost 共用
  inner 层统一钳制（limit ≤10000 同 /v1/activities 用户裁决先例、offset ≤1000000）
- ✅ [P2] src-tauri/src/remote_upload.rs:150 — S3 SigV4 canonical_uri 用自造
  url_encode_path（`+()!=@` 等编码成 %XX），实际发送的是 reqwest Url::parse 归一化路径
  （同类字符保持原样），两种形态不一致 → path_prefix 含此类字符时恒 SignatureDoesNotMatch
  — canonical_uri 改 parsed.path()，删除 url_encode_path
- ✅ [P2] src-tauri/src/localhost_api.rs:189 — reason_phrase 缺 403 映射且未知状态回落
  "OK"，回调签名失败/令牌不匹配等 403 响应状态行为 `HTTP/1.1 403 OK` — 补 403 Forbidden
- ✅ [P2] src-tauri/src/agent/model.rs:958 — ensure_stream_status 对非 2xx 的
  response.text() 不受 deadline 约束，半死端点慢速滴漏错误 body 可无限挂起 — 包 deadline
  剩余时间（另设 10s 上限），超时/出错按空 body 报错（行为同前）
- ✅ [P2] src-tauri/src/agent/tools.rs:676-708 — collected_references 4 处
  `if let Ok`/`unwrap_or(0)`/`unwrap_or_default` 把 Mutex 中毒吞成引用收集永久失效零日志
  — 统一 unwrap_or_else(into_inner)（纯内存缓冲）
- ✅ [P2] src-tauri/src/agent/tools.rs:1614 — bing_search_html 注释称「DNS、SSL 也一并
  重试」与实现矛盾（非 connect/timeout 立即返回）— 修正注释
- ✅ [P2] src-tauri/src/agent/orchestrator.rs:633 + tools.rs:3510 — 测试库不清理（orchestrator
  每次 temp 目录建 work-review-*.db 三件套；tools 用固定名 wr-web-tools-test.db）CI 长期
  累积 — orchestrator 抽 TestDb 守卫（drop 先关连接再删三件套）；tools 改唯一名+收尾清理
- ✅ [P2] crates/core/src/database.rs:3176 — save_hourly_summary 在 UPSERT 后返回
  last_insert_rowid()（DO UPDATE 分支下 SQLite 规定未定义，实为陈旧值）；唯一生产调用方
  忽略返回值 — 签名改 Result<()>，注释陷阱
- ✅ [P2] crates/core/src/database.rs:7399,7714-7739 — 7 处测试收尾只删主库文件（WAL 伴生
  文件 CI 持续累积，第九轮清理战役残留）— drop(db) 后 remove_db_files 三件套
- ✅ [P2] crates/core/src/database.rs:3811 — search_memory 文档注释整段粘贴两遍 — 删重复
- ✅ [P2] crates/core/src/database.rs:1253-1375 — get_last_activity_by_app /
  get_latest_activity_by_app / _by_app_title / _by_url 四函数逐字同构（仅过滤列与回溯起点
  不同）— 提取 latest_activity_by_filter 共享 helper + today_start_ts（SQL 形状与参数序不变）
- ✅ [P2] crates/core/src/file_monitor.rs:62 — debouncer 的 Err 批次
  unwrap_or_default() 静默吞掉 notify 错误（watch 失效时事件丢失无痕迹）— 补 warn（条数+首条）
- ✅ [P2] crates/core/src/file_monitor.rs:73 — 同批去重 rows.iter().any() 线性扫描 O(n²)：
  移动含数千文件的目录时单批事件上万约 5×10⁷ 次路径比较卡 worker — HashSet<(path,type)>
  去重（首个命中语义不变）
- ✅ [P2] crates/core/src/analysis/cloud.rs:131,207 — 两处非 2xx 时 response.text() 全文
  （网关大 HTML）拼进 AppError 并 warn 落盘（第九轮 model.rs:261,478 已截 300，core 侧漏网）
  — 补 chars().take(300)
- ✅ [P2] crates/core/src/analysis/summary.rs:1040,1048 — decide_block_order 两条失败路径
  （AI 调用 .ok()? 与 JSON 解析 .ok()?）完全静默，编排永久回退默认顺序无从排查 — 补 warn
- ✅ [P2] crates/core/src/knowledge.rs:272 — builtin_domain_category 在 170 项
  DOMAIN_TABLE 循环内每次 format!(".{suffix}") 堆分配（分类热路径逐活动调用）—
  strip_suffix + ends_with('.') 免分配等价改写
- ✅ [P2] crates/core/src/generation_params.rs:118 — apply_ollama 生产代码
  expect("ollama body is object")（契约靠调用方维持，第九轮 category.rs 同款已裁决防御化）
  — as_object_mut 失败时 warn + 跳过 options 写入
- ✅ [P2] src-tauri/src/main.rs:1420 — 锁屏期间采集循环每轮（约 5s）打一条 info「屏幕已
  锁定」，整夜锁屏上千行 — 仅进入锁屏时打一次（解锁复位）
- ✅ [P2] src-tauri/src/ocr.rs:686-703 — Windows OCR 两条失败路径把整段 stdout（识别出的
  屏幕文本 JSON）warn 明文落盘未过敏感过滤（偏离本仓脱敏口径）— 只记长度/状态码
- ✅ [P2] src-tauri/src/commands/shared.rs:266 — persist_app_config 不刷新
  config_load_status：storage_cleanup_task 读的启动快照从不再赋值，启动时 Corrupted
  （fail-safe 跳过清理保护数据）的用户在设置页修复配置后周期清理永久跳过直到重启 —
  成功落盘后把 fail-safe 状态置 Loaded
- ✅ [P2] src-tauri/src/screen_lock.rs — is_locked: Arc<AtomicBool> 字段只写不读、
  set_locked 全仓无调用点，#![allow(dead_code)] 屏蔽 — 全部删除（含 allow）
- ✅ [P2] src-tauri/src/monitor.rs:4486 — get_cf_dict_number 是 unsafe fn 却无 # Safety
  文档（前轮 FFI 补注漏网）— 补注
- ✅ [P2] src-tauri/src/commands/system.rs:294 — refresh_dock_icon 回退硬编码
  /Applications/Work Review.app/...（更名 Yue-Review 后永不命中且静默）— 删除回退
  （bundle 内找不到即放弃重设）
- ✅ [P2] src-tauri/src/commands/system.rs:1268 — save_background_image 对 IPC base64
  无长度上限、image::load_from_memory 无解码限制，解压炸弹可把内存撑到数 GB — 载荷钳
  20MB + image::Limits（8192px/256MB）
- ✅ [P2] src-tauri/src/commands/category.rs:299 — delete_custom_category 的 reassign_to
  无任何校验：传不存在的 key 或恰等于被删 key 时规则与全部历史活动被批量重定向到无效分类
  — 校验必须存在于内置/自定义分类且 ≠ 被删 key，否则回退 "other" 并 warn

### 批次 2（TS/CI/脚本/文档，9 项）

- ✅ [P1] src/lib/i18n/locales/{zh-CN,en,zh-TW,ar}.ts — settings.account* 6 键在四语言
  全部误嵌 settingsStorage 分组（8403aea 同类回归），组件 t('settings.account*') 两级回退
  失败，账号卡片渲染裸键名 — 四语言同步移入 settings 组（总叶子数不变，1_242）
- ✅ [P2] src/lib/utils/updater.ts:113 — RUNTIME_STATUS_PATTERNS 死映射
  /^当前未发现可安装的在线更新$/ → updater.noInstallAvailable（后端已无该状态消息，
  永不可能命中）+ 四语言死键 — 删除映射与键，index.test.ts 叶子数钉 1_243→1_242
- ✅ [P2] scripts/capture-readme-pages.ts:839 — 桌宠移除删掉设置页 index 3 页签后仍按旧
  索引点击：重拍 README 截图整体错位一档且无报错（化身→隐私→存储→接入全偏）— 按现
  tabs 数组修正、删化身条目、captureSettingsTab 加索引越界守卫；mock config 清理
  avatar_*/break_reminder_* 与 avatar_input_supported 残留；契约测试断言同步
- ✅ [P2] scripts/appimage/uninstall.sh — 未随改名适配：pgrep/.desktop/图标/AppImage
  glob/DATA_DIR 全部只认 Work_Review 旧名（deb/uninstall.sh、reinstall.sh 均已双名，
  唯此脚本漏改）— 按 Yue_Review/Yue-Review/Work_Review 多名字探测改写，新旧数据目录双清
- ✅ [P2] scripts/deb/reinstall.sh:33 — --help 的 sed '2,13p' 没跟上第九轮扩充的
  Local-deb lookup / 无远程回退说明 — 改 2,17p
- ✅ [P2] scripts/deb/reinstall.sh:361 — OGA_TMP 未走 register_cleanup（DEB/TMP_FWD 均已
  注册）：set -e 下半途失败或 Ctrl-C 残留 /tmp/wr-silent.*.oga — 补注册
- ✅ [P2] .gitlab-ci.yml:54,596 — 容器打包作业的 CARGO_HOME 挂在
  $CI_PROJECT_DIR/.cargo-home，不在 GIT_CLEAN_FLAGS 与 version-bump git clean 排除清单：
  每次发版容器内 crate registry 全量重下 — 两处补 -e .cargo-home
- ✅ [P2] README.zh.md:62,130 — 桌宠移除漏删简体：功能列表「桌面化身 Beta」小节 + 截图区
  桌面化身图片引用（指向已删除的 png；README.md/tw 已清）— 删除
- ✅ [P2] scripts/capture-readme-pages.test.ts:39 — 契约测试钉死『设置-桌面化身』标签把
  过期行为固化 — 断言与现页签对齐

## 第十轮待过目批次（.svelte，按红线待用户过目，本轮未动）

- [P1] src/routes/Overview.svelte:2086-2087,2128 — 919a937 在两行 HTML 注释中间插入整个
  文件活动卡片段落，注释在 2087 行提前闭合，2128 行「行布局复用 app-usage-chart 同款网格
  （名称+次数下标 | 进度条 | 行数变更） -->」成裸文本，概览页两卡片间渲染可见乱文 —
  补回闭合并另写完整独立注释
- [P1] src/routes/Overview.svelte:2103 — 文件活动 keyed each 的 key 是
  path+':'+occurred_at（秒级），同路径同秒多事件（保存文件时 created+modified 连发常见）
  产生重复 key，Svelte 4 keyed each 直接抛 "Cannot have duplicate keys" 中断组件更新 —
  key 拼入 event_type 或 size
- [P1] src/routes/settings/Settings.svelte:346-347,480-487 — saveConfig 失败写入与
  loadConfig 共用的 error，模板 `{:else if error}` 分支优先于 config 分支：一次保存失败后
  整个设置页被「加载失败」横幅整体替换，点重试（loadConfig）还丢弃未保存修改 — 保存错误
  改独立 saveError 状态内联展示
- [P1] src/routes/settings/Settings.svelte:588 + SettingsNodeGateway.svelte:185,191 —
  节点网关 toast 事件链无消费者（父组件只挂 on:change 未挂 on:toast，Svelte 4 dispatch
  不冒泡）：保存失败/token 显示/轮换/复制等反馈全部静默丢弃 — 补 on:toast 转 showToast
- [P2] src/routes/Report.svelte:571-573 — deletePreset 经局部别名深层赋值不触发 Svelte
  失效：删除非活跃预设后胶片不消失（删除活跃预设仅因 persistReportPrompt 内部整体赋值而
  侥幸刷新）— 直接对 config.xxx 赋值
- [P2] src/routes/Report.svelte:354-361 — savePresets 捕获保存异常后仅 console.error，
  调用方仍按成功收尾，重启后改动静默丢失 — 返回成功与否由调用方 toast 报错并回滚
- [P2] src/routes/Timeline.svelte:1604 — {#each timelineRows} 无 key：实时 upsert 插队首
  整体重排，无 key each 按索引原地改写全部行 DOM（缩略图/图标 src 变更）整列表闪烁 —
  用 kind+key 组合键
- [P2] src/routes/Timeline.svelte:1657-1784,1108-1111 — getTimelineIconSrc 同一渲染被调
  3 次（每次全表扫描图标规则）；viewActivity 的 getTimelineThumbnail 连调两次（已有局部
  变量未复用）— 行首 {@const} 复用
- [P2] src/routes/settings/Settings.svelte:347 — 保存失败回退文案误用
  t('common.loadFailedRetry')（「加载失败，请稍后重试」，loadConfig 复制粘贴残留）—
  换保存失败文案键
- [P2] src/routes/settings/SettingsGeneral.svelte:417-421 — 工作目标小时输入无 clamp 不
  回写（同文件 356-369 标准工时有修好先例）：手输 30 直接存 1800 分钟 — 复用 clamp+回写
- [P2] src/routes/settings/SettingsAppearance.svelte:118-146 — handleBgFileSelect 的
  FileReader 只挂 onload 未挂 onerror：读文件失败后 bgUploading 永久 true，「选择图片」
  按钮锁死 — 补 onerror 复位并提示
- [P2] src/routes/settings/nodeGateway/{DeviceRegistryPanel,DeviceIdentityPanel}.svelte —
  新增设备行与名称/URL/Token 输入全程不 dispatch save/change：填完关页静默丢失且主页
  dirty 不置位 — on:blur 时 dispatch('save')（与 BotCredentialsPanel 同款）
- [P2] src/routes/settings/SettingsNodeGateway.svelte:166-188 — persistConfig 单个
  try/catch 同时覆盖 save_config 与 3 个状态查询 IPC：保存成功但状态刷新失败也按「保存
  失败」上报 — 状态刷新独立 try/catch 失败仅记日志
- [P2] src/routes/settings/SettingsNodeGateway.svelte:171 — 保存成功后 dispatch('change')
  使 Settings.svelte 无条件 dirty=true：节点页每次自动保存后主页恒「未保存」，cache 订阅
  被 dirty 门挡住 — 复用 autosaved detail 协议
- [P2] src/routes/settings/SettingsSystem.svelte:574-579,687-689 — 桌宠段删除后死 CSS：
  .permission-inline-meta 与 :global(.dark) .permission-overview-platform 全仓零引用 —
  删除
- [P2] src/lib/App.svelte:710-728 — 日报定时器工作记忆分支在结束到午夜间每分钟无条件
  getRuntimeConfig()（约 360 次/晚空转 IPC，即使 memory_enabled=false 或当日已合成）—
  先用本地已同步 runtimeConfig 短路再决定拉取

## 第十轮待确认（修复会改对外行为/语义或需产品决策，未经确认不动）

- ⏸ [P2] src-tauri/src/bot_common.rs:383 — /report 的日期参数经 resolve_single_date
  （_ => s 无格式校验）原样 format! 进设备 API URL path，`..`/`?`/`#` 可改写 bot 对设备
  发出的 GET 请求路径（携带 Bearer token）；仅 GET 且响应形状校验兜底，泄漏面有限 —
  建议白名单 `^\d{4}-\d{2}-\d{2}$`；属输入收紧，与 /generate body date 一并统一需裁决
- ⏸ [P2] src-tauri/src/main.rs:1964 — 合并路径 latest.id 为 None 时 log::error 后
  continue，连带跳过本轮浮窗记账与 screenshot-taken 事件；id 来自 DB 查询实际恒 Some —
  是否改防御性返回涉及行为取舍
- ⏸ [P2] src-tauri/src/storage.rs:198-223 — 超限清理从最旧删起但不保护「当天」目录：
  storage_limit_mb 小于当天已产出体积时正在写的当天截图也会被删 — 是否排除今天目录属
  产品决策
- ⏸ [P2] crates/core/src/database.rs:5068-5128 — search_memory_chunks_semantic 全程持
  连接锁全表流式扫描+逐行点积（万级块百毫秒级，阻塞采集热路径）；注释称流式为省内存 —
  物化后释放锁需权衡内存
- ⏸ [P2] crates/core/src/database.rs:555-572 — parse_date_bounds 对非法日期串静默忽略
  （fail-open）：MCP 传错格式 date_from/time 过滤整段失效返回全范围 — 改报错属对外行为变更
- ⏸ [P2] crates/core/src/database.rs:1395-1454 — merge_activity 读旧 OCR 与 UPDATE 两条
  语句未包事务，理论窗口内并发删除/合并丢一次 OCR 追加（采集单写者设计触发概率极低）—
  unchecked_transaction 可无行为变化闭环，是否值得加请确认
- ⏸ [P2] crates/core/src/database.rs:1086-1101 — file_events 表只增不删无保留策略（与
  已 ⏸ gitlab_commits 同族）— 数据保留决策
- ⏸ [P2] src-tauri/src/agent/tools.rs:1587-1617 — bing_search_html 最坏 3×20s+退避
  ≈63s 不感知 deadline，放大已知 ⏸ executor.rs:442 的单轮预算问题 — 与 442 同批定案
- ⏸ [P2] src-tauri/src/agent/model.rs:894-902 — LineBuffer 单行无上限且 drive_stream 无
  总量上限（恶意端点超长无换行行使缓冲无界）— 与已知 ⏸ model.rs:986/1259 同一威胁模型
  待定项
- ⏸ [P2] src-tauri/src/agent/executor.rs:855-865 — emit_control_event 的
  send_control().await（mpsc 投递 ack 等待）不在取消保护内：前端存活但卡死时 executor 在
  ConfirmRequest/Done 发送上永久挂起无法被打断 — 给 Done 加取消包裹会收不到终态，属
  行为变更
- ⏸ [P2] src-tauri/src/commands/system.rs:1292-1296,1344-1346 — save/clear_background_image
  直改 state.config 后 config.save()，绕过 persist_app_config（不 stamp_local_sync/
  spawn_push/emit_config_changed/normalize）：启用配置同步的设备上背景图变更不上推且会
  被其他端整包推送覆盖 — 「背景图仅本机生效」是否有意需确认
- ⏸ [P2] src-tauri/src/commands/report.rs:330-331 — 日报自动导出 ensure_export_dir_
  components_safe 不 canonicalize（与手动导出口径不一）：父目录预置指向 .ssh/LaunchAgents
  的符号链接可绕过组件检查写入 — create_dir_all 后 canonicalize 复核可不动入参契约
- ⏸ [P2] src/routes/Timeline.svelte:439-458 — confirmPrivacyRule 的 get_config→改→
  save_config 整包回写与全库 14 处同款并发时读-改-写丢失更新 — 根治需后端专用命令，
  涉 IPC 契约变更
- ⏸ [P2] src/routes/ask/Ask.svelte:134-159 + modelPresentation.ts — 显示回退链平行实现
  两份已现终点漂移（行为当前等价）— 是否统一走 modelPresentation.ts 酌定
- ⏸ [P2] src/routes/settings/Settings.svelte:566-567 — filemonitor 标签图标分支渲染两个
  并排 SVG（其余七个标签单图标，桌宠移除前已存在）疑似残留 — 有意与否需过目
- ⏸ [P2] src/routes/settings/nodeGateway/ApiExamplesPanel.svelte:23,46-67 — examples
  初始化快照 curlBase 依赖 SettingsNodeGateway 的 {:if loading} 门控时序，面板移出该
  分支即静默回退演示地址 — 改 $: 派生或注释钉住
- ⏸ [P2] scripts/ci/linux-builder-22.Dockerfile:22 — 供应链白名单不覆盖构建器基镜像
  （daocloud 拉取未按 digest 钉定，镜像内 apt/Node/Rust 不在白名单）— 钉 FROM digest
  涉运维流程
- ⏸ [P2] .gitlab-ci.yml:58 — WINDOWS_RUNNER_TAG: "windows, rust" 单字符串含逗号空格，
  GitLab 不拆分 tags 数组元素：当前能调度推测 runner 恰以字面单 tag 注册，重注册为双
  tag 会让全部 Windows 作业永久 pending — 需核实 runner 实际注册形态
- ⏸ [P2] tests/e2e/{installer-isolation,ui}/reports/ 生成物已入库（.gitignore 只忽略
  artifacts/html）— 有意留档还是漏忽略请裁决

## 第十轮暂缓（💤）

- 💤 crates/core/src/database.rs 约 15 处 query_map(...).filter_map(|r| r.ok()) 把行级
  解码错误静默吞掉、结果集无声截断 — 至少对 Err 行计数 warn，关键读路径改 collect
  Result；面广需专门批次
- 💤 src-tauri/src/main.rs:982-1002 等 — should_confirm_idle 的 _screenshot_confirmed
  参数已忽略但 input_idle 时每次截图仍整图解码算哈希只喂诊断计数 — 加 debug 门控或删
  参数需先查证 NO_CHANGE_COUNT 外部消费
- 💤 巨型组件拆分（Timeline 3556/Overview 2597/Report 1746/Ask 1317 行）— 已知条目，
  行号持续增长，建议尽快排期专门批次

## 第十轮测试与流水线

- 批次 1（Rust）：cargo fmt/clippy（0 warning）/test 全绿（412+224+21+30=687 通过），
  npm run test:frontend 733 通过；main 流水线全绿：
  <https://home.chenkaidi.top:509/chenkaidi/work-review/-/pipelines/2926>
- 批次 2：npm run check(0 错误)/test:frontend(733)/build 全绿 + bash -n 两个改动脚本；
  main 流水线全绿：
  <https://home.chenkaidi.top:509/chenkaidi/work-review/-/pipelines/2930>
  （批次 1 全绿后 version-bump 自动发 v1.1.95，批次 2 推送 rebase 其上为 de85951）
# 第八轮（2026-09-22）

范围：八分片并行复扫（main+commands / monitor+screenshot+ocr / localhost+avatar+杂项 /
agent+五bot / crates 三库 / 前端纯 TS / 43 个 .svelte 全量 / CI+admin-core+capabilities+Cargo），
与七轮台账逐条去重。新发现 30 条（2 P1 + 28 P2），另否决分片误报 2 条（admin Dockerfile
示例漂移——实际已无 cargo 挂载示例；rust-verify-windows 缺 timeout——:285 已有）。
分三批修复 24 条行为保持项：批次 1 = Rust（13 项）、批次 2 = CI/构建（8 项）、批次 3 = TS（2 项）；
14 条涉行为语义列待确认；1 条 .svelte 待过目；2 条暂缓。

## 第八轮已修复

### 批次 1（Rust，13 项）

- ✅ [P1] src-tauri/src/storage.rs:86,133 + crates/core/src/config.rs — screenshot_retention_days
  无上限钳制，超大值（配置同步/手改配置直通 u32）使 `date_naive() - Duration::days` 越过
  NaiveDate::MIN panic：启动清理路径崩即启动失败，周期清理路径 panic 被吞成 warn 后清理永久失效
  （"无上限数值→chrono panic"家族第三处，parse_temporal_range/storage_limit_mb 之后）—
  normalize 钳 [0,3650]（0=永久保留不变）+ 两处使用点 min(3650) 兜底，配回归测试
- ✅ [P2] crates/core/src/privacy.rs:25-38 — 四个敏感模式（银行卡/手机号/身份证/邮箱）`\b`
  Unicode 词边界致 CJK 邻接整条漏配；第七轮采集/桌宠日志统一走的就是本 filter_text，脱敏实际
  失效 — 去 \b + 匹配后 ASCII 邻位手动校验（ocr.rs 已修款），配中文邻接/订单号不误伤回归
- ✅ [P2] src-tauri/src/main.rs:919 — save_data_dir_preference 对 data-location.json 非原子写，
  崩溃留半截 JSON 致下次启动静默回退默认数据目录 — 临时文件（pid+纳秒）+rename 原子写
- ✅ [P2] src-tauri/src/node_gateway.rs:73-126 — 第六轮并发 ensure 修复不完整：rename 冲突
  回退分支返回 Ok(()) 但调用方仍返回自生成幽灵 device_id（Windows 真实可达），Unix rename 覆盖
  使读-判-写竞态原样存在 — 进程内 NODE_IDENTITY_WRITE_LOCK 串行化（localhost_api token 写锁
  同款）+ 落盘后重读返回磁盘身份
- ✅ [P2] src-tauri/src/main.rs:3944-3961 — entity_classify_task 对批内每个 domain 实体各调一次
  无 LIMIT 14 天浏览器 URL 全量查询且在 AppState 锁内（一批最多 12 次全表扫描持全局锁，阻塞
  采集循环与 UI 命令）— 整批一次扫描建 domain→ids 映射复用，回溯修正 id 集合不变
- ✅ [P2] src-tauri/src/main.rs:2016,2294,2326,2358,2493 — debug 级日志明文记录窗口签名/标题/URL
  （第七轮只修了 info 级两处）；RUST_LOG=debug 排障时邮件主题/URL 进日志 — 落日志前
  filter_text 脱敏，热路径 log_enabled 守卫下才加锁
- ✅ [P2] crates/core/src/database.rs:3365-3425 — get_activities_by_normalized_app_name /
  get_activities_by_domain 无 LIMIT 全列（含 ocr_text）全量载入+前导通配 LIKE 全表扫描，分类
  修正 UI 对高频应用触发时载入全历史 — 两段式：轻量 (id, app_name/browser_url) 筛选 + 按 id
  分块（500）取全列，结果集不变
- ✅ [P2] src-tauri/src/ocr.rs:745 — macOS Vision OCR AppleScript 路径插值未转义，路径含 `"`/`\`
  时脚本编译失败 OCR 静默降级（Windows 侧 PowerShell 有转义）— 补 `\\` 与 `\"` 转义
- ✅ [P2] src-tauri/src/monitor.rs:2904 — 注释写"提取第一个部分"、变量名 first_part，代码实为
  split(" - ").last()（macOS "Document - AppName" 格式取 last 才对）— 注释与变量名对齐代码
- ✅ [P2] src-tauri/src/wecom_aibot.rs:604 — 内联 Duration::from_secs(…max(60)) 与 bot_common
  共享 helper 重复（其余 4 bot 均已调用），floor 调整必漏改本处 — 改调 report_generate_timeout
- ✅ [P2] src-tauri/src/avatar_engine.rs:243,325 — "文档/阅读"判定 needle 数组两分支逐字重复，
  单边演化即行为漂移 — 抽 DOC_TITLE_KEYWORDS / DOC_URL_KEYWORDS 共享常量
- ✅ [P2] monitor.rs:731,896,1327,1343 / screenshot.rs:500,1519 / idle_detector.rs:121 /
  screen_lock.rs:88 — 平台 FFI unsafe 块无 SAFETY 注释（第三轮已为 commands 侧补齐）— 逐块补
  SAFETY 注释（含提前返回路径资源释放说明）
- ✅ [P2] src-tauri/src/lib 辅助 — 上述修复各自配的回归测试（storage 钳制 / privacy 中文邻接）

### 批次 2（CI/构建，8 项）

- ✅ [P1] .gitlab-ci.yml:752 — 更新签名私钥（base64 全文）以 `-e VAR="$VAR"` 进 docker 客户端
  进程 argv，打包期间（数十分钟）同机进程可经 /proc/<pid>/cmdline 读取（linux runner 同机跑
  生产 admin 栈）— 改无值 `-e VAR` 形式（docker 从作业环境读值，argv 只留变量名）
- ✅ [P2] .gitlab-ci.yml:575 — version-bump 认证 `-c http.extraHeader=Basic <token>` 进 git 进程
  argv — 改 GIT_CONFIG_COUNT/KEY_0/VALUE_0 环境变量（仅本用户可读）
- ✅ [P2] .gitlab-ci.yml:571 — `git tag` 不幂等：job 在 tag 创建后 push 前被取消即残留本地 tag，
  同版本重跑恒 "already exists" 失败且需人工上 runner 清理 — 改 `git tag -f`（版本号由内容推导）
- ✅ [P2] .gitlab-ci.yml:576 — 分支+tag 两次引用一次 push 无原子性，中途失败 tag 先落会产生指向
  main 之外提交的 tag、触发 preflight 必红的 tag 流水线 — 改 `git push --atomic`
- ✅ [P2] .gitlab-ci.yml:262-518 — build-release-windows 与五个 package-daily-* 无作业级 timeout
  （default: timeout 本实例不生效，实测死于 3600s）— 六个 job 补 `timeout: 3h`（同 .rust-verify
  已修款；rust-verify-windows/.release-compile/.tauri-package 已有，经逐块核验不重加）
- ✅ [P2] .gitlab-ci.yml:529-536 + scripts/ciVersionFlow.test.ts:31 — version-bump needs 七项默认
  artifacts: true，把三平台数十 MB 二进制下载进从不消费的 bump 作业 — 全改 `artifacts: false`，
  契约断言同步为 `- job: X + artifacts: false` 形态（PUSH_AUTH 钉断言同步为 GIT_CONFIG 形态）
- ✅ [P2] .gitlab-ci.yml:204,967 — nightly-gate 取消 API 与 publish-release 上传循环的 curl
  `-H "PRIVATE-TOKEN: $…"` 进 curl 进程 argv — 改 mktemp header 文件 + `--header @file` + trap 清理
- ✅ [P2] scripts/ci/linux-builder-22.Dockerfile:35 + Cargo.toml — builder 镜像 Node 20 与
  engines>=22/其余作业 Node 22 漂移，改 setup_22.x；workspace release profile 补
  `[profile.release.build-override] opt-level = 1`（build script/proc-macro 免 O3，缩短三平台
  release 编译）

### 批次 3（TS 去重，2 项）

- ✅ [P2] src/lib/utils/appDisplay.ts:19,23 — 安装器词表 ['setup','install','installer','uninstall']
  相邻两函数重复定义 — 抽 INSTALLER_TOKENS 共享
- ✅ [P2] src/lib/stores/categories.ts + categories.test.ts:54 — 两个 store 四处逐字重复的
  "订阅即退出"快照读取 — 抽 snapshotStore<T> 助手；契约测试同步改钉 snapshotStore 4 处 + 助手签名

## 第八轮待确认（修复会改对外行为/语义，未经确认不动）

- ⏸ [P2] src-tauri/src/main.rs:1925-1949+797-848 — 两套"频繁切换窗口"提醒机制并存（局部
  6次/5分钟/冷却30分钟/硬编码中文 vs AvatarNudgeRuntime 8次/3分钟/冷却20分钟/persona 键），
  前者 emit 绕过 i18n（en/zh-TW 用户气泡恒中文）且同行为可能收两条相似提醒 — 合并为单一
  runtime（行为变更：提醒频率与非中文文案改变）
- ⏸ [P2] src-tauri/src/commands/config.rs:234-321 — change_data_dir 迁移全程不暂停录制：复制
  screenshots 期间新截图落旧目录致迁移后缩略图断链，backup_to 后切换前写旧库的活动行丢失 —
  迁移前置 is_paused（行为变更：迁移期间录制暂停）
- ⏸ [P2] src-tauri/src/commands/avatar.rs:74,275 / stats.rs:819 / system.rs:686,714,759 — 命令层
  平台子进程 .output() 全部无超时（macOS osascript 权限弹窗未决时 get_running_apps 永久挂起）—
  归既有 ⏸ "平台工具 .output() 无超时"家族同批处理
- ⏸ [P2] src-tauri/src/commands/ask.rs:2050 + main.rs（SetAppCategory/set_app_locale/托盘开关等
  五处）— clone 配置→放锁→整体 persist 的丢失更新窗口 — 归既有 ⏸ persist_app_config 收敛家族
- ⏸ [P2] src-tauri/src/agent/tools.rs:948-957 — search_memory_execute 手写域名过滤与权威
  filter_activities_by_privacy 口径漂移（按 url+标题 contains，而非 extract_domain+domain_matches）：
  同一隐私规则两条工具路径结果不一致（当前方向为过滤过宽，安全但语义漂移）— 对齐权威实现
  （行为变更：工具结果内容）
- ⏸ [P2] src-tauri/src/agent/events.rs:122-146 — default_tool_label 缺 4 个长期记忆工具映射，
  步骤卡/确认卡一律显示「处理中」— 补映射（行为变更：UI 文案）
- ⏸ [P2] src-tauri/src/agent/orchestrator.rs:82,102,364 — 无模型普通聊天问候判定 "hi" 用子串
  contains："which day is it" 等短问句误入 Direct 回英文问候语 — 改词边界精确比对（行为变更：
  回复语义）
- ⏸ [P2] crates/core/src/database.rs:4143 — recall_user_memories 的 Rust to_lowercase（Unicode
  折叠）与 SQL LOWER（仅 ASCII）不一致，非 ASCII 大写查询召回不到 — 与既有 ⏸
  search_user_memories 同批定目标语言面（行为变更：召回语义）
- ⏸ [P2] crates/core/src/analysis/mod.rs:596-599 — generate_activity_timeline 把 window_title
  原样拼 markdown 表格单元格，`|` 未转义破坏日报"活动时间线"表结构（"标题 | 站名"形态极常见）
  — 转义后再拼行（行为变更：报告渲染输出；与第四轮 ⏸ GitLab 字段转义同族，可同批决策）
- ⏸ [P2] crates/mcp-server/src/main.rs:612-627（附注）— filter_activities_by_privacy 的
  Anonymize 分支除已知 ⏸ 的 ocr_text 外 screenshot_path 也未清空（DB 内为相对路径、本地 MCP
  客户端无直接读文件面，危害低）— 与 ocr_text ⏸ 同批顺带
- ⏸ [P2] src-tauri/src/monitor.rs:4372 — get_overlay_windows 前台排除用规范化显示名与 CGWindow
  原始进程名恒等比较（通用进程名应用前台+有合格浮窗时浮窗被记成独立活动、时长双计）；修复需给
  ActiveWindow 携带 pid 或传参（跨结构接口面）— 待确认后动
- ⏸ [P2] .gitlab-ci.yml:158-166 — deploy-admin-core-yx 无 needs，被整条 app 流水线连坐（任一
  无关 job 失败 admin 部署即不执行、正常路径也要等最慢 mac 编译）— 加 needs: [] 解耦（行为变更：
  部署时机语义，需确认 admin 是否有意随 app 全绿才部署）
- ⏸ [P2] .gitlab-ci.yml:172 — docker build --no-cache 使 admin 镜像每次 main push（含 docs-only）
  全量重建且 image ID 变化触发生产容器重启；但 73b1dc5 加 --no-cache 是为修 BuildKit clone 层
  缓存致旧代码的真实事故，REF build-arg 是否足以击穿缓存需实测验证 — 待确认后再动
- ⏸ [P2] src-tauri/tauri.conf.json:107 — additionalBrowserArgs 全窗口关 SmartScreen+混合内容防护
  （--disable-features=msSmartScreenProtection / --allow-running-insecure-content /
  --unsafely-treat-insecure-origin-as-secure）为内网 http admin 的刻意取舍（a680887，修打包版
  登录被混合内容拦截）— 正解是给 admin 上 https/反代证书后移除三个 flag（行为变更/部署语义）

## 第八轮待过目批次（.svelte，按红线待用户过目）

- 💤 [P2] src/routes/settings/components/SettingsAI.svelte:951-979 — 模型下拉无法回显"手动输入"
  的当前模型：手动配置后点「刷新模型」成功且列表不含该名字时，模板切到 select 而选项无匹配，
  下拉显示空白/停在 __manual__，与实际配置不一致（配置值本身未丢失）— 渲染 select 前若当前
  模型不在列表自动置 showManualInput（或注入额外 option）（行为变更）

## 第八轮暂缓（💤）

- 💤 [P2] src/lib/i18n/locales/*.ts（四语言）— i18n 死键清账：memoryPage 全部 20 键、
  settings.summary 全部 21 键、settingsGeneral.localApi* 约 24 键、nodeGateway* 约 18 键及一批
  散键在四语言成套零引用（合计 164+ 叶子/语言）；需"全键引用扫描 + 动态键白名单（如
  report.blockNames.*）"专门批次一次性清，同步叶子数契约（现钉 1_391）与四语言 parity
- 💤 [P2] .gitlab-ci.yml:379-518 — 三平台 package-daily 的 PowerShell after_script 约 27 行 ×2
  逐字重复 + macOS 四作业 before_script/variables 模板重复 ×4 — 抽隐藏 job 模板 + !reference；
  发布关键路径大改 + ciArtifacts/ciUpdateChain 契约同步，单独批次

## 第八轮复核无新问题（抽样说明）

- 前七轮修复面复核完好：safe_screenshot_path、validated_join 全覆盖、token 原子写+写锁、
  accept 循环容错、Firefox 会话缓存、is_browser_app 门禁、SSRF 护栏与重定向逐跳校验、
  dingtalk sessionWebhook 白名单、wrapper 三处 wrap_untrusted_content、FIREFOX_SESSION_CACHE
  等均在位；deadline 贯通所有 await 点；avatar_input 各平台输入线程生命周期无泄漏路径
- MSSQL 注入面：crates 内 format!/push_str 拼 SQL 仅两处安全点（常量列投影/常量条件拼装）；
  VACUUM INTO 单引号转义在位；IN 列表为 i64 自拼接
- 前端 XSS 面：4 处 {@html} 均走 marked→DOMPurify 闭合；无 innerHTML/eval 直通面；
  定时器/监听器/store 订阅 onDestroy 配对完整；异步竞态 requestId/快照守卫完整
- 非问题澄清：Claude 请求连续 user 轮次（Anthropic 服务端自动合并，非 bug）；admin Dockerfile
  头部示例已无 cargo 挂载（分片报告的"示例漂移"不成立）

# 第七轮（2026-09-21）

范围：七分片并行复扫（main+commands / monitor+screenshot+ocr / localhost+avatar+杂项 /
agent+五bot / crates 三库 / 前端纯 TS / Svelte 组件 / CI+scripts+admin-core），与
前六轮清单逐条去重。新发现 22 条确认问题（3 P1 + 19 P2）+ 1 条仓库级 P1（updater
私钥入库），分三批修复：批次 1 = Rust 安全/停机（18955b2）、批次 2 = Rust 健壮性
（48e6037）、批次 3 = TS 死代码 + CI 诊断（b93e7a3）；12 条涉行为/部署语义列待确认；
2 条涉 .svelte 归过目批次。

## 第七轮已修复

### 批次 1（Rust 安全/停机，18955b2）

- ✅ [P1] crates/core/src/analysis/cloud.rs:240 — screenshots_dir.join(DB
  screenshot_path) 未校验即读文件并 base64 外发 AI 端点：绝对路径整体替换/`..`
  越界（调用方 report.rs:249 传 data_dir）可读任意文件——与第一轮已修
  timeline.rs:124（P0）同族，crates/core 因无 validated_join 被漏网 — 新增
  safe_screenshot_path（拒绝绝对/根/`..`/Prefix + 拼接后前缀复查）+ 平台分组回归测试
- ✅ [P1] src-tauri/src/commands/shared.rs:46 — parse_temporal_range 对"最近/近/过去
  N 天"的 N 无上限，超大 N 使 chrono::Duration::days 越界 panic；问题文本经
  orchestrator fast_answer（FastPath 与 Agent 回退两路）直达此处，"近99999999天"
  即可打崩一次助手请求 — N 夹紧 [0,3650] + 回归测试
- ✅ [P1] src-tauri/src/localhost_api.rs:512 — accept 循环对任何 accept 错误 `?`
  传播终止整个本地 API：Windows 客户端在 accept 前重置（WSAECONNRESET）即触发，
  MCP live 上下文/设备面从此永久停机 — 瞬态错误记日志 + 50ms 退避重试，
  连续 200 次失败仍按异常退出上报
- ✅ [P2] src-tauri/src/main.rs:3600 — remote_upload_backfill_task 对 screenshot_path
  只查 is_absolute 即 data_dir.join 并把文件内容上传远程存储：`..` 越界可把
  data_dir 之外文件传上用户远程存储 — 复用 validated_join（commands/mod.rs 增
  pub(crate) 再导出）

### 批次 2（Rust 健壮性，48e6037）

- ✅ [P2] src-tauri/src/main.rs:2394/1998 — 采集循环"应用切换"日志（隐私过滤之前）
  与桌宠循环"状态切换"日志（完全无过滤）以 info 级记录原始窗口标题/URL：邮件主题、
  IM 会话名绕过隐私规则持久进日志 — 落日志前经 privacy_filter.filter_text 同款脱敏
- ✅ [P2] src-tauri/src/localhost_api.rs:216 — token 直接 truncate+write 非原子：
  写中途崩溃留半截文件且 ensure 只对"缺失或空"自愈，持旧 token 客户端持续 401 —
  临时文件（pid+纳秒）+ rename 原子写（同 node_gateway 第六轮模式），Unix 保持 0600
- ✅ [P2] src-tauri/src/localhost_api.rs:292 — ensure/rotate 读-判-写无并发保护：
  双并发 ensure 各自生成不同 token 先后落盘，先返回方配置的客户端恒 401（node_gateway
  同族问题的本文件面）— LOCALHOST_API_TOKEN_WRITE_LOCK 串行化 ensure/rotate
- ✅ [P2] src-tauri/src/monitor.rs:3387 — macOS get_browser_url 缺 is_browser_app
  门禁（Linux/Windows 入口均有）：名称含 arc/safari 等子串的非浏览器应用（Archive
  Utility/Archiver）每轮 spawn osascript 查 Arc 等 URL 并串号归属 — 入口补门禁
- ✅ [P2] src-tauri/src/monitor.rs:619 — Firefox 会话文件（recovery.jsonlz4 可达数十
  MB）每轮轮询全量读取+mozlz4 解压+JSON 解析无缓存（Windows 有 3s TTL 缓存，
  macOS/Linux 无），浏览期间常驻 CPU 热点 — 按（应用,标题）键 3s 成功/2s 失败 TTL
  缓存，与 Windows 口径一致
- ✅ [P2] src-tauri/src/ocr.rs:1538 — IP 过滤正则 `\b` 是 Unicode 词边界：CJK 汉字属
  词字符，中文与 IP 直接相邻（OCR 中文输出极常见）时整条漏配明文落库 — 与银行卡号
  同款匹配后邻位手动校验 + 回归测试
- ✅ [P2] src-tauri/src/agent/tools.rs:1292 — trend_comparison 总时长变化为负时
  format_duration_compact(delta) 命中 <=0 早退返回 "0s"，模型收到 "0s (-37.5%)"
  误以为时长没变 — 新增 signed_duration_compact（负值取绝对值+负号）+ 回归测试
- ✅ [P2] crates/core/src/database.rs:3887 — delete_assistant_conversation 两条
  DELETE 未包事务，第二条失败留 0 消息空壳会话（同表 create/append 已有事务先例）—
  unchecked_transaction 包裹
- ✅ [P2] src-tauri/src/telegram_bot.rs:142 — 守卫按 trim 判空但原样传 API：带首尾
  空白 token 通过守卫后 401（wecom_aibot id/secret 已 trim，同族对齐）— 入口 trim
- ✅ [P2] src-tauri/src/agent/model.rs:629 — parse_openai_response doc 注释仍写
  "Ollama 也用这个"（第六轮已改走 parse_ollama_response）误导维护者 — 注释更正

### 批次 3（TS 死代码 + CI 诊断/性能，b93e7a3）

- ✅ [P2] src/lib/stores/cache.ts:29,85 — CacheState.hourlySummaries 全仓零读零写
  死状态 — 删除 + TypeScriptMigration.contract.ts 契约 + cache.test.ts fixture 同步
- ✅ [P2] src/lib/stores/assistant.ts:366 — updateLastStreaming 生产零调用死 API
  且 findIndex 取第一条 streaming 消息与注释语义相悖（并发双流更新错消息）— 删除 +
  契约同步；updateMessageById 已有独立测试覆盖，两个死 API 测试随之移除
- ✅ [P2] .gitlab-ci.yml:186 — 健康检查失败诊断 `docker logs workreview-admin` 指向
  不存在的容器名（compose 部署实际为 yue-review-admin-1），排障误导 — 改 compose logs
- ✅ [P2] .gitlab-ci.yml:173 — `docker network create workreview-net` 死步骤
  （compose 未引用该网络）— 删除；:153 头注释同步为 compose 拓扑
- ✅ [P2] .gitlab-ci.yml:242 — cargo-llvm-cov 装进 $CARGO_HOME/bin（项目内）不在
  PATH：预装缺失时每条流水线重复整编且永不自愈（现状靠 runner 预装掩盖）—
  export PATH="$CARGO_HOME/bin:$PATH"
- ✅ [P2] scripts/ci/linux-package-inner.sh:5 — npm cache /build/.npmc 不在
  GIT_CLEAN_FLAGS 排除清单：每次发版 checkout 被清、全量重下依赖 — 改 /build/.npm
  （与其余作业一致的持久缓存）
- ✅ [P2] .gitlab-ci.yml:732 — ci-caches/linux22 死 mkdir（525df65 删除挂载后无
  消费者）— 删除并留注释

## 第七轮仓库级（需人工处置）

- ⚠️ [P1] tauri-updater.key（minisign 私钥，CI 签名变量的原始材料）自首次提交起
  被 git 跟踪。现仓库为私有自建 GitLab（GitHub 公开镜像经核实不含此文件、自建站
  raw 需登录），且私钥有口令加密，实际暴露面有限；但密钥材料入库违反密钥管理基线，
  任何读仓库权限扩散/GitLab 配置失误即升级为更新链投毒（全量安装端自动应用恶意
  更新）。轮换涉及：新钥签名 + 旧钥过渡发布 + 历史清除（filter-repo）+ CI 变量
  更新，会短暂影响更新链——需专项决策，本任务不动

## 第七轮待确认（修复会改对外行为/部署语义，未经确认不动）

- ⏸ [P1] admin-core/compose.yaml:31,36 — POSTGRES_PASSWORD 兜底 changeit-init-only
  且全仓无 WR_DB_PASSWORD 注入点：空卷首启即众所周知的口令；:31 ports 绑 0.0.0.0
  （15433），内网可直连存有 OA 账号+bcrypt 的库 — 改 127.0.0.1:15433 +
  ${WR_DB_PASSWORD:?} fail-fast；涉生产库当前初始化状态与部署流程，需确认
- ⏸ [P2] scripts/ci/has-code-changes.sh:17 — docs-only 白名单仅 src/*|src-tauri/*|crates/*，
  Cargo.lock/package.json 等根级影响产物的变更被判 docs-only 跳过发版（依赖安全修复
  不随发版到达用户）；窄白名单可能是 1c8be35 有意设计，需确认再补
- ⏸ [P2] .gitlab-ci.yml:270/310/140 — .release-compile（三平台全量编译）与 frontend
  job 的 rules 不排 schedule：夜间流水线白跑（唯一消费者 version-bump 仅 push）；
  build-release-macos 不在 macos-package resource_group，可与夜间 mac 包并行挤占
  唯一 mac runner — 补 schedule 排除/资源组改变夜间节奏，需确认
- ⏸ [P2] crates/mcp-server/src/main.rs:657 — filter_memory_results_by_privacy 的
  Anonymize 分支清 title/url 但不清 excerpt（search_activities 返回项的 OCR 派生
  片段原样外发）：与第六轮 ⏸ filter_activities_by_privacy ocr_text 同根因，
  建议同批决策
- ⏸ [P2] src-tauri/src/monitor.rs:240 — is_browser_internal_segment 的 tab_patterns
  含裸 "tab"/"tabs" contains（len<20）：含 "table"/"Tabs vs Spaces" 等的正常标题段
  被当内部标签计数过滤、窗口标题信息静默丢失 — 收紧为带数字计数形态会改变入库内容
- ⏸ [P2] src-tauri/src/commands/report.rs:98 — 自动导出文件名取自 date 参数仅替换
  `/\` 分隔符、无 ^\d{4}-\d{2}-\d{2}$ 白名单：本地 API/ActionBridge 传任意值时
  Windows 冒号可写 NTFS ADS（UI 恒传合法日期，可达性需核调用契约）
- ⏸ [P2] src-tauri/src/agent/model.rs:986 — 流式装配器按 tc["index"] 扩容
  partial_calls 无上限：自建中转返回极大 index 时内存放大（威胁模型是否含恶意端点）
- ⏸ [P2] src-tauri/src/bot_common.rs:355 — /reports 设备返回 JSON 无 dates 数组时
  return None，四个调用方兜底成"未知命令"误导排障 — 改明确异常文案即改 bot 回复
- ⏸ [P2] src-tauri/src/config_sync.rs:92 — pull_and_apply 的 synced_at 读取与 config
  克隆分两把锁，间隙内的本地保存可被更旧远端信封覆盖（与既有 ⏸ push 乱序同族的
  "最后写入获胜"设计弱点）
- ⏸ [P2] admin-core/compose.yaml:27 — postgres:16-alpine 直连 docker.io（该服务器
  已被实测限流，fresh-host 首次部署可能拉取失败；同机 linux-builder-22.Dockerfile
  已为此走 daocloud）
- ⏸ [P2] src-tauri/src/ocr.rs:335 — 超时 kill 后无条件 join() 排空读线程：孙进程
  继承管道句柄时永久阻塞（既有 ⏸ monitor.rs:95 同族未登记实例，归并同批处理）
- ⏸ [P2] crates/core/src/gitlab.rs:524 — GitlabClient 允许任意 scheme base_url：
  PRIVATE-TOKEN 可明文出网；私有化内网 http 是该功能中心用例，加 https 强制破坏
  存量内网用户（历轮 ⏸ validate_model_endpoint 同族口径）

## 第七轮待过目批次（.svelte，按红线待用户过目）

- 💤 [P2] src/routes/avatar/AvatarWindow.svelte:317 — 专注会话期间 focusNowMs 每秒
  tick 使 bubble 每秒换新对象身份，响应式块每秒触发 scheduleInteractiveRegionsSync
  （querySelectorAll+全量 getBoundingClientRect+set_avatar_interactive_regions IPC
  约 1500 次/25 分钟，几何变化本已由 ResizeObserver 覆盖）— 同步块改依赖结构信号
  或 invoke 前 diff
- 💤 [P2] src/App.svelte:241 — AvatarWindow 动态 import 无 .catch：chunk 加载失败
  （损坏安装/安全软件拦截）时 rejection 无人处理且模板无 else 分支，桌宠窗口永久
  空白无提示 — 补 .catch 记日志（占位渲染属视觉改动，随过目批次）

## 第七轮暂缓（💤）

- 💤 [P2] src-tauri/src/screenshot.rs:923 — 多屏拼接逐像素 put_pixel（8K 多屏约
  3300 万次/帧，仅 All 模式）可改 imageops::overlay 整块拷贝；输出应逐像素等价但
  无图像内容回归测试兜底，坐标错一位即花屏，不在本任务冒险

# 第六轮（2026-09-20）

范围：五轮之后增量（admin Dockerfile fetch 修复等 CI 提交）+ 全库六分片复扫——
src-tauri 核心 33 文件（main/monitor/localhost_api/commands 全部）、agent+bots、
crates 三库（database 7.9k 行 SQL 面、mcp-server、skills-engine）、前端纯 TS
（stores/utils/i18n/routes 逻辑层）、Svelte 组件（Timeline/Overview/Settings 全家）、
CI/scripts/tests。六分片各按 rust.md/typescript.md/cross-cutting 清单五维度检查并与
前五轮清单逐条去重。新发现 28 条（3 P1 + 25 P2）：17 条行为保持本轮修复（批次 1
Rust 3c513d0+af97fa4 / 批次 2 前端 c144aee / 批次 3 CI 5de05a3），7 条涉行为/平台
语义列待确认，4 条涉 .svelte 归过目批次。

## 第六轮已修复

### 批次 1（Rust，3c513d0 + 契约同步 af97fa4）

- ✅ [P1] src-tauri/src/agent/model.rs:306 — `chat_ollama`（非流式）请求 Ollama 原生
  `/api/chat`（顶层 `message`、无 `choices`）却用 `parse_openai_response` 解析：所有
  非流式 Ollama 调用（avatar_proactive 主动开口 / 流式失败回退 / executor 无事件分支）
  content/tool_calls 恒空，桌宠配 Ollama 时主动开口永久静默失效 — 新增
  `parse_ollama_response` 按原生格式解析（与流式装配器 `message.content` 口径一致）
  + 回归测试
- ✅ [P2] src-tauri/src/agent/model.rs:955 — OpenAI 兼容流式装配器把 reasoning_content
  增量无条件 `push_str` 进 content：思考型模型（DeepSeek-R1/Qwen3）最终回答=思维链+正文
  拼接并持久化，与非流式 parse_openai_response（仅正文为空时兜底）语义矛盾 — 思维链
  单独累积、finish 仅正文为空时兜底；流式增量照发；前端 ReasoningModelCompat 契约测试
  同步为验证分离累积语义
- ✅ [P1] crates/core/src/database.rs:3473/3530/3568 — 记忆搜索三个 FTS 面
  （search_activities_fts / search_hourly_fts / search_reports_fts）把 FTS5 rank
  （bm25 分数，REAL）按 `row.get::<_, i64>` 读取：遇小数 FromSql 失败、
  `filter_map(row.ok())` 静默丢光全部命中行——记忆搜索的 FTS 通道从未返回过结果，
  一直靠 LIKE fallback 兜底（仅 rank 恰为整数的退化情形偶尔工作）；本轮新增回归测试
  时实测揪出（GROUP BY 修复的验证测试先红于此）— rank 改按 f64 读取（调用方均未使用
  rank 值，排序在 SQL 内完成）
- ✅ [P2] crates/core/src/database.rs:3581 — search_reports_fts `GROUP BY d.date` 的
  content/ai_mode/rank 均为裸列（SQLite 取值未定义）：同日 zh-CN+en 双份日报双双命中时
  正文语言随机漂移 — 附加唯一 `MIN(d.created_at)` 聚合，SQLite 裸列规则保证取自最早行
  （多语言用户从"随机"变"确定"）+ 回归测试
- ✅ [P2] src-tauri/src/commands/system.rs:740 — macOS 图标转换 sips 输出仍硬编码共享
  `/tmp/app_icon_{app}_{pid}.png`（第三轮只改了缓存目录）：可预测路径多用户机可被预置
  符号链接写穿 — 改 env::temp_dir + 纳秒随机后缀 + 文件名字符净化
- ✅ [P2] src-tauri/src/commands/system.rs:1339 — get_background_image 对
  config.background_image 直接 data_dir.join 后读文件回 base64（截图读取/删除均已
  validated_join，此处漏网；config 同步载荷可写入绝对路径/`..` 越界读）— 复用
  validated_join
- ✅ [P2] src-tauri/src/monitor.rs:4406 — get_overlay_windows 用 dedup_by 去重同名应用，
  但该 API 只折叠相邻元素：CGWindowList 按 z 序、同应用双浮窗被其他应用隔开时失效，
  main.rs 浮窗记账对同 tick 第二条浮窗再次合并 → PiP 时长双倍记账 — 改 app_name
  全局保序去重
- ✅ [P2] src-tauri/src/node_gateway.rs:105 — 设备身份写入临时名固定 with_extension("tmp")：
  首启两个状态查询并发 ensure 时 Unix 后写者覆盖先写者（两调用方 device_id 不一致）、
  Windows rename 目标已存在直接失败 — 临时名加 pid+纳秒，rename 冲突回退采用已落盘身份
- ✅ [P2] crates/core/src/gitlab.rs:504 — entries_to_rows 的 author_email 未 strip：
  fetch 侧 strip_control_chars 只覆盖 title/author_name/project/branch，入库行单行化
  不变量漏该字段（被劫持服务端可注入 `\n## ` 伪造小节）— 入库前补同一清洗（合法邮箱
  恒等）+ 回归测试

### 批次 2（前端 TS + e2e，c144aee）

- ✅ [P2] src/routes/report/reportSections.ts:136 — sectionNumberPrefix 只特判 `en`，
  ar locale 落入中文数字分支（阿语界面看到"一、二、"）— 改 CJK 白名单（zh-CN/zh-TW），
  其余语言西文数字（zh 字节不变）
- ✅ [P2] src/lib/stores/assistant.ts:331 — setMessages 残留 slice(-40) 截断（第三轮
  从 appendMessage 修掉的同类语义在死 API 上残留，生产零调用；未来调用方装入超 40 条
  会话即静默丢历史）— 去截断
- ✅ [P2] src/lib/stores/iconCache.ts:251,257 — 每个图标完成（成功/失败两分支）都同步
  persistIconCache()：整包序列化+localStorage 同步写（单项可达上百 KB base64），批量
  预热时主线程卡顿累积 — 改 100ms 节流（与 updateIconStore 同款）；测试同步为等待
  首次落盘
- ✅ [P2] tests/e2e/ui/tauri-mock.ts:108 — baseSpec 对象重复键 get_ai_providers
  （108 行 `[]` 死代码，运行时 last-wins 取 116 行）— 删除死键
- ✅ [P2] tests/e2e/installer-isolation/run.ps1:66,49 — curl.exe 缺 `-f`（404 时 HTML
  错误页写进 zip，错误延迟到 Expand-Archive 晦涩暴露）；超时报错拼接用错变量 `$Args`
  （恒空，丢失完整安装参数）— 补 `-f` + 改 `$ArgList`

### 批次 3（CI，5de05a3）

- ✅ [P1] .gitlab-ci.yml:720 — GLIBC 兼容门禁放 after_script：GitLab 不以 after_script
  退出码判定作业成败（gitlab-org/gitlab#21008），check-linux-glibc.sh 的 exit 1 永不
  生效——产物 GLIBC 超限时仅打印 ❌ 作业仍绿，不兼容包照常流入 publish/发布 — 移入
  .tauri-package-linux script 末尾（artifacts when:always 仍收集产物便于排查）
- ✅ [P2] .gitlab-ci.yml:892 — publish-release 补签循环 rawkey（base64 解码后的
  minisign 私钥）只在成功路径 rm：set -e 下 minisign 失败即带私钥残留共享 runner
  /tmp（与第三/五轮 stage-tauri-key/setup-codesign 同族清理缺口）— EXIT trap 兜底
- ✅ [P2] scripts/deb/reinstall.sh:363 — 第 363 行 `trap 'rm -rf "$TMP_FWD"' EXIT`
  整体覆盖第 195 行 `trap 'rm -f "$DEB"' EXIT`（bash 后设 trap 替换先设）：全新机器
  走下载路径时下载的 .deb 退出不再清理 — 合并为 register_cleanup/on_exit_cleanup
  注册表统一清理

## 第六轮待确认（修复会改对外行为/平台语义，未经确认不动）

- ⏸ [P1] crates/mcp-server/src/main.rs:612-627 — filter_activities_by_privacy 的
  Anonymize 分支清空 title/url/path/screenshot 但不清 ocr_text：用户标记"内容脱敏"
  的活动经 query_timeline/get_work_sessions/analyze_intents/历史回退把整段 OCR 屏幕
  文本（正是脱敏要挡的内容）原样发给 MCP 客户端 — Anonymize 分支补 `ocr_text = None`；
  改变 MCP 工具对既有会话的返回内容（ocr_text 从有值变 null），建议与既有 ⏸
  localhost live 上下文绕过同批决策
- ⏸ [P2] crates/core/src/config.rs:500-526 — get_app_privacy_level 的 contains 匹配
  无最小长度护栏：1-2 字符隐私规则（手滑输入 "a"/"w"）命中几乎所有应用，采集面被
  静默 Ignored/Anonymized（统计数据无声丢失）；分类规则侧同族匹配有 len>=3 护栏此处
  独缺 — 补护栏会缩小存量短规则的命中范围，待确认长度取 2 还是 3
- ⏸ [P2] src-tauri/src/localhost_api.rs:832 — /v1/reports/generate 直调
  generate_report_inner，绕过 generating_report 并发标志（该标志只在 Tauri 命令包装
  与 ActionBridge 检查）：UI 生成中经本地 API/MCP 再触发会并发跑两个 AI 日报
  （双倍模型调用、后写覆盖先写）— 守卫下沉 inner 后，并发 HTTP 调用从"重复执行"变
  400「正在生成中」，调用方可见契约变化待确认
- ⏸ [P2] src/App.svelte:707-762 — 日报自动生成/工作记忆合成失败后 lastAutoGenDate
  不写：端点配错/停服的持久性失败会在每天工作结束时间后每分钟重试整轮 AI 调用
  （整夜捶打失败端点+反复 warn 日志）— 加指数退避或当日失败上限改变"直到成功为止"
  的重试语义（涉 .svelte），待确认
- ⏸ [P2] src-tauri/src/avatar_input.rs:1811-1844 — Linux Wayland 输入联动线程每
  120ms spawn 短命 gdbus/kdotool/hyprctl 进程 + 每次成功采样重查全部显示器（~8 次/秒
  常驻开销，与既有 💤 idle_detector.rs:177 同族但频率翻倍且永不停止）— 降频/缓存
  改变跟手延迟与热插拔新鲜度，平台路径无法本地自动化验证，随 spawn 战役一并设计
- ⏸ [P2] .gitlab-ci.yml:163-164 — deploy-admin-core-yx 规则 `CI_COMMIT_BRANCH=="main"`
  同样命中夜间 schedule（schedule 配置在 main 分支）：fork 无变更的夜晚也会重建镜像
  并重启生产 admin 容器 — 补 `$CI_PIPELINE_SOURCE=="push"` 改变现有部署节奏，待确认
  现状是否有意设计
- ⏸ [P2] .gitlab-ci.yml:754-782+924 — 便携 zip 由 after_script 产出（失败不影响作业
  状态，zip 可缺）、文件名无版本号且残留旧版 zip 会被 publish 照常上传挂到新版
  release（generate/verify 对无版本号文件名放行）— 加新鲜度/存在性断言是发布收紧，
  待确认（可先只加告警）

## 第六轮待过目批次（.svelte，按红线待用户过目）

- 💤 [P2] src/routes/settings/components/SettingsAI.svelte:669 — refreshModels 无
  requestId/provider 快照防竞态：fetch 在途时切换 provider，旧 provider 慢响应后到
  覆盖新 provider 的 fetchedModels（模型下拉显示错误服务商列表）— 参考 Ask.svelte
  refreshDynamicPrompts 的 requestId 写法
- 💤 [P2] src/routes/timeline/Timeline.svelte:2066 — 隐私级别说明 class 查找写成
  "单元素数组包裹查找表再取 [0]"，每次渲染新建对象（复制粘贴残留）— 抽 const 查找表
- 💤 [P2] src/lib/components/StatsCard.svelte:57-116 — href/非 href 两分支各自完整
  复制同一套 4 图标 SVG 模板块（约 60 行逐字重复，已现漂移风险）— 包裹共享内容或
  抽 snippet
- 💤 [P2] src/routes/avatar/AvatarWindow.svelte:993 — onMoved 每次触发无条件
  refreshBubbleEdge（每秒数十次 × 2 个 IPC；scheduleAvatarPositionSave 有 240ms 防抖
  而它没有；+138 中心近似在跟进卡片展开态偏移）— 合并防抖 + 跨线判定才写

## 第六轮复核无新问题（抽样说明）

- src-tauri 核心：SQL 全参数化、日期均 NaiveDate 白名单、validated_join 三面在位
  （唯 background_image 缺口已修）、localhost token 常量时间比较、PowerShell/AS
  内嵌值转义正确、WGC/UIA 票据熔断无竞态、GDI/HICON/DIB 逐对释放、OCR 信号量有界
- agent/bots：SSRF 护栏（IPv4-mapped/CGNAT/组播/内网 TLD）与重定向逐跳校验在位、
  五 bot 验签/时间窗/ct_eq/redact 完整、Deadline 贯通与取消三路收束完备、字节切片
  解析均由 ASCII find 推导无 UTF-8 边界 panic、生产代码无裸 unwrap
- crates：FTS build_fts_query 引号转义闭合、VACUUM INTO 转义在位、语义记忆游标独立行
  与指纹门禁完好、config 原子写+0600+恢复矩阵有测试、DST/时区翻转正确；daily_reports
  遗留死表确认无生产写入方（删除需迁移批次）
- 前端 TS：XSS 面无新增（3 处 {@html} 均 marked→DOMPurify 闭合）、requestEventGate/
  requestId 竞态守卫完整、四语言 1359 key 零缺失、focusTrap/updater 监听器配对释放
- Svelte 组件：定时器/监听器/订阅清理完整、Timeline/Overview/Report 异步竞态双守卫
  在位、空态守卫完整、第五轮过目批次条目在现行号下原样未动
- CI/scripts：verify→compile→version 门禁链、publish 版本化清理/stale-sig 判定、
  bump-version 三轮修复完好、admin Dockerfile fetch-by-SHA+回退与 viewport 守卫在位、
  CI 契约测试 18/18 通过无漂移

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

### 批次 3（推送后流水线监控发现，commit 见 git log）

- ✅ [P1] admin-core/Dockerfile:25,44 — 27ba788 引入的 `git clone --depth 1 --branch
  <裸SHA>` 必现失败：--branch 只接受分支/标签名，报 "Remote branch not found"——
  2598/2604 两条 main 流水线的 deploy-admin-core-yx 均死于此（version-bump 因
  when: always 照常 bump，掩盖了红流水线）。修法 = `git init` + `git fetch --depth 1
  origin <SHA>` + `checkout FETCH_HEAD`（fetch-by-SHA 在本自托管 GitLab 实测可用，
  服务端 allowAnySHA1InWant 开启）；上游 gitee 未开该配置时回退 fetch master，
  兼容 Dockerfile 默认参数的本地构建。

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
