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

/// 渲染底部状态栏（倒数第二行）
/// 格式：[模式] 当前画笔 ██ #RRGGBB | (坐标) | 消息
#[allow(clippy::too_many_arguments)]
pub fn render_status_bar(
    w: &mut impl io::Write,
    row: u16,
    term_width: u16,
    mode: DisplayMode,
    brush_color: Rgb,
    cursor_x: u16,
    cursor_y: u16,
    status_message: &str,
) -> io::Result<()> {
    // 先用背景色空格预填充整行，确保背景颜色覆盖且不会因内容长度不同而残留旧内容
    crossterm::queue!(
        w,
        crossterm::cursor::MoveTo(0, row),
        style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
        style::Print(" ".repeat(term_width as usize)),
    )?;

    // 回到行首，覆写实际内容（不再追加填充空格，避免超宽换行）
    crossterm::queue!(
        w,
        crossterm::cursor::MoveTo(0, row),
        style::SetForegroundColor(style::Color::White),
    )?;

    // ── 模式标签 ──
    let (mode_str, mode_bg) = match mode {
        DisplayMode::Draw => (" DRAW ", style::Color::Rgb { r: 0, g: 120, b: 215 }),
        DisplayMode::Command => (" CMD  ", style::Color::Rgb { r: 200, g: 80, b: 0 }),
    };
    crossterm::queue!(
        w,
        style::SetBackgroundColor(mode_bg),
        style::SetForegroundColor(style::Color::White),
        style::Print(mode_str),
        style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
    )?;

    // ── 当前画笔：「当前画笔 ██ #RRGGBB」 ──
    // 使用固定文本"当前画笔"而非"画笔x"，兼容通过命令设置自定义颜色的情况
    crossterm::queue!(
        w,
        style::SetForegroundColor(style::Color::White),
        style::Print(" 当前画笔 "),
        // 颜色块：2 个空格，背景色为画笔颜色
        style::SetBackgroundColor(style::Color::Rgb {
            r: brush_color.0,
            g: brush_color.1,
            b: brush_color.2,
        }),
        style::Print("  "),
        // 颜色代码
        style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
        style::SetForegroundColor(style::Color::Rgb { r: 180, g: 180, b: 180 }),
        style::Print(format!(
            " #{:02X}{:02X}{:02X}",
            brush_color.0, brush_color.1, brush_color.2
        )),
    )?;

    // ── 分隔符 + 光标坐标 ──
    crossterm::queue!(
        w,
        style::SetForegroundColor(style::Color::Grey),
        style::Print(format!(" | ({cursor_x},{cursor_y})")),
    )?;

    // ── 状态消息（如果有） ──
    if !status_message.is_empty() {
        crossterm::queue!(
            w,
            style::SetForegroundColor(style::Color::Grey),
            style::Print(" | "),
            style::SetForegroundColor(style::Color::Rgb { r: 255, g: 200, b: 50 }),
            style::Print(status_message),
        )?;
    }

    crossterm::queue!(w, style::ResetColor)?;

    Ok(())
}

/// 渲染调色板栏/命令行（最后一行）
/// 绘制模式：显示候选画笔「画笔0 ██ 画笔1 ██ ... 画笔9 ██」
/// 命令模式：显示命令输入「:command_text」
pub fn render_command_line(
    w: &mut impl io::Write,
    row: u16,
    term_width: u16,
    command_buffer: &str,
    mode: DisplayMode,
    palette: &[Rgb; 10],
    active_palette_index: usize,
) -> io::Result<()> {
    // 先用背景色空格预填充整行，再回到行首覆写内容
    crossterm::queue!(
        w,
        crossterm::cursor::MoveTo(0, row),
        style::SetBackgroundColor(style::Color::Rgb { r: 30, g: 30, b: 30 }),
        style::Print(" ".repeat(term_width as usize)),
        crossterm::cursor::MoveTo(0, row),
        style::SetForegroundColor(style::Color::White),
    )?;

    if mode == DisplayMode::Command {
        // 命令模式：显示 ":" 提示符和输入内容
        crossterm::queue!(
            w,
            style::SetForegroundColor(style::Color::Yellow),
            style::Print(":"),
            style::SetForegroundColor(style::Color::White),
            style::Print(command_buffer),
        )?;
    } else {
        // 绘制模式：显示候选画笔
        // 每个条目格式「画笔x ██ 」（文本 + 颜色块 + 空格分隔）
        for (i, color) in palette.iter().enumerate() {
            // 当前选中的画笔用亮白色文本高亮，其余用暗灰色
            let text_color = if i == active_palette_index {
                style::Color::White
            } else {
                style::Color::DarkGrey
            };
            crossterm::queue!(
                w,
                // 恢复调色板栏背景
                style::SetBackgroundColor(style::Color::Rgb { r: 30, g: 30, b: 30 }),
                style::SetForegroundColor(text_color),
                // 「画笔x」文本标签
                style::Print(format!("画笔{i}")),
                // 颜色块：2 个空格，背景色为该调色板颜色
                style::SetBackgroundColor(style::Color::Rgb {
                    r: color.0,
                    g: color.1,
                    b: color.2,
                }),
                style::Print("  "),
                // 条目间空格分隔
                style::SetBackgroundColor(style::Color::Rgb { r: 30, g: 30, b: 30 }),
                style::Print(" "),
            )?;
        }
    }

    crossterm::queue!(w, style::ResetColor)?;

    Ok(())
}

/// 渲染帮助信息覆盖层
/// 在画布中央显示帮助面板
pub fn render_help(
    w: &mut impl io::Write,
    term_width: u16,
    term_height: u16,
) -> io::Result<()> {
    let help_lines = [
        "╔════════════════════════════════════════════════╗",
        "║            PixTerm 帮助信息                    ║",
        "║                                                ║",
        "║  鼠标操作:                                     ║",
        "║    左键点击/拖拽  绘制像素                     ║",
        "║    右键点击/拖拽  橡皮擦（清除像素）           ║",
        "║                                                ║",
        "║  键盘快捷键:                                   ║",
        "║    方向键         移动光标                     ║",
        "║    空格           在光标位置绘制               ║",
        "║    0-9            切换调色板颜色               ║",
        "║    Ctrl+S         保存画布                     ║",
        "║    Ctrl+Z         撤销                         ║",
        "║    Ctrl+Y         重做                         ║",
        "║    Ctrl+Q         退出                         ║",
        "║    ESC            进入/退出命令模式            ║",
        "║                                                ║",
        "║  命令模式（ESC 进入）:                         ║",
        "║    :help              显示此帮助               ║",
        "║    :save [文件名]     保存画布                 ║",
        "║    :load [文件名]     加载画布                 ║",
        "║    :export [文件名]   导出 PNG 图片            ║",
        "║    :import 文件名     导入 PNG 图片            ║",
        "║    :quit / :q         退出程序                 ║",
        "║    :paint x y [#HEX]  绘制像素                ║",
        "║    :paint x1:x2 y1:y2 范围绘制                ║",
        "║    :undo              撤销                     ║",
        "║    :redo              重做                     ║",
        "║    :color x y         查看像素颜色             ║",
        "║    :clear             清空画布                 ║",
        "║    #RRGGBB            切换画笔颜色             ║",
        "║                                                ║",
        "║          按任意键关闭帮助                      ║",
        "╚════════════════════════════════════════════════╝",
    ];

    let panel_height = help_lines.len() as u16;
    let panel_width = 50u16;
    let start_col = term_width.saturating_sub(panel_width) / 2;
    let start_row = term_height.saturating_sub(panel_height) / 2;

    for (i, line) in help_lines.iter().enumerate() {
        let row = start_row + i as u16;
        if row >= term_height {
            break;
        }
        crossterm::queue!(
            w,
            crossterm::cursor::MoveTo(start_col, row),
            style::SetBackgroundColor(style::Color::Rgb { r: 20, g: 20, b: 50 }),
            style::SetForegroundColor(style::Color::Rgb { r: 200, g: 200, b: 255 }),
            style::Print(line),
        )?;
    }

    crossterm::queue!(w, style::ResetColor)?;

    Ok(())
}
