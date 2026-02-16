// PixTerm - Renderer 模块
// 终端渲染：画布 + 状态栏 + 命令行 + 帮助

use crate::canvas::Canvas;
use crate::color::Rgb;
use crate::ui::{self, DisplayMode};
use crossterm::style;
use std::io::{self, Write};

/// 状态栏占用的终端行数（状态栏 1 行 + 命令行/快捷键提示 1 行）
pub const STATUS_BAR_HEIGHT: u16 = 2;

/// 默认背景色（画布空白区域）：深灰色棋盘格模式的两种颜色
const BG_COLOR_A: Rgb = (45, 45, 45);
const BG_COLOR_B: Rgb = (55, 55, 55);

/// 渲染完整的一帧画面
/// 包括：画布区域、键盘光标、状态栏、命令行/快捷键提示、帮助覆盖层
///
/// # 参数
/// - `stdout` - 标准输出句柄
/// - `canvas` - 画布数据
/// - `viewport_x`, `viewport_y` - 视口偏移（像素坐标系，非终端坐标系）
/// - `zoom` - 缩放等级（1-8），每个逻辑像素占 2*zoom 列 x zoom 行
/// - `cursor_x`, `cursor_y` - 画布上的逻辑光标坐标
/// - `brush_color` - 当前画笔颜色
/// - `palette` - 调色板颜色数组
/// - `active_palette_index` - 当前选中的调色板索引
/// - `mode` - 当前应用模式
/// - `command_buffer` - 命令输入缓冲区
/// - `status_message` - 状态消息
/// - `show_help` - 是否显示帮助覆盖层
pub fn render_frame(
    stdout: &mut io::Stdout,
    canvas: &Canvas,
    viewport_x: i32,
    viewport_y: i32,
    zoom: u16,
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
        zoom,
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
        zoom,
        term_width,
        canvas_area_height,
        brush_color,
    )?;

    // ===== 3. 渲染状态栏（倒数第二行） =====
    ui::render_status_bar(
        stdout,
        canvas_area_height,  // 状态栏行号 = 画布区域高度（即画布区最后一行的下一行）
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
        canvas_area_height + 1,  // 命令行在状态栏下方一行
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
/// 遍历终端可见区域的每个字符位置，计算对应的画布坐标并输出像素颜色
fn render_canvas_area(
    stdout: &mut io::Stdout,
    canvas: &Canvas,
    viewport_x: i32,
    viewport_y: i32,
    zoom: u16,
    term_width: u16,
    canvas_area_height: u16,
) -> io::Result<()> {
    // 每个逻辑像素在终端中的宽度（列数）：2 * zoom
    let pixel_width = 2 * zoom as i32;
    // 每个逻辑像素在终端中的高度（行数）：zoom
    let pixel_height = zoom as i32;

    // 逐行渲染
    for term_row in 0..canvas_area_height {
        // 移动光标到当前行行首
        crossterm::queue!(stdout, crossterm::cursor::MoveTo(0, term_row))?;

        // 计算当前终端行对应的画布 Y 坐标
        // term_row + viewport_y → 在画布坐标系中的像素偏移 → 除以 pixel_height 得到逻辑 Y
        let canvas_y_raw = term_row as i32 + viewport_y;
        let canvas_y = if canvas_y_raw >= 0 {
            canvas_y_raw / pixel_height
        } else {
            // 负数除法需要向负无穷取整
            (canvas_y_raw - pixel_height + 1) / pixel_height
        };

        // 逐列渲染（每次输出一个字符宽度的空格）
        let mut col = 0u16;
        while col < term_width {
            // 计算当前终端列对应的画布 X 坐标
            let canvas_x_raw = col as i32 + viewport_x;
            let canvas_x = if canvas_x_raw >= 0 {
                canvas_x_raw / pixel_width
            } else {
                (canvas_x_raw - pixel_width + 1) / pixel_width
            };

            // 判断是否在画布范围内
            let color = if canvas_x >= 0
                && canvas_x < canvas.width as i32
                && canvas_y >= 0
                && canvas_y < canvas.height as i32
            {
                // 在画布内：获取像素颜色
                match canvas.get_pixel(canvas_x as u16, canvas_y as u16) {
                    Some(rgb) => rgb,
                    None => {
                        // 空像素：使用棋盘格背景模式以区分空白和画布外
                        if (canvas_x + canvas_y) % 2 == 0 {
                            BG_COLOR_A
                        } else {
                            BG_COLOR_B
                        }
                    }
                }
            } else {
                // 画布外区域：使用纯黑色背景
                (25, 25, 25)
            };

            // 计算当前逻辑像素在这一行中还剩多少列要填充
            // 即从当前 col 到该逻辑像素右边界之间的列数
            let pixel_start_col = canvas_x * pixel_width - viewport_x;
            let pixel_end_col = pixel_start_col + pixel_width;
            let fill_count = (pixel_end_col - col as i32).min(term_width as i32 - col as i32);
            let fill_count = fill_count.max(1) as usize;

            // 输出带背景色的空格字符
            crossterm::queue!(
                stdout,
                style::SetBackgroundColor(style::Color::Rgb {
                    r: color.0,
                    g: color.1,
                    b: color.2,
                }),
                style::Print(" ".repeat(fill_count)),
            )?;

            col += fill_count as u16;
        }
    }

    // 重置样式
    crossterm::queue!(stdout, style::ResetColor)?;

    Ok(())
}

/// 渲染键盘光标
/// 在光标所在的画布像素位置绘制高亮边框效果
fn render_cursor(
    stdout: &mut io::Stdout,
    cursor_x: u16,
    cursor_y: u16,
    viewport_x: i32,
    viewport_y: i32,
    zoom: u16,
    term_width: u16,
    canvas_area_height: u16,
    brush_color: Rgb,
) -> io::Result<()> {
    // 每个逻辑像素在终端中的尺寸
    let pixel_width = 2 * zoom as i32;
    let pixel_height = zoom as i32;

    // 计算光标像素在终端中的起始列和行
    let screen_col = cursor_x as i32 * pixel_width - viewport_x;
    let screen_row = cursor_y as i32 * pixel_height - viewport_y;

    // 检查光标是否在可见区域内
    if screen_col + pixel_width <= 0
        || screen_col >= term_width as i32
        || screen_row + pixel_height <= 0
        || screen_row >= canvas_area_height as i32
    {
        return Ok(()); // 光标不在可见区域内，不绘制
    }

    // 使用画笔颜色的反色作为光标边框颜色，确保可见性
    let border_color = (
        255 - brush_color.0,
        255 - brush_color.1,
        255 - brush_color.2,
    );

    // 绘制光标边框（像素区域的第一行和最后一行用特殊字符标记）
    for dy in 0..pixel_height {
        let row = screen_row + dy;
        // 跳过不在可见区域内的行
        if row < 0 || row >= canvas_area_height as i32 {
            continue;
        }
        for dx in 0..pixel_width {
            let c = screen_col + dx;
            // 跳过不在可见区域内的列
            if c < 0 || c >= term_width as i32 {
                continue;
            }
            // 只绘制边框（第一行、最后一行、第一列、最后一列）
            let is_border = dy == 0
                || dy == pixel_height - 1
                || dx == 0
                || dx == pixel_width - 1;
            if is_border {
                crossterm::queue!(
                    stdout,
                    crossterm::cursor::MoveTo(c as u16, row as u16),
                    style::SetBackgroundColor(style::Color::Rgb {
                        r: border_color.0,
                        g: border_color.1,
                        b: border_color.2,
                    }),
                    style::Print(" "),
                )?;
            }
        }
    }

    // 重置样式
    crossterm::queue!(stdout, style::ResetColor)?;

    Ok(())
}
