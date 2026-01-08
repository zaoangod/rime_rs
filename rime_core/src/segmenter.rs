//! `segmenter`：切分输入为音节段（segment）。
//!
//! 目前 `Segmenter` 只是对 `Analyzer` 的一个薄封装：
//! - `Analyzer` 定义 “raw -> Analysis”
//! - `Segmenter` 提供同样的能力，方便未来替换为更复杂的切分器

use crate::engine::{Analysis, Analyzer};

/// Segmenter：把输入解析为音节段（segment）并给出 preedit。
///
/// 目前我们直接复用 `Analyzer` 的能力；后续可以替换为更复杂的 `segmenter`（例如支持光标、断词等）。
pub trait Segmenter: Send + Sync {
    fn segment(&self, input: &str) -> Analysis;
}

impl<T> Segmenter for T
where
    T: Analyzer,
{
    fn segment(&self, input: &str) -> Analysis {
        self.analyze(input)
    }
}
