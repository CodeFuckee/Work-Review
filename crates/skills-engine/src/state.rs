use crate::model::{SignalSource, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 技能状态（用户态，可学习）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillState {
    pub skill_id: SkillId,
    /// 学习得到的参数
    #[serde(default)]
    pub learned_params: HashMap<String, serde_json::Value>,
    /// 学习历史（最近 N 条）
    #[serde(default)]
    pub learning_history: Vec<LearningRecord>,
    /// 统计计数
    #[serde(default)]
    pub stats: SkillStats,
    /// 是否暂停学习
    #[serde(default)]
    pub learning_paused: bool,
    /// 上次学习时间（Unix 秒）
    #[serde(default)]
    pub last_learning_at: Option<u64>,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
}

/// 学习记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningRecord {
    pub timestamp: u64,
    pub signal: SignalSource,
    pub field_path: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub confidence: f64,
}

/// 技能执行统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillStats {
    /// 总执行次数
    pub total_executions: u64,
    /// 成功次数
    pub success_count: u64,
    /// 失败次数
    pub failure_count: u64,
    /// 平均执行耗时（毫秒）
    pub avg_duration_ms: f64,
    /// 最近执行时间
    pub last_executed_at: Option<u64>,
    /// 用户接受次数
    pub accepted_count: u64,
    /// 用户忽略次数
    pub ignored_count: u64,
}

impl SkillState {
    pub fn new(skill_id: SkillId) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            skill_id,
            learned_params: HashMap::new(),
            learning_history: Vec::new(),
            stats: SkillStats::default(),
            learning_paused: false,
            last_learning_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// 记录学习结果
    pub fn record_learning(
        &mut self,
        signal: SignalSource,
        field_path: String,
        old_value: serde_json::Value,
        new_value: serde_json::Value,
        confidence: f64,
        max_history: usize,
    ) {
        let record = LearningRecord {
            timestamp: Self::now_secs(),
            signal,
            field_path,
            old_value,
            new_value,
            confidence,
        };
        self.learning_history.push(record);
        if self.learning_history.len() > max_history {
            self.learning_history
                .drain(0..self.learning_history.len() - max_history);
        }
        self.last_learning_at = Some(Self::now_secs());
        self.updated_at = Self::now_secs();
    }

    /// 记录执行结果
    pub fn record_execution(&mut self, success: bool, duration_ms: u64) {
        self.stats.total_executions += 1;
        if success {
            self.stats.success_count += 1;
        } else {
            self.stats.failure_count += 1;
        }
        let total = self.stats.total_executions;
        self.stats.avg_duration_ms = self.stats.avg_duration_ms * (total - 1) as f64 / total as f64
            + duration_ms as f64 / total as f64;
        self.stats.last_executed_at = Some(Self::now_secs());
        self.updated_at = Self::now_secs();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 新建状态应带技能id与初始零统计() {
        let state = SkillState::new("daily_review".into());
        assert_eq!(state.skill_id, "daily_review");
        assert!(state.learned_params.is_empty());
        assert!(state.learning_history.is_empty());
        assert_eq!(state.stats.total_executions, 0);
        assert!(!state.learning_paused);
        assert!(state.created_at > 0);
        assert_eq!(state.updated_at, state.created_at);
    }

    #[test]
    fn 执行统计应累计成功失败并增量平均耗时() {
        let mut state = SkillState::new("s".into());
        state.record_execution(true, 100);
        state.record_execution(true, 200);
        state.record_execution(false, 300);
        assert_eq!(state.stats.total_executions, 3);
        assert_eq!(state.stats.success_count, 2);
        assert_eq!(state.stats.failure_count, 1);
        // 增量平均：(100 + 200) / 2 = 150 → (150*2 + 300) / 3 = 200
        assert!((state.stats.avg_duration_ms - 200.0).abs() < 1e-9);
        assert!(state.stats.last_executed_at.is_some());
    }

    #[test]
    fn 学习记录应按上限淘汰最旧条目() {
        let mut state = SkillState::new("s".into());
        for i in 0..5 {
            state.record_learning(
                SignalSource::AcceptSuggestion,
                format!("field_{i}"),
                serde_json::json!(null),
                serde_json::json!(i),
                0.5,
                3,
            );
        }
        assert_eq!(state.learning_history.len(), 3, "应只保留最近 3 条");
        assert_eq!(
            state.learning_history[0].field_path, "field_2",
            "最旧的 field_0/1 被淘汰"
        );
        assert!(state.last_learning_at.is_some());
    }
}
