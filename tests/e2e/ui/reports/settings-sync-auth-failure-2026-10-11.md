## E2E 测试报告（同步卡认证失败处理，2026-10-11）
- 运行环境：`E2E_CHANNEL=chrome npx playwright test tests/e2e/ui/settings-sync-auth-failure.spec.ts`（headless，系统 Chrome；vite preview:5177 复用——vite dev 因 inotify ENOSPC 不可用，既有先例绕法）
- 结果统计：总 5 条 | 通过 5 | 失败 0 | 跳过 0 | 耗时 ~20s；复跑一轮仍 5/5，无 flaky
- 失败分析（逐条）：无
- 过程中的非产品失败（已解决）：
  | 现象 | 归因 | 处置 |
  |------|------|------|
  | 首轮 5 条全挂：Chromium distribution 'msedge' not found | 环境问题：本机无 Edge（CI 也未接入 e2e） | playwright.config.ts channel 支持 `E2E_CHANNEL` 覆盖，本机走 `chrome` |
  | 二轮 001–003 挂：等「云端配置」标签超时 | 测试脚本缺陷：vite preview 服务的是模板重构前的旧 dist | 重新 `npm run build` 后全绿（页面快照证实修复行为本身已正确渲染） |
- 覆盖评估：计划 5 条全部实现——P0 修复路径（旧版 200+success:false）×1、401 新语义 ×1、上传动作防假成功 ×1、正常有配置/空配置回归 ×2；拉取差异弹窗与真实服务端联调不在范围（源码结构断言与纯函数单测已覆盖，且无真实管理端可依赖）
- 稳定性评估：两轮全绿；mock 为纯静态表（addInitScript），无网络/时序依赖
- 结论：达到通过标准；套件就绪
