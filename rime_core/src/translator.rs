//! `translator`：把 segment 翻译成候选（查词、组句、联想）。
//!
//! 当前实现：
//! - `DictTranslator`：基于 `Dictionary::lookup_span`，支持：
//!   - 直查（start..end）
//!   - 单词候选（从 start 起枚举 1..=max_word_len）
//!   - 组句候选（beam search，覆盖 start..end）

use crate::{dictionary::Dictionary, model::Candidate};

/// Translator：把某段 segment 转成候选。
pub trait Translator: Send + Sync {
    fn translate(
        &self,
        segments: &[String],
        start: usize,
        end: usize,
        limit: usize,
    ) -> Vec<Candidate>;
}

/// 词典翻译器（基于 Dictionary::lookup_span），并提供一个轻量“组句”能力。
pub struct DictTranslator<'a, D> {
    /// 词典引用（查词发生在这里）
    pub dict: &'a D,
    /// 单个词候选最多覆盖段数
    pub max_word_length: u8,
    /// 每个 span 查询最多取多少条（控制组合规模）
    pub per_span_limit: usize,
}

impl<'a, D> DictTranslator<'a, D>
where
    D: Dictionary,
{
    pub fn translate_with_composition(
        &self,
        segment: &[String],
        start: usize,
        end: usize,
        limit: usize,
    ) -> Vec<Candidate> {
        let limit: usize = limit.max(1);
        let mut out: Vec<Candidate> = Vec::new();

        // 0) 直查 start..end
        let mut direct: Vec<Candidate> = self.dict.lookup_span(segment, start, end, limit);
        for c in &mut direct {
            c.segment_start = start;
            c.segment_end = end;
        }
        (&mut out).append(&mut direct);

        // 1) 单词候选（从 start 开始，枚举长度 1..=max_word_len）
        let max_j = (start + (self.max_word_length as usize).max(1)).min(end);
        for j in (start + 1)..=max_j {
            let mut cands = self
                .dict
                .lookup_span(segment, start, j, self.per_span_limit.max(1));
            for c in &mut cands {
                c.segment_start = start;
                c.segment_end = j;
            }
            out.append(&mut cands);
        }

        // 2) 组句候选（覆盖 start..end）
        if out.len() < limit {
            let mut composed =
                self.compose_sentence_candidates(segment, start, end, limit - out.len());
            out.append(&mut composed);
        }

        out
    }

    fn compose_sentence_candidates(
        &self,
        segments: &[String],
        start: usize,
        end: usize,
        limit: usize,
    ) -> Vec<Candidate> {
        if limit == 0 || start >= end || end > segments.len() {
            return Vec::new();
        }

        #[derive(Clone)]
        struct Path {
            text: String,
            score: i64,
        }

        let beam_k = limit.max(8).min(64);
        let mut beams: Vec<Vec<Path>> = vec![Vec::new(); end + 1];
        beams[start].push(Path {
            text: String::new(),
            score: 0,
        });

        for i in start..end {
            if beams[i].is_empty() {
                continue;
            }
            beams[i].sort_by(|a, b| b.score.cmp(&a.score));
            beams[i].truncate(beam_k);
            let cur_paths = beams[i].clone();

            let max_j = (i + (self.max_word_length as usize).max(1)).min(end);
            for j in (i + 1)..=max_j {
                let words = self
                    .dict
                    .lookup_span(segments, i, j, self.per_span_limit.max(1));
                if words.is_empty() {
                    continue;
                }
                let len_bonus = ((j - i) as i64) * 1_000;
                for p in &cur_paths {
                    for w in &words {
                        let mut text = String::new();
                        if p.text.is_empty() {
                            text.push_str(&w.text);
                        } else {
                            text.push_str(&p.text);
                            text.push_str(&w.text);
                        }
                        let score = p.score + (w.weight as i64) + len_bonus;
                        beams[j].push(Path { text, score });
                    }
                }
            }
        }

        let mut finals = beams[end].clone();
        finals.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.text.cmp(&b.text)));
        finals.truncate(limit);
        finals
            .into_iter()
            .map(|p| Candidate {
                text: p.text,
                comment: Some("compose".to_string()),
                weight: (p.score.min(i64::from(i32::MAX))) as i32,
                segment_start: start,
                segment_end: end,
            })
            .collect()
    }
}

impl<'a, D> Translator for DictTranslator<'a, D>
where
    D: Dictionary,
{
    fn translate(
        &self,
        segments: &[String],
        start: usize,
        end: usize,
        limit: usize,
    ) -> Vec<Candidate> {
        self.translate_with_composition(segments, start, end, limit)
    }
}
