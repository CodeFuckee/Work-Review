# =============================================================================
# work-review 管理后台 · Admin.Core 一体化镜像（.NET API + admin.ui 静态资源）
#
# 模式沿用 bim5-d-lightweight-platform 的 admin-core/Dockerfile（2026-09-15/16
# 在 yx runner POC 与实测通过），本实例为 work-review 专用部署：
#   - 品牌化：工作回顾管理后台
#   - 生产配置经 -v 挂载 ConfigCenter 覆盖（密钥不进镜像）
#   - PG provider 换装 + SPA 回退补丁 + pnpm 钉 9.15.9（POC-2/4/5 结论）
#
# 构建：docker build -t workreview-admin:latest admin-core/
# 运行：docker run -d --name workreview-admin -p 4191:18010 \
#         -v $HOME/workreview/admin-config:/app/ConfigCenter workreview-admin:latest
# =============================================================================

# 默认上游；work-review CI 以 build-arg 指向自管 fork（含业务模块），token 经 CI 变量注入
ARG ADMIN_CORE_REPO=https://gitee.com/zhontai/Admin.Core.git
ARG ADMIN_CORE_REF=master
ARG NODE_IMAGE=node:20-alpine

# ---------- 阶段 1：构建 .NET Host ----------
FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build
ARG ADMIN_CORE_REPO
ARG ADMIN_CORE_REF
ARG NUGET_SOURCE=https://api.nuget.org/v3/index.json
RUN git clone --depth 1 --branch "${ADMIN_CORE_REF}" "${ADMIN_CORE_REPO}" /src/Admin.Core
# SPA 回退补丁：history 路由深链接（/login 等）回退 index.html（fork 已含补丁时幂等跳过）
COPY spa-fallback.patch /tmp/spa-fallback.patch
RUN cd /src/Admin.Core && (git apply /tmp/spa-fallback.patch 2>/dev/null || true) \
    && grep -q "SPA fallback" src/modules/admin/ZhonTai.Admin.Core/HostApp.cs
# 启用 PostgreSQL provider（上游默认只启用 Sqlite+QuestDb）
RUN cd /src/Admin.Core/src/modules/admin/ZhonTai.Admin.Host \
    && sed -i 's|<PackageReference Include="FreeSql.Provider.Sqlite" Version="3.5.311" />|<PackageReference Include="FreeSql.Provider.PostgreSQL" Version="3.5.311" />|' ZhonTai.Admin.Host.csproj \
    && grep -q 'FreeSql.Provider.PostgreSQL" Version="3.5.311"' ZhonTai.Admin.Host.csproj
WORKDIR /src/Admin.Core/src
RUN dotnet nuget add source "${NUGET_SOURCE}" -n override >/dev/null 2>&1 || true \
    && dotnet publish modules/admin/ZhonTai.Admin.Host/ZhonTai.Admin.Host.csproj \
       -c Release -o /publish

# ---------- 阶段 2：构建 admin.ui（Vue3 + Element Plus，pnpm） ----------
FROM ${NODE_IMAGE} AS ui
ARG ADMIN_CORE_REPO
ARG ADMIN_CORE_REF
RUN apk add --no-cache git \
    && git clone --depth 1 --branch "${ADMIN_CORE_REF}" "${ADMIN_CORE_REPO}" /src/Admin.Core \
    && rm -f /src/Admin.Core/pnpm-workspace.yaml
# 根级 pnpm-workspace.yaml 会让子目录 pnpm install 报 packages field missing（上游新增），删除之
WORKDIR /src/Admin.Core/ui/zhontai.ui.admin.vue3
# pnpm 钉 9.x：corepack 默认拉 pnpm 12，其 supply-chain policy 与上游锁文件不兼容
# 品牌化（工作回顾管理后台）：侧栏/登录页标题 + 浏览器标题与 meta + 页脚
RUN npm i -g pnpm@9.15.9 \
    && pnpm install --frozen-lockfile \
    && sed -i "s|globalTitle: '中台Admin'|globalTitle: '工作回顾管理后台'|" src/stores/themeConfig.ts \
    && sed -i -e "s|globalViceTitle: '中台Admin'|globalViceTitle: '工作回顾'|" \
              -e "s|globalViceTitleMsg: '后台权限管理框架'|globalViceTitleMsg: '工作回顾集中管理平台'|" src/stores/themeConfig.ts \
    && sed -i -e "s|t('中台Admin')|t('工作回顾管理后台')|" \
              -e "s|Copyright © 2022 中台Admin All rights reserved.|Copyright © 2026 工作回顾 All rights reserved.|" src/layout/footer/index.vue \
    && sed -i -e 's|<title>中台Admin</title>|<title>工作回顾管理后台</title>|' \
              -e 's|content="中台Admin"|content="工作回顾管理后台"|g' index.html \
    && grep -q "工作回顾管理后台" src/stores/themeConfig.ts index.html src/layout/footer/index.vue \
    && npm run build \
    && sed -i 's|"VITE_API_URL": *"[^"]*"|"VITE_API_URL": ""|' dist/env.config.js \
    && grep -q '"VITE_API_URL": ""' dist/env.config.js \
    && ! grep -q "中台Admin" dist/index.html dist/env.config.js \
    && grep -q "工作回顾管理后台" dist/index.html

# ---------- 阶段 3：运行镜像 ----------
FROM mcr.microsoft.com/dotnet/aspnet:10.0
WORKDIR /app
ENV TZ=Asia/Shanghai
# 生产配置（dbconfig/jwtconfig/appconfig）经 -v 挂载覆盖 /app/ConfigCenter；
# dbconfig 需同时换主库与 Log 库两条 connectionString 为 PG 串（POC-2 教训）。
COPY --from=build /publish .
COPY --from=ui /src/Admin.Core/ui/zhontai.ui.admin.vue3/dist ./wwwroot
EXPOSE 18010
ENTRYPOINT ["dotnet", "ZhonTai.Admin.Host.dll"]
