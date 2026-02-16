// PixTerm - Command 模块
// 命令模式解析与执行

use crate::color::Rgb;

/// 命令枚举，表示用户在命令模式下输入的各种命令
#[derive(Debug, PartialEq)]
pub enum Command {
    /// 显示帮助信息
    Help,
    /// 保存画布到文件（可选文件名，默认 "canvas.json"）
    Save(Option<String>),
    /// 从文件加载画布（可选文件名，默认 "canvas.json"）
    Load(Option<String>),
    /// 退出程序
    Quit,
    /// 在指定坐标绘制指定颜色的像素
    /// `(x, y, color)`
    Paint(u16, u16, Rgb),
    /// 撤销上一步操作
    Undo,
    /// 重做上一步被撤销的操作
    Redo,
    /// 查看指定坐标处的像素颜色
    /// `(x, y)`
    Color(u16, u16),
    /// 清空画布
    Clear,
    /// 切换画笔颜色（通过直接输入 Hex 颜色代码）
    SetBrushColor(Rgb),
}

/// 解析命令字符串为 Command 枚举
/// `input` - 用户输入的命令字符串（不含前导 `:`）
/// 返回 `Some(Command)` 或 `None`（命令不合法时）
pub fn parse_command(input: &str) -> Option<Command> {
    // 去除首尾空白
    let trimmed = input.trim();

    // 空输入不是有效命令
    if trimmed.is_empty() {
        return None;
    }

    // 检查是否为 Hex 颜色代码（以 # 开头）
    if trimmed.starts_with('#') {
        // 尝试解析为颜色
        let color = crate::color::parse_hex(trimmed)?;
        return Some(Command::SetBrushColor(color));
    }

    // 按空白分割成命令名和参数
    let parts: Vec<&str> = trimmed.splitn(4, ' ').collect();
    let cmd_name = parts[0].to_lowercase();

    match cmd_name.as_str() {
        // :help — 显示帮助
        "help" => Some(Command::Help),

        // :save [文件名] — 保存画布
        "save" => {
            let filename = parts.get(1).map(|s| s.to_string());
            Some(Command::Save(filename))
        }

        // :load [文件名] — 加载画布
        "load" => {
            let filename = parts.get(1).map(|s| s.to_string());
            Some(Command::Load(filename))
        }

        // :quit 或 :q — 退出程序
        "quit" | "q" => Some(Command::Quit),

        // :paint x y #RRGGBB — 在指定坐标绘制指定颜色
        "paint" => {
            // 需要至少 3 个参数：x, y, color
            if parts.len() < 4 {
                return None;
            }
            let x: u16 = parts[1].parse().ok()?;
            let y: u16 = parts[2].parse().ok()?;
            let color = crate::color::parse_hex(parts[3])?;
            Some(Command::Paint(x, y, color))
        }

        // :undo — 撤销
        "undo" => Some(Command::Undo),

        // :redo — 重做
        "redo" => Some(Command::Redo),

        // :color x y — 查看指定坐标的颜色
        "color" => {
            if parts.len() < 3 {
                return None;
            }
            let x: u16 = parts[1].parse().ok()?;
            let y: u16 = parts[2].parse().ok()?;
            Some(Command::Color(x, y))
        }

        // :clear — 清空画布
        "clear" => Some(Command::Clear),

        // 未识别的命令
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help() {
        assert_eq!(parse_command("help"), Some(Command::Help));
        assert_eq!(parse_command("HELP"), Some(Command::Help));
    }

    #[test]
    fn test_parse_save() {
        assert_eq!(parse_command("save"), Some(Command::Save(None)));
        assert_eq!(
            parse_command("save myfile.json"),
            Some(Command::Save(Some("myfile.json".to_string())))
        );
    }

    #[test]
    fn test_parse_load() {
        assert_eq!(parse_command("load"), Some(Command::Load(None)));
        assert_eq!(
            parse_command("load art.json"),
            Some(Command::Load(Some("art.json".to_string())))
        );
    }

    #[test]
    fn test_parse_quit() {
        assert_eq!(parse_command("quit"), Some(Command::Quit));
        assert_eq!(parse_command("q"), Some(Command::Quit));
    }

    #[test]
    fn test_parse_paint() {
        assert_eq!(
            parse_command("paint 10 20 #FF0000"),
            Some(Command::Paint(10, 20, (255, 0, 0)))
        );
        // 参数不足
        assert_eq!(parse_command("paint 10 20"), None);
        // 无效坐标
        assert_eq!(parse_command("paint abc 20 #FF0000"), None);
    }

    #[test]
    fn test_parse_undo_redo() {
        assert_eq!(parse_command("undo"), Some(Command::Undo));
        assert_eq!(parse_command("redo"), Some(Command::Redo));
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_command("color 5 10"), Some(Command::Color(5, 10)));
        assert_eq!(parse_command("color 5"), None); // 参数不足
    }

    #[test]
    fn test_parse_clear() {
        assert_eq!(parse_command("clear"), Some(Command::Clear));
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(
            parse_command("#FF00FF"),
            Some(Command::SetBrushColor((255, 0, 255)))
        );
        assert_eq!(parse_command("#ZZZZZZ"), None); // 无效 hex
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse_command(""), None);
        assert_eq!(parse_command("   "), None);
    }

    #[test]
    fn test_parse_unknown() {
        assert_eq!(parse_command("foobar"), None);
    }
}
