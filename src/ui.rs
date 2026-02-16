// PixTerm - UI 模块
// 状态栏、帮助信息 UI 组件

use crate::color::Rgb;
use crossterm::style;
use std::io;

/// 应用模式枚举，用于状态栏显示
#[derive(Clone, Copy, PartialEq)]
pub enum DisplayMode {
    /// 绘制模式
    Draw,
    /// 命令模式
    Command,
}

/// 渲染底部状态栏
/// 格式：[模式] 画笔x [颜色块] | 调色板 | (坐标) | 消息
pub fn render_status_bar(
    stdout: &mut io::Stdout,
    row: u16,
    term_width: u16,
    mode: DisplayMode,
    brush_color: Rgb,
    palette: &[Rgb; 10],
    active_palette_index: usize,
    cursor_x: u16,
    cursor_y: u16,
    status_message: &str,
) -> io::Result<()> {
    // 移动光标到状态栏行首，设置状态栏背景
    crossterm::queue!(
        stdout,
        crossterm::cursor::MoveTo(0, row),
        style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
        style::SetForegroundColor(style::Color::White),
    )?;

    // 模式标签
    let (mode_str, mode_bg) = match mode {
        DisplayMode::Draw => (" DRAW ", style::Color::Rgb { r: 0, g: 120, b: 215 }),
        DisplayMode::Command => (" CMD  ", style::Color::Rgb { r: 200, g: 80, b: 0 }),
    };
    crossterm::queue!(
        stdout,
        style::SetBackgroundColor(mode_bg),
        style::SetForegroundColor(style::Color::White),
        style::Print(mode_str),
        style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
    )?;

    // 画笔信息：画笔x [颜色块]
    // "x" 为当前调色板索引号，颜色块用两个空格背景着色表示，外加方括号标记
    crossterm::queue!(
        stdout,
        style::SetForegroundColor(style::Color::White),
        style::Print(format!(" 画笔{active_palette_index} ")),
        // 左方括号
        style::SetForegroundColor(style::Color::Grey),
        style::Print("["),
        // 颜色块：两个空格宽度，背景色为画笔颜色
        style::SetBackgroundColor(style::Color::Rgb {
            r: brush_color.0,
            g: brush_color.1,
            b: brush_color.2,
        }),
        style::Print("  "),
        // 右方括号
        style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
        style::SetForegroundColor(style::Color::Grey),
        style::Print("]"),
    )?;

    // 调色板色块：显示 0-9 数字键对应的颜色
    crossterm::queue!(
        stdout,
        style::SetForegroundColor(style::Color::Grey),
        style::Print(" | "),
    )?;
    for (i, color) in palette.iter().enumerate() {
        // 当前选中的调色板索引用方括号高亮
        if i == active_palette_index {
            crossterm::queue!(
                stdout,
                style::SetForegroundColor(style::Color::Yellow),
                style::Print("["),
            )?;
        }
        // 绘制颜色色块（显示数字编号，背景为对应颜色）
        crossterm::queue!(
            stdout,
            style::SetBackgroundColor(style::Color::Rgb {
                r: color.0,
                g: color.1,
                b: color.2,
            }),
            // 根据颜色亮度选择前景色，确保数字可读
            style::SetForegroundColor(if color.0 as u16 + color.1 as u16 + color.2 as u16 > 382 {
                style::Color::Black
            } else {
                style::Color::White
            }),
            style::Print(format!("{i}")),
            style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
        )?;
        if i == active_palette_index {
            crossterm::queue!(
                stdout,
                style::SetForegroundColor(style::Color::Yellow),
                style::Print("]"),
            )?;
        }
    }

    // 光标坐标
    crossterm::queue!(
        stdout,
        style::SetForegroundColor(style::Color::Grey),
        style::Print(format!(" | ({cursor_x},{cursor_y})")),
    )?;

    // 状态消息（如果有）
    if !status_message.is_empty() {
        crossterm::queue!(
            stdout,
            style::SetForegroundColor(style::Color::Rgb { r: 255, g: 200, b: 50 }),
            style::Print(format!(" | {status_message}")),
        )?;
    }

    // 用空格填充剩余宽度，确保背景颜色覆盖整行
    crossterm::queue!(
        stdout,
        style::SetForegroundColor(style::Color::Reset),
        style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
        style::Print(" ".repeat(term_width as usize)),
    )?;

    // 重置样式
    crossterm::queue!(stdout, style::ResetColor)?;

    Ok(())
}

/// 渲染命令行输入区域（状态栏下方一行）
pub fn render_command_line(
    stdout: &mut io::Stdout,
    row: u16,
    term_width: u16,
    command_buffer: &str,
    mode: DisplayMode,
) -> io::Result<()> {
    // 移动光标到命令行行首
    crossterm::queue!(
        stdout,
        crossterm::cursor::MoveTo(0, row),
        style::SetBackgroundColor(style::Color::Rgb { r: 30, g: 30, b: 30 }),
        style::SetForegroundColor(style::Color::White),
    )?;

    if mode == DisplayMode::Command {
        // 命令模式：显示 ":" 提示符和输入内容
        crossterm::queue!(
            stdout,
            style::SetForegroundColor(style::Color::Yellow),
            style::Print(":"),
            style::SetForegroundColor(style::Color::White),
            style::Print(command_buffer),
        )?;
    } else {
        // 绘制模式：显示快捷键提示
        crossterm::queue!(
            stdout,
            style::SetForegroundColor(style::Color::DarkGrey),
            style::Print(" ESC:命令模式 | 0-9:调色板 | Ctrl+S:保存 | Ctrl+Z/Y:撤销/重做 | Ctrl+Q:退出"),
        )?;
    }

    // 填充剩余宽度
    crossterm::queue!(
        stdout,
        style::Print(" ".repeat(term_width as usize)),
    )?;

    // 重置样式
    crossterm::queue!(stdout, style::ResetColor)?;

    Ok(())
}

/// 渲染帮助信息覆盖层
/// 在画布中央显示帮助面板
pub fn render_help(
    stdout: &mut io::Stdout,
    term_width: u16,
    term_height: u16,
) -> io::Result<()> {
    let help_lines = [
        "╔══════════════════════════════════════════╗",
        "║          PixTerm 帮助信息                ║",
        "║                                          ║",
        "║  鼠标操作:                               ║",
        "║    左键点击/拖拽  绘制像素               ║",
        "║    右键点击       橡皮擦（清除像素）     ║",
        "║                                          ║",
        "║  键盘快捷键:                             ║",
        "║    方向键         移动光标               ║",
        "║    空格           在光标位置绘制         ║",
        "║    0-9            切换调色板颜色         ║",
        "║    Ctrl+S         保存画布               ║",
        "║    Ctrl+Z         撤销                   ║",
        "║    Ctrl+Y         重做                   ║",
        "║    Ctrl+Q         退出                   ║",
        "║    ESC            进入/退出命令模式      ║",
        "║                                          ║",
        "║  命令模式（ESC 进入）:                   ║",
        "║    :help            显示此帮助           ║",
        "║    :save [文件名]   保存画布             ║",
        "║    :load [文件名]   加载画布             ║",
        "║    :quit / :q       退出程序             ║",
        "║    :paint x y #HEX  绘制指定位置         ║",
        "║    :undo            撤销                 ║",
        "║    :redo            重做                 ║",
        "║    :color x y       查看像素颜色         ║",
        "║    :clear           清空画布             ║",
        "║    #RRGGBB          切换画笔颜色         ║",
        "║                                          ║",
        "║        按任意键关闭帮助                  ║",
        "╚══════════════════════════════════════════╝",
    ];

    let panel_height = help_lines.len() as u16;
    let panel_width = 44u16;
    let start_col = term_width.saturating_sub(panel_width) / 2;
    let start_row = term_height.saturating_sub(panel_height) / 2;

    for (i, line) in help_lines.iter().enumerate() {
        let row = start_row + i as u16;
        if row >= term_height {
            break;
        }
        crossterm::queue!(
            stdout,
            crossterm::cursor::MoveTo(start_col, row),
            style::SetBackgroundColor(style::Color::Rgb { r: 20, g: 20, b: 50 }),
            style::SetForegroundColor(style::Color::Rgb { r: 200, g: 200, b: 255 }),
            style::Print(line),
        )?;
    }

    crossterm::queue!(stdout, style::ResetColor)?;

    Ok(())
}
