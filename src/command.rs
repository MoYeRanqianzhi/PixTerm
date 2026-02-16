// PixTerm - Command 模块
// 命令模式解析与执行

use crate::color::Rgb;

/// 命令枚举，表示用户在命令模式下输入的各种命令
#[derive(Debug, PartialEq)]
pub enum Command {
    /// 显示帮助信息
    Help,
    /// 保存画布到文件（可选文件名，默认 "canvas.ptd"）
    Save(Option<String>),
    /// 从文件加载画布（可选文件名，默认 "canvas.ptd"）
    Load(Option<String>),
    /// 退出程序
    Quit,
    /// 在指定坐标范围绘制像素
    /// `x_range` — X 坐标范围 (start, end)，单坐标时 start == end
    /// `y_range` — Y 坐标范围 (start, end)，单坐标时 start == end
    /// `color` — 可选颜色，`None` 表示使用当前画笔颜色
    Paint {
        x_range: (u16, u16),
        y_range: (u16, u16),
        color: Option<Rgb>,
    },
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
    /// 导出画布为 PNG 图片（可选文件名，默认 "canvas.png"）
    Export(Option<String>),
    /// 从 PNG 图片导入像素到画布（文件名必须指定）
    Import(String),
}

/// 解析坐标参数，支持单值和范围两种格式
/// - 单值：`"5"` → `(5, 5)`
/// - 范围：`"5:10"` → `(5, 10)`
/// 范围的起始值必须 ≤ 结束值，否则返回 `None`
fn parse_coord_range(s: &str) -> Option<(u16, u16)> {
    if let Some((a, b)) = s.split_once(':') {
        // 范围格式：解析冒号两侧的起始值和结束值
        let start: u16 = a.parse().ok()?;
        let end: u16 = b.parse().ok()?;
        // 起始值必须小于等于结束值
        if start > end {
            return None;
        }
        Some((start, end))
    } else {
        // 单值格式：起始和结束相同
        let v: u16 = s.parse().ok()?;
        Some((v, v))
    }
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

        // :paint x y [#RRGGBB] — 在指定坐标/范围绘制像素
        // 支持的格式：
        //   paint x y             — 单像素，使用当前画笔颜色
        //   paint x y #RRGGBB     — 单像素，使用指定颜色
        //   paint x1:x2 y1:y2     — 矩形范围，使用当前画笔颜色
        //   paint x1:x2 y1:y2 #RR — 矩形范围，使用指定颜色
        //   paint x y1:y2         — 单列范围（x 固定，y 变化）
        //   paint x1:x2 y         — 单行范围（y 固定，x 变化）
        "paint" => {
            // 至少需要 2 个参数：x 坐标和 y 坐标
            if parts.len() < 3 {
                return None;
            }
            // 解析 X 坐标（支持单值或范围）
            let x_range = parse_coord_range(parts[1])?;
            // 解析 Y 坐标（支持单值或范围）
            let y_range = parse_coord_range(parts[2])?;
            // 第 4 个参数为可选颜色，不提供则使用当前画笔颜色
            let color = if parts.len() >= 4 {
                Some(crate::color::parse_hex(parts[3])?)
            } else {
                None
            };
            Some(Command::Paint { x_range, y_range, color })
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

        // :export [文件名] — 导出画布为 PNG 图片
        "export" => {
            let filename = parts.get(1).map(|s| s.to_string());
            Some(Command::Export(filename))
        }

        // :import 文件名 — 从 PNG 图片导入像素
        "import" => {
            // import 命令必须指定文件名
            let filename = parts.get(1)?;
            Some(Command::Import(filename.to_string()))
        }

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
        // 单像素 + 指定颜色
        assert_eq!(
            parse_command("paint 10 20 #FF0000"),
            Some(Command::Paint {
                x_range: (10, 10),
                y_range: (20, 20),
                color: Some((255, 0, 0)),
            })
        );
        // 单像素 + 使用当前画笔颜色（不指定颜色）
        assert_eq!(
            parse_command("paint 10 20"),
            Some(Command::Paint {
                x_range: (10, 10),
                y_range: (20, 20),
                color: None,
            })
        );
        // 无效坐标
        assert_eq!(parse_command("paint abc 20 #FF0000"), None);
        // 参数不足
        assert_eq!(parse_command("paint 10"), None);
    }

    #[test]
    fn test_parse_paint_range() {
        // 矩形范围 + 指定颜色
        assert_eq!(
            parse_command("paint 0:10 5:15 #00FF00"),
            Some(Command::Paint {
                x_range: (0, 10),
                y_range: (5, 15),
                color: Some((0, 255, 0)),
            })
        );
        // 矩形范围 + 使用当前画笔颜色
        assert_eq!(
            parse_command("paint 0:10 5:15"),
            Some(Command::Paint {
                x_range: (0, 10),
                y_range: (5, 15),
                color: None,
            })
        );
        // 混合：单 x + 范围 y
        assert_eq!(
            parse_command("paint 5 0:20"),
            Some(Command::Paint {
                x_range: (5, 5),
                y_range: (0, 20),
                color: None,
            })
        );
        // 混合：范围 x + 单 y
        assert_eq!(
            parse_command("paint 0:10 5"),
            Some(Command::Paint {
                x_range: (0, 10),
                y_range: (5, 5),
                color: None,
            })
        );
        // 起始大于结束 → 无效
        assert_eq!(parse_command("paint 10:5 0:3"), None);
        assert_eq!(parse_command("paint 0:3 10:5"), None);
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
    fn test_parse_export() {
        // 无参数：使用默认文件名
        assert_eq!(parse_command("export"), Some(Command::Export(None)));
        // 指定文件名
        assert_eq!(
            parse_command("export myart.png"),
            Some(Command::Export(Some("myart.png".to_string())))
        );
    }

    #[test]
    fn test_parse_import() {
        // 必须指定文件名
        assert_eq!(
            parse_command("import photo.png"),
            Some(Command::Import("photo.png".to_string()))
        );
        // 无文件名 → 无效
        assert_eq!(parse_command("import"), None);
    }

    #[test]
    fn test_parse_unknown() {
        assert_eq!(parse_command("foobar"), None);
    }
}
