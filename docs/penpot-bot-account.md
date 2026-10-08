# Penpot 机器人账号（work-review）

私有化 Penpot 实例为本项目创建的机器人账号，用于 MCP 插件通道「bot 账号分道」等
自动化设计操作场景（配方见个人知识库《自托管 Penpot MCP 接入》：每项目一个 bot 账号，
独立 Chrome profile 登录后打开项目文件并连接 MCP 插件）。

## 账号信息

| 项 | 值 |
| --- | --- |
| 实例地址 | https://home.chenkaidi.top:4443/ |
| 邮箱（登录名） | work-review-bot@chenkaidi.top |
| 密码 | `dwBZkRGd2gI6JwAzfe3Pxk` |
| 显示名 | work-review Bot |
| 所属团队 | 越建科（编辑者）+ 个人 Default 团队（owner，2026-10-08 对齐补建） |
| 团队角色 | 越建科：编辑者（can_edit=true，非 owner/admin） |
| Profile ID | f8c58da1-01ce-4400-8758-d89d79bc0620 |
| 团队 ID | 越建科 20bfdff4-2184-800d-8008-bdadb7bcab69；个人 Default 团队 8269da40-26de-4a99-8f82-4ea2b7ad61b7 |
| 创建日期 | 2026-10-08 |

## 使用说明

- 密码为该账号独立生成的随机密码（与主账号无关）。如需修改：登录后在
  Settings → Change password 修改，并同步更新本文件。
- 本文件包含敏感凭据，仅限私有仓库内部使用，勿同步到公开渠道。

## 专用 MCP 入口（2026-10-08 已配置）

- 项目根 `.mcp.json` 配置了 `penpot-work-review` 入口（Streamable HTTP，
  `https://home.chenkaidi.top:4443/mcp/stream?userToken=…`，token 为本账号在
  Integrations → MCP Server 生成的密钥）。`.mcp.json` 已加入 `.gitignore`
  （token 不入库，与本机其他项目惯例一致）；token 轮换时在 Penpot 设置页重新
  生成并更新本地 `.mcp.json`。
- 生效前提：work-review 专用 Chrome（启动台 work-review 入口）以本账号登录并
  打开目标文件、MCP 插件保持连接；MCP 入口在 ZCode 会话启动时加载，新会话生效。

## 验证记录（2026-10-08）

- `POST /api/main/methods/login-with-password` → 200，签发 `auth-token` 会话 Cookie；
- `GET /api/main/methods/get-teams`（携带会话）→ 越建科：`can-edit=true`（编辑者）；
  Default 团队对齐后复测：双团队（Default is-default=true/owner/admin/editor +
  越建科 editor），结构与主账号一致。
