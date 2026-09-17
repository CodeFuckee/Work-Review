use crate::executor::{ExecutionContext, ExecutionResult, SkillExecutor};
use crate::model::SkillPackage;
use crate::registry::{builtin_skills, SkillRegistry};

/// 技能引擎：顶层 API
pub struct SkillEngine {
    registry: SkillRegistry,
}

impl SkillEngine {
    /// 创建引擎并加载内置技能
    pub fn new() -> Self {
        let mut registry = SkillRegistry::new();
        for skill in builtin_skills() {
            registry.register(skill);
        }
        Self { registry }
    }

    /// 执行技能
    pub fn execute(&mut self, skill_id: &str, ctx: &ExecutionContext) -> ExecutionResult {
        SkillExecutor::execute(&mut self.registry, skill_id, ctx)
    }

    /// 列出所有技能
    pub fn list_skills(&self) -> Vec<&SkillPackage> {
        self.registry.list_all()
    }

    /// 获取技能状态
    pub fn get_skill_state(&self, id: &str) -> Option<&crate::state::SkillState> {
        self.registry.get_state(id)
    }

    /// 获取所有技能的执行统计
    pub fn get_all_stats(&self) -> Vec<(&str, &crate::state::SkillStats)> {
        let mut result = Vec::new();
        for pkg in self.registry.list_all() {
            if let Some(stats) = self.registry.get_execution_stats(&pkg.id) {
                result.push((pkg.id.as_str(), stats));
            }
        }
        result
    }
}

impl Default for SkillEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn 引擎应加载全部内置技能() {
        let engine = SkillEngine::new();
        let ids: Vec<&str> = engine.list_skills().iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids.len(), 5);
        assert!(ids.contains(&"daily_review"));
        assert!(ids.contains(&"focus_session_advisor"));
    }

    #[test]
    fn 引擎执行应走通并更新状态与统计() {
        let mut engine = SkillEngine::new();
        let result = engine.execute(
            "weekly_summary",
            &ExecutionContext {
                params: HashMap::new(),
                db_path: ":memory:".into(),
                ai_endpoint: None,
                ai_api_key: None,
                ai_model: None,
                allow_script_skills: false,
            },
        );
        assert!(result.success, "{:?}", result.error);
        assert!(
            result.output.contains("query_prepared"),
            "markdown 模板应渲染查询数据: {}",
            result.output
        );
        assert!(engine.get_skill_state("weekly_summary").is_some());
        let stats = engine.get_all_stats();
        assert_eq!(stats.len(), 5, "每个注册技能都有统计条目");
        let weekly = stats
            .iter()
            .find(|(id, _)| *id == "weekly_summary")
            .unwrap();
        assert_eq!(weekly.1.total_executions, 1);
        assert_eq!(weekly.1.success_count, 1);
    }
}
