/// 候选词（可被 UI 展示与用户选择）。
///
/// 注意：`segment_start/segment_end` 是**对当前 segment 切分结果的索引范围**，
/// 用于 `Context` 推进 `confirmed`。
#[derive(Debug, Clone)]
pub struct Candidate {
    /// 候选展示文本（提交文本）
    pub text: String,
    /// 备注（例如来源 key、是否 compose 等）
    pub comment: Option<String>,
    /// 权重（越大越靠前），由词典/模型决定
    pub weight: i32,
    /// 覆盖的音节段范围：[segment_start, segment_end)
    pub segment_start: usize,
    pub segment_end: usize,
}

/// 引擎给 UI 的“快照视图”。
///
/// 设计目标：
/// - UI 层只读 `UiState`，不直接读写 `Context`
/// - 便于 GUI/CLI 输出与调试
#[derive(Debug, Clone)]
pub struct UiState {
    /// 原始输入字符串（未上屏的拼音/简拼）
    pub raw_input: String,
    /// preedit 展示（例如 "ni hao ma"）
    pub preedit: String,
    /// 音节段切分结果（用于组词、选词推进）
    pub segment: Vec<String>,
    /// 光标所在段位置（第一版默认在末尾）
    pub caret: usize,
    /// 已确认段范围的结束位置：[0, confirm)
    pub confirm: usize,
    /// 已确认文本（内部 composition）
    pub confirm_text: String,
    /// 当前可选候选列表（通常是“从 confirm 开始”的候选）
    pub candidate_list: Vec<Candidate>,
}
