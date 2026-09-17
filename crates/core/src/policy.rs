use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// 权限级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ReadActivities,
    ReadReports,
    ReadStats,
    ReadSessions,
    ReadConfig,
    WriteReport,
    WriteConfig,
    ExecuteAi,
    ExecuteSkill,
    ReadDeviceStatus,
}

/// 调用来源
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallSource {
    /// MCP Tool 调用
    McpTool {
        tool_name: String,
        client_id: Option<String>,
    },
    /// Skill 执行
    SkillExecution { skill_id: String },
    /// Tauri 前端调用
    Frontend { route: Option<String> },
    /// Localhost API 调用
    LocalhostApi { endpoint: String },
}

/// 策略决策
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyDecision {
    /// 允许
    Allow,
    /// 脱敏后允许
    AllowSanitized,
    /// 拒绝
    Deny,
}

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: u64,
    pub timestamp: u64,
    pub source: CallSource,
    pub permission: Permission,
    pub decision: PolicyDecision,
    pub denied_reason: Option<String>,
    pub duration_ms: u64,
}

static AUDIT_COUNTER: AtomicU64 = AtomicU64::new(1);

impl AuditEntry {
    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn new(
        source: CallSource,
        permission: Permission,
        decision: PolicyDecision,
        denied_reason: Option<String>,
        duration_ms: u64,
    ) -> Self {
        Self {
            id: AUDIT_COUNTER.fetch_add(1, Ordering::Relaxed),
            timestamp: Self::now_secs(),
            source,
            permission,
            decision,
            denied_reason,
            duration_ms,
        }
    }
}

/// 策略执行器
pub struct PolicyEnforcer {
    /// 审计日志（内存，最近 N 条）
    audit_log: Vec<AuditEntry>,
    max_audit_entries: usize,
    /// MCP 客户端权限白名单
    mcp_permissions: Vec<Permission>,
    /// Localhost API 权限
    localhost_permissions: Vec<Permission>,
    /// Skill 权限映射
    skill_permissions: HashMap<String, Vec<Permission>>,
}

impl PolicyEnforcer {
    pub fn new(config: &AppConfig) -> Self {
        let mcp_permissions = vec![
            Permission::ReadActivities,
            Permission::ReadReports,
            Permission::ReadStats,
            Permission::ReadSessions,
            Permission::ReadConfig,
            Permission::WriteReport,
            Permission::ExecuteAi,
            Permission::ExecuteSkill,
            Permission::ReadDeviceStatus,
        ];

        let localhost_permissions = if config.localhost_api_enabled {
            vec![
                Permission::ReadActivities,
                Permission::ReadReports,
                Permission::ReadStats,
                Permission::ReadSessions,
                Permission::ReadDeviceStatus,
                Permission::WriteReport,
            ]
        } else {
            Vec::new()
        };

        Self {
            audit_log: Vec::new(),
            max_audit_entries: 1000,
            mcp_permissions,
            localhost_permissions,
            skill_permissions: HashMap::new(),
        }
    }

    /// 检查权限并记录审计日志
    pub fn check_permission(
        &mut self,
        source: &CallSource,
        permission: Permission,
    ) -> PolicyDecision {
        let start = std::time::Instant::now();
        let (decision, reason) = self.evaluate(source, permission);
        let entry = AuditEntry::new(
            source.clone(),
            permission,
            decision,
            reason,
            start.elapsed().as_millis() as u64,
        );
        self.record_audit(entry);
        decision
    }

    /// 注册 Skill 权限
    pub fn register_skill_permissions(&mut self, skill_id: &str, permissions: Vec<Permission>) {
        self.skill_permissions
            .insert(skill_id.to_string(), permissions);
    }

    /// 获取统计：按来源分组的调用次数
    pub fn get_call_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        for entry in &self.audit_log {
            let key = match &entry.source {
                CallSource::McpTool { tool_name, .. } => format!("mcp:{tool_name}"),
                CallSource::SkillExecution { skill_id } => format!("skill:{skill_id}"),
                CallSource::Frontend { route, .. } => {
                    format!("frontend:{}", route.as_deref().unwrap_or("unknown"))
                }
                CallSource::LocalhostApi { endpoint } => format!("api:{endpoint}"),
            };
            *stats.entry(key).or_insert(0) += 1;
        }
        stats
    }

    fn evaluate(
        &self,
        source: &CallSource,
        permission: Permission,
    ) -> (PolicyDecision, Option<String>) {
        match source {
            CallSource::McpTool { .. } => {
                if self.mcp_permissions.contains(&permission) {
                    (PolicyDecision::Allow, None)
                } else {
                    (
                        PolicyDecision::Deny,
                        Some(format!("MCP 客户端无 {permission:?} 权限")),
                    )
                }
            }
            CallSource::SkillExecution { skill_id } => {
                if let Some(perms) = self.skill_permissions.get(skill_id) {
                    if perms.contains(&permission) {
                        (PolicyDecision::Allow, None)
                    } else {
                        (
                            PolicyDecision::Deny,
                            Some(format!("技能 {skill_id} 无 {permission:?} 权限")),
                        )
                    }
                } else {
                    // 未注册权限的技能，只允许读操作
                    match permission {
                        Permission::ReadActivities
                        | Permission::ReadReports
                        | Permission::ReadStats
                        | Permission::ReadSessions
                        | Permission::ReadConfig
                        | Permission::ReadDeviceStatus => (PolicyDecision::Allow, None),
                        _ => (
                            PolicyDecision::Deny,
                            Some(format!("技能 {skill_id} 未注册 {permission:?} 权限")),
                        ),
                    }
                }
            }
            CallSource::Frontend { .. } => (PolicyDecision::Allow, None),
            CallSource::LocalhostApi { .. } => {
                if self.localhost_permissions.contains(&permission) {
                    (PolicyDecision::Allow, None)
                } else {
                    (
                        PolicyDecision::Deny,
                        Some(format!("Localhost API 无 {permission:?} 权限")),
                    )
                }
            }
        }
    }

    fn record_audit(&mut self, entry: AuditEntry) {
        self.audit_log.push(entry);
        if self.audit_log.len() > self.max_audit_entries {
            self.audit_log
                .drain(0..self.audit_log.len() - self.max_audit_entries);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    fn enforcer(localhost_enabled: bool) -> PolicyEnforcer {
        let config = AppConfig {
            localhost_api_enabled: localhost_enabled,
            ..AppConfig::default()
        };
        PolicyEnforcer::new(&config)
    }

    #[test]
    fn mcp工具读操作放行写配置拒绝且拒绝带原因() {
        let mut enforcer = enforcer(true);
        let mcp = CallSource::McpTool {
            tool_name: "query_timeline".into(),
            client_id: Some("client-1".into()),
        };
        assert_eq!(
            enforcer.check_permission(&mcp, Permission::ReadActivities),
            PolicyDecision::Allow
        );
        assert_eq!(
            enforcer.check_permission(&mcp, Permission::WriteConfig),
            PolicyDecision::Deny
        );
        // 拒绝决策应记录拒绝原因
        let last = enforcer.audit_log.last().unwrap();
        assert!(
            last.denied_reason
                .as_deref()
                .unwrap_or_default()
                .contains("WriteConfig"),
            "拒绝原因应包含被拒权限名"
        );
    }

    #[test]
    fn 前端来源默认全部放行() {
        let mut enforcer = enforcer(true);
        let frontend = CallSource::Frontend {
            route: Some("/report".into()),
        };
        for permission in [
            Permission::ReadActivities,
            Permission::WriteConfig,
            Permission::ExecuteAi,
        ] {
            assert_eq!(
                enforcer.check_permission(&frontend, permission),
                PolicyDecision::Allow,
                "{permission:?} 应放行"
            );
        }
    }

    #[test]
    fn localhost开关关闭时应拒绝一切调用() {
        let mut enforcer = enforcer(false);
        let api = CallSource::LocalhostApi {
            endpoint: "/api/activities".into(),
        };
        assert_eq!(
            enforcer.check_permission(&api, Permission::ReadActivities),
            PolicyDecision::Deny,
            "开关关闭时连读操作也应拒绝"
        );
    }

    #[test]
    fn localhost开关开启时读放行写配置拒绝() {
        let mut enforcer = enforcer(true);
        let api = CallSource::LocalhostApi {
            endpoint: "/api/stats".into(),
        };
        assert_eq!(
            enforcer.check_permission(&api, Permission::ReadStats),
            PolicyDecision::Allow
        );
        assert_eq!(
            enforcer.check_permission(&api, Permission::WriteConfig),
            PolicyDecision::Deny
        );
    }

    #[test]
    fn 未注册权限的技能只放行读操作() {
        let mut enforcer = enforcer(true);
        let skill = CallSource::SkillExecution {
            skill_id: "unknown_skill".into(),
        };
        assert_eq!(
            enforcer.check_permission(&skill, Permission::ReadReports),
            PolicyDecision::Allow
        );
        assert_eq!(
            enforcer.check_permission(&skill, Permission::WriteConfig),
            PolicyDecision::Deny
        );
    }

    #[test]
    fn 注册权限后技能按名单放行与拒绝() {
        let mut enforcer = enforcer(true);
        let skill = CallSource::SkillExecution {
            skill_id: "daily_review".into(),
        };
        enforcer.register_skill_permissions(
            "daily_review",
            vec![Permission::ReadStats, Permission::ExecuteAi],
        );
        assert_eq!(
            enforcer.check_permission(&skill, Permission::ExecuteAi),
            PolicyDecision::Allow
        );
        assert_eq!(
            enforcer.check_permission(&skill, Permission::ReadSessions),
            PolicyDecision::Deny,
            "已注册名单的技能不再享受未注册只读兜底"
        );
    }

    #[test]
    fn 审计日志应截断到上限并按来源分组统计() {
        let mut enforcer = enforcer(true);
        for i in 0..1200 {
            let source = if i % 2 == 0 {
                CallSource::Frontend {
                    route: Some(format!("/route-{}", i % 3)),
                }
            } else {
                CallSource::LocalhostApi {
                    endpoint: "/api/x".into(),
                }
            };
            enforcer.check_permission(&source, Permission::ReadStats);
        }
        assert!(
            enforcer.audit_log.len() <= 1000,
            "审计日志应截断到上限: {}",
            enforcer.audit_log.len()
        );
        let stats = enforcer.get_call_stats();
        assert!(!stats.is_empty());
        assert!(stats
            .keys()
            .all(|k| k.starts_with("frontend:") || k.starts_with("api:")));
        let api_total: u64 = stats
            .iter()
            .filter(|(k, _)| k.starts_with("api:"))
            .map(|(_, v)| v)
            .sum();
        assert_eq!(
            api_total, 500,
            "截断应丢弃最旧 200 条（含 100 条 api），保留窗口内 api 恰为 500"
        );
    }

    #[test]
    fn 审计条目id应单调递增() {
        let a = AuditEntry::new(
            CallSource::Frontend { route: None },
            Permission::ReadStats,
            PolicyDecision::Allow,
            None,
            1,
        );
        let b = AuditEntry::new(
            CallSource::Frontend { route: None },
            Permission::ReadStats,
            PolicyDecision::Allow,
            None,
            1,
        );
        assert!(b.id > a.id, "审计 id 应递增保证可排序");
        assert!(b.timestamp >= a.timestamp);
    }
}
