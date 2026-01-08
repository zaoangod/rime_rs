//! `processor`：输入事件处理链。
//!
//! 这里的 Processor 类似 Rime 的 processor 概念：按顺序处理 `InputEvent`，
//! 对 `Context` 做状态变更，并可产生 `Action`（例如 Commit）。
//!
//! 当前链路（`Session::new` 默认组装）：
//! - `EditingProcessor`：编辑输入（Char/Backspace/Clear）并触发重新切分
//! - `SelectionProcessor`：选词（Space/Select(n)）推进 confirmed
//! - `EnterCommitProcessor`：回车提交（confirmed_text + raw_input）

use crate::{
    context::Context,
    engine::Analysis,
    key_event::{Action, InputEvent},
    model::UiState,
};

/// 给 processors 的对象安全引擎接口（避免在 processors 层引入泛型爆炸）。
pub trait EngineFacade {
    /// 切分输入：raw -> (segment + preedit)
    fn analyze(&self, raw_input: &str) -> Analysis;
    /// 组合输出：根据 segment/caret/confirmed 生成 UiState（候选等）
    fn compose_with_state(
        &self,
        raw_input: &str,
        analysis: Analysis,
        confirmed: usize,
        caret: Option<usize>,
        confirmed_text: String,
    ) -> UiState;
}

/// Processor 执行结果：是否“消费”了本次事件。
///
/// - `Consume`：本 processor 已处理该事件，后续 processor 不再执行
/// - `Continue`：本 processor 不处理该事件，交给下一个 processor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    Consume,
    Continue,
}

/// Processor：处理输入事件并改变 Context；必要时产生输出动作（Commit 等）。
pub trait Processor: Send + Sync {
    fn process(
        &mut self,
        engine: &dyn EngineFacade,
        context: &mut Context,
        input_event: &InputEvent,
    ) -> (ProcessStatus, Vec<Action>);
}

/// 编辑输入的 processor（插入/退格/清空）。
pub struct EditingProcessor;

impl Processor for EditingProcessor {
    fn process(
        &mut self,
        engine: &dyn EngineFacade,
        context: &mut Context,
        input_event: &InputEvent,
    ) -> (ProcessStatus, Vec<Action>) {
        match *input_event {
            InputEvent::Char(ch) => {
                // 匹配输入字符是否( a-z | A-Z )
                if ch.is_ascii_alphabetic() || ch == '\'' {
                    (&mut context.raw_input).push(ch.to_ascii_lowercase());
                    (&mut *context).reanalyze(engine);
                }
                (ProcessStatus::Consume, Vec::new())
            }
            InputEvent::Backspace => {
                (&mut context.raw_input).pop();
                (&mut *context).reanalyze(engine);
                (ProcessStatus::Consume, Vec::new())
            }
            InputEvent::Clear => {
                (&mut *context).reset();
                (ProcessStatus::Consume, Vec::new())
            }
            _ => (ProcessStatus::Continue, Vec::new()),
        }
    }
}

pub struct SelectionProcessor;

impl Processor for SelectionProcessor {
    fn process(
        &mut self,
        engine: &dyn EngineFacade,
        context: &mut Context,
        input_event: &InputEvent,
    ) -> (ProcessStatus, Vec<Action>) {
        match *input_event {
            // 输入的是空格键
            InputEvent::Space => {
                let action: Vec<Action> = (&mut *context).select_candidate(engine, 0);
                (ProcessStatus::Consume, action)
            }
            // 输入的是1-9数字
            InputEvent::Select(i) => {
                let action: Vec<Action> = (&mut *context).select_candidate(engine, i);
                (ProcessStatus::Consume, action)
            }
            _ => (ProcessStatus::Continue, Vec::new()),
        }
    }
}

pub struct EnterCommitProcessor;

impl Processor for EnterCommitProcessor {
    fn process(
        &mut self,
        engine: &dyn EngineFacade,
        context: &mut Context,
        input_event: &InputEvent,
    ) -> (ProcessStatus, Vec<Action>) {
        match *input_event {
            InputEvent::Enter => (ProcessStatus::Consume, (&mut *context).commit_on_enter()),
            _ => (ProcessStatus::Continue, Vec::new()),
        }
    }
}
