// PixTerm - Canvas 模块
// 画布数据结构：基于 HashMap 的无限大小稀疏像素网格

use crate::color::Rgb;
use std::collections::HashMap;

/// 无限大小画布数据结构
/// 使用 HashMap 稀疏存储，仅记录有颜色的像素，无边界限制
pub struct Canvas {
    /// 稀疏像素存储：键为 (x, y) 坐标，值为 RGB 颜色
    /// 不存在的键即为空像素
    pixels: HashMap<(u16, u16), Rgb>,
}

impl Canvas {
    /// 创建空白画布（无大小限制）
    pub fn new() -> Self {
        Self {
            pixels: HashMap::new(),
        }
    }

    /// 获取指定坐标处的像素颜色
    /// 返回 `Some(color)` 或 `None`（空像素）
    pub fn get_pixel(&self, x: u16, y: u16) -> Option<Rgb> {
        self.pixels.get(&(x, y)).copied()
    }

    /// 设置指定坐标处的像素颜色
    /// `Some(color)` 绘制颜色，`None` 清除像素（橡皮擦）
    pub fn set_pixel(&mut self, x: u16, y: u16, color: Option<Rgb>) {
        match color {
            Some(c) => {
                self.pixels.insert((x, y), c);
            }
            None => {
                self.pixels.remove(&(x, y));
            }
        }
    }

    /// 清空画布：移除所有像素
    pub fn clear(&mut self) {
        self.pixels.clear();
    }

    /// 计算所有像素的边界框
    /// 返回 `Some((min_x, min_y, max_x, max_y))`，画布为空时返回 `None`
    pub fn bounding_box(&self) -> Option<(u16, u16, u16, u16)> {
        if self.pixels.is_empty() {
            return None;
        }
        let mut min_x = u16::MAX;
        let mut min_y = u16::MAX;
        let mut max_x = 0u16;
        let mut max_y = 0u16;
        for &(x, y) in self.pixels.keys() {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        Some((min_x, min_y, max_x, max_y))
    }

    /// 获取内部像素 HashMap 的引用（供 file 模块序列化使用）
    pub fn pixels(&self) -> &HashMap<(u16, u16), Rgb> {
        &self.pixels
    }

    /// 画布是否为空（没有任何像素）
    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_canvas() {
        let canvas = Canvas::new();
        // 空画布任意坐标返回 None
        assert_eq!(canvas.get_pixel(0, 0), None);
        assert_eq!(canvas.get_pixel(1000, 1000), None);
        assert!(canvas.is_empty());
    }

    #[test]
    fn test_set_get_pixel() {
        let mut canvas = Canvas::new();
        // 设置一个像素
        canvas.set_pixel(5, 3, Some((255, 0, 0)));
        assert_eq!(canvas.get_pixel(5, 3), Some((255, 0, 0)));
        assert!(!canvas.is_empty());
        // 清除像素
        canvas.set_pixel(5, 3, None);
        assert_eq!(canvas.get_pixel(5, 3), None);
        assert!(canvas.is_empty());
    }

    #[test]
    fn test_infinite_coords() {
        let mut canvas = Canvas::new();
        // 可以在任意 u16 坐标绘制
        canvas.set_pixel(0, 0, Some((255, 0, 0)));
        canvas.set_pixel(10000, 20000, Some((0, 255, 0)));
        canvas.set_pixel(u16::MAX, u16::MAX, Some((0, 0, 255)));
        assert_eq!(canvas.get_pixel(0, 0), Some((255, 0, 0)));
        assert_eq!(canvas.get_pixel(10000, 20000), Some((0, 255, 0)));
        assert_eq!(canvas.get_pixel(u16::MAX, u16::MAX), Some((0, 0, 255)));
    }

    #[test]
    fn test_bounding_box() {
        let mut canvas = Canvas::new();
        assert_eq!(canvas.bounding_box(), None);
        canvas.set_pixel(5, 10, Some((255, 0, 0)));
        canvas.set_pixel(20, 3, Some((0, 255, 0)));
        assert_eq!(canvas.bounding_box(), Some((5, 3, 20, 10)));
    }

    #[test]
    fn test_clear() {
        let mut canvas = Canvas::new();
        canvas.set_pixel(0, 0, Some((255, 0, 0)));
        canvas.set_pixel(3, 3, Some((0, 255, 0)));
        canvas.clear();
        assert_eq!(canvas.get_pixel(0, 0), None);
        assert_eq!(canvas.get_pixel(3, 3), None);
        assert!(canvas.is_empty());
    }
}
