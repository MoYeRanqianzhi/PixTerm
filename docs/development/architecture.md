# PixTerm 架构说明

## 项目结构

```
src/
├── main.rs          # 入口：终端初始化/恢复、panic hook、启动 App
├── app.rs           # App 核心状态、事件循环、模式管理、业务逻辑
├── canvas.rs        # Canvas 画布数据结构（基于 HashMap 的无限稀疏像素网格）
├── renderer.rs      # 终端渲染：画布区域 + 光标 + 全屏/增量渲染
├── input.rs         # 键盘/鼠标事件处理与分发（返回重绘提示）
├── command.rs       # 命令模式解析（字符串 → Command 枚举，支持坐标范围）
├── history.rs       # 撤销/重做历史栈管理
├── color.rs         # 颜色工具：Hex 解析、调色板、预设色
├── file.rs          # JSON 文件保存/加载（基于边界框序列化）
└── ui.rs            # 状态栏、调色板栏、命令行、帮助面板 UI 组件
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
    /// 稀疏像素存储：键为 (x, y) 坐标，值为 RGB 颜色
    /// 不存在的键即为空像素
    pixels: HashMap<(u16, u16), Rgb>,
}
```

- 基于 `HashMap<(u16, u16), Rgb>` 的无限大小稀疏存储
- 无边界限制，任意 `u16` 坐标均合法
- `get_pixel(x, y)` → `Option<Rgb>`（不存在的键返回 `None`）
- `set_pixel(x, y, color)` → `Some` 插入，`None` 删除
- `clear()` 清空所有像素
- `bounding_box()` → 计算所有像素的最小包围矩形（供保存使用）

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

- `push_undo(entries)` — 推入操作组，自动清空 redo 栈
- `undo()` / `redo()` — 弹出并返回操作组
- 支持批量操作：鼠标拖拽的多像素修改作为一组
- 范围绘制（`paint x1:x2 y1:y2`）的所有像素变化也作为一组

### Command（命令）— `command.rs`

```rust
pub enum Command {
    Help,
    Save(Option<String>),
    Load(Option<String>),
    Quit,
    Paint {
        x_range: (u16, u16),   // X 坐标范围，单值时 start == end
        y_range: (u16, u16),   // Y 坐标范围，单值时 start == end
        color: Option<Rgb>,    // None = 使用当前画笔颜色
    },
    Undo,
    Redo,
    Color(u16, u16),
    Clear,
    SetBrushColor(Rgb),
}
```

**坐标范围解析**：

`parse_coord_range(s)` 支持两种格式：
- 单值：`"5"` → `(5, 5)`
- 范围：`"5:10"` → `(5, 10)`，要求 start ≤ end

### App（应用状态）— `app.rs`

持有所有运行时状态：画布、历史、模式、画笔、调色板、视口、光标等。

关键字段：
- `canvas: Canvas` — 无限大小画布
- `history: History` — 撤销/重做管理
- `mode: AppMode` — 绘制模式 / 命令模式
- `brush_color: Rgb` — 当前画笔颜色
- `palette: [Rgb; 10]` — 数字键 0-9 对应的调色板
- `viewport_x/y: i32` — 视口偏移
- `cursor_x/y: u16` — 键盘光标坐标
- `force_full_redraw: bool` — 全屏重绘标记（键盘/窗口事件设置）
- `dirty_pixels: Vec<(u16, u16)>` — 脏像素列表（增量渲染用）

关键方法：
- `run()` — 事件循环（poll → handle → 选择渲染策略）
- `paint_pixel()` — 单次像素操作（立即入历史栈）
- `paint_pixel_stroke()` — 拖拽笔画像素（累积到临时 stroke + 记录脏像素）
- `finish_stroke()` — 结束笔画（整体入历史栈）
- `screen_to_canvas()` — 终端坐标 → 画布逻辑坐标
- `execute_command()` — 命令执行分发（支持范围绘制）

## 坐标系与像素映射

### 坐标系统

- **画布坐标**：逻辑像素坐标 `(canvas_x, canvas_y)`，范围 `[0, 65535]`
- **终端坐标**：字符位置 `(col, row)`，由 `crossterm` 提供
- **视口偏移**：`(viewport_x, viewport_y)` 控制画布在终端中的显示位置

### 像素映射（固定大小）

每个逻辑像素在终端中占据：
- **宽度**：2 列（终端字符宽高比约 1:2，2 列使像素接近正方形）
- **高度**：1 行

坐标转换：
```
canvas_x = (term_col + viewport_x) / 2
canvas_y = term_row + viewport_y
```

状态栏占据底部 2 行（不参与画布渲染区域）。

## 渲染管线

### 两级渲染架构

PixTerm 使用两级渲染策略来最小化画面抖动：

#### 1. 全屏渲染（`render_frame`）

用于需要刷新整个画面的场景：
- 键盘操作（模式切换、快捷键、光标移动等）
- 窗口尺寸变化
- 初始渲染

渲染流程：
1. **画布区域** — 逐行扫描，同色像素批量合并输出
2. **键盘光标** — 在光标位置绘制反色 `[]` 标记
3. **状态栏** — 模式标签、画笔色块、坐标、消息
4. **调色板栏/命令行** — 候选画笔或命令输入
5. **帮助覆盖层** — 居中帮助面板（可选）

#### 2. 增量渲染（`render_dirty_pixels`）

用于鼠标拖拽绘制场景：
- 仅更新发生变化的像素单元格
- 不触碰画布其余区域、状态栏、光标
- 极低开销：每个脏像素仅 1 次 MoveTo + 1 次 Print

### 渲染优化策略

| 优化 | 效果 |
|------|------|
| **BufWriter 64KB** | 所有输出缓冲在内存，flush 时一次性写入，消除画面撕裂 |
| **事件驱动渲染** | 仅在事件发生后渲染，无事件时不重绘 |
| **批量事件消费** | 消费所有待处理事件后才渲染一次，减少中间帧 |
| **同色像素合并** | 相邻同色像素合并为单次 Print，减少转义码数量 |
| **增量渲染** | 鼠标绘制时仅更新变化的像素，避免全屏重绘 |
| **Moved 事件过滤** | 忽略无按键的鼠标移动事件，不触发任何渲染 |
| **预分配缓冲** | 空格字符串预分配，避免渲染热路径中的堆分配 |

## 事件处理流程

`handle_event` 返回 `bool` 表示是否需要重绘，同时通过 `app.force_full_redraw` 标记全屏重绘需求。

```
Event::Key → force_full_redraw = true, return true
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

Event::Mouse → return bool (无 force_full_redraw)
  ├── Down(Left)  → 开始绘制笔画 → dirty_pixels
  ├── Down(Right) → 开始擦除笔画 → dirty_pixels
  ├── Drag(Left)  → 连续绘制 → dirty_pixels
  ├── Drag(Right) → 连续擦除 → dirty_pixels
  ├── Up          → 结束笔画（推入历史栈）→ return false
  └── Moved/其他  → return false（不触发任何渲染）

Event::Resize → force_full_redraw = true, return true
```

事件循环渲染决策：
```
if needs_redraw:
    if force_full_redraw → 全屏渲染
    elif dirty_pixels 非空 → 增量渲染
```

## 文件格式

JSON 格式，由 `serde` 序列化/反序列化。

保存时通过 `bounding_box()` 计算最小包围矩形，仅序列化有内容的区域：

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

`pixels[y][x]`：`null` 为空，`[r, g, b]` 为有色像素。坐标相对于边界框原点。

加载时遍历二维数组，将非 null 像素插入 HashMap。

## Windows 平台注意事项

- **KeyEventKind 过滤**：Windows 下 crossterm 报告 Press/Release/Repeat 三种键盘事件，必须过滤仅处理 Press 事件
- **鼠标捕获**：crossterm 在 Windows 上使用 Windows Console API 而非 ANSI 转义码，不可混用原始 ANSI 代码
- **Ctrl+滚轮缩放**：Windows Terminal 内置功能，在终端层面处理，应用无需（也无法）干预
- **旧版 ConHost**：不支持 Ctrl+滚轮缩放等现代终端特性
