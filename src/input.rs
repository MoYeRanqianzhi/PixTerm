// PixTerm - Input 模块
// 键盘/鼠标事件处理与分发

use crate::app::App;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// 处理终端事件的顶层分发函数
/// 根据当前应用模式（绘制/命令）将事件路由到对应的处理器
pub fn handle_event(app: &mut App, event: Event) {
    match event {
        // 键盘事件
        Event::Key(key_event) => {
            // 如果正在显示帮助，任意键关闭帮助
            if app.show_help {
                app.show_help = false;
                return;
            }
            match app.mode {
                crate::app::AppMode::Draw => handle_draw_mode_key(app, key_event),
                crate::app::AppMode::Command => handle_command_mode_key(app, key_event),
            }
        }
        // 鼠标事件（仅绘制模式处理）
        Event::Mouse(mouse_event) => {
            // 帮助界面中忽略鼠标事件
            if app.show_help {
                return;
            }
            handle_mouse_event(app, mouse_event);
        }
        // 终端大小变化事件：无需特殊处理，渲染时会自动获取新尺寸
        Event::Resize(_, _) => {}
        // 其他事件忽略
        _ => {}
    }
}

/// 绘制模式下的键盘事件处理
fn handle_draw_mode_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Ctrl+Q — 退出程序
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.running = false;
        }
        // Ctrl+S — 保存画布
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.save_canvas(None);
        }
        // Ctrl+Z — 撤销
        KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.apply_undo();
        }
        // Ctrl+Y — 重做
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.apply_redo();
        }
        // ESC — 进入命令模式
        KeyCode::Esc => {
            app.mode = crate::app::AppMode::Command;
            app.command_buffer.clear();
            app.status_message = "命令模式（输入命令后按 Enter 执行，ESC 返回）".to_string();
        }
        // 空格 — 在键盘光标位置绘制像素
        KeyCode::Char(' ') => {
            app.paint_pixel(app.cursor_x, app.cursor_y, Some(app.brush_color));
        }
        // 数字键 0-9 — 切换调色板颜色
        KeyCode::Char(c @ '0'..='9') => {
            let index = (c as u8 - b'0') as usize;
            app.active_palette_index = index;
            app.brush_color = app.palette[index];
            app.status_message = format!(
                "调色板 {}: #{:02X}{:02X}{:02X}",
                index, app.brush_color.0, app.brush_color.1, app.brush_color.2
            );
        }
        // 方向键 — 移动键盘光标（同时滚动视口使光标可见）
        KeyCode::Up => {
            if app.cursor_y > 0 {
                app.cursor_y -= 1;
            }
            app.ensure_cursor_visible();
        }
        KeyCode::Down => {
            if app.cursor_y < app.canvas.height - 1 {
                app.cursor_y += 1;
            }
            app.ensure_cursor_visible();
        }
        KeyCode::Left => {
            if app.cursor_x > 0 {
                app.cursor_x -= 1;
            }
            app.ensure_cursor_visible();
        }
        KeyCode::Right => {
            if app.cursor_x < app.canvas.width - 1 {
                app.cursor_x += 1;
            }
            app.ensure_cursor_visible();
        }
        // 删除键 — 清除光标处像素（橡皮擦）
        KeyCode::Delete => {
            app.paint_pixel(app.cursor_x, app.cursor_y, None);
        }
        // 其他按键忽略
        _ => {}
    }
}

/// 命令模式下的键盘事件处理
fn handle_command_mode_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // ESC — 取消命令，返回绘制模式
        KeyCode::Esc => {
            app.mode = crate::app::AppMode::Draw;
            app.command_buffer.clear();
            app.status_message.clear();
        }
        // Enter — 执行命令
        KeyCode::Enter => {
            // 取出命令缓冲区内容并清空
            let input = app.command_buffer.clone();
            app.command_buffer.clear();
            // 执行命令后回到绘制模式
            app.mode = crate::app::AppMode::Draw;
            app.execute_command(&input);
        }
        // Backspace — 删除最后一个字符
        KeyCode::Backspace => {
            app.command_buffer.pop();
        }
        // 普通字符输入 — 追加到命令缓冲区
        KeyCode::Char(c) => {
            // 命令模式下忽略 Ctrl 修饰的字符，避免误输入
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                app.command_buffer.push(c);
            }
        }
        // 其他按键忽略
        _ => {}
    }
}

/// 鼠标事件处理
fn handle_mouse_event(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        // 鼠标左键按下 — 开始绘制笔画
        MouseEventKind::Down(MouseButton::Left) => {
            // 将终端坐标转换为画布逻辑坐标
            if let Some((cx, cy)) = app.screen_to_canvas(mouse.column, mouse.row) {
                // 开始新的笔画
                app.is_drawing = true;
                app.current_stroke.clear();
                // 绘制第一个像素
                app.paint_pixel_stroke(cx, cy, Some(app.brush_color));
            }
        }
        // 鼠标右键按下 — 橡皮擦（清除像素）
        MouseEventKind::Down(MouseButton::Right) => {
            if let Some((cx, cy)) = app.screen_to_canvas(mouse.column, mouse.row) {
                app.is_drawing = true;
                app.current_stroke.clear();
                app.paint_pixel_stroke(cx, cy, None);
            }
        }
        // 鼠标拖拽 — 连续绘制
        MouseEventKind::Drag(MouseButton::Left) => {
            if app.is_drawing {
                if let Some((cx, cy)) = app.screen_to_canvas(mouse.column, mouse.row) {
                    app.paint_pixel_stroke(cx, cy, Some(app.brush_color));
                }
            }
        }
        // 鼠标右键拖拽 — 连续擦除
        MouseEventKind::Drag(MouseButton::Right) => {
            if app.is_drawing {
                if let Some((cx, cy)) = app.screen_to_canvas(mouse.column, mouse.row) {
                    app.paint_pixel_stroke(cx, cy, None);
                }
            }
        }
        // 鼠标释放 — 结束笔画，将笔画记录推入历史
        MouseEventKind::Up(_) => {
            if app.is_drawing {
                app.finish_stroke();
            }
        }
        // 鼠标滚轮向上 — 放大
        MouseEventKind::ScrollUp => {
            if app.zoom < 8 {
                app.zoom += 1;
                app.status_message = format!("缩放: {}x", app.zoom);
            }
        }
        // 鼠标滚轮向下 — 缩小
        MouseEventKind::ScrollDown => {
            if app.zoom > 1 {
                app.zoom -= 1;
                app.status_message = format!("缩放: {}x", app.zoom);
            }
        }
        // 其他鼠标事件忽略
        _ => {}
    }
}
