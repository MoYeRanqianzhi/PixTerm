// PixTerm - Canvas 模块
// 画布数据结构：二维像素网格

use crate::color::Rgb;

/// 画布数据结构
/// 存储二维像素网格，每个像素为可选的 RGB 颜色
/// `None` 表示该像素为空（透明/默认背景色）
pub struct Canvas {
    /// 画布宽度（像素列数）
    pub width: u16,
    /// 画布高度（像素行数）
    pub height: u16,
    /// 二维像素网格：pixels[y][x]，`None` 表示空像素
    pub pixels: Vec<Vec<Option<Rgb>>>,
}

impl Canvas {
    /// 创建指定尺寸的空白画布
    /// 所有像素初始化为 `None`（空/透明）
    pub fn new(width: u16, height: u16) -> Self {
        // 构建 height 行 x width 列的二维向量，全部填充 None
        let pixels = vec![vec![None; width as usize]; height as usize];
        Self {
            width,
            height,
            pixels,
        }
    }

    /// 获取指定坐标处的像素颜色
    /// 如果坐标越界则返回 `None`
    pub fn get_pixel(&self, x: u16, y: u16) -> Option<Rgb> {
        // 边界检查：坐标必须在画布范围内
        if x >= self.width || y >= self.height {
            return None;
        }
        // 返回该位置的像素值（可能是 None 或 Some(color)）
        self.pixels[y as usize][x as usize]
    }

    /// 设置指定坐标处的像素颜色
    /// `color` 为 `None` 时表示清除该像素（橡皮擦效果）
    /// 如果坐标越界则不执行任何操作
    pub fn set_pixel(&mut self, x: u16, y: u16, color: Option<Rgb>) {
        // 边界检查：坐标必须在画布范围内
        if x < self.width && y < self.height {
            self.pixels[y as usize][x as usize] = color;
        }
    }

    /// 调整画布尺寸
    /// 缩小时裁剪超出部分，扩大时新像素填充 `None`
    pub fn resize(&mut self, new_width: u16, new_height: u16) {
        // 调整每一行的列数
        for row in &mut self.pixels {
            row.resize(new_width as usize, None);
        }
        // 如果新高度更大，添加新行（全部为 None）
        // 如果新高度更小，截断多余行
        self.pixels
            .resize(new_height as usize, vec![None; new_width as usize]);
        // 更新尺寸字段
        self.width = new_width;
        self.height = new_height;
    }

    /// 清空画布：将所有像素重置为 `None`
    pub fn clear(&mut self) {
        for row in &mut self.pixels {
            for pixel in row.iter_mut() {
                *pixel = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_canvas() {
        let canvas = Canvas::new(16, 8);
        assert_eq!(canvas.width, 16);
        assert_eq!(canvas.height, 8);
        // 所有像素应为 None
        assert_eq!(canvas.get_pixel(0, 0), None);
        assert_eq!(canvas.get_pixel(15, 7), None);
    }

    #[test]
    fn test_set_get_pixel() {
        let mut canvas = Canvas::new(10, 10);
        // 设置一个像素
        canvas.set_pixel(5, 3, Some((255, 0, 0)));
        assert_eq!(canvas.get_pixel(5, 3), Some((255, 0, 0)));
        // 清除像素
        canvas.set_pixel(5, 3, None);
        assert_eq!(canvas.get_pixel(5, 3), None);
    }

    #[test]
    fn test_out_of_bounds() {
        let mut canvas = Canvas::new(10, 10);
        // 越界读取应返回 None
        assert_eq!(canvas.get_pixel(10, 0), None);
        assert_eq!(canvas.get_pixel(0, 10), None);
        // 越界写入不应 panic
        canvas.set_pixel(10, 10, Some((255, 0, 0)));
    }

    #[test]
    fn test_resize() {
        let mut canvas = Canvas::new(4, 4);
        canvas.set_pixel(1, 1, Some((0, 255, 0)));
        // 扩大画布
        canvas.resize(8, 8);
        assert_eq!(canvas.width, 8);
        assert_eq!(canvas.height, 8);
        // 原有像素保留
        assert_eq!(canvas.get_pixel(1, 1), Some((0, 255, 0)));
        // 新区域为 None
        assert_eq!(canvas.get_pixel(7, 7), None);
    }

    #[test]
    fn test_clear() {
        let mut canvas = Canvas::new(4, 4);
        canvas.set_pixel(0, 0, Some((255, 0, 0)));
        canvas.set_pixel(3, 3, Some((0, 255, 0)));
        canvas.clear();
        // 清空后所有像素为 None
        assert_eq!(canvas.get_pixel(0, 0), None);
        assert_eq!(canvas.get_pixel(3, 3), None);
    }
}
