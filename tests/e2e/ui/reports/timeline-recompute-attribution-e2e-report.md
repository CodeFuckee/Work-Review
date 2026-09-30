# E2E 测试报告（今日项目卡手动归因按钮，2026-09-30）

- 运行环境：`npx playwright test tests/e2e/ui/timeline-recompute-attribution.spec.ts`，msedge，headless 自测两轮（有头待用户确认）；vite web 模式 127.0.0.1:5177 + tauri-mock IPC（plane_attributions_for_date 动态可变 mock：首调空 / filled / failed / pending 四模式驱动真实交互闭环）
- 结果统计：总 3 条 | 通过 3 | 失败 0 | 跳过 0 | 8.8~22s/轮；全套件 23 条回归 23/23 全绿

## 用例结果

| 编号 | 用例 | 结果 |
| --- | --- | --- |
| E2E-001 | 点击重发归因请求（invoke 计数 +1）且新归因落到卡上（auto chip 出现） | ✅ |
| E2E-002 | 计算中按钮禁用且 spinner 出现；mock 放行后恢复可用 | ✅ |
| E2E-003 | 归因失败（ok:false）时失败提示行出现且 4 秒自动消失 | ✅ |

## 覆盖评估

- 计划 3 条全部实现并执行；成功/进行中/失败三态全覆盖。
- 回归：全套件 23 条（既有 20 + 本次 3）全绿。

## 稳定性评估

- 两轮全绿；动态 mock 经 addInitScript 参数传递（无闭包依赖），用例独立无竞态。

## 结论

- 套件就绪，纳入回归清单。无产品缺陷。
