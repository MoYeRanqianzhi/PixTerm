# PixTerm 架构说明

## 项目结构

```
src/
├── main.rs          # 入口：终端初始化/恢复、panic hook、启动 App
├── app.rs           # App 核心状态、事件循环、模式管理、业务逻辑
├── canvas.rs        # Canvas 画布数据结构（二维像素网格）
├── renderer.rs      # 终端渲染：画布区域 + 光标 + 状态栏 + 命令行
├── input.rs         # 键盘/鼠标事件处理与分发
├── command.rs       # 命令模式解析（字符串 → Command 枚举）
├── history.rs       # 撤销/重做历史栈管理
├── color.rs         # 颜色工具：Hex 解析、调色板、预设色
├── file.rs          # JSON 文件保存/加载
└── ui.rs            # 状态栏、命令行、帮助面板 UI 组件
```

## 模块依赖关系

```
main.rs
  └── app.rs
        ├── canvas.rs    （画布数据）
        ├── color.rs     （颜色工具）
        ├── command.rs   （命令解析）
        ├── file.rs      （文件 I/O）
        ├── history.rs   （撤销/重做）
        ├── input.rs     （事件处理 → 调用 App 方法）
        ├── renderer.rs  （渲染画面）
        │     └── ui.rs  （UI 组件）
        └── ui.rs        （DisplayMode 枚举）
```

## 核心数据结构

### Canvas（画布）— `canvas.rs`

```rust
pub struct Canvas {
    pub width: u16,                          // 画布宽度（列数）
    pub height: u16,                         // 画布高度（行数）
    pub pixels: Vec<Vec<Option<Rgb>>>,       // pixels[y][x]，None = 空像素
}
```

- `get_pixel(x, y)` / `set_pixel(x, y, color)` 带边界检查
- `clear()` 重置所有像素为 None
- `resize()` 调整尺寸（裁剪/扩展）

### History（历史管理）— `history.rs`

```rust
pub struct HistoryEntry {
    pub x: u16, pub y: u16,
    pub old_color: Option<Rgb>,
    pub new_color: Option<Rgb>,
}

pub struct History {
    undo_stack: Vec<Vec<HistoryEntry>>,  // 每组 = 一次操作
    redo_stack: Vec<Vec<HistoryEntry>>,
}
```

- `push_undo(entries)` — 推入操作，自动清空 redo 栈
- `undo()` / `redo()` — 弹出并返回操作组
- 支持批量操作：鼠标拖拽的多像素修改作为一组

### App（应用状态）— `app.rs`

持有所有运行时状态：画布、历史、模式、画笔、调色板、缩放、视口、光标等。

关键方法：
- `run()` — 事件循环（poll → handle → render）
- `paint_pixel()` — 单次像素操作（立即入历史栈）
- `paint_pixel_stroke()` — 拖拽笔画像素（累积到临时 stroke）
- `finish_stroke()` — 结束笔画（整体入历史栈）
- `screen_to_canvas()` — 终端坐标 → 画布逻辑坐标
- `execute_command()` — 命令执行分发

## 坐标系与缩放

### 坐标系统

- **画布坐标**：逻辑像素坐标 `(canvas_x, canvas_y)`，范围 `[0, width)` x `[0, height)`
- **终端坐标**：字符位置 `(col, row)`，由 `crossterm` 提供
- **视口偏移**：`(viewport_x, viewport_y)` 控制画布在终端中的显示位置

### 缩放公式

每个逻辑像素在终端中占据：
- **宽度**：`2 * zoom` 列（因为终端字符宽高比约 1:2）
- **高度**：`zoom` 行

坐标转换：
```
canvas_x = (term_col + viewport_x) / (2 * zoom)
canvas_y = (term_row + viewport_y) / zoom
```

## 渲染管线

`render_frame()` 每帧执行：

1. **清屏** — 逐行逐列输出画布区域像素
   - 画布内空像素 → 棋盘格背景（区分空白和画布外）
   - 画布外 → 深灰色背景
2. **键盘光标** — 在光标位置绘制反色边框
3. **状态栏** — 模式标签、画笔色块、调色板、坐标、消息
4. **命令行** — 命令模式显示输入；绘制模式显示快捷键提示
5. **帮助覆盖层** — 居中显示帮助面板（可选）

所有渲染使用 `crossterm::queue!` 缓冲，最后 `flush()` 一次性输出。

## 事件处理流程

```
Event::Key
  ├── show_help == true → 关闭帮助
  ├── AppMode::Draw → handle_draw_mode_key()
  │     ├── Ctrl+Q → 退出
  │     ├── Ctrl+S → 保存
  │     ├── Ctrl+Z/Y → 撤销/重做
  │     ├── ESC → 进入命令模式
  │     ├── Space → 绘制像素
  │     ├── 0-9 → 切换调色板
  │     ├── 方向键 → 移动光标
  │     └── Delete → 擦除像素
  └── AppMode::Command → handle_command_mode_key()
        ├── ESC → 返回绘制模式
        ├── Enter → 执行命令
        ├── Backspace → 删除字符
        └── Char → 追加到缓冲区

Event::Mouse
  ├── Down(Left) → 开始绘制笔画
  ├── Down(Right) → 开始擦除笔画
  ├── Drag(Left/Right) → 连续绘制/擦除
  ├── Up → 结束笔画（推入历史栈）
  ├── ScrollUp → 放大
  └── ScrollDown → 缩小
```

## 文件格式

JSON 格式，由 `serde` 序列化/反序列化：

```json
{
  "version": "1.0.0",
  "width": 32,
  "height": 32,
  "pixels": [
    [null, [255, 0, 0], null, ...],
    ...
  ]
}
```

`pixels[y][x]`：`null` 为空，`[r, g, b]` 为有色像素。
