# 全量代码评审问题清单与处置结果（2026-09-15 第一轮 / 2026-09-16 第二轮）

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
