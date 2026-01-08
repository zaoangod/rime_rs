//! `Session`：对上层（CLI/GUI）提供的会话对象。
//!
//! `Session` 自身不做业务逻辑判断，而是：
//! - 持有 `Context`（状态）
//! - 持有 processors 链（可插拔）
//! - 把每次 `InputEvent` 依次交给 processors，直到被消费
//! - 最后输出 `UiState` + `Action`

use crate::{
    context::Context,
    dictionary::Dictionary,
    engine::Analyzer,
    engine::Engine,
    key_event::{Action, InputEvent},
    model::UiState,
    processor::{EditingProcessor, EnterCommitProcessor, ProcessStatus, Processor, SelectionProcessor},
    segmenter::Segmenter,
};

/// 输入法会话（一次输入过程的状态机容器）。
pub struct Session<D, P> {
    /// 引擎（包含词典、analyzer/segmentor、translator/filter 编排）
    engine: Engine<D, P>,
    /// 会话上下文（processors 共享）
    ctx: Context,
    /// processors 链（可配置/可扩展）
    processors: Vec<Box<dyn Processor>>,
}

impl<D, P> Session<D, P>
where
    D: Dictionary,
    P: Analyzer + Segmenter,
{
    /// 创建会话，并组装默认 processors 链。
    pub fn new(engine: Engine<D, P>) -> Self {
        Self {
            engine,
            ctx: Context::default(),
            processors: vec![
                Box::new(EditingProcessor),
                Box::new(SelectionProcessor),
                Box::new(EnterCommitProcessor),
            ],
        }
    }

    /// 获取当前 UI 快照（只读）。
    pub fn ui_state(&self) -> UiState {
        self.ctx.ui_state(&self.engine)
    }

    /// 处理一个输入事件，返回最新 UI 快照与动作列表。
    pub fn handle(&mut self, ev: InputEvent) -> (UiState, Vec<Action>) {
        let mut actions = Vec::new();
        for p in &mut self.processors {
            let (status, mut a) = p.process(&self.engine, &mut self.ctx, &ev);
            actions.append(&mut a);
            if status == ProcessStatus::Consume {
                break;
            }
        }
        (self.ctx.ui_state(&self.engine), actions)
    }
}
