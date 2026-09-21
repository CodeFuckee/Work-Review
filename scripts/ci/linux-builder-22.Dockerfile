# =============================================================================
# work-review · Linux 打包构建器（Ubuntu 22.04 基线）
#
# 为什么存在：linux-x64 runner 主机是 Ubuntu 24.04（glibc 2.39），直接构建的
# 产物要求 GLIBC_2.39，超出发布基线 2.35（Ubuntu 22.04，GLIBC 兼容门禁拦截，
# v1.1.74/75 发布失败根因）。本镜像把工具链钉回 22.04，产物恢复 ≤2.35 兼容。
#
# 一次性预构建（在 yx runner 上执行，镜像持久复用）：
#   docker build -t wr-linux-builder:22 -f scripts/ci/linux-builder-22.Dockerfile scripts/ci
#
# 使用（.gitlab-ci.yml 的 .tauri-package-linux）：
#   docker run --rm --user "$(id -u):$(id -g)" \
#     -e TAURI_SIGNING_PRIVATE_KEY -e TAURI_SIGNING_PRIVATE_KEY_PASSWORD \
#     -v "$CI_PROJECT_DIR":/build -w /build \
#     -v /home/mdc/workreview/ci-caches/linux22/cargo:/usr/local/cargo \
#     -v /home/mdc/workreview/ci-caches/linux22/rustup:/usr/local/rustup \
#     wr-linux-builder:22 bash scripts/ci/linux-package-inner.sh
# =============================================================================

# docker.io 对该服务器限流，基镜像走 daocloud 镜像源
FROM docker.m.daocloud.io/library/ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=Asia/Shanghai

# webkit2gtk-4.1 与 Tauri v2 Linux 构建依赖（jammy 官方源即可满足）
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl wget file git gnupg build-essential pkg-config \
        libwebkit2gtk-4.1-dev libssl-dev libxdo-dev \
        libayatana-appindicator3-dev librsvg2-dev \
        xz-utils jq \
    && rm -rf /var/lib/apt/lists/*

# Node.js 22（NodeSource）：对齐 package.json engines >=22 与其余 CI 作业，
# 保证用户拿到的安装包与 verify 流水线测过的前端构建出自同一工具链
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && node --version

# Rust 工具链（与 CI 钉版一致 1.97.1）；cargo/rustup 目录在运行时挂载持久缓存
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/local/node/bin:$PATH
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.97.1 --profile minimal \
    && rustc --version
