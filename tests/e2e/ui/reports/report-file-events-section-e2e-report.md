# E2E 测试报告（日报「文件修改」小节，2026-09-29）

- 运行环境：`npx playwright test tests/e2e/ui/report-file-events-section.spec.ts`，msedge，headless 自测（有头待用户确认）；vite web 模式 127.0.0.1:5177 + tauri-mock IPC（mock `get_saved_report` 返回带 WR_BLOCK 标记的日报 markdown，无真实后端）
- 结果统计：总 4 条 | 通过 4 | 失败 0 | 跳过 0 | 耗时 ~5s（含既有 5 条归因套件一起回归 9/9 全绿）

## 过程记录（无头自测阶段发现并修复的问题）

| 轮次 | 现象 | 归因 | 处置 |
| --- | --- | --- | --- |
| 1 | E2E-004 点「隐藏此段」后小节不消失、无任何报错 | 测试脚本缺陷：mock 的 get_config 缺 `daily_report_custom_prompt/prompt_presets/hidden_blocks/export_dir` 字段，`isReportConfig` 校验失败 → config=null → 钉选/隐藏处理函数静默 return（用 invoke 调用记录探针定位：`set_report_block_preference` 从未被调用） | 补齐 get_config mock 形状 |
| 2 | 4/4 全绿 | — | 两轮稳定 |

## 用例结果

| 编号 | 用例 | 结果 |
| --- | --- | --- |
| E2E-001 | 日报含文件块：渲染出「文件修改」标题 + 统计行 + 目录 Top 行 + 其余合并行 | ✅ |
| E2E-002 | 文件修改小节显示在 GitLab 今日提交之后（DOM 顺序断言） | ✅ |
| E2E-003 | 日报无文件块：小节不出现（空态隐藏） | ✅ |
| E2E-004 | 隐藏小节 → 「生成设置」抽屉恢复列表出现「文件修改」条目 → 点击恢复回正文 | ✅ |

## 覆盖评估

- 计划 4 条全部实现并执行；不测项（后端聚合逻辑）由 `crates/core/src/file_events_report.rs` 7 条单测覆盖。
- 顺带回归：既有时间线归因套件 5 条全过。

## 稳定性评估

- 连续两轮全绿，用例独立（各自装 mock），无等待竞态。

## 结论

- 套件就绪，纳入回归清单（`npx playwright test tests/e2e/ui/`）。
