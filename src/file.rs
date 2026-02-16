// PixTerm - File 模块
// JSON 文件保存/加载

use crate::canvas::Canvas;
use crate::color::Rgb;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// 画布文件的 JSON 序列化结构体
/// 用于将画布数据持久化到磁盘
#[derive(Serialize, Deserialize)]
pub struct SaveData {
    /// 文件格式版本号
    pub version: String,
    /// 画布宽度
    pub width: u16,
    /// 画布高度
    pub height: u16,
    /// 像素数据：二维数组，每个元素为 null 或 [r, g, b]
    pub pixels: Vec<Vec<Option<[u8; 3]>>>,
}

/// 将画布保存为 JSON 文件
/// `canvas` - 要保存的画布引用
/// `path` - 输出文件路径
/// 返回 `io::Result<()>`，文件写入失败时返回错误
pub fn save_canvas(canvas: &Canvas, path: &Path) -> io::Result<()> {
    // 将画布像素数据转换为可序列化格式
    // Rgb 元组 (u8, u8, u8) → [u8; 3] 数组（JSON 中表现为 [r, g, b]）
    let pixels: Vec<Vec<Option<[u8; 3]>>> = canvas
        .pixels
        .iter()
        .map(|row| {
            row.iter()
                .map(|pixel| pixel.map(|(r, g, b)| [r, g, b]))
                .collect()
        })
        .collect();

    // 构造序列化数据结构
    let save_data = SaveData {
        version: "1.0.0".to_string(),
        width: canvas.width,
        height: canvas.height,
        pixels,
    };

    // 序列化为格式化的 JSON 字符串（便于人工阅读和调试）
    let json = serde_json::to_string_pretty(&save_data)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // 写入文件
    fs::write(path, json)
}

/// 从 JSON 文件加载画布
/// `path` - 输入文件路径
/// 返回 `io::Result<Canvas>`，文件不存在或格式错误时返回错误
pub fn load_canvas(path: &Path) -> io::Result<Canvas> {
    // 读取文件内容
    let json = fs::read_to_string(path)?;

    // 反序列化 JSON 字符串为 SaveData 结构
    let save_data: SaveData =
        serde_json::from_str(&json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // 将 [u8; 3] 数组转换回 Rgb 元组
    let pixels: Vec<Vec<Option<Rgb>>> = save_data
        .pixels
        .iter()
        .map(|row| {
            row.iter()
                .map(|pixel| pixel.map(|arr| (arr[0], arr[1], arr[2])))
                .collect()
        })
        .collect();

    // 构造 Canvas 结构体
    let canvas = Canvas {
        width: save_data.width,
        height: save_data.height,
        pixels,
    };

    Ok(canvas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_save_and_load() {
        // 创建一个小画布并绘制几个像素
        let mut canvas = Canvas::new(4, 4);
        canvas.set_pixel(0, 0, Some((255, 0, 0)));
        canvas.set_pixel(1, 1, Some((0, 255, 0)));
        canvas.set_pixel(3, 3, Some((0, 0, 255)));

        // 保存到临时文件
        let path = Path::new("test_canvas_save_load.json");
        save_canvas(&canvas, path).unwrap();

        // 从文件加载
        let loaded = load_canvas(path).unwrap();

        // 验证尺寸一致
        assert_eq!(loaded.width, 4);
        assert_eq!(loaded.height, 4);
        // 验证像素数据一致
        assert_eq!(loaded.get_pixel(0, 0), Some((255, 0, 0)));
        assert_eq!(loaded.get_pixel(1, 1), Some((0, 255, 0)));
        assert_eq!(loaded.get_pixel(3, 3), Some((0, 0, 255)));
        // 未绘制的像素应为 None
        assert_eq!(loaded.get_pixel(2, 2), None);

        // 清理临时文件
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_nonexistent_file() {
        // 加载不存在的文件应返回错误
        let result = load_canvas(Path::new("nonexistent_file_xyz.json"));
        assert!(result.is_err());
    }
}
