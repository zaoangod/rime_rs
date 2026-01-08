use crate::model::Candidate;

/// 词典抽象：core 不关心词典来自文件/内存/网络。
///
/// 约定：
/// - `segment` 是切分后的音节段序列（例如 `["qi","shi"]` 或简拼 `["q","s"]`）
/// - `start..end` 是段索引范围，含 start 不含 end
/// - `Candidate.segment_start/segment_end` 由调用方（translator/engine）按需填充
pub trait Dictionary: Send + Sync {
    /// 查询音节段 `segment[start..end]` 对应的候选词（通常是精确匹配）。
    /// - `limit`: 返回候选数量上限（实现可自行 clamp）
    fn lookup_span(&self, segment: &[String], start: usize, end: usize, limit: usize) -> Vec<Candidate>;

    /// 查询整段输入（默认走 `lookup_span(0..len)`）。
    fn lookup(&self, segment: &[String], limit: usize) -> Vec<Candidate> {
        self.lookup_span(segment, 0, segment.len(), limit)
    }
}
