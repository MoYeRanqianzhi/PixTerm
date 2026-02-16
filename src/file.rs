// PixTerm - File 模块
// 文件保存/加载（支持 .ptd 压缩格式和 .json 纯文本格式）
// PNG 图片导入/导出

use crate::canvas::Canvas;
use crate::color::Rgb;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use image::{ImageBuffer, Rgba};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read as _, Write as _};
use std::path::Path;

/// 画布文件的 JSON 序列化结构体
/// 用于将画布数据持久化到磁盘（.ptd 和 .json 共用此结构）
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

/// 将画布保存到文件
/// 根据文件扩展名选择存储格式：
/// - `.ptd` → JSON 序列化后 gzip 压缩（紧凑格式，体积小）
/// - 其他（`.json`、无后缀等）→ JSON 纯文本（pretty-print，便于人工阅读）
///
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

    // 根据扩展名判断存储格式
    let is_ptd = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ptd"));

    if is_ptd {
        // .ptd 格式：紧凑 JSON → gzip 压缩
        // 使用紧凑格式（to_string 而非 to_string_pretty）以进一步减小体积
        let json = serde_json::to_string(&save_data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // 创建输出文件，使用 gzip 压缩写入
        let file = fs::File::create(path)?;
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write_all(json.as_bytes())?;
        // finish() 完成压缩并刷新所有缓冲数据
        encoder.finish()?;
    } else {
        // .json 或其他格式：格式化 JSON 纯文本（便于人工阅读和调试）
        let json = serde_json::to_string_pretty(&save_data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(path, json)?;
    }

    Ok(())
}

/// 从文件加载画布
/// 根据文件扩展名选择解析方式：
/// - `.ptd` → 读取字节 → gzip 解压 → JSON 反序列化
/// - 其他 → 读取文本 → JSON 反序列化
///
/// `path` - 输入文件路径
/// 返回 `io::Result<Canvas>`，文件不存在或格式错误时返回错误
pub fn load_canvas(path: &Path) -> io::Result<Canvas> {
    // 根据扩展名判断文件格式
    let is_ptd = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ptd"));

    // 读取并反序列化 JSON 数据
    let save_data: SaveData = if is_ptd {
        // .ptd 格式：读取二进制数据 → gzip 解压 → JSON 反序列化
        let compressed = fs::read(path)?;
        let mut decoder = GzDecoder::new(&compressed[..]);
        let mut json = String::new();
        decoder.read_to_string(&mut json)?;
        serde_json::from_str(&json)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
    } else {
        // .json 或其他格式：直接读取文本 → JSON 反序列化
        let json = fs::read_to_string(path)?;
        serde_json::from_str(&json)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
    };

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

/// 将画布导出为 PNG 图片
/// 使用边界框计算图像尺寸，空画布导出为 1×1 透明图像
/// 有色像素写入为不透明 RGBA，空像素写入为完全透明
///
/// `canvas` - 要导出的画布引用
/// `path` - 输出 PNG 文件路径
/// 返回 `io::Result<()>`，图像创建或写入失败时返回错误
pub fn export_png(canvas: &Canvas, path: &Path) -> io::Result<()> {
    // 计算边界框尺寸
    let (width, height, min_x, min_y) = match canvas.bounding_box() {
        Some((min_x, min_y, max_x, max_y)) => {
            let w = (max_x - min_x + 1) as u32;
            let h = (max_y - min_y + 1) as u32;
            (w, h, min_x, min_y)
        }
        None => {
            // 空画布：导出 1×1 透明图像
            (1, 1, 0, 0)
        }
    };

    // 创建 RGBA 图像缓冲区
    let img = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_fn(width, height, |x, y| {
        // 将图像像素坐标转换回画布绝对坐标
        let canvas_x = min_x + x as u16;
        let canvas_y = min_y + y as u16;
        match canvas.get_pixel(canvas_x, canvas_y) {
            // 有色像素：不透明 (alpha = 255)
            Some((r, g, b)) => Rgba([r, g, b, 255]),
            // 空像素：完全透明 (alpha = 0)
            None => Rgba([0, 0, 0, 0]),
        }
    });

    // 保存为 PNG 文件
    img.save(path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(())
}

/// 从 PNG 图片导入像素数据到新画布
/// 读取图像的每个像素，alpha > 0 的像素写入画布
///
/// `path` - 输入 PNG 文件路径
/// 返回 `io::Result<Canvas>`，图像读取失败时返回错误
pub fn import_png(path: &Path) -> io::Result<Canvas> {
    // 打开图像文件并转换为 RGBA8 格式
    let img = image::open(path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
        .to_rgba8();

    let mut canvas = Canvas::new();

    // 遍历所有像素，将 alpha > 0 的像素写入画布
    for (x, y, pixel) in img.enumerate_pixels() {
        let Rgba([r, g, b, a]) = *pixel;
        if a > 0 {
            // 图像坐标直接映射为画布坐标（u16 范围限制）
            if x <= u16::MAX as u32 && y <= u16::MAX as u32 {
                canvas.set_pixel(x as u16, y as u16, Some((r, g, b)));
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
    fn test_save_and_load_json() {
        // 创建画布并绘制几个像素
        let mut canvas = Canvas::new();
        canvas.set_pixel(0, 0, Some((255, 0, 0)));
        canvas.set_pixel(1, 1, Some((0, 255, 0)));
        canvas.set_pixel(3, 3, Some((0, 0, 255)));

        // 保存到 JSON 格式临时文件
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
    fn test_save_and_load_ptd() {
        // 创建画布并绘制像素
        let mut canvas = Canvas::new();
        canvas.set_pixel(0, 0, Some((255, 0, 0)));
        canvas.set_pixel(5, 5, Some((0, 255, 0)));
        canvas.set_pixel(10, 10, Some((0, 0, 255)));

        // 保存到 .ptd 压缩格式
        let path = Path::new("test_canvas_save_load.ptd");
        save_canvas(&canvas, path).unwrap();

        // 验证文件存在且非空
        let file_size = fs::metadata(path).unwrap().len();
        assert!(file_size > 0);

        // 从 .ptd 文件加载
        let loaded = load_canvas(path).unwrap();

        // 验证像素数据一致
        assert_eq!(loaded.get_pixel(0, 0), Some((255, 0, 0)));
        assert_eq!(loaded.get_pixel(5, 5), Some((0, 255, 0)));
        assert_eq!(loaded.get_pixel(10, 10), Some((0, 0, 255)));
        assert_eq!(loaded.get_pixel(3, 3), None);

        // 清理临时文件
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_save_empty_canvas() {
        // 空画布保存到 JSON 应成功
        let canvas = Canvas::new();
        let path = Path::new("test_empty_canvas.json");
        save_canvas(&canvas, path).unwrap();

        // 加载回来仍应为空
        let loaded = load_canvas(path).unwrap();
        assert!(loaded.is_empty());

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_save_empty_canvas_ptd() {
        // 空画布保存到 .ptd 应成功
        let canvas = Canvas::new();
        let path = Path::new("test_empty_canvas.ptd");
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

    #[test]
    fn test_export_import_png() {
        // 创建画布并绘制像素
        let mut canvas = Canvas::new();
        canvas.set_pixel(0, 0, Some((255, 0, 0)));
        canvas.set_pixel(1, 0, Some((0, 255, 0)));
        canvas.set_pixel(0, 1, Some((0, 0, 255)));

        // 导出为 PNG
        let path = Path::new("test_export_import.png");
        export_png(&canvas, path).unwrap();

        // 验证 PNG 文件存在且非空
        let file_size = fs::metadata(path).unwrap().len();
        assert!(file_size > 0);

        // 从 PNG 导入
        let imported = import_png(path).unwrap();

        // 验证像素数据往返一致
        assert_eq!(imported.get_pixel(0, 0), Some((255, 0, 0)));
        assert_eq!(imported.get_pixel(1, 0), Some((0, 255, 0)));
        assert_eq!(imported.get_pixel(0, 1), Some((0, 0, 255)));
        // 空像素（alpha=0）不应被导入
        assert_eq!(imported.get_pixel(1, 1), None);

        // 清理临时文件
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_export_empty_canvas_png() {
        // 空画布应导出为 1×1 透明 PNG
        let canvas = Canvas::new();
        let path = Path::new("test_export_empty.png");
        export_png(&canvas, path).unwrap();

        // 验证文件存在
        assert!(path.exists());

        // 导入回来应为空画布（1×1 透明像素，alpha=0 被跳过）
        let imported = import_png(path).unwrap();
        assert!(imported.is_empty());

        fs::remove_file(path).unwrap();
    }
}
