# E2E 测试报告（时间线「今日项目」归因卡，2026-09-29）

- 运行环境：`npx playwright test tests/e2e/ui/timeline-projects-card.spec.ts`，msedge，headless 自测两轮（有头待用户确认）；vite web 模式 127.0.0.1:5177 + tauri-mock IPC（get/set_daily_project_adjustment 在 init script 内维护可变状态驱动真实交互闭环）
- 结果统计：总 4 条 | 通过 4 | 失败 0 | 跳过 0 | ~5s/轮；全套件 20/20 两轮全绿

## 过程记录（无头自测阶段发现并修复的问题）

| 轮次 | 现象 | 归因 | 处置 |
| --- | --- | --- | --- |
| 1 | 卡不渲染（活动为空的场景） | 实现缺陷：挂载点落在 `activities.length === 0` 空态的 else 分支内，空态时整卡不渲染 | 挂载点上移为页面级（loading/error/empty 各态均显示） |
| 2 | 全量跑时 8 条既有时间线用例挂、整页断言超时 | 实现缺陷：旧 spec 的 mock 表无新命令 → invoke resolve null → `planeProjects = null` 流入 `$:` 派生表达式 → `null.filter` 抛 TypeError 破坏整页渲染 | 组件对 invoke 结果统一 `?? []` 兜底（真实环境命令缺失同场景也受益） |
| 2 | builtin-indicator 4 条在冷启动后必挂（此前走运绿） | 既有测试缺陷：缺登录种子，依赖 onMount 前完成断言的时序竞态 | 补 auth 种子（与其余 spec 对齐） |

## 用例结果

| 编号 | 用例 | 结果 |
| --- | --- | --- |
| E2E-001 | 归因聚合出两枚 auto chip（蓝色）且卡可见 | ✅ |
| E2E-002 | 添加项目：调 set(removed=false) 且紫色 manual chip 出现 | ✅ |
| E2E-003 | 删除 auto 项目：调 set(removed=true) 且 chip 消失、其他 chip 不受影响 | ✅ |
| E2E-004 | 未配置 Plane（下拉源空且无归因）时整卡隐藏 | ✅ |

## 覆盖评估

- 计划 4 条全部实现并执行；替换编辑（改）交互与删除共用 set 命令链路，由 E2E-002/003 组合覆盖。
- 回归：全套件 20 条（含既有 16 条）两轮全绿；顺带修复既有 builtin-indicator 冷启动 flaky。

## 稳定性评估

- 两轮全绿（热 server 7-8s/轮）；冷启动首跑慢但不影响结果（null 兜底修复后）。

## 结论

- 套件就绪，纳入回归清单。无未决产品缺陷。
