//! 客户端登录/注册的 HTTP 代理命令。
//!
//! 背景：WebView 页面源在 Windows 是 `https://tauri.localhost`、Linux 是 `tauri://localhost`
//! （wry 注册为 secure），均为安全上下文——页面 fetch 纯 HTTP 内网地址属混合内容，会被
//! 渲染层直接拒绝且零网络请求（v1.1.81 Windows、v1.2.22 Linux 两次复现，详见知识库
//! tauri-https-scheme-mixed-content 笔记）。各平台的豁免手段互不通用（Windows 有
//! additionalBrowserArgs，WebKitGTK 没有）。登录/注册由本命令在 Rust 侧发请求，
//! 完全绕开 WebView 网络栈：任意 http/https 服务器地址全平台可用，CSP 白名单也随之
//! 失去约束力（fetch 兜底路径仍受限，保留白名单以兼容浏览器预览）。

use serde::Serialize;
use serde_json::Value;

/// 登录/注册请求的统一超时：与前端原 fetch 兜底的 AbortSignal.timeout(15s) 对齐
const REQUEST_TIMEOUT_SECS: u64 = 15;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientHttpPostJsonResult {
    /// 网络层是否成功送达并收到 HTTP 响应；false = 连接失败/超时/域名解析失败
    pub delivered: bool,
    pub status: Option<u16>,
    /// 响应体文本（delivered 且可读时）
    pub body: Option<String>,
    /// 网络层错误描述（delivered=false 时）
    pub error: Option<String>,
}

impl ClientHttpPostJsonResult {
    fn network_failure(error: String) -> Self {
        Self {
            delivered: false,
            status: None,
            body: None,
            error: Some(error),
        }
    }
}

/// 在 Rust 侧发起 POST JSON 请求（当前仅客户端登录/注册使用）。
///
/// 不在此处解析业务响应：保持纯 HTTP 透传，JSON 解包沿用前端既有逻辑，
/// 避免两端各持一份 Admin.Core 包装结构的映射。
#[tauri::command]
pub async fn client_http_post_json(
    url: String,
    body: Value,
) -> Result<ClientHttpPostJsonResult, String> {
    if url.trim().is_empty() {
        return Ok(ClientHttpPostJsonResult::network_failure(
            "服务器地址为空".to_string(),
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {e}"))?;

    let send = async { client.post(&url).json(&body).send().await };
    let response = match tokio::time::timeout(
        std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS + 1),
        send,
    )
    .await
    {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => return Ok(ClientHttpPostJsonResult::network_failure(e.to_string())),
        Err(_) => {
            return Ok(ClientHttpPostJsonResult::network_failure(format!(
                "连接超时（{REQUEST_TIMEOUT_SECS} 秒）"
            )))
        }
    };

    let status = response.status().as_u16();
    match response.text().await {
        Ok(text) => Ok(ClientHttpPostJsonResult {
            delivered: true,
            status: Some(status),
            body: Some(text),
            error: None,
        }),
        Err(e) => Ok(ClientHttpPostJsonResult {
            delivered: true,
            status: Some(status),
            body: None,
            error: Some(format!("读取响应失败: {e}")),
        }),
    }
}
