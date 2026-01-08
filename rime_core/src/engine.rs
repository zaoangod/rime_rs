use crate::dictionary::Dictionary;
use crate::filter::{DedupSortTruncate, Filter};
use crate::model::Candidate;
use crate::model::UiState;
use crate::segmenter::Segmenter;
use crate::translator::DictTranslator;

/// 解析结果（segment + preedit）。
#[derive(Debug, Clone)]
pub struct Analysis {
    /// 切分后的音节段（全拼：`["qi","shi"]`；简拼：`["q","s"]`）
    pub segment: Vec<String>,
    /// 展示用 preedit（例如 `"qi shi"` / `"q s"`）
    pub preedit: String,
}

/// 纯接口：把 raw input 解析为音节段（segment）并给出 preedit 展示。
///
/// 备注：当前 `rime_pinyin::QuanpinPreeditor` 同时承担“全拼切分 + 简拼 fallback”。
pub trait Analyzer: Send + Sync {
    fn analyze(&self, input: &str) -> Analysis;
}

/// 引擎：负责把输入状态（segment/caret/confirmed）转成 `UiState`。
///
/// 结构上对应你规划的流水线：
/// - engine（编排） -> segmentor/analyzer（切分） -> translator（查词/组句） -> filter（去重/排序） -> 输出 UiState
pub struct Engine<D, A> {
    /// segmentor/analyzer（目前同一个对象实现）
    analyzer: A,
    /// 词典（TSV 或其他实现）
    dictionary: D,
    /// 候选词数量（1-9）；超出范围时回退到默认值
    candidate_limit: u8,
    /// 组词时单个“词”最多覆盖多少个音节段
    max_word_length: u8,
    /// 每个 span 查询最多取多少条（用于控制 beam search 扩展规模）
    per_span_limit: usize,
}

impl<D, A> Engine<D, A>
where
    D: Dictionary,
    A: Analyzer + Segmenter,
{
    pub fn new(dictionary: D, analyzer: A) -> Self {
        Self {
            dictionary,
            analyzer,
            candidate_limit: 9,
            max_word_length: 4,
            per_span_limit: 16,
        }
    }

    /// 设置候选词数量上限（1..=9）；非法值会回退到 9。
    pub fn candidate_limit(mut self, limit: u8) -> Self {
        if limit < 2 || limit > 9 {
            self.candidate_limit = 9;
        } else {
            self.candidate_limit = limit;
        }
        self
    }

    /// 限制组词时单个词最多覆盖多少个音节段。
    pub fn max_word_length(mut self, n: u8) -> Self {
        self.max_word_length = n.max(1);
        self
    }

    /// 将 raw_input 切分成 segment + preedit（不包含候选生成）。
    pub fn analyze(&self, raw_input: &str) -> Analysis {
        self.analyzer.analyze(raw_input)
    }

    /// 快捷接口：从 raw_input 直接生成 `UiState`（默认 confirmed=0, caret=末尾）。
    pub fn compose(&self, raw_input: &str) -> UiState {
        let analysis: Analysis = self.analyze(raw_input);
        self.compose_with_state(raw_input, analysis, 0, None, String::new())
    }

    /// 面向 Session：给定 segment/caret/confirm，生成“下一段要选”的候选。
    ///
    /// - `confirm`: 已确认到哪个段位置（不含）
    /// - `caret`: 光标位置；None 表示末尾
    /// - `confirm_text`: 已确认文本（用于 UI 展示与最终 Commit 聚合）
    pub fn compose_with_state(
        &self,
        raw_input: &str,
        analysis: Analysis,
        confirm: usize,
        caret: Option<usize>,
        confirm_text: String,
    ) -> UiState {
        let preedit: String = analysis.preedit;
        let segment: Vec<String> = analysis.segment;
        let caret: usize = caret.unwrap_or(segment.len()).min(segment.len());
        let confirmed: usize = confirm.min(caret);

        // 只对 [confirmed, caret) 生成候选，便于“逐段确认”的交互模型。
        let candidate_list = if segment.is_empty() || confirmed >= caret {
            Vec::new()
        } else {
            self.compose_from_segment(&segment, confirmed, caret)
        };
        UiState {
            raw_input: raw_input.to_owned(),
            preedit,
            segment,
            caret,
            confirm: confirmed,
            confirm_text,
            candidate_list,
        }
    }

    fn compose_from_segment(&self, segment: &[String], start: usize, end: usize) -> Vec<Candidate> {
        // translator：负责查词与组句
        let translator = DictTranslator {
            dict: &self.dictionary,
            max_word_length: self.max_word_length,
            per_span_limit: self.per_span_limit,
        };
        let out = translator.translate_with_composition(
            segment,
            start,
            end,
            usize::from(self.candidate_limit),
        );
        // filter：负责去重/排序/截断
        DedupSortTruncate {
            limit: self.candidate_limit,
        }
        .apply(out)
    }
}

impl<D, A> crate::processor::EngineFacade for Engine<D, A>
where
    D: Dictionary,
    A: Analyzer + Segmenter,
{
    fn analyze(&self, raw_input: &str) -> Analysis {
        Engine::<D, A>::analyze(self, raw_input)
    }

    fn compose_with_state(
        &self,
        raw_input: &str,
        analysis: Analysis,
        confirmed: usize,
        caret: Option<usize>,
        confirmed_text: String,
    ) -> UiState {
        Engine::<D, A>::compose_with_state(
            self,
            raw_input,
            analysis,
            confirmed,
            caret,
            confirmed_text,
        )
    }
}
