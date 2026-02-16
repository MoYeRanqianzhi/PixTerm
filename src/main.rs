// PixTerm - 终端像素画板程序
// 入口模块：终端初始化/恢复、启动 App

// 引入各子模块
mod app;
mod canvas;
mod color;
mod command;
mod file;
mod history;
mod input;
mod renderer;
mod ui;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::panic;

/// 程序入口点
/// 负责终端环境的初始化和恢复，以及启动应用主循环
fn main() {
    // 设置 panic hook：在 panic 时恢复终端状态，避免终端被留在 raw mode
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // 尝试恢复终端状态
        let _ = cleanup_terminal();
        // 调用原始 panic handler 输出错误信息
        original_hook(panic_info);
    }));

    // 初始化终端并运行应用
    match run_app() {
        Ok(()) => {}
        Err(e) => {
            // 确保终端已恢复
            let _ = cleanup_terminal();
            eprintln!("PixTerm 运行出错: {e}");
            std::process::exit(1);
        }
    }
}

/// 初始化终端环境 → 运行应用 → 恢复终端环境
fn run_app() -> io::Result<()> {
    // 获取标准输出句柄
    let mut stdout = io::stdout();

    // 初始化终端：
    // 1. 启用 raw mode（禁用行缓冲和回显，直接接收按键事件）
    terminal::enable_raw_mode()?;
    // 2. 进入备用屏幕（保留原始终端内容）
    // 3. 启用鼠标捕获（接收鼠标点击、拖拽、滚轮事件）
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // 创建应用实例（默认 32x32 画布）
    let mut app = app::App::new(32, 32);

    // 运行应用主循环
    let result = app.run();

    // 无论运行结果如何，都恢复终端状态
    cleanup_terminal()?;

    result
}

/// 恢复终端状态
/// 禁用鼠标捕获 → 离开备用屏幕 → 禁用 raw mode → 显示光标
fn cleanup_terminal() -> io::Result<()> {
    let mut stdout = io::stdout();
    // 禁用鼠标捕获
    execute!(stdout, DisableMouseCapture)?;
    // 离开备用屏幕（恢复原始终端内容）
    execute!(stdout, LeaveAlternateScreen)?;
    // 禁用 raw mode（恢复正常的行缓冲和回显）
    terminal::disable_raw_mode()?;
    // 显示光标（渲染时可能隐藏了光标）
    execute!(stdout, crossterm::cursor::Show)?;
    Ok(())
}
