# PixTerm 待办事项

## 当前标记为 `#[cfg(test)]` 的函数

以下函数目前仅在测试代码中使用，标记了 `#[cfg(test)]` 以消除编译警告。
当对应功能实现时，应移除 `#[cfg(test)]` 标记使其在生产代码中可用。

### `canvas::is_empty()` — `src/canvas.rs`

```rust
#[cfg(test)]
pub fn is_empty(&self) -> bool
```

**计划用途**：

- **退出前保存提示**：退出程序时检查画布是否为空，非空且未保存时提示用户确认
- **状态栏显示**：在状态栏显示画布像素计数或空/非空状态
- **清空确认**：执行 `clear` 命令前，若画布非空则要求二次确认

### `color::rgb_to_hex()` — `src/color.rs`

```rust
#[cfg(test)]
pub fn rgb_to_hex(color: Rgb) -> String
```

**计划用途**：

- **颜色信息显示统一化**：当前 `app.rs` 和 `ui.rs` 中多处使用 `format!("#{:02X}{:02X}{:02X}", ...)` 内联格式化，应统一调用此函数减少重复
- **剪贴板复制**：实现 `color x y` 命令的颜色值复制到剪贴板功能时使用
- **导出颜色列表**：导出画布使用的所有颜色列表时使用

### `history::can_undo()` / `history::can_redo()` — `src/history.rs`

```rust
#[cfg(test)]
pub fn can_undo(&self) -> bool

#[cfg(test)]
pub fn can_redo(&self) -> bool
```

**计划用途**：

- **状态栏撤销/重做指示器**：在状态栏显示撤销/重做是否可用（如灰色/高亮图标）
- **快捷键反馈优化**：按 Ctrl+Z/Y 时，若不可撤销/重做则显示不同的提示信息（区分"没有历史"与"已到最早/最新状态"）

## 功能规划

### 近期

- [ ] 退出前未保存提示（需要 `canvas::is_empty()`）
- [ ] 统一颜色格式化为 `rgb_to_hex()` 调用
- [ ] 状态栏显示撤销/重做可用状态（需要 `can_undo()` / `can_redo()`）

### 中期

- [ ] 多图层支持
- [ ] 选区与复制/粘贴
- [ ] 画笔大小调整（1x1、2x2、3x3 等）
- [ ] 自定义调色板持久化

### 远期

- [ ] 动画帧编辑（多帧画布 + 帧切换）
- [ ] 终端预览播放动画
- [ ] GIF 导出

## 已知问题

- **Ctrl+S 保存无确认覆盖**：保存时如果文件已存在会直接覆盖，不会提示用户确认
- **大图片导入可能卡顿**：导入超过 u16 范围（65535×65535）的 PNG 图片时，超出范围的像素会被静默跳过
- **旧版 ConHost 兼容性**：Windows 旧版 ConHost 不支持 Ctrl+滚轮缩放等现代终端特性，推荐使用 Windows Terminal
- **命令模式无补全**：命令模式不支持 Tab 补全或历史记录上下翻页

## 更新日志

### v0.1.1-alpha.2 (开发中)

**核心功能**：
- 基于 HashMap 稀疏存储的无限画布
- 鼠标左键绘制、右键擦除、拖拽连续绘制
- 键盘方向键导航、空格绘制、Delete 擦除
- 10 色调色板（数字键 0-9 快速切换）
- 自定义 Hex 颜色输入（`#RRGGBB`）
- 撤销/重做（Ctrl+Z/Y），拖拽笔画作为整体操作单元
- 命令模式（ESC 进入）：help/save/load/quit/paint/undo/redo/color/clear
- paint 命令支持坐标范围批量绘制（`paint x1:x2 y1:y2`）
- 帮助覆盖层（`:help` 命令或 F1）

**文件格式**：
- `.ptd` 压缩存储格式（gzip 压缩 JSON，默认格式）
- `.json` 纯文本格式（兼容，便于人工阅读）
- PNG 导出（`export` 命令，RGBA 图片）
- PNG 导入（`import` 命令，支持多种颜色类型）

**渲染优化**：
- 两级渲染架构（全屏渲染 + 增量渲染）
- BufWriter 64KB 缓冲消除画面撕裂
- 事件驱动渲染 + 批量事件消费
- 同色像素批量合并输出

**构建优化**：
- Fat LTO + 单 codegen unit + panic=abort + strip
- 直接使用 png crate 替代 image crate（减少 6 个传递依赖）
