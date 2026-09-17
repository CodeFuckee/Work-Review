use crate::model::{LearningStrategy, SignalSource, SkillPackage};
use crate::state::SkillState;
use std::collections::HashMap;

/// 自适应学习引擎
pub struct AdaptiveEngine;

impl AdaptiveEngine {
    /// 处理学习信号
    pub fn process_signal(
        state: &mut SkillState,
        package: &SkillPackage,
        signal: SignalSource,
        context: &HashMap<String, serde_json::Value>,
    ) -> AdaptiveResult {
        if !package.adaptive.enabled || state.learning_paused {
            return AdaptiveResult::Skipped;
        }

        let config = &package.adaptive;
        let mut updates = Vec::new();

        for field in &config.learnable_fields {
            if !field.signal_sources.contains(&signal) {
                continue;
            }

            let field_path = &field.field_path;
            let current_value = state
                .learned_params
                .get(field_path)
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            let new_value = match field.strategy {
                LearningStrategy::Frequency => Self::frequency_update(
                    &current_value,
                    context,
                    field_path,
                    config.max_change_rate,
                ),
                LearningStrategy::WeightedAverage => Self::weighted_avg_update(
                    &current_value,
                    context,
                    field_path,
                    config.max_change_rate,
                ),
                LearningStrategy::ThresholdTuning => Self::threshold_update(
                    &current_value,
                    context,
                    field_path,
                    config.max_change_rate,
                    signal.clone(),
                ),
                LearningStrategy::RankPreference => {
                    Self::rank_update(&current_value, context, field_path, signal.clone())
                }
            };

            if let Some(nv) = new_value {
                let confidence =
                    Self::compute_confidence(&current_value, &nv, config.max_change_rate);
                state.record_learning(
                    signal.clone(),
                    field_path.clone(),
                    current_value.clone(),
                    nv.clone(),
                    confidence,
                    100,
                );
                state.learned_params.insert(field_path.clone(), nv.clone());
                updates.push(FieldUpdate {
                    field_path: field_path.clone(),
                    old_value: current_value,
                    new_value: nv,
                    confidence,
                });
            }
        }

        if updates.is_empty() {
            AdaptiveResult::NoChange
        } else {
            AdaptiveResult::Updated(updates)
        }
    }

    /// 回滚到指定时间之前的状态
    pub fn rollback(state: &mut SkillState, before_timestamp: u64) -> usize {
        let original_len = state.learning_history.len();
        let to_keep: Vec<_> = state
            .learning_history
            .drain(..)
            .filter(|r| r.timestamp >= before_timestamp)
            .collect();
        let rolled_back = original_len - to_keep.len();
        state.learning_history = to_keep;

        // 重建 learned_params from remaining history
        state.learned_params.clear();
        for record in &state.learning_history {
            state
                .learned_params
                .insert(record.field_path.clone(), record.new_value.clone());
        }

        rolled_back
    }

    /// 暂停学习
    pub fn pause_learning(state: &mut SkillState) {
        state.learning_paused = true;
    }

    /// 恢复学习
    pub fn resume_learning(state: &mut SkillState) {
        state.learning_paused = false;
    }

    fn frequency_update(
        current: &serde_json::Value,
        context: &HashMap<String, serde_json::Value>,
        field_path: &str,
        max_change: f64,
    ) -> Option<serde_json::Value> {
        let new_val = context.get(field_path)?;
        match (current, new_val) {
            (serde_json::Value::Number(n), serde_json::Value::Number(m)) => {
                let n = n.as_f64()?;
                let m = m.as_f64()?;
                let delta = (m - n).abs();
                if delta / n.max(1.0) <= max_change {
                    Some(serde_json::json!((n + m) / 2.0))
                } else {
                    let clamped = n + (m - n).signum() * n.max(1.0) * max_change;
                    Some(serde_json::json!(clamped))
                }
            }
            _ => Some(new_val.clone()),
        }
    }

    fn weighted_avg_update(
        current: &serde_json::Value,
        context: &HashMap<String, serde_json::Value>,
        field_path: &str,
        max_change: f64,
    ) -> Option<serde_json::Value> {
        let weight = 0.3;
        let new_val = context.get(field_path)?;
        match (current, new_val) {
            (serde_json::Value::Number(n), serde_json::Value::Number(m)) => {
                let n = n.as_f64()?;
                let m = m.as_f64()?;
                let avg = n * (1.0 - weight) + m * weight;
                let delta = (avg - n).abs();
                if delta / n.max(1.0) <= max_change {
                    Some(serde_json::json!(avg))
                } else {
                    let clamped = n + (avg - n).signum() * n.max(1.0) * max_change;
                    Some(serde_json::json!(clamped))
                }
            }
            _ => Some(new_val.clone()),
        }
    }

    fn threshold_update(
        current: &serde_json::Value,
        context: &HashMap<String, serde_json::Value>,
        field_path: &str,
        max_change: f64,
        signal: SignalSource,
    ) -> Option<serde_json::Value> {
        let adjustment = match signal {
            SignalSource::AcceptSuggestion => 0.05,
            SignalSource::IgnoreSuggestion => -0.05,
            _ => return None,
        };
        match current {
            serde_json::Value::Number(n) => {
                let n = n.as_f64()?;
                let new_val = n + adjustment;
                let delta = (new_val - n).abs();
                if delta <= max_change {
                    Some(serde_json::json!(new_val))
                } else {
                    None
                }
            }
            _ => context.get(field_path).cloned(),
        }
    }

    fn rank_update(
        current: &serde_json::Value,
        context: &HashMap<String, serde_json::Value>,
        field_path: &str,
        signal: SignalSource,
    ) -> Option<serde_json::Value> {
        let adjustment = match signal {
            SignalSource::AcceptSuggestion | SignalSource::FollowupAction => 1,
            SignalSource::IgnoreSuggestion => -1,
            _ => 0,
        };
        if adjustment == 0 {
            return None;
        }
        match current {
            serde_json::Value::Number(n) => {
                let n = n.as_i64()?;
                Some(serde_json::json!(n + adjustment))
            }
            _ => context.get(field_path).cloned(),
        }
    }

    fn compute_confidence(
        old: &serde_json::Value,
        new: &serde_json::Value,
        max_change: f64,
    ) -> f64 {
        match (old.as_f64(), new.as_f64()) {
            (Some(o), Some(n)) if o > 0.0 => {
                let change = (n - o).abs() / o;
                1.0 - (change / max_change).min(1.0)
            }
            _ => 0.5,
        }
    }
}

/// 自适应更新结果
#[derive(Debug)]
pub enum AdaptiveResult {
    /// 更新成功
    Updated(Vec<FieldUpdate>),
    /// 无变化
    NoChange,
    /// 已跳过（暂停或不启用）
    Skipped,
}

/// 字段更新详情
#[derive(Debug)]
pub struct FieldUpdate {
    pub field_path: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AdaptiveConfig, LearnableField, LearningStrategy, SignalSource, SkillCategory, SkillPackage,
    };
    use std::collections::HashMap;

    fn package(adaptive: AdaptiveConfig) -> SkillPackage {
        SkillPackage {
            id: "s".into(),
            name: "s".into(),
            description: "".into(),
            version: "1.0.0".into(),
            author: None,
            category: SkillCategory::Automation,
            pipeline: crate::model::Pipeline {
                query: crate::model::QueryStep::DailyStats { date: "d".into() },
                transforms: vec![],
                output: crate::model::OutputStep::Text {
                    template: "t".into(),
                },
            },
            adaptive,
            enabled: true,
            required_permissions: vec![],
        }
    }

    fn learnable(strategy: LearningStrategy, sources: Vec<SignalSource>) -> AdaptiveConfig {
        AdaptiveConfig {
            enabled: true,
            learnable_fields: vec![LearnableField {
                field_path: "prompt_template".into(),
                strategy,
                signal_sources: sources,
            }],
            ..Default::default()
        }
    }

    fn ctx_value(v: serde_json::Value) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("prompt_template".to_string(), v);
        map
    }

    #[test]
    fn 自适应关闭或学习暂停时应跳过() {
        let mut cfg = learnable(
            LearningStrategy::Frequency,
            vec![SignalSource::AcceptSuggestion],
        );
        cfg.enabled = false;
        let mut state = SkillState::new("s".into());
        assert!(matches!(
            AdaptiveEngine::process_signal(
                &mut state,
                &package(cfg.clone()),
                SignalSource::AcceptSuggestion,
                &HashMap::new()
            ),
            AdaptiveResult::Skipped
        ));
        cfg.enabled = true;
        AdaptiveEngine::pause_learning(&mut state);
        assert!(matches!(
            AdaptiveEngine::process_signal(
                &mut state,
                &package(cfg),
                SignalSource::AcceptSuggestion,
                &HashMap::new()
            ),
            AdaptiveResult::Skipped
        ));
        AdaptiveEngine::resume_learning(&mut state);
        assert!(!state.learning_paused, "恢复后应可继续学习");
    }

    #[test]
    fn 信号不在字段来源名单内应无变化() {
        let cfg = learnable(
            LearningStrategy::Frequency,
            vec![SignalSource::AcceptSuggestion],
        );
        let pkg = package(cfg);
        let mut state = SkillState::new("s".into());
        let result = AdaptiveEngine::process_signal(
            &mut state,
            &pkg,
            SignalSource::IgnoreSuggestion,
            &ctx_value(serde_json::json!(100)),
        );
        assert!(matches!(result, AdaptiveResult::NoChange));
    }

    #[test]
    fn 频率策略小偏差应取平均大偏差应按最大幅度钳制() {
        let cfg = learnable(
            LearningStrategy::Frequency,
            vec![SignalSource::AcceptSuggestion],
        );
        let pkg = package(cfg);
        let mut state = SkillState::new("s".into());
        state
            .learned_params
            .insert("prompt_template".into(), serde_json::json!(100.0));

        // 偏差 10% ≤ 20%：取平均 (100+110)/2 = 105
        let result = AdaptiveEngine::process_signal(
            &mut state,
            &pkg,
            SignalSource::AcceptSuggestion,
            &ctx_value(serde_json::json!(110.0)),
        );
        match result {
            AdaptiveResult::Updated(updates) => {
                assert_eq!(updates[0].new_value, serde_json::json!(105.0));
                assert!(updates[0].confidence > 0.0);
            }
            other => panic!("应有更新: {other:?}"),
        }
        assert_eq!(
            state.learned_params["prompt_template"],
            serde_json::json!(105.0),
            "学习结果应写回参数"
        );
        assert_eq!(state.learning_history.len(), 1);

        // 偏差远超 20%：钳制为 105 + 1*105*0.2 = 126
        let result = AdaptiveEngine::process_signal(
            &mut state,
            &pkg,
            SignalSource::AcceptSuggestion,
            &ctx_value(serde_json::json!(500.0)),
        );
        match result {
            AdaptiveResult::Updated(updates) => {
                assert!((updates[0].new_value.as_f64().unwrap() - 126.0).abs() < 1e-9);
            }
            other => panic!("应有更新: {other:?}"),
        }
        assert_eq!(state.learning_history.len(), 2);
    }

    #[test]
    fn 加权平均策略应按03权重收敛() {
        let cfg = learnable(
            LearningStrategy::WeightedAverage,
            vec![SignalSource::FollowupAction],
        );
        let pkg = package(cfg);
        let mut state = SkillState::new("s".into());
        state
            .learned_params
            .insert("prompt_template".into(), serde_json::json!(100.0));
        let result = AdaptiveEngine::process_signal(
            &mut state,
            &pkg,
            SignalSource::FollowupAction,
            &ctx_value(serde_json::json!(110.0)),
        );
        match result {
            AdaptiveResult::Updated(updates) => {
                let expected = 100.0 * 0.7 + 110.0 * 0.3;
                assert!((updates[0].new_value.as_f64().unwrap() - expected).abs() < 1e-9);
            }
            other => panic!("应有更新: {other:?}"),
        }
    }

    #[test]
    fn 阈值策略接受加0点05忽略减0点05其余信号不动() {
        let cfg = learnable(
            LearningStrategy::ThresholdTuning,
            vec![
                SignalSource::AcceptSuggestion,
                SignalSource::IgnoreSuggestion,
            ],
        );
        let pkg = package(cfg);
        let mut state = SkillState::new("s".into());
        state
            .learned_params
            .insert("prompt_template".into(), serde_json::json!(0.5));

        AdaptiveEngine::process_signal(
            &mut state,
            &pkg,
            SignalSource::AcceptSuggestion,
            &HashMap::new(),
        );
        assert_eq!(
            state.learned_params["prompt_template"],
            serde_json::json!(0.55)
        );

        AdaptiveEngine::process_signal(
            &mut state,
            &pkg,
            SignalSource::IgnoreSuggestion,
            &HashMap::new(),
        );
        assert_eq!(
            state.learned_params["prompt_template"],
            serde_json::json!(0.5)
        );

        // 非接受/忽略信号（阈值策略返回 None）→ 无变化
        let result = AdaptiveEngine::process_signal(
            &mut state,
            &pkg,
            SignalSource::FollowupAction,
            &HashMap::new(),
        );
        assert!(matches!(result, AdaptiveResult::NoChange));
    }

    #[test]
    fn 排序偏好策略应按信号加减计分() {
        let cfg = learnable(
            LearningStrategy::RankPreference,
            vec![
                SignalSource::AcceptSuggestion,
                SignalSource::FollowupAction,
                SignalSource::IgnoreSuggestion,
            ],
        );
        let pkg = package(cfg);
        let mut state = SkillState::new("s".into());
        state
            .learned_params
            .insert("prompt_template".into(), serde_json::json!(5));

        AdaptiveEngine::process_signal(
            &mut state,
            &pkg,
            SignalSource::FollowupAction,
            &HashMap::new(),
        );
        assert_eq!(
            state.learned_params["prompt_template"],
            serde_json::json!(6)
        );

        AdaptiveEngine::process_signal(
            &mut state,
            &pkg,
            SignalSource::IgnoreSuggestion,
            &HashMap::new(),
        );
        assert_eq!(
            state.learned_params["prompt_template"],
            serde_json::json!(5)
        );
    }

    #[test]
    fn 首次学习无当前值时应直接采用上下文值() {
        let cfg = learnable(
            LearningStrategy::Frequency,
            vec![SignalSource::ManualCategoryEdit],
        );
        let pkg = package(cfg);
        let mut state = SkillState::new("s".into());
        let result = AdaptiveEngine::process_signal(
            &mut state,
            &pkg,
            SignalSource::ManualCategoryEdit,
            &ctx_value(serde_json::json!("development")),
        );
        match result {
            AdaptiveResult::Updated(updates) => {
                assert_eq!(updates[0].new_value, serde_json::json!("development"));
            }
            other => panic!("应有更新: {other:?}"),
        }
    }

    #[test]
    fn 回滚应清除早于时间戳的记录并按剩余历史重建参数() {
        let mut state = SkillState::new("s".into());

        state.record_learning(
            SignalSource::AcceptSuggestion,
            "prompt_template".into(),
            serde_json::json!(null),
            serde_json::json!("old"),
            0.5,
            100,
        );
        state.record_learning(
            SignalSource::AcceptSuggestion,
            "prompt_template".into(),
            serde_json::json!("old"),
            serde_json::json!("new"),
            0.8,
            100,
        );

        // 时间下界为 0：所有记录都应保留，参数按历史重建并清掉脏字段
        state
            .learned_params
            .insert("dirty".into(), serde_json::json!(1));
        assert_eq!(AdaptiveEngine::rollback(&mut state, 0), 0);
        assert_eq!(state.learning_history.len(), 2);
        assert_eq!(
            state.learned_params.get("prompt_template"),
            Some(&serde_json::json!("new")),
            "参数应按剩余历史的最后一条重建"
        );
        assert!(!state.learned_params.contains_key("dirty"));

        // 时间下界推到当前之后：全部记录被回滚，参数清空
        let future = SkillState::now_secs() + 3600;
        let rolled = AdaptiveEngine::rollback(&mut state, future);
        assert_eq!(rolled, 2);
        assert!(state.learning_history.is_empty());
        assert!(state.learned_params.is_empty());
    }
}
