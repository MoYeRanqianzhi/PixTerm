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

/// 渲染底部状态栏（第一行）
/// 显示：模式指示 | 画笔颜色预览 | 调色板色块 | 光标坐标 | 状态消息
/// `row` - 状态栏所在的终端行号
/// `term_width` - 终端宽度（列数）
/// `mode` - 当前应用模式
/// `brush_color` - 当前画笔颜色
/// `palette` - 10 个调色板颜色
/// `active_palette_index` - 当前选中的调色板索引
/// `cursor_x`, `cursor_y` - 画布上的光标逻辑坐标
/// `status_message` - 要显示的状态消息文本
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
    // 移动光标到状态栏行首
    crossterm::queue!(
        stdout,
        crossterm::cursor::MoveTo(0, row),
        // 使用深灰色背景铺满整行，区分画布区域和状态栏
        style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
        style::SetForegroundColor(style::Color::White),
    )?;

    // 模式指示文本
    let mode_str = match mode {
        DisplayMode::Draw => " DRAW ",
        DisplayMode::Command => " CMD  ",
    };
    // 模式标签使用高亮背景
    let mode_bg = match mode {
        DisplayMode::Draw => style::Color::Rgb { r: 0, g: 120, b: 215 },
        DisplayMode::Command => style::Color::Rgb { r: 200, g: 80, b: 0 },
    };
    crossterm::queue!(
        stdout,
        style::SetBackgroundColor(mode_bg),
        style::SetForegroundColor(style::Color::White),
        style::Print(mode_str),
        // 恢复状态栏默认背景
        style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
    )?;

    // 画笔颜色预览：显示两个空格宽的色块 + hex 值
    crossterm::queue!(
        stdout,
        style::SetForegroundColor(style::Color::Grey),
        style::Print(" Brush:"),
        style::SetBackgroundColor(style::Color::Rgb {
            r: brush_color.0,
            g: brush_color.1,
            b: brush_color.2,
        }),
        style::Print("  "), // 两个空格作为色块
        style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
        style::SetForegroundColor(style::Color::White),
        style::Print(format!(
            " #{:02X}{:02X}{:02X}",
            brush_color.0, brush_color.1, brush_color.2
        )),
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
        // 绘制颜色色块（2个空格宽度）
        crossterm::queue!(
            stdout,
            style::SetBackgroundColor(style::Color::Rgb {
                r: color.0,
                g: color.1,
                b: color.2,
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
    // 计算已输出的大致字符数（简化处理：直接用空格填满到行尾）
    crossterm::queue!(
        stdout,
        style::SetBackgroundColor(style::Color::Rgb { r: 40, g: 40, b: 40 }),
        style::Print(" ".repeat(term_width as usize)),
    )?;

    // 重置样式
    crossterm::queue!(stdout, style::ResetColor)?;

    Ok(())
}

/// 渲染命令行输入区域（状态栏下方一行）
/// `row` - 命令行所在的终端行号
/// `term_width` - 终端宽度
/// `command_buffer` - 当前命令输入缓冲区内容
/// `mode` - 当前模式（仅命令模式时显示输入提示）
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
        // 命令行使用更深的背景色
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
/// 在画布中央显示半透明帮助面板
/// `term_width`, `term_height` - 终端尺寸
pub fn render_help(
    stdout: &mut io::Stdout,
    term_width: u16,
    term_height: u16,
) -> io::Result<()> {
    // 帮助文本内容
    let help_lines = [
        "╔══════════════════════════════════════════╗",
        "║          PixTerm 帮助信息                ║",
        "║                                          ║",
        "║  鼠标操作:                               ║",
        "║    左键点击/拖拽  绘制像素               ║",
        "║    右键点击       橡皮擦（清除像素）     ║",
        "║    滚轮上/下      放大/缩小              ║",
        "║                                          ║",
        "║  键盘快捷键:                             ║",
        "║    方向键         移动视口               ║",
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

    // 计算帮助面板在终端中的居中起始位置
    let panel_height = help_lines.len() as u16;
    let panel_width = 44u16; // 帮助面板固定宽度
    // 水平居中
    let start_col = if term_width > panel_width {
        (term_width - panel_width) / 2
    } else {
        0
    };
    // 垂直居中
    let start_row = if term_height > panel_height {
        (term_height - panel_height) / 2
    } else {
        0
    };

    // 逐行绘制帮助面板
    for (i, line) in help_lines.iter().enumerate() {
        let row = start_row + i as u16;
        // 超出终端高度则停止
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

    // 重置样式
    crossterm::queue!(stdout, style::ResetColor)?;

    Ok(())
}
