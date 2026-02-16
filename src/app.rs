// PixTerm - App 模块
// 核心应用状态、事件循环、模式管理
// 优化：BufWriter 缓冲输出 + 事件驱动渲染（仅在事件发生时重绘）

use crate::canvas::Canvas;
use crate::color::{self, Rgb};
use crate::command::{self, Command};
use crate::file;
use crate::history::{History, HistoryEntry};
use crate::input;
use crate::renderer;
use crate::ui::DisplayMode;
use crossterm::event;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::time::Duration;

/// 固定像素宽度：每个逻辑像素占 2 终端列
const PIXEL_WIDTH: i32 = 2;
/// 固定像素高度：每个逻辑像素占 1 终端行
const PIXEL_HEIGHT: i32 = 1;

/// 应用模式枚举
#[derive(Clone, Copy, PartialEq)]
pub enum AppMode {
    /// 绘制模式：鼠标/键盘操作画布
    Draw,
    /// 命令模式：输入文本命令
    Command,
}

/// 应用核心状态结构体
/// 持有画布、历史管理器、UI 状态等所有运行时数据
pub struct App {
    /// 画布数据（无限大小，基于 HashMap 稀疏存储）
    pub canvas: Canvas,
    /// 撤销/重做历史管理器
    pub history: History,
    /// 当前应用模式（绘制/命令）
    pub mode: AppMode,
    /// 当前画笔颜色
    pub brush_color: Rgb,
    /// 调色板：数字键 0-9 对应的 10 个颜色
    pub palette: [Rgb; 10],
    /// 当前选中的调色板索引
    pub active_palette_index: usize,
    /// 视口 X 偏移（终端列 0 对应画布坐标系中的位置）
    pub viewport_x: i32,
    /// 视口 Y 偏移（终端行 0 对应画布坐标系中的位置）
    pub viewport_y: i32,
    /// 键盘光标在画布上的逻辑 X 坐标
    pub cursor_x: u16,
    /// 键盘光标在画布上的逻辑 Y 坐标
    pub cursor_y: u16,
    /// 命令模式输入缓冲区
    pub command_buffer: String,
    /// 底部状态消息文本
    pub status_message: String,
    /// 是否显示帮助覆盖层
    pub show_help: bool,
    /// 是否继续运行主循环
    pub running: bool,
    /// 鼠标拖拽绘制中标记
    pub is_drawing: bool,
    /// 当前拖拽笔画的临时历史记录（鼠标释放时整体推入历史栈）
    pub current_stroke: Vec<HistoryEntry>,
    /// 是否显示键盘光标（首次使用方向键/空格后才激活，避免初始状态下左上角出现多余的 [] 标记）
    pub show_cursor: bool,
    /// 强制全屏重绘标记（键盘操作、窗口尺寸变化等需要整体刷新时设置）
    pub force_full_redraw: bool,
    /// 脏像素列表：记录自上次渲染以来发生变化的画布坐标
    /// 用于增量渲染，鼠标拖拽绘制时仅更新变化的像素而非全屏重绘
    pub dirty_pixels: Vec<(u16, u16)>,
}

impl App {
    /// 创建新的应用实例（无限画布，无需指定尺寸）
    pub fn new() -> Self {
        let palette = color::default_palette();
        Self {
            canvas: Canvas::new(),
            history: History::new(),
            mode: AppMode::Draw,
            // 默认画笔颜色为调色板第一个颜色（黑色）
            brush_color: palette[0],
            palette,
            active_palette_index: 0,
            viewport_x: 0,
            viewport_y: 0,
            cursor_x: 0,
            cursor_y: 0,
            command_buffer: String::new(),
            status_message: String::new(),
            show_help: false,
            running: true,
            is_drawing: false,
            current_stroke: Vec::new(),
            show_cursor: false,
            force_full_redraw: false,
            dirty_pixels: Vec::new(),
        }
    }

    /// 启动应用主事件循环
    /// 优化策略：
    /// 1. 使用 BufWriter 将整帧输出缓冲后一次性刷新，消除逐像素写入造成的画面撕裂
    /// 2. 仅在接收到事件后才渲染，避免无变化时的无谓重绘
    /// 3. 批量消费所有待处理事件后再渲染一次，减少拖拽绘制时的中间帧
    /// 4. 基于 needs_redraw 标记过滤无视觉变化的事件（如鼠标移动），进一步减少重绘次数
    /// 5. 两级渲染：全屏重绘（键盘/窗口事件）与增量渲染（鼠标绘制仅更新脏像素）
    pub fn run(&mut self) -> io::Result<()> {
        // BufWriter 将所有 queue! 输出缓冲在内存中，flush 时一次性写入终端
        // 64KB 缓冲区足够容纳一帧完整的终端输出（120×40 终端约需 10-30KB）
        let stdout = io::stdout();
        let mut writer = BufWriter::with_capacity(65536, stdout.lock());

        // 初始渲染（显示空画布和状态栏）
        self.render_to(&mut writer)?;

        // 主循环：持续运行直到 running 标记为 false
        while self.running {
            // 阻塞等待事件，超时 100ms（仅用于周期性检查退出条件）
            if event::poll(Duration::from_millis(100))? {
                // 处理第一个事件，记录是否需要重绘
                let evt = event::read()?;
                let mut needs_redraw = input::handle_event(self, evt);

                // 批量消费所有已就绪的事件，避免逐事件渲染
                // poll(0ms) 为非阻塞检查：有事件则立即读取，无事件则跳出
                while self.running && event::poll(Duration::from_millis(0))? {
                    let evt = event::read()?;
                    // 任何一个事件需要重绘则标记为 true（位或累积）
                    needs_redraw |= input::handle_event(self, evt);
                }

                // 根据变化类型选择渲染策略
                if needs_redraw {
                    if self.force_full_redraw {
                        // 全屏重绘：键盘操作、窗口尺寸变化、模式切换等
                        // 需要刷新整个画面（画布 + 状态栏 + 命令行 + 光标）
                        self.render_to(&mut writer)?;
                        self.force_full_redraw = false;
                        self.dirty_pixels.clear();
                    } else if !self.dirty_pixels.is_empty() {
                        // 增量渲染：鼠标拖拽绘制时仅更新发生变化的像素单元格
                        // 避免全屏重绘带来的画面抖动，大幅提升绘制流畅度
                        renderer::render_dirty_pixels(
                            &mut writer,
                            &self.canvas,
                            &self.dirty_pixels,
                            self.viewport_x,
                            self.viewport_y,
                        )?;
                        self.dirty_pixels.clear();
                    }
                }
            }
            // 超时时不渲染——没有事件意味着没有变化
        }

        Ok(())
    }

    /// 渲染当前帧到指定缓冲写入器
    fn render_to(&self, w: &mut impl Write) -> io::Result<()> {
        let display_mode = match self.mode {
            AppMode::Draw => DisplayMode::Draw,
            AppMode::Command => DisplayMode::Command,
        };

        renderer::render_frame(
            w,
            &self.canvas,
            self.viewport_x,
            self.viewport_y,
            self.cursor_x,
            self.cursor_y,
            self.brush_color,
            &self.palette,
            self.active_palette_index,
            display_mode,
            &self.command_buffer,
            &self.status_message,
            self.show_help,
            self.show_cursor,
        )
    }

    /// 在画布上绘制单个像素（独立操作，立即推入历史栈）
    /// 无限画布：任意 u16 坐标均合法，无边界检查
    pub fn paint_pixel(&mut self, x: u16, y: u16, color: Option<Rgb>) {
        let old_color = self.canvas.get_pixel(x, y);
        // 颜色未变则跳过（避免产生无意义的历史记录）
        if old_color == color {
            return;
        }
        self.canvas.set_pixel(x, y, color);
        let entry = HistoryEntry {
            x,
            y,
            old_color,
            new_color: color,
        };
        self.history.push_undo(vec![entry]);
    }

    /// 在拖拽笔画中绘制单个像素（累积到 current_stroke，不立即入栈）
    /// 同时将变化的坐标推入 dirty_pixels 以供增量渲染使用
    /// 无限画布：任意 u16 坐标均合法，无边界检查
    pub fn paint_pixel_stroke(&mut self, x: u16, y: u16, color: Option<Rgb>) {
        let old_color = self.canvas.get_pixel(x, y);
        // 颜色未变则跳过
        if old_color == color {
            return;
        }
        self.canvas.set_pixel(x, y, color);
        self.current_stroke.push(HistoryEntry {
            x,
            y,
            old_color,
            new_color: color,
        });
        // 记录脏像素坐标，用于增量渲染（仅更新该像素而非全屏重绘）
        self.dirty_pixels.push((x, y));
    }

    /// 结束鼠标拖拽笔画，将累积的修改作为一组操作推入历史栈
    pub fn finish_stroke(&mut self) {
        self.is_drawing = false;
        if !self.current_stroke.is_empty() {
            let stroke = std::mem::take(&mut self.current_stroke);
            self.history.push_undo(stroke);
        }
    }

    /// 执行撤销操作
    pub fn apply_undo(&mut self) {
        if let Some(entries) = self.history.undo() {
            for entry in entries.iter().rev() {
                self.canvas.set_pixel(entry.x, entry.y, entry.old_color);
            }
            self.status_message = "已撤销".to_string();
        } else {
            self.status_message = "没有可撤销的操作".to_string();
        }
    }

    /// 执行重做操作
    pub fn apply_redo(&mut self) {
        if let Some(entries) = self.history.redo() {
            for entry in &entries {
                self.canvas.set_pixel(entry.x, entry.y, entry.new_color);
            }
            self.status_message = "已重做".to_string();
        } else {
            self.status_message = "没有可重做的操作".to_string();
        }
    }

    /// 保存画布到文件
    /// 默认使用 .ptd 压缩格式，也可通过指定文件名使用 .json 格式
    pub fn save_canvas(&mut self, filename: Option<&str>) {
        let path_str = filename.unwrap_or("canvas.ptd");
        let path = Path::new(path_str);
        match file::save_canvas(&self.canvas, path) {
            Ok(()) => {
                self.status_message = format!("已保存到 {path_str}");
            }
            Err(e) => {
                self.status_message = format!("保存失败: {e}");
            }
        }
    }

    /// 从文件加载画布
    /// 默认使用 .ptd 压缩格式，也可通过指定文件名使用 .json 格式
    pub fn load_canvas(&mut self, filename: Option<&str>) {
        let path_str = filename.unwrap_or("canvas.ptd");
        let path = Path::new(path_str);
        match file::load_canvas(path) {
            Ok(loaded) => {
                self.canvas = loaded;
                self.history.clear();
                self.cursor_x = 0;
                self.cursor_y = 0;
                self.status_message = format!("已加载 {path_str}");
            }
            Err(e) => {
                self.status_message = format!("加载失败: {e}");
            }
        }
    }

    /// 导出画布为 PNG 图片
    /// `filename` - 可选文件名，默认 "canvas.png"
    pub fn export_png(&mut self, filename: Option<&str>) {
        let path_str = filename.unwrap_or("canvas.png");
        let path = Path::new(path_str);
        match file::export_png(&self.canvas, path) {
            Ok(()) => {
                self.status_message = format!("已导出到 {path_str}");
            }
            Err(e) => {
                self.status_message = format!("导出失败: {e}");
            }
        }
    }

    /// 从 PNG 图片导入像素到画布
    /// 导入后清空历史记录并重置光标位置
    /// `filename` - PNG 文件名
    pub fn import_png(&mut self, filename: &str) {
        let path = Path::new(filename);
        match file::import_png(path) {
            Ok(loaded) => {
                self.canvas = loaded;
                self.history.clear();
                self.cursor_x = 0;
                self.cursor_y = 0;
                self.viewport_x = 0;
                self.viewport_y = 0;
                self.status_message = format!("已导入 {filename}");
            }
            Err(e) => {
                self.status_message = format!("导入失败: {e}");
            }
        }
    }

    /// 执行解析后的命令
    pub fn execute_command(&mut self, input: &str) {
        match command::parse_command(input) {
            Some(Command::Help) => {
                self.show_help = true;
            }
            Some(Command::Save(filename)) => {
                self.save_canvas(filename.as_deref());
            }
            Some(Command::Load(filename)) => {
                self.load_canvas(filename.as_deref());
            }
            Some(Command::Quit) => {
                self.running = false;
            }
            Some(Command::Paint { x_range, y_range, color }) => {
                // 使用指定颜色或当前画笔颜色
                let paint_color = color.unwrap_or(self.brush_color);
                let (x1, x2) = x_range;
                let (y1, y2) = y_range;
                // 收集所有实际发生变化的像素，作为一组操作推入历史栈
                let mut entries = Vec::new();
                for y in y1..=y2 {
                    for x in x1..=x2 {
                        let old_color = self.canvas.get_pixel(x, y);
                        let new_color = Some(paint_color);
                        // 跳过颜色未变的像素，避免产生无意义的历史记录
                        if old_color != new_color {
                            self.canvas.set_pixel(x, y, new_color);
                            entries.push(HistoryEntry {
                                x,
                                y,
                                old_color,
                                new_color,
                            });
                        }
                    }
                }
                if !entries.is_empty() {
                    let count = entries.len();
                    self.history.push_undo(entries);
                    // 单像素和范围使用不同的状态消息格式
                    if x1 == x2 && y1 == y2 {
                        self.status_message = format!(
                            "已在 ({x1},{y1}) 绘制 #{:02X}{:02X}{:02X}",
                            paint_color.0, paint_color.1, paint_color.2
                        );
                    } else {
                        self.status_message = format!(
                            "已绘制 {count} 像素 ({x1}:{x2},{y1}:{y2}) #{:02X}{:02X}{:02X}",
                            paint_color.0, paint_color.1, paint_color.2
                        );
                    }
                } else {
                    self.status_message = "无变化（目标区域已是相同颜色）".to_string();
                }
            }
            Some(Command::Undo) => {
                self.apply_undo();
            }
            Some(Command::Redo) => {
                self.apply_redo();
            }
            Some(Command::Color(x, y)) => {
                match self.canvas.get_pixel(x, y) {
                    Some(color) => {
                        self.status_message = format!(
                            "({x},{y}) 的颜色: #{:02X}{:02X}{:02X}",
                            color.0, color.1, color.2
                        );
                    }
                    None => {
                        self.status_message = format!("({x},{y}) 为空像素");
                    }
                }
            }
            Some(Command::Clear) => {
                self.canvas.clear();
                self.history.clear();
                self.status_message = "画布已清空".to_string();
            }
            Some(Command::SetBrushColor(color)) => {
                self.brush_color = color;
                self.status_message = format!(
                    "画笔颜色: #{:02X}{:02X}{:02X}",
                    color.0, color.1, color.2
                );
            }
            Some(Command::Export(filename)) => {
                self.export_png(filename.as_deref());
            }
            Some(Command::Import(filename)) => {
                self.import_png(&filename);
            }
            None => {
                self.status_message = format!("未知命令: {input}");
            }
        }
    }

    /// 将终端屏幕坐标转换为画布逻辑坐标
    /// 无限画布：只要坐标非负且可表示为 u16 即有效
    /// 返回 `None` 如果坐标为负数或在状态栏区域
    pub fn screen_to_canvas(&self, col: u16, row: u16) -> Option<(u16, u16)> {
        let (_, term_height) = crossterm::terminal::size().ok()?;
        let canvas_area_height = term_height.saturating_sub(renderer::STATUS_BAR_HEIGHT);

        // 点击在状态栏区域内则忽略
        if row >= canvas_area_height {
            return None;
        }

        // 终端坐标 → 画布逻辑坐标
        let canvas_x_raw = col as i32 + self.viewport_x;
        let canvas_y_raw = row as i32 + self.viewport_y;

        // 负坐标无效
        if canvas_x_raw < 0 || canvas_y_raw < 0 {
            return None;
        }

        let canvas_x = canvas_x_raw / PIXEL_WIDTH;
        let canvas_y = canvas_y_raw / PIXEL_HEIGHT;

        // 确保在 u16 范围内
        if canvas_x <= u16::MAX as i32 && canvas_y <= u16::MAX as i32 {
            Some((canvas_x as u16, canvas_y as u16))
        } else {
            None
        }
    }

    /// 确保键盘光标在当前视口中可见
    pub fn ensure_cursor_visible(&mut self) {
        let (term_width, term_height) = match crossterm::terminal::size() {
            Ok(size) => size,
            Err(_) => return,
        };
        let canvas_area_height = term_height.saturating_sub(renderer::STATUS_BAR_HEIGHT);

        // 光标像素在终端中的位置
        let screen_col = self.cursor_x as i32 * PIXEL_WIDTH - self.viewport_x;
        let screen_row = self.cursor_y as i32 * PIXEL_HEIGHT - self.viewport_y;

        // 水平方向：光标超出左边界时向左滚动，超出右边界时向右滚动
        if screen_col < 0 {
            self.viewport_x = self.cursor_x as i32 * PIXEL_WIDTH;
        }
        if screen_col + PIXEL_WIDTH > term_width as i32 {
            self.viewport_x = self.cursor_x as i32 * PIXEL_WIDTH - term_width as i32 + PIXEL_WIDTH;
        }
        // 垂直方向
        if screen_row < 0 {
            self.viewport_y = self.cursor_y as i32 * PIXEL_HEIGHT;
        }
        if screen_row + PIXEL_HEIGHT > canvas_area_height as i32 {
            self.viewport_y =
                self.cursor_y as i32 * PIXEL_HEIGHT - canvas_area_height as i32 + PIXEL_HEIGHT;
        }
    }
}
