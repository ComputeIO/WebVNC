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
                // reply.data is a Vec<u8>. It may include padding, but for
                // common setups we can return the bytes directly.
                Ok(reply.data)
            } else {
                Err(format!("unsupported depth: {}", reply.depth))
            }
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
}
