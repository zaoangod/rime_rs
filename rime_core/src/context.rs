//! `Context`：processor 链共享的唯一状态容器。
//!
//! 约定：
//! - `raw_input`：用户尚未上屏的输入串（全拼/简拼）
//! - `analysis`：对 `raw_input` 的切分结果（`segment` + `preedit`）
//! - `confirm/confirm_text`：已确认的段范围与对应文本（用于“逐段选词”）
use crate::{engine::Analysis, key_event::Action, model::UiState, processor::EngineFacade};

/// 输入会话上下文：processor 链共享的唯一状态。
#[derive(Debug, Clone)]
pub struct Context {
    /// 原始输入（未上屏）
    pub raw_input: String,
    /// 切分结果（由 `EngineFacade::analyze` 产生）
    pub analysis: Analysis,
    /// 光标所在段位置（第一版默认在末尾）
    pub caret: usize,
    /// 已确认段范围的结束位置：[0, confirm)
    pub confirm: usize,
    /// 已确认文本（内部 composition）
    pub confirm_text: String,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            raw_input: String::new(),
            analysis: Analysis {
                segment: Vec::new(),
                preedit: String::new(),
            },
            caret: 0,
            confirm: 0,
            confirm_text: String::new(),
        }
    }
}

impl Context {
    /// 清空会话状态（等价于重新开始一次输入）。
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 重新对 `raw_input` 进行切分，并同步更新 `caret/confirm` 的边界。
    pub fn reanalyze(&mut self, engine: &dyn EngineFacade) {
        self.analysis = engine.analyze(&self.raw_input);
        self.caret = self.analysis.segment.len();
        if self.confirm > self.caret {
            self.confirm = self.caret;
            (&mut self.confirm_text).clear();
        }
    }

    /// 生成 UI 层只读快照。
    pub fn ui_state(&self, engine: &dyn EngineFacade) -> UiState {
        engine.compose_with_state(
            &self.raw_input,
            self.analysis.clone(),
            self.confirm,
            Some(self.caret),
            self.confirm_text.clone(),
        )
    }

    /// Enter 的默认行为：提交“已确认 + 原始输入”。
    pub fn commit_on_enter(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();
        if !self.raw_input.is_empty() || !self.confirm_text.is_empty() {
            let mut s = String::new();
            s.push_str(&self.confirm_text);
            s.push_str(&self.raw_input);
            if !s.is_empty() {
                actions.push(Action::Commit(s));
            }
        }
        self.reset();
        actions
    }

    /// 选词推进 confirm；若全部确认则 Commit 并 reset。
    pub fn select_candidate(&mut self, engine: &dyn EngineFacade, index: usize) -> Vec<Action> {
        if self.raw_input.is_empty() || self.confirm >= self.caret {
            return Vec::new();
        }
        let ui = self.ui_state(engine);
        let Some(cand) = ui.candidate_list.get(index) else {
            return Vec::new();
        };
        if cand.segment_start != self.confirm {
            return Vec::new();
        }
        if cand.segment_end <= cand.segment_start || cand.segment_end > self.caret {
            return Vec::new();
        }
        self.confirm_text.push_str(&cand.text);
        self.confirm = cand.segment_end;

        if self.confirm == self.caret {
            if !self.confirm_text.is_empty() {
                return vec![Action::Commit(std::mem::take(&mut self.confirm_text))];
            }
            self.reset();
        }
        Vec::new()
    }
}
