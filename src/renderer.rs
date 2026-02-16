// PixTerm - Renderer 模块
// 终端渲染：画布 + 状态栏 + 命令行 + 帮助

use crate::canvas::Canvas;
use crate::color::Rgb;
use crate::ui::{self, DisplayMode};
use crossterm::style;
use std::io::{self, Write};

/// 状态栏占用的终端行数（状态栏 1 行 + 命令行/快捷键提示 1 行）
pub const STATUS_BAR_HEIGHT: u16 = 2;

/// 每个逻辑像素在终端中占 2 列（终端字符宽高比约 1:2，2 列使像素接近正方形）
const PIXEL_WIDTH: i32 = 2;
/// 每个逻辑像素在终端中占 1 行
const PIXEL_HEIGHT: i32 = 1;

/// 渲染完整的一帧画面
/// 包括：画布区域、键盘光标、状态栏、命令行/快捷键提示、帮助覆盖层
pub fn render_frame(
    stdout: &mut io::Stdout,
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
) -> io::Result<()> {
    // 获取终端尺寸
    let (term_width, term_height) = crossterm::terminal::size()?;

    // 画布可用渲染区域的高度（终端高度减去状态栏高度）
    let canvas_area_height = term_height.saturating_sub(STATUS_BAR_HEIGHT);

    // 隐藏光标以避免渲染过程中闪烁
    crossterm::queue!(stdout, crossterm::cursor::Hide)?;

    // ===== 1. 渲染画布区域 =====
    render_canvas_area(
        stdout,
        canvas,
        viewport_x,
        viewport_y,
        term_width,
        canvas_area_height,
    )?;

    // ===== 2. 渲染键盘光标（如果在可见区域内）=====
    render_cursor(
        stdout,
        cursor_x,
        cursor_y,
        viewport_x,
        viewport_y,
        term_width,
        canvas_area_height,
        brush_color,
    )?;

    // ===== 3. 渲染状态栏（倒数第二行） =====
    ui::render_status_bar(
        stdout,
        canvas_area_height,
        term_width,
        mode,
        brush_color,
        palette,
        active_palette_index,
        cursor_x,
        cursor_y,
        status_message,
    )?;

    // ===== 4. 渲染命令行/快捷键提示（最后一行） =====
    ui::render_command_line(
        stdout,
        canvas_area_height + 1,
        term_width,
        command_buffer,
        mode,
    )?;

    // ===== 5. 帮助覆盖层（如果开启） =====
    if show_help {
        ui::render_help(stdout, term_width, term_height)?;
    }

    // 刷新输出缓冲区，将所有 queue! 的命令一次性写入终端
    stdout.flush()?;

    Ok(())
}

/// 渲染画布区域
/// 遍历终端可见区域的每一行，计算对应的画布坐标并输出像素颜色
/// 无限画布：无边界限制，仅检查像素是否存在
fn render_canvas_area(
    stdout: &mut io::Stdout,
    canvas: &Canvas,
    viewport_x: i32,
    viewport_y: i32,
    term_width: u16,
    canvas_area_height: u16,
) -> io::Result<()> {
    // 逐行渲染
    for term_row in 0..canvas_area_height {
        // 移动光标到当前行行首
        crossterm::queue!(stdout, crossterm::cursor::MoveTo(0, term_row))?;

        // 当前终端行对应的画布 Y 坐标（固定 1 行 = 1 逻辑像素）
        let canvas_y = term_row as i32 + viewport_y;

        // 逐列渲染，每次跳 PIXEL_WIDTH 列（一个逻辑像素宽度）
        let mut col = 0i32;
        while col < term_width as i32 {
            // 当前终端列对应的画布 X 坐标（固定 2 列 = 1 逻辑像素）
            let canvas_x = (col + viewport_x) / PIXEL_WIDTH;

            // 本像素在这一行还需要填充多少列
            let pixel_end_col = (canvas_x + 1) * PIXEL_WIDTH - viewport_x;
            let fill = (pixel_end_col - col).min(term_width as i32 - col).max(1) as usize;

            // 无限画布：只要坐标非负且在 u16 范围内，就尝试获取像素
            let pixel = if canvas_x >= 0
                && canvas_y >= 0
                && canvas_x <= u16::MAX as i32
                && canvas_y <= u16::MAX as i32
            {
                canvas.get_pixel(canvas_x as u16, canvas_y as u16)
            } else {
                None
            };

            if let Some(rgb) = pixel {
                // 有颜色的像素：设置背景色
                crossterm::queue!(
                    stdout,
                    style::SetBackgroundColor(style::Color::Rgb {
                        r: rgb.0,
                        g: rgb.1,
                        b: rgb.2,
                    }),
                    style::Print(" ".repeat(fill)),
                )?;
            } else {
                // 空像素：使用终端默认背景（无颜色）
                crossterm::queue!(
                    stdout,
                    style::ResetColor,
                    style::Print(" ".repeat(fill)),
                )?;
            }

            col += fill as i32;
        }
    }

    // 重置样式
    crossterm::queue!(stdout, style::ResetColor)?;

    Ok(())
}

/// 渲染键盘光标
/// 在光标所在的画布像素位置绘制反色标记（固定 2 列 x 1 行）
fn render_cursor(
    stdout: &mut io::Stdout,
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

    // 检查光标是否在可见区域内
    if screen_col + PIXEL_WIDTH <= 0
        || screen_col >= term_width as i32
        || screen_row < 0
        || screen_row >= canvas_area_height as i32
    {
        return Ok(());
    }

    // 使用画笔颜色的反色作为光标颜色，确保可见性
    let cursor_color = (
        255 - brush_color.0,
        255 - brush_color.1,
        255 - brush_color.2,
    );

    // 在光标位置绘制反色方括号标记 "[]"
    let left_col = screen_col.max(0) as u16;
    let right_col = (screen_col + PIXEL_WIDTH - 1).min(term_width as i32 - 1) as u16;
    let row = screen_row as u16;

    crossterm::queue!(
        stdout,
        style::SetForegroundColor(style::Color::Rgb {
            r: cursor_color.0,
            g: cursor_color.1,
            b: cursor_color.2,
        }),
        style::SetBackgroundColor(style::Color::Reset),
    )?;

    // 左半边 "["
    if screen_col >= 0 && screen_col < term_width as i32 {
        crossterm::queue!(
            stdout,
            crossterm::cursor::MoveTo(left_col, row),
            style::Print("["),
        )?;
    }
    // 右半边 "]"
    if screen_col + 1 >= 0 && screen_col + 1 < term_width as i32 {
        crossterm::queue!(
            stdout,
            crossterm::cursor::MoveTo(right_col, row),
            style::Print("]"),
        )?;
    }

    // 重置样式
    crossterm::queue!(stdout, style::ResetColor)?;

    Ok(())
}
