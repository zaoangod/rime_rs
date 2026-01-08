//! `rime_core`：纯逻辑层（std-only），不做任何 I/O。
//!
//! 设计目标：
//! - **核心可复用**：CLI/GUI/服务端都能复用同一套逻辑
//! - **分层清晰**：engine -> processor -> segmenter -> translator -> filter -> 输出（`UiState`）
//! - **易演进**：先跑通最小功能，再逐步替换/扩展 processor 与 translator
pub mod context;
pub mod dictionary;
pub mod engine;
pub mod filter;
pub mod key_event;
pub mod model;
pub mod processor;
pub mod segmenter;
pub mod session;
pub mod translator;
