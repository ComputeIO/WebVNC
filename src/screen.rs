//! Minimal framebuffer / screen abstraction for the x11vnc PoC.
//!
//! This is intentionally small and safe: it models a simple RGB24
//! framebuffer with basic operations used by the higher-level server.

#[derive(Debug, Clone)]
pub struct Screen {
    width: u32,
    height: u32,
    // RGB24 buffer, row-major: width * height * 3 bytes
    pixels: Vec<u8>,
}

impl Screen {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(3);
        Screen {
            width,
            height,
            pixels: vec![0u8; size],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.width = w;
        self.height = h;
        let size = (w as usize).saturating_mul(h as usize).saturating_mul(3);
        self.pixels.resize(size, 0);
    }

    /// Fill the entire screen with an RGB color tuple
    pub fn fill(&mut self, r: u8, g: u8, b: u8) {
        let mut idx = 0usize;
        while idx + 2 < self.pixels.len() {
            self.pixels[idx] = r;
            self.pixels[idx + 1] = g;
            self.pixels[idx + 2] = b;
            idx += 3;
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Option<(u8, u8, u8)> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 3;
        Some((self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2]))
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 3;
        self.pixels[idx] = r;
        self.pixels[idx + 1] = g;
        self.pixels[idx + 2] = b;
        true
    }

    pub fn raw_pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Mutable access to the raw framebuffer bytes (RGB24).
    pub fn raw_pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_fill() {
        let mut s = Screen::new(10, 5);
        assert_eq!(s.width(), 10);
        assert_eq!(s.height(), 5);
        s.fill(0x12, 0x34, 0x56);
        assert_eq!(s.get_pixel(0, 0).unwrap(), (0x12, 0x34, 0x56));
        assert_eq!(s.get_pixel(9, 4).unwrap(), (0x12, 0x34, 0x56));
    }

    #[test]
    fn set_and_get_pixel() {
        let mut s = Screen::new(3, 3);
        assert!(s.set_pixel(1, 1, 10, 20, 30));
        assert_eq!(s.get_pixel(1, 1).unwrap(), (10, 20, 30));
        assert!(!s.set_pixel(4, 4, 1, 2, 3));
    }

    #[test]
    fn resize_keeps_correct_length() {
        let mut s = Screen::new(4, 4);
        let old_len = s.raw_pixels().len();
        s.resize(2, 2);
        assert_eq!(s.raw_pixels().len(), 2 * 2 * 3);
        assert!(s.raw_pixels().len() < old_len);
    }
}
