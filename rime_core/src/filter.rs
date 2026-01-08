//! `filter`：候选后处理（去重/排序/裁剪/过滤等）。

use crate::model::Candidate;

/// Filter：对候选列表做后处理（去重、排序、裁剪、字符集过滤等）。
pub trait Filter: Send + Sync {
    fn apply(&self, candidates: Vec<Candidate>) -> Vec<Candidate>;
}

/// 默认 filter：按 weight 倒序排序，按 (text, span) 去重，截断到 limit。
pub struct DedupSortTruncate {
    pub limit: u8,
}

impl Filter for DedupSortTruncate {
    fn apply(&self, mut candidates: Vec<Candidate>) -> Vec<Candidate> {
        let limit = usize::from(self.limit.max(1));
        candidates.sort_by(|a, b| b.weight.cmp(&a.weight).then_with(|| a.text.cmp(&b.text)));
        candidates.dedup_by(|a, b| {
            a.text == b.text && a.segment_start == b.segment_start && a.segment_end == b.segment_end
        });
        candidates.truncate(limit);
        candidates
    }
}
