// PixTerm - History 模块
// 撤销/重做历史栈

use crate::color::Rgb;

/// 单个像素操作的历史记录
/// 记录一次像素修改的坐标、旧颜色和新颜色，用于撤销/重做
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryEntry {
    /// 被修改像素的 X 坐标
    pub x: u16,
    /// 被修改像素的 Y 坐标
    pub y: u16,
    /// 修改前的颜色（`None` 表示修改前为空像素）
    pub old_color: Option<Rgb>,
    /// 修改后的颜色（`None` 表示修改后为空像素，即擦除操作）
    pub new_color: Option<Rgb>,
}

/// 撤销/重做历史管理器
/// 使用两个栈分别存储撤销和重做操作
/// 每个栈元素是一组 `HistoryEntry`（对应一次笔画/批量操作）
pub struct History {
    /// 撤销栈：每个元素为一次操作（可能包含多个像素修改，如拖拽绘制）
    undo_stack: Vec<Vec<HistoryEntry>>,
    /// 重做栈：撤销后的操作被推入此栈，允许重做
    redo_stack: Vec<Vec<HistoryEntry>>,
}

impl History {
    /// 创建空的历史管理器
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// 将一组操作推入撤销栈
    /// 同时清空重做栈（新操作发生后，之前的重做历史失效）
    /// `entries` 为空时不执行任何操作（避免空记录）
    pub fn push_undo(&mut self, entries: Vec<HistoryEntry>) {
        // 忽略空操作
        if entries.is_empty() {
            return;
        }
        // 推入撤销栈
        self.undo_stack.push(entries);
        // 新操作使重做栈失效，清空之
        self.redo_stack.clear();
    }

    /// 执行撤销操作
    /// 从撤销栈弹出最近一次操作，返回需要回退的像素修改列表
    /// 同时将该操作推入重做栈
    /// 返回 `None` 表示没有可撤销的操作
    pub fn undo(&mut self) -> Option<Vec<HistoryEntry>> {
        // 从撤销栈弹出最近的操作组
        let entries = self.undo_stack.pop()?;
        // 推入重做栈以便稍后重做
        self.redo_stack.push(entries.clone());
        Some(entries)
    }

    /// 执行重做操作
    /// 从重做栈弹出最近一次被撤销的操作，返回需要重新应用的像素修改列表
    /// 同时将该操作推回撤销栈
    /// 返回 `None` 表示没有可重做的操作
    pub fn redo(&mut self) -> Option<Vec<HistoryEntry>> {
        // 从重做栈弹出最近的操作组
        let entries = self.redo_stack.pop()?;
        // 推回撤销栈
        self.undo_stack.push(entries.clone());
        Some(entries)
    }

    /// 清空所有历史记录（撤销栈和重做栈）
    /// 通常在加载新画布或清空画布时调用
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// 查询撤销栈是否为空
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// 查询重做栈是否为空
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助函数：创建一个简单的历史条目
    fn make_entry(x: u16, y: u16, old: Option<Rgb>, new: Option<Rgb>) -> HistoryEntry {
        HistoryEntry {
            x,
            y,
            old_color: old,
            new_color: new,
        }
    }

    #[test]
    fn test_push_and_undo() {
        let mut history = History::new();
        // 初始状态无法撤销
        assert!(!history.can_undo());
        // 推入一次操作（在 (0,0) 绘制红色）
        let entry = make_entry(0, 0, None, Some((255, 0, 0)));
        history.push_undo(vec![entry.clone()]);
        assert!(history.can_undo());
        // 撤销该操作
        let undone = history.undo().unwrap();
        assert_eq!(undone.len(), 1);
        assert_eq!(undone[0], entry);
        // 撤销后不可再撤销
        assert!(!history.can_undo());
        // 但可以重做
        assert!(history.can_redo());
    }

    #[test]
    fn test_redo() {
        let mut history = History::new();
        let entry = make_entry(1, 1, None, Some((0, 255, 0)));
        history.push_undo(vec![entry.clone()]);
        // 撤销
        history.undo();
        assert!(history.can_redo());
        // 重做
        let redone = history.redo().unwrap();
        assert_eq!(redone.len(), 1);
        assert_eq!(redone[0], entry);
        // 重做后可以再次撤销
        assert!(history.can_undo());
        // 不可再重做
        assert!(!history.can_redo());
    }

    #[test]
    fn test_new_action_clears_redo() {
        let mut history = History::new();
        // 操作 1
        history.push_undo(vec![make_entry(0, 0, None, Some((255, 0, 0)))]);
        // 撤销操作 1（进入重做栈）
        history.undo();
        assert!(history.can_redo());
        // 推入新操作 2 → 重做栈应被清空
        history.push_undo(vec![make_entry(1, 1, None, Some((0, 0, 255)))]);
        assert!(!history.can_redo());
    }

    #[test]
    fn test_push_empty_ignored() {
        let mut history = History::new();
        // 推入空操作不应增加栈大小
        history.push_undo(vec![]);
        assert!(!history.can_undo());
    }

    #[test]
    fn test_batch_entries() {
        let mut history = History::new();
        // 模拟拖拽绘制多个像素（一组操作）
        let entries = vec![
            make_entry(0, 0, None, Some((255, 0, 0))),
            make_entry(1, 0, None, Some((255, 0, 0))),
            make_entry(2, 0, None, Some((255, 0, 0))),
        ];
        history.push_undo(entries.clone());
        // 撤销时应返回整组操作
        let undone = history.undo().unwrap();
        assert_eq!(undone.len(), 3);
    }

    #[test]
    fn test_clear() {
        let mut history = History::new();
        history.push_undo(vec![make_entry(0, 0, None, Some((255, 0, 0)))]);
        history.push_undo(vec![make_entry(1, 1, None, Some((0, 255, 0)))]);
        history.undo(); // 一条进入重做栈
        history.clear();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }
}
