// PixTerm - App 模块
// 核心应用状态、事件循环、模式管理

use crate::canvas::Canvas;
use crate::color::{self, Rgb};
use crate::command::{self, Command};
use crate::file;
use crate::history::{History, HistoryEntry};
use crate::input;
use crate::renderer;
use crate::ui::DisplayMode;
use crossterm::event;
use std::io;
use std::path::Path;
use std::time::Duration;

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
    /// 画布数据
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
    /// 缩放等级（1-8），每个逻辑像素占 2*zoom 列 x zoom 行
    pub zoom: u16,
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
}

impl App {
    /// 创建新的应用实例
    /// `width`, `height` - 画布初始尺寸
    pub fn new(width: u16, height: u16) -> Self {
        // 获取默认调色板
        let palette = color::default_palette();
        Self {
            canvas: Canvas::new(width, height),
            history: History::new(),
            mode: AppMode::Draw,
            // 默认画笔颜色为调色板第一个颜色（黑色）
            brush_color: palette[0],
            palette,
            active_palette_index: 0,
            // 默认缩放等级 2（每像素 4 列 x 2 行）
            zoom: 2,
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
        }
    }

    /// 启动应用主事件循环
    /// 循环执行：渲染 → 等待事件 → 处理事件
    pub fn run(&mut self) -> io::Result<()> {
        // 获取标准输出句柄
        let mut stdout = io::stdout();

        // 初始渲染
        self.render_to(&mut stdout)?;

        // 主循环：持续运行直到 running 标记为 false
        while self.running {
            // 等待事件（200ms 超时，用于定期刷新状态消息等）
            if event::poll(Duration::from_millis(200))? {
                // 读取事件
                let evt = event::read()?;
                // 分发事件到 input 模块处理
                input::handle_event(self, evt);
            }
            // 每次循环都重新渲染（保证 UI 一致性）
            self.render_to(&mut stdout)?;
        }

        Ok(())
    }

    /// 渲染当前帧到指定输出
    fn render_to(&self, stdout: &mut io::Stdout) -> io::Result<()> {
        // 将 AppMode 转换为 UI 层的 DisplayMode
        let display_mode = match self.mode {
            AppMode::Draw => DisplayMode::Draw,
            AppMode::Command => DisplayMode::Command,
        };

        renderer::render_frame(
            stdout,
            &self.canvas,
            self.viewport_x,
            self.viewport_y,
            self.zoom,
            self.cursor_x,
            self.cursor_y,
            self.brush_color,
            &self.palette,
            self.active_palette_index,
            display_mode,
            &self.command_buffer,
            &self.status_message,
            self.show_help,
        )
    }

    /// 在画布上绘制单个像素（独立操作，立即推入历史栈）
    /// 用于键盘空格绘制、删除键擦除等单次操作
    pub fn paint_pixel(&mut self, x: u16, y: u16, color: Option<Rgb>) {
        // 边界检查
        if x >= self.canvas.width || y >= self.canvas.height {
            return;
        }
        // 获取旧颜色
        let old_color = self.canvas.get_pixel(x, y);
        // 如果颜色相同则不记录历史
        if old_color == color {
            return;
        }
        // 修改画布
        self.canvas.set_pixel(x, y, color);
        // 记录历史（作为独立的一次操作）
        let entry = HistoryEntry {
            x,
            y,
            old_color,
            new_color: color,
        };
        self.history.push_undo(vec![entry]);
    }

    /// 在拖拽笔画中绘制单个像素（不立即推入历史栈，而是累积到 current_stroke）
    /// 用于鼠标拖拽绘制，整个拖拽动作结束后才作为一组操作推入历史
    pub fn paint_pixel_stroke(&mut self, x: u16, y: u16, color: Option<Rgb>) {
        // 边界检查
        if x >= self.canvas.width || y >= self.canvas.height {
            return;
        }
        // 获取旧颜色
        let old_color = self.canvas.get_pixel(x, y);
        // 如果颜色相同则跳过（避免重复记录拖拽经过同一像素）
        if old_color == color {
            return;
        }
        // 修改画布
        self.canvas.set_pixel(x, y, color);
        // 记录到当前笔画的临时历史中
        self.current_stroke.push(HistoryEntry {
            x,
            y,
            old_color,
            new_color: color,
        });
    }

    /// 结束鼠标拖拽笔画，将累积的像素修改作为一组操作推入历史栈
    pub fn finish_stroke(&mut self) {
        self.is_drawing = false;
        if !self.current_stroke.is_empty() {
            // 将整个笔画作为一次历史操作
            let stroke = std::mem::take(&mut self.current_stroke);
            self.history.push_undo(stroke);
        }
    }

    /// 执行撤销操作
    /// 从历史栈弹出最近的操作组，将画布上的像素恢复为旧颜色
    pub fn apply_undo(&mut self) {
        if let Some(entries) = self.history.undo() {
            // 逆序恢复像素（保证多像素操作的正确回退顺序）
            for entry in entries.iter().rev() {
                self.canvas.set_pixel(entry.x, entry.y, entry.old_color);
            }
            self.status_message = "已撤销".to_string();
        } else {
            self.status_message = "没有可撤销的操作".to_string();
        }
    }

    /// 执行重做操作
    /// 从重做栈弹出最近被撤销的操作组，重新应用到画布
    pub fn apply_redo(&mut self) {
        if let Some(entries) = self.history.redo() {
            // 正序重新应用像素修改
            for entry in &entries {
                self.canvas.set_pixel(entry.x, entry.y, entry.new_color);
            }
            self.status_message = "已重做".to_string();
        } else {
            self.status_message = "没有可重做的操作".to_string();
        }
    }

    /// 保存画布到 JSON 文件
    /// `filename` - 可选文件名，`None` 时使用默认文件名 "canvas.json"
    pub fn save_canvas(&mut self, filename: Option<&str>) {
        let path_str = filename.unwrap_or("canvas.json");
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

    /// 从 JSON 文件加载画布
    /// `filename` - 可选文件名，`None` 时使用默认文件名 "canvas.json"
    pub fn load_canvas(&mut self, filename: Option<&str>) {
        let path_str = filename.unwrap_or("canvas.json");
        let path = Path::new(path_str);
        match file::load_canvas(path) {
            Ok(loaded) => {
                self.canvas = loaded;
                // 加载后清空历史（新画布没有旧历史）
                self.history.clear();
                // 重置光标位置到原点
                self.cursor_x = 0;
                self.cursor_y = 0;
                self.status_message = format!("已加载 {path_str}");
            }
            Err(e) => {
                self.status_message = format!("加载失败: {e}");
            }
        }
    }

    /// 执行解析后的命令
    /// `input` - 用户输入的命令字符串
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
            Some(Command::Paint(x, y, color)) => {
                self.paint_pixel(x, y, Some(color));
                self.status_message = format!(
                    "已在 ({x},{y}) 绘制 #{:02X}{:02X}{:02X}",
                    color.0, color.1, color.2
                );
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
            None => {
                self.status_message = format!("未知命令: {input}");
            }
        }
    }

    /// 将终端屏幕坐标转换为画布逻辑坐标
    /// 返回 `Some((canvas_x, canvas_y))` 如果坐标在画布范围内
    /// 返回 `None` 如果坐标在画布外或在状态栏区域
    pub fn screen_to_canvas(&self, col: u16, row: u16) -> Option<(u16, u16)> {
        // 获取终端尺寸以排除状态栏区域
        let (_, term_height) = crossterm::terminal::size().ok()?;
        let canvas_area_height = term_height.saturating_sub(renderer::STATUS_BAR_HEIGHT);

        // 点击位置在状态栏区域内则忽略
        if row >= canvas_area_height {
            return None;
        }

        // 每个逻辑像素在终端中的尺寸
        let pixel_width = 2 * self.zoom as i32;
        let pixel_height = self.zoom as i32;

        // 终端坐标 → 画布坐标
        let canvas_x_raw = col as i32 + self.viewport_x;
        let canvas_y_raw = row as i32 + self.viewport_y;

        // 计算画布逻辑坐标（向负无穷取整）
        let canvas_x = if canvas_x_raw >= 0 {
            canvas_x_raw / pixel_width
        } else {
            return None; // 画布左侧外
        };
        let canvas_y = if canvas_y_raw >= 0 {
            canvas_y_raw / pixel_height
        } else {
            return None; // 画布上方外
        };

        // 边界检查
        if canvas_x >= 0
            && canvas_x < self.canvas.width as i32
            && canvas_y >= 0
            && canvas_y < self.canvas.height as i32
        {
            Some((canvas_x as u16, canvas_y as u16))
        } else {
            None
        }
    }

    /// 确保键盘光标在当前视口中可见
    /// 如果光标移出了可见区域，自动调整视口偏移
    pub fn ensure_cursor_visible(&mut self) {
        // 获取终端尺寸
        let (term_width, term_height) = match crossterm::terminal::size() {
            Ok(size) => size,
            Err(_) => return,
        };
        let canvas_area_height = term_height.saturating_sub(renderer::STATUS_BAR_HEIGHT);

        // 每个逻辑像素在终端中的尺寸
        let pixel_width = 2 * self.zoom as i32;
        let pixel_height = self.zoom as i32;

        // 光标像素在终端中的位置
        let screen_col = self.cursor_x as i32 * pixel_width - self.viewport_x;
        let screen_row = self.cursor_y as i32 * pixel_height - self.viewport_y;

        // 如果光标太靠左，向左滚动视口
        if screen_col < 0 {
            self.viewport_x = self.cursor_x as i32 * pixel_width;
        }
        // 如果光标太靠右，向右滚动视口
        if screen_col + pixel_width > term_width as i32 {
            self.viewport_x = self.cursor_x as i32 * pixel_width - term_width as i32 + pixel_width;
        }
        // 如果光标太靠上，向上滚动视口
        if screen_row < 0 {
            self.viewport_y = self.cursor_y as i32 * pixel_height;
        }
        // 如果光标太靠下，向下滚动视口
        if screen_row + pixel_height > canvas_area_height as i32 {
            self.viewport_y =
                self.cursor_y as i32 * pixel_height - canvas_area_height as i32 + pixel_height;
        }
    }
}
