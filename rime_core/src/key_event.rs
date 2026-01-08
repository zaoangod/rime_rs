/// 输入事件（逻辑键盘事件）。
///
/// 说明：
/// - `Session`/processor 只关心“语义事件”，不关心具体平台键值。
/// - CLI/GUI 层负责把系统按键转换成这些事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    /// 输入一个字符（通常是 a-z 或 `'`）
    Char(char),
    /// 删除光标前一个字符
    Backspace,
    /// 空格（当前实现里等同于选择高亮候选）
    Space,
    /// 回车（当前实现里：提交 confirmed_text + raw_input）
    Enter,
    /// 清空当前会话（类似 Esc）
    Clear,
    /// 选择候选词（1-9）
    Select(usize),
    /// 退出（上层用；core 可忽略）
    Exit,
}

/// 引擎输出动作（对 UI/宿主的“副作用”请求）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// 提交文本（上屏）
    Commit(String),
}
