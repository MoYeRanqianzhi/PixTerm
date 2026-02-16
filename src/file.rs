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
    /// 画布宽度（由像素边界框计算得出）
    pub width: u16,
    /// 画布高度（由像素边界框计算得出）
    pub height: u16,
    /// 像素数据：二维数组，每个元素为 null 或 [r, g, b]
    /// 坐标系原点为边界框左上角 (min_x, min_y)
    pub pixels: Vec<Vec<Option<[u8; 3]>>>,
}

/// 将画布保存为 JSON 文件
/// `canvas` - 要保存的画布引用
/// `path` - 输出文件路径
/// 返回 `io::Result<()>`，文件写入失败时返回错误
pub fn save_canvas(canvas: &Canvas, path: &Path) -> io::Result<()> {
    // 获取像素边界框；画布为空时保存空文件（0x0）
    let (width, height, min_x, min_y) = match canvas.bounding_box() {
        Some((min_x, min_y, max_x, max_y)) => {
            // 边界框宽高 = max - min + 1
            let w = max_x - min_x + 1;
            let h = max_y - min_y + 1;
            (w, h, min_x, min_y)
        }
        None => {
            // 画布为空，保存 0x0 空画布
            (0, 0, 0, 0)
        }
    };

    // 构造二维像素数组：以边界框左上角为原点
    let mut pixels: Vec<Vec<Option<[u8; 3]>>> = Vec::with_capacity(height as usize);
    for row in 0..height {
        let mut row_data: Vec<Option<[u8; 3]>> = Vec::with_capacity(width as usize);
        for col in 0..width {
            // 将相对坐标转换为画布绝对坐标
            let abs_x = min_x + col;
            let abs_y = min_y + row;
            // 查询像素颜色并转换为 [r, g, b] 格式
            let pixel = canvas
                .get_pixel(abs_x, abs_y)
                .map(|(r, g, b)| [r, g, b]);
            row_data.push(pixel);
        }
        pixels.push(row_data);
    }

    // 构造序列化数据结构
    let save_data = SaveData {
        version: "1.0.0".to_string(),
        width,
        height,
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

    // 创建空画布，逐像素填充
    let mut canvas = Canvas::new();

    // 遍历二维像素数组，将有颜色的像素写入 HashMap
    for (y, row) in save_data.pixels.iter().enumerate() {
        for (x, pixel) in row.iter().enumerate() {
            if let Some(arr) = pixel {
                let color: Rgb = (arr[0], arr[1], arr[2]);
                canvas.set_pixel(x as u16, y as u16, Some(color));
            }
        }
    }

    Ok(canvas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_save_and_load() {
        // 创建画布并绘制几个像素
        let mut canvas = Canvas::new();
        canvas.set_pixel(0, 0, Some((255, 0, 0)));
        canvas.set_pixel(1, 1, Some((0, 255, 0)));
        canvas.set_pixel(3, 3, Some((0, 0, 255)));

        // 保存到临时文件
        let path = Path::new("test_canvas_save_load.json");
        save_canvas(&canvas, path).unwrap();

        // 从文件加载
        let loaded = load_canvas(path).unwrap();

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
    fn test_save_empty_canvas() {
        // 空画布保存应成功
        let canvas = Canvas::new();
        let path = Path::new("test_empty_canvas.json");
        save_canvas(&canvas, path).unwrap();

        // 加载回来仍应为空
        let loaded = load_canvas(path).unwrap();
        assert!(loaded.is_empty());

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_nonexistent_file() {
        // 加载不存在的文件应返回错误
        let result = load_canvas(Path::new("nonexistent_file_xyz.json"));
        assert!(result.is_err());
    }
}
