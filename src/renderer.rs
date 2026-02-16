// PixTerm - Renderer 模块
// 终端渲染：画布 + 状态栏 + 命令行 + 帮助
// 优化策略：同色像素批量输出以减少转义码数量，配合 BufWriter 单次刷新

use crate::canvas::Canvas;
use crate::color::Rgb;
use crate::ui::{self, DisplayMode};
use crossterm::style;
use std::io::{self, Write};

/// 状态栏占用的终端行数（状态栏 1 行 + 调色板栏/命令行 1 行）
pub const STATUS_BAR_HEIGHT: u16 = 2;

/// 每个逻辑像素在终端中占 2 列（终端字符宽高比约 1:2，2 列使像素接近正方形）
const PIXEL_WIDTH: i32 = 2;
/// 每个逻辑像素在终端中占 1 行
const PIXEL_HEIGHT: i32 = 1;

/// 渲染完整的一帧画面
/// 包括：画布区域、键盘光标、状态栏、调色板栏/命令行、帮助覆盖层
/// 接受泛型 Writer 以支持 BufWriter 等缓冲写入器
pub fn render_frame(
    w: &mut impl Write,
    canvas: &Canvas,
    viewport_x: i32,
    viewport_y: i32,
    cursor_x: u16,
    cursor_y: u16,
    brush_color: Rgb,
    palette: &[Rgb; 10],
    active_palette_index: usize,
    mode: DisplayMode,
    command_buffer: &str,
    status_message: &str,
    show_help: bool,
    show_cursor: bool,
) -> io::Result<()> {
    // 获取终端尺寸
    let (term_width, term_height) = crossterm::terminal::size()?;

    // 画布可用渲染区域的高度（终端高度减去状态栏高度）
    let canvas_area_height = term_height.saturating_sub(STATUS_BAR_HEIGHT);

    // 隐藏终端光标以避免渲染过程中闪烁
    crossterm::queue!(w, crossterm::cursor::Hide)?;

    // ===== 1. 渲染画布区域（同色像素批量输出优化） =====
    render_canvas_area(w, canvas, viewport_x, viewport_y, term_width, canvas_area_height)?;

    // ===== 2. 渲染键盘光标（仅在用户使用过键盘导航后才显示） =====
    if show_cursor {
        render_cursor(
            w,
            cursor_x,
            cursor_y,
            viewport_x,
            viewport_y,
            term_width,
            canvas_area_height,
            brush_color,
        )?;
    }

    // ===== 3. 渲染状态栏（倒数第二行） =====
    ui::render_status_bar(
        w,
        canvas_area_height,
        term_width,
        mode,
        brush_color,
        cursor_x,
        cursor_y,
        status_message,
    )?;

    // ===== 4. 渲染调色板栏/命令行（最后一行） =====
    ui::render_command_line(
        w,
        canvas_area_height + 1,
        term_width,
        command_buffer,
        mode,
        palette,
        active_palette_index,
    )?;

    // ===== 5. 帮助覆盖层（如果开启） =====
    if show_help {
        ui::render_help(w, term_width, term_height)?;
    }

    // 一次性将整帧数据刷新到终端（BufWriter 在此处实际执行系统调用）
    w.flush()?;

    Ok(())
}

/// 渲染画布区域
/// 核心优化：将同一行中连续的相同颜色像素合并为一次 Print 调用
/// 空画布时每行仅输出一次 ResetColor + 整行空格，转义码数量极低
fn render_canvas_area(
    w: &mut impl Write,
    canvas: &Canvas,
    viewport_x: i32,
    viewport_y: i32,
    term_width: u16,
    canvas_area_height: u16,
) -> io::Result<()> {
    // 预分配空格缓冲区，避免每个像素/批次重复分配 String
    let spaces = " ".repeat(term_width as usize);

    // 逐行渲染
    for term_row in 0..canvas_area_height {
        // 移动终端光标到当前行行首
        crossterm::queue!(w, crossterm::cursor::MoveTo(0, term_row))?;

        // 当前终端行对应的画布 Y 坐标
        let canvas_y = term_row as i32 + viewport_y;

        // ── 批量输出：追踪当前颜色运行段 ──
        // run_color: 当前连续段的颜色（None = 空像素/终端默认背景）
        // run_len: 当前连续段已累积的终端列数
        let mut run_color: Option<Rgb> = None;
        let mut run_len: usize = 0;
        // 标记是否已启动第一个运行段
        let mut run_started = false;

        let mut col = 0i32;
        while col < term_width as i32 {
            // 当前终端列对应的画布 X 逻辑坐标
            let canvas_x = (col + viewport_x) / PIXEL_WIDTH;

            // 计算当前像素在此行还需占据多少终端列
            let pixel_end_col = (canvas_x + 1) * PIXEL_WIDTH - viewport_x;
            let fill = (pixel_end_col - col).min(term_width as i32 - col).max(1) as usize;

            // 查询该坐标处的像素颜色（坐标非负且在 u16 范围内才有意义）
            let pixel = if canvas_x >= 0
                && canvas_y >= 0
                && canvas_x <= u16::MAX as i32
                && canvas_y <= u16::MAX as i32
            {
                canvas.get_pixel(canvas_x as u16, canvas_y as u16)
            } else {
                None
            };

            // 如果与当前运行段颜色相同，扩展运行段长度
            if run_started && pixel == run_color {
                run_len += fill;
            } else {
                // 颜色不同：先刷出前一段，再开始新段
                if run_started {
                    flush_color_run(w, run_color, &spaces[..run_len])?;
                }
                run_color = pixel;
                run_len = fill;
                run_started = true;
            }

            col += fill as i32;
        }

        // 刷出当前行最后一个运行段
        if run_started {
            flush_color_run(w, run_color, &spaces[..run_len])?;
        }
    }

    // 重置样式，避免污染后续输出
    crossterm::queue!(w, style::ResetColor)?;

    Ok(())
}

/// 刷出一段同色像素的终端输出
/// `color` — 像素颜色（None 为空像素，使用终端默认背景）
/// `fill_str` — 预切好的空格切片，长度即为要填充的列数
#[inline]
fn flush_color_run(w: &mut impl Write, color: Option<Rgb>, fill_str: &str) -> io::Result<()> {
    match color {
        Some(rgb) => {
            // 有颜色的像素段：设置 RGB 背景色后输出空格
            crossterm::queue!(
                w,
                style::SetBackgroundColor(style::Color::Rgb {
                    r: rgb.0,
                    g: rgb.1,
                    b: rgb.2,
                }),
                style::Print(fill_str),
            )?;
        }
        None => {
            // 空像素段：重置为终端默认背景后输出空格
            crossterm::queue!(w, style::ResetColor, style::Print(fill_str))?;
        }
    }
    Ok(())
}

/// 渲染键盘光标
/// 在光标所在画布像素位置绘制反色方括号标记「[]」（固定 2 列 × 1 行）
fn render_cursor(
    w: &mut impl Write,
    cursor_x: u16,
    cursor_y: u16,
    viewport_x: i32,
    viewport_y: i32,
    term_width: u16,
    canvas_area_height: u16,
    brush_color: Rgb,
) -> io::Result<()> {
    // 计算光标像素在终端中的位置
    let screen_col = cursor_x as i32 * PIXEL_WIDTH - viewport_x;
    let screen_row = cursor_y as i32 * PIXEL_HEIGHT - viewport_y;

    // 光标完全不在可见区域内则跳过
    if screen_col + PIXEL_WIDTH <= 0
        || screen_col >= term_width as i32
        || screen_row < 0
        || screen_row >= canvas_area_height as i32
    {
        return Ok(());
    }

    // 使用画笔颜色的反色作为光标前景色，确保在任何背景上都可见
    let cursor_fg = (
        255 - brush_color.0,
        255 - brush_color.1,
        255 - brush_color.2,
    );

    let left_col = screen_col.max(0) as u16;
    let right_col = (screen_col + PIXEL_WIDTH - 1).min(term_width as i32 - 1) as u16;
    let row = screen_row as u16;

    crossterm::queue!(
        w,
        style::SetForegroundColor(style::Color::Rgb {
            r: cursor_fg.0,
            g: cursor_fg.1,
            b: cursor_fg.2,
        }),
        style::SetBackgroundColor(style::Color::Reset),
    )?;

    // 左半边 "["
    if screen_col >= 0 && screen_col < term_width as i32 {
        crossterm::queue!(w, crossterm::cursor::MoveTo(left_col, row), style::Print("["))?;
    }
    // 右半边 "]"
    if screen_col + 1 >= 0 && screen_col + 1 < term_width as i32 {
        crossterm::queue!(
            w,
            crossterm::cursor::MoveTo(right_col, row),
            style::Print("]"),
        )?;
    }

    crossterm::queue!(w, style::ResetColor)?;

    Ok(())
}

/// 增量渲染：仅更新指定的脏像素单元格
/// 鼠标拖拽绘制时调用此函数代替 render_frame，避免全屏重绘导致的画面抖动
/// 每个脏像素仅输出一次 MoveTo + SetBackgroundColor + Print，开销极低
///
/// # 参数
/// - `w` — 缓冲写入器
/// - `canvas` — 画布数据引用
/// - `dirty` — 发生变化的画布坐标列表
/// - `viewport_x/y` — 当前视口偏移
pub fn render_dirty_pixels(
    w: &mut impl Write,
    canvas: &Canvas,
    dirty: &[(u16, u16)],
    viewport_x: i32,
    viewport_y: i32,
) -> io::Result<()> {
    // 获取终端尺寸，用于判断像素是否在可见区域内
    let (term_width, term_height) = crossterm::terminal::size()?;
    let canvas_area_height = term_height.saturating_sub(STATUS_BAR_HEIGHT);

    // 隐藏终端光标，避免渲染过程中光标闪烁
    crossterm::queue!(w, crossterm::cursor::Hide)?;

    // 逐个渲染脏像素
    for &(cx, cy) in dirty {
        // 计算该画布像素在终端中的屏幕列位置
        let screen_col = cx as i32 * PIXEL_WIDTH - viewport_x;
        // 计算该画布像素在终端中的屏幕行位置
        let screen_row = cy as i32 * PIXEL_HEIGHT - viewport_y;

        // 像素完全不在可见区域内则跳过渲染
        if screen_col + PIXEL_WIDTH <= 0
            || screen_col >= term_width as i32
            || screen_row < 0
            || screen_row >= canvas_area_height as i32
        {
            continue;
        }

        // 计算实际需要填充的终端列范围（处理左右边界裁剪）
        let col_start = screen_col.max(0) as u16;
        let col_end = (screen_col + PIXEL_WIDTH).min(term_width as i32) as u16;
        let row = screen_row as u16;
        let fill_width = (col_end - col_start) as usize;

        // 获取该坐标处的像素颜色
        let color = canvas.get_pixel(cx, cy);

        // 移动终端光标到像素位置
        crossterm::queue!(w, crossterm::cursor::MoveTo(col_start, row))?;

        // PIXEL_WIDTH == 2，fill_width 最大为 2
        // 使用固定字符串切片避免每次堆分配
        let fill = &"  "[..fill_width];

        match color {
            Some(rgb) => {
                // 有颜色的像素：设置 RGB 背景色后输出空格
                crossterm::queue!(
                    w,
                    style::SetBackgroundColor(style::Color::Rgb {
                        r: rgb.0,
                        g: rgb.1,
                        b: rgb.2,
                    }),
                    style::Print(fill),
                )?;
            }
            None => {
                // 空像素：重置为终端默认背景后输出空格
                crossterm::queue!(w, style::ResetColor, style::Print(fill))?;
            }
        }
    }

    // 重置样式，避免污染后续输出
    crossterm::queue!(w, style::ResetColor)?;

    // 一次性刷新所有脏像素的渲染数据到终端
    w.flush()?;

    Ok(())
}
