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
| 所属团队 | 越建科 |
| 团队角色 | 编辑者（can_edit=true，非 owner/admin） |
| Profile ID | f8c58da1-01ce-4400-8758-d89d79bc0620 |
| 团队 ID | 20bfdff4-2184-800d-8008-bdadb7bcab69 |
| 创建日期 | 2026-10-08 |

## 使用说明

- 密码为该账号独立生成的随机密码（与主账号无关）。如需修改：登录后在
  Settings → Change password 修改，并同步更新本文件。
- 本文件包含敏感凭据，仅限私有仓库内部使用，勿同步到公开渠道。

## 验证记录（2026-10-08）

- `POST /api/main/methods/login-with-password` → 200，签发 `auth-token` 会话 Cookie；
- `GET /api/main/methods/get-teams`（携带会话）→ 返回「越建科」团队，权限
  `is-owner=false / is-admin=false / can-edit=true`，即编辑者。
