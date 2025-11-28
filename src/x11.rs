//! Minimal X11 abstraction for the PoC.
//!
//! Behavior is feature-gated: by default (no `x11` feature) we keep a safe
//! stub implementation. When `x11` feature is enabled we use the `x11rb`
//! crate to talk to an X11 server and implement real capture functionality.

#[cfg(not(feature = "x11"))]
#[derive(Debug)]
pub struct DisplayHandle {
    // stub-only for now
    pub name: String,
}

#[cfg(not(feature = "x11"))]
impl DisplayHandle {
    pub fn connect(display_name: Option<&str>) -> Result<Self, String> {
        // In the real implementation we'd use x11rb to open the connection.
        let name = display_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| "stub-display".to_string());
        Ok(DisplayHandle { name })
    }

    /// Capture a snapshot of the root window as RGB24 data (width*height*3).
    /// The PoC stub returns a simple pattern of the requested size.
    pub fn capture_root(&self, width: u32, height: u32) -> Result<Vec<u8>, String> {
        if width == 0 || height == 0 {
            return Err("invalid dims".to_string());
        }
        let mut buf = Vec::with_capacity((width as usize) * (height as usize) * 3);
        for y in 0..height {
            for x in 0..width {
                // simple gradient: r=x, g=y, b=(x+y)
                let r = (x % 256) as u8;
                let g = (y % 256) as u8;
                let b = ((x + y) % 256) as u8;
                buf.push(r);
                buf.push(g);
                buf.push(b);
            }
        }
        Ok(buf)
    }

    /// Inject a key event into the display (press/release) — stubbed when
    /// the 'x11' feature is not enabled.
    pub fn inject_key_event(&self, _keysym: u32, _pressed: bool) -> Result<(), String> {
        Ok(())
    }

    /// Inject a pointer event (x,y and button_mask) into the display — stub.
    pub fn inject_pointer_event(&self, _x: i32, _y: i32, _button_mask: u8) -> Result<(), String> {
        Ok(())
    }
}

/// Normalize raw image bytes with `bpp` bytes-per-pixel into an RGB24
/// vector (R, G, B per pixel). Accepts 3 or 4 byte source pixels.
fn normalize_to_rgb24(src: &[u8], width: usize, height: usize, bpp: usize) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err("invalid dims".to_string());
    }
    if bpp != 3 && bpp != 4 {
        return Err("unsupported bpp".to_string());
    }
    let npix = width.saturating_mul(height);
    let expected = npix.saturating_mul(bpp);
    if src.len() < expected {
        return Err("source buffer too small".to_string());
    }
    let mut out = Vec::with_capacity(npix * 3);
    if bpp == 3 {
        out.extend_from_slice(&src[..expected]);
        return Ok(out);
    }
    // bpp == 4
    for i in 0..npix {
        let si = i * 4;
        out.push(src[si]);
        out.push(src[si + 1]);
        out.push(src[si + 2]);
    }
    Ok(out)
}

// -- real x11 feature-enabled implementation --------------------------------------------------
#[cfg(feature = "x11")]
mod x11_real {
    use std::sync::Arc;
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::{ConnectionExt as XProtoExt, ImageFormat};
    use x11rb::rust_connection::RustConnection;

    #[derive(Debug)]
    pub struct DisplayHandle {
        pub name: String,
        conn: Arc<RustConnection>,
        screen_num: usize,
    }

    impl DisplayHandle {
        pub fn connect(display_name: Option<&str>) -> Result<Self, String> {
            let (conn, screen_num) =
                x11rb::connect(display_name).map_err(|e| format!("connect: {:?}", e))?;
            let conn = Arc::new(conn);
            let name = display_name
                .map(|s| s.to_string())
                .unwrap_or_else(|| "x11-display".to_string());
            Ok(DisplayHandle {
                name,
                conn,
                screen_num,
            })
        }

        /// Capture the root window as raw bytes. This uses GetImage (ZPixmap)
        /// and returns the raw pixel bytes returned by the server. Consumers
        /// may need to convert pixel order/depth depending on the server.
        pub fn capture_root(&self, width: u32, height: u32) -> Result<Vec<u8>, String> {
            if width == 0 || height == 0 {
                return Err("invalid dims".to_string());
            }
            // Determine the root window for the screen
            let setup = self.conn.setup();
            let screen = &setup.roots[self.screen_num];
            let root = screen.root;

            // Request a ZPixmap image for the root window
            let cookie = self
                .conn
                .get_image(false, root, 0, 0, width, height, u32::MAX)
                .map_err(|e| format!("get_image send error: {:?}", e))?;
            let reply = cookie
                .reply()
                .map_err(|e| format!("get_image reply error: {:?}", e))?;

            // The reply.data contains raw bytes. For many servers this will be
            // in BGR or RGB order depending on depth; we return it as-is for
            // PoC consumers to inspect. For safety, convert to an RGB24 vector
            // if the server reports 24 bits_per_pixel.
            if reply.depth == 24 || reply.depth == 32 {
                // reply.data may include padding or alpha bytes. Normalize
                // it into a strict RGB24 (width * height * 3) vector.
                let bpp = if reply.depth == 32 { 4 } else { 3 };
                let expected_len = (width as usize) * (height as usize) * bpp;
                if reply.data.len() < expected_len {
                    return Err(format!("unexpected image data length: {} < {}", reply.data.len(), expected_len));
                }
                // Normalize rows: many servers return tightly packed row data
                // for simple GetImage, but there may be padding — handle the
                // common case: contiguous pixels with `bpp` bytes each.
                
                normalize_to_rgb24(&reply.data[..expected_len], width as usize, height as usize, bpp)
            } else {
                Err(format!("unsupported depth: {}", reply.depth))
            }
        }

        /// Inject a key event into the display (press/release)
        pub fn inject_key_event(&self, _keysym: u32, _pressed: bool) -> Result<(), String> {
            // TODO: real implementation using XTest / XI2
            Ok(())
        }

        /// Inject a pointer event (x,y and button_mask) into the display
        pub fn inject_pointer_event(&self, _x: i32, _y: i32, _button_mask: u8) -> Result<(), String> {
            // TODO: real implementation using XTest / XI2
            Ok(())
        }
    }
}

#[cfg(feature = "x11")]
pub use x11_real::DisplayHandle;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_and_capture_ok() {
        let d = DisplayHandle::connect(None).unwrap();
        let buf = d.capture_root(8, 4).unwrap();
        assert_eq!(buf.len(), 8 * 4 * 3);
        // check first pixel matches expected gradient
        assert_eq!(&buf[..3], &[0u8, 0u8, 0u8]);
    }

    #[test]
    fn normalize_to_rgb24_3bpp() {
        // two pixels with RGB values: A=(1,2,3), B=(4,5,6)
        let src = vec![1u8, 2u8, 3u8, 4u8, 5u8, 6u8];
        let out = normalize_to_rgb24(&src, 2, 1, 3).unwrap();
        assert_eq!(out, src);
    }

    #[test]
    fn normalize_to_rgb24_4bpp() {
        // two pixels with RGBA values: A=(1,2,3,0xFF), B=(4,5,6,0xAA)
        let src = vec![1u8, 2u8, 3u8, 0xFF, 4u8, 5u8, 6u8, 0xAA];
        let out = normalize_to_rgb24(&src, 2, 1, 4).unwrap();
        assert_eq!(out, vec![1u8, 2u8, 3u8, 4u8, 5u8, 6u8]);
    }
}
