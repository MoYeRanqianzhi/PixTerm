// PixTerm - Color 模块
// 颜色工具：Hex 解析、调色板、预设色

/// RGB 颜色类型别名，使用 (R, G, B) 三元组表示，每个分量 0-255
pub type Rgb = (u8, u8, u8);

/// 解析 Hex 颜色字符串为 RGB 元组
/// 支持格式：`#RRGGBB`（带 # 前缀的 6 位十六进制字符串）
/// 返回 `Some((r, g, b))` 或 `None`（格式不合法时）
pub fn parse_hex(input: &str) -> Option<Rgb> {
    // 去除首尾空白
    let s = input.trim();
    // 必须以 '#' 开头且总长度为 7（# + 6 位 hex）
    if !s.starts_with('#') || s.len() != 7 {
        return None;
    }
    // 取 '#' 后面的 6 个十六进制字符
    let hex = &s[1..];
    // 分别解析 RR、GG、BB 两位十六进制数
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

/// 将 RGB 颜色格式化为 Hex 字符串（如 `#FF00AA`）
#[cfg(test)]
pub fn rgb_to_hex(color: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", color.0, color.1, color.2)
}

/// 默认调色板：10 个预设颜色，对应数字键 0-9
/// 涵盖常用基础色彩，方便快速切换
pub fn default_palette() -> [Rgb; 10] {
    [
        (0, 0, 0),       // 0: 黑色
        (255, 255, 255),  // 1: 白色
        (255, 0, 0),      // 2: 红色
        (0, 255, 0),      // 3: 绿色
        (0, 0, 255),      // 4: 蓝色
        (255, 255, 0),    // 5: 黄色
        (255, 0, 255),    // 6: 品红
        (0, 255, 255),    // 7: 青色
        (255, 128, 0),    // 8: 橙色
        (128, 0, 255),    // 9: 紫色
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_valid() {
        assert_eq!(parse_hex("#FF0000"), Some((255, 0, 0)));
        assert_eq!(parse_hex("#00ff00"), Some((0, 255, 0)));
        assert_eq!(parse_hex("#0000FF"), Some((0, 0, 255)));
        assert_eq!(parse_hex("#ABCDEF"), Some((171, 205, 239)));
    }

    #[test]
    fn test_parse_hex_invalid() {
        assert_eq!(parse_hex("FF0000"), None);   // 缺少 # 前缀
        assert_eq!(parse_hex("#FFF"), None);      // 太短
        assert_eq!(parse_hex("#GGGGGG"), None);   // 非法十六进制字符
        assert_eq!(parse_hex(""), None);          // 空字符串
    }

    #[test]
    fn test_rgb_to_hex() {
        assert_eq!(rgb_to_hex((255, 0, 0)), "#FF0000");
        assert_eq!(rgb_to_hex((0, 255, 0)), "#00FF00");
        assert_eq!(rgb_to_hex((0, 0, 255)), "#0000FF");
    }
}
