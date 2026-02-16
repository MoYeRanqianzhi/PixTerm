# PixTerm 架构说明

## 项目结构

```
pixterm/
├── Cargo.toml       # 项目配置、依赖声明、release 构建优化
├── README.md        # 项目简介、快捷键、文件格式
├── CLAUDE.md        # LLM 协作开发指南
├── docs/
│   └── development/
│       ├── architecture.md  # 本文件：模块架构详解
│       ├── user-guide.md    # 详细使用指南
│       └── todo.md          # 待办事项与版本历史
└── src/
    ├── main.rs      # 入口：终端初始化/恢复、panic hook、启动 App
    ├── app.rs       # App 核心状态、事件循环、模式管理、业务逻辑
    ├── canvas.rs    # Canvas 画布数据结构（基于 HashMap 的无限稀疏像素网格）
    ├── renderer.rs  # 终端渲染：画布区域 + 光标 + 全屏/增量渲染
    ├── input.rs     # 键盘/鼠标事件处理与分发（返回重绘提示）
    ├── command.rs   # 命令模式解析（字符串 → Command 枚举，支持坐标范围）
    ├── history.rs   # 撤销/重做历史栈管理
    ├── color.rs     # 颜色工具：Hex 解析、调色板、预设色
    ├── file.rs      # 文件 I/O：.ptd 压缩保存/加载、.json 纯文本、PNG 导入/导出
    └── ui.rs        # 状态栏、调色板栏、命令行、帮助面板 UI 组件
```

## 模块依赖关系

```
main.rs
  └── app.rs             ← 持有所有运行时状态，协调各模块
        ├── canvas.rs    （画布数据：HashMap 稀疏存储）
        ├── color.rs     （颜色工具：Hex 解析、调色板）
        ├── command.rs   （命令解析：字符串 → Command 枚举）
        ├── file.rs      （文件 I/O：PTD/JSON/PNG）
        │     └── 外部依赖：serde, serde_json, flate2, png
        ├── history.rs   （撤销/重做：双栈管理）
        ├── input.rs     （事件处理：键盘/鼠标 → 调用 App 方法）
        ├── renderer.rs  （渲染画面：全屏/增量两级策略）
        │     └── ui.rs  （UI 组件：状态栏、调色板栏、帮助面板）
        └── ui.rs        （DisplayMode 枚举）
```

**外部依赖**：

| crate | 用途 | 选型理由 |
|-------|------|---------|
| `crossterm` | 终端控制（raw mode、鼠标捕获、事件轮询） | 跨平台终端库，Windows 原生支持 |
| `serde` + `serde_json` | 画布数据序列化/反序列化 | Rust 生态标准序列化方案 |
| `flate2` | gzip 压缩/解压（.ptd 格式） | 纯 Rust 实现，无系统库依赖 |
| `png` | PNG 图片编解码 | 直接使用 png crate 而非 image crate，减少 6 个传递依赖 |

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
- 无边界限制，任意 `u16` 坐标均合法（范围 0–65535）
- `get_pixel(x, y)` → `Option<Rgb>`（不存在的键返回 `None`）
- `set_pixel(x, y, color)` → `Some` 插入，`None` 删除
- `clear()` 清空所有像素
- `bounding_box()` → 计算所有像素的最小包围矩形（供保存/导出使用）

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
    Save(Option<String>),      // 保存画布（默认 canvas.ptd）
    Load(Option<String>),      // 加载画布（默认 canvas.ptd）
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
    Export(Option<String>),     // 导出 PNG（默认 canvas.png）
    Import(String),            // 导入 PNG（文件名必填）
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
- `execute_command()` — 命令执行分发（支持范围绘制、导入/导出）
- `save_canvas()` / `load_canvas()` — 画布保存/加载（默认 .ptd 格式）
- `export_png()` — 导出画布为 PNG 图片
- `import_png()` — 从 PNG 导入像素（重置历史和光标）

## 坐标系与像素映射

### 坐标系统

- **画布坐标**：逻辑像素坐标 `(canvas_x, canvas_y)`，范围 `[0, 65535]`
- **终端坐标**：字符位置 `(col, row)`，由 `crossterm` 提供
- **视口偏移**：`(viewport_x, viewport_y)` 控制画布在终端中的显示位置

### 像素映射（固定大小）

每个逻辑像素在终端中占据：
- **宽度**：2 列（终端字符宽高比约 1:2，2 列使像素接近正方形）
- **高度**：1 行

坐标转换公式：
```
canvas_x = (term_col + viewport_x) / PIXEL_WIDTH     // PIXEL_WIDTH = 2
canvas_y = term_row + viewport_y                      // PIXEL_HEIGHT = 1
```

状态栏占据底部 2 行（`STATUS_BAR_HEIGHT = 2`），不参与画布渲染区域。

## 渲染管线

### 两级渲染架构

PixTerm 使用两级渲染策略来最小化画面抖动：

#### 1. 全屏渲染（`render_frame`）

用于需要刷新整个画面的场景：
- 键盘操作（模式切换、快捷键、光标移动等）
- 窗口尺寸变化
- 初始渲染

渲染流程：
1. **画布区域** — 逐行扫描，同色像素批量合并输出（`render_canvas_area`）
2. **键盘光标** — 在光标位置绘制反色 `[]` 标记（`render_cursor`）
3. **状态栏** — 模式标签、画笔色块、坐标、消息（`ui::render_status_bar`）
4. **调色板栏/命令行** — 候选画笔或命令输入（`ui::render_command_line`）
5. **帮助覆盖层** — 居中帮助面板（`ui::render_help`，可选）

#### 2. 增量渲染（`render_dirty_pixels`）

用于鼠标拖拽绘制场景：
- 仅更新发生变化的像素单元格
- 不触碰画布其余区域、状态栏、光标
- 极低开销：每个脏像素仅 1 次 MoveTo + 1 次 Print

### 渲染优化策略

| 优化 | 实现位置 | 效果 |
|------|---------|------|
| **BufWriter 64KB** | `app.rs` `run()` | 所有输出缓冲在内存，flush 时一次性写入，消除画面撕裂 |
| **事件驱动渲染** | `app.rs` `run()` | 仅在事件发生后渲染，无事件时不重绘 |
| **批量事件消费** | `app.rs` `run()` | 消费所有待处理事件后才渲染一次，减少中间帧 |
| **同色像素合并** | `renderer.rs` `render_canvas_area()` | 相邻同色像素合并为单次 Print，减少转义码数量 |
| **增量渲染** | `renderer.rs` `render_dirty_pixels()` | 鼠标绘制时仅更新变化的像素，避免全屏重绘 |
| **Moved 事件过滤** | `input.rs` `handle_mouse_event()` | 忽略无按键的鼠标移动事件，不触发任何渲染 |
| **预分配缓冲** | `renderer.rs` `render_canvas_area()` | 空格字符串预分配，避免渲染热路径中的堆分配 |

## 事件处理流程

`handle_event` 返回 `bool` 表示是否需要重绘，同时通过 `app.force_full_redraw` 标记全屏重绘需求。

```
Event::Key → force_full_redraw = true, return true
  ├── show_help == true → 关闭帮助
  ├── AppMode::Draw → handle_draw_mode_key()
  │     ├── Ctrl+Q → 退出
  │     ├── Ctrl+S → 保存（默认 canvas.ptd）
  │     ├── Ctrl+Z/Y → 撤销/重做
  │     ├── ESC → 进入命令模式
  │     ├── Space → 绘制像素
  │     ├── ` (反引号) → 切换光标显示
  │     ├── 0-9 → 切换调色板
  │     ├── 方向键 → 移动光标 + ensure_cursor_visible()
  │     └── Delete → 擦除像素
  └── AppMode::Command → handle_command_mode_key()
        ├── ESC → 返回绘制模式
        ├── Enter → 执行命令 (execute_command)
        ├── Backspace → 删除字符
        └── Char → 追加到缓冲区（忽略 Ctrl 组合）

Event::Mouse → return bool (无 force_full_redraw)
  ├── Down(Left)  → 开始绘制笔画 → dirty_pixels
  ├── Down(Right) → 开始擦除笔画 → dirty_pixels
  ├── Drag(Left)  → 连续绘制 → dirty_pixels
  ├── Drag(Right) → 连续擦除 → dirty_pixels
  ├── Up          → 结束笔画（推入历史栈）→ return false
  └── Moved/其他  → return false（不触发任何渲染）

Event::Resize → force_full_redraw = true, return true
```

事件循环渲染决策（`app.rs` `run()`）：
```
if needs_redraw:
    if force_full_redraw → render_frame()（全屏渲染）
    elif dirty_pixels 非空 → render_dirty_pixels()（增量渲染）
```

## 文件 I/O — `file.rs`

### 数据结构

所有画布数据共用 `SaveData` 序列化结构（.ptd 和 .json 共用）：

```rust
#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub version: String,          // 格式版本号（"1.0.0"）
    pub width: u16,               // 边界框宽度
    pub height: u16,              // 边界框高度
    pub pixels: Vec<Vec<Option<[u8; 3]>>>,  // pixels[y][x]
}
```

JSON 表示：
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

`pixels[y][x]`：`null` 为空像素，`[r, g, b]` 为有色像素。坐标相对于边界框原点。

### 保存/加载流程

保存时通过 `bounding_box()` 计算最小包围矩形，仅序列化有内容的区域。
加载时遍历二维数组，将非 null 像素插入 HashMap。

格式选择由文件扩展名决定：

| 格式 | 扩展名 | 序列化 | 存储方式 |
|------|--------|--------|---------|
| PTD | `.ptd` | `serde_json::to_string`（紧凑） | gzip 压缩 (`GzEncoder`) |
| JSON | `.json` / 其他 | `serde_json::to_string_pretty` | 纯文本写入 |

### PNG 导入/导出

直接使用 `png` crate（而非 `image` crate）以减少传递依赖：

**导出**（`export_png`）：
1. `bounding_box()` 计算尺寸（空画布 → 1×1）
2. 构建 RGBA 像素缓冲区（`vec![0u8; w*h*4]`），有色像素设置 alpha=255
3. `png::Encoder` 配置 RGBA/8-bit → `write_header()` → `write_image_data()`

**导入**（`import_png`）：
1. `png::Decoder` + `EXPAND | STRIP_16` 变换（索引色→RGB，16-bit→8-bit）
2. `read_info()` → `next_frame()` 读取像素缓冲区
3. 根据 `color_type`（Grayscale/GrayscaleAlpha/Rgb/Rgba）解析每个像素
4. alpha > 0 的像素写入画布

## 构建优化 — `Cargo.toml`

`[profile.release]` 配置了以下编译优化选项，牺牲编译时间换取运行性能和二进制体积：

```toml
[profile.release]
opt-level = 3         # 最高优化等级
lto = "fat"           # 跨所有 crate 全链接时优化
codegen-units = 1     # 单编译单元，允许 LLVM 全局优化
panic = "abort"       # 移除 unwind 表，减小体积
strip = true          # 剥离调试符号
```

| 选项 | 效果 |
|------|------|
| `lto = "fat"` | 跨 crate 内联和死代码消除，缩小体积、提升运行速度 |
| `codegen-units = 1` | 放弃编译并行性，换取更优的全局优化 |
| `panic = "abort"` | 移除 unwind 相关代码（landing pad、.eh_frame），体积显著减小 |
| `strip = true` | 剥离符号表和调试信息，体积进一步缩小 |

## 测试策略

每个模块包含独立的 `#[cfg(test)] mod tests` 测试模块：

| 模块 | 测试内容 |
|------|---------|
| `canvas.rs` | 像素读写、无限坐标、边界框计算、清空 |
| `color.rs` | Hex 颜色解析（合法/非法）、格式化 |
| `command.rs` | 各命令解析（help/save/load/export/import/paint/undo/redo/color/clear/hex）、边界情况 |
| `file.rs` | JSON 往返、PTD 压缩往返、PNG 往返、空画布处理、文件不存在 |
| `history.rs` | 撤销/重做、批量操作、新操作清空 redo 栈、空操作忽略 |

运行所有测试：

```bash
cargo test
```

## Windows 平台注意事项

- **KeyEventKind 过滤**：Windows 下 crossterm 报告 Press/Release/Repeat 三种键盘事件，`input.rs` 中必须过滤仅处理 `KeyEventKind::Press` 事件，否则每个按键被执行两次
- **鼠标捕获**：crossterm 在 Windows 上使用 Windows Console API 而非 ANSI 转义码，不可混用原始 ANSI 代码
- **Ctrl+滚轮缩放**：Windows Terminal 内置功能，在终端层面处理，应用无需（也无法）干预
- **旧版 ConHost**：不支持 Ctrl+滚轮缩放等现代终端特性，推荐使用 Windows Terminal
