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
    pub fn inject_pointer_event(&mut self, _x: i32, _y: i32, _button_mask: u32) -> Result<(), String> {
        Ok(())
    }
}

#[allow(dead_code)]
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

// -- real x11 feature-enabled implementation -------------------------------------------------
#[cfg(feature = "x11")]
mod x11_real {
    use std::sync::Arc;
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::{self, ImageFormat, ConnectionExt};
    use x11rb::rust_connection::RustConnection;

    #[derive(Debug)]
    pub struct DisplayHandle {
        pub name: String,
        conn: Arc<RustConnection>,
        screen_num: usize,
        xtest_present: bool,
        last_button_mask: u32,
    }

    impl DisplayHandle {
        pub fn connect(display_name: Option<&str>) -> Result<Self, String> {
            let (conn, screen_num) = x11rb::connect(display_name).map_err(|e| format!("connect: {:?}", e))?;
            let conn = Arc::new(conn);
            let name = display_name.map(|s| s.to_string()).unwrap_or_else(|| "x11-display".to_string());

            // check for XTest presence
            let conn_ref: &RustConnection = &*conn;
            let mut xtest_flag = false;
            if let Ok(cookie) = conn_ref.query_extension(b"XTEST") {
                if let Ok(reply) = cookie.reply() {
                    xtest_flag = reply.present;
                }
            }

            Ok(DisplayHandle {
                name,
                conn,
                screen_num,
                xtest_present: xtest_flag,
                last_button_mask: 0,
            })
        }

        pub fn capture_root(&self, width: u32, height: u32) -> Result<Vec<u8>, String> {
            if width == 0 || height == 0 {
                return Err("invalid dims".to_string());
            }
            let conn_ref: &RustConnection = &*self.conn;
            let setup = conn_ref.setup();
            let screen = &setup.roots[self.screen_num];
            let root = screen.root;

            let cookie = conn_ref.get_image(ImageFormat::Z_PIXMAP, root, 0, 0, width as u16, height as u16, u32::MAX)
                .map_err(|e| format!("get_image send error: {:?}", e))?;
            let reply = cookie.reply().map_err(|e| format!("get_image reply error: {:?}", e))?;

            if reply.depth == 24 || reply.depth == 32 {
                let bpp = if reply.depth == 32 { 4 } else { 3 };
                let expected_len = (width as usize) * (height as usize) * bpp;
                if reply.data.len() < expected_len {
                    return Err(format!("unexpected image data length: {} < {}", reply.data.len(), expected_len));
                }
                super::normalize_to_rgb24(&reply.data[..expected_len], width as usize, height as usize, bpp)
            } else {
                Err(format!("unsupported depth: {}", reply.depth))
            }

        }

        pub fn inject_key_event(&self, keysym: u32, pressed: bool) -> Result<(), String> {
            let conn_ref: &RustConnection = &*self.conn;
            let setup = conn_ref.setup();
            let min_keycode = setup.min_keycode;
            let max_keycode = setup.max_keycode;
            let len = (max_keycode as i32 - min_keycode as i32 + 1) as u8;
            let map = conn_ref.get_keyboard_mapping(min_keycode, len)
                .map_err(|e| format!("get_keyboard_mapping failed: {:?}", e))?
                .reply()
                .map_err(|e| format!("get_keyboard_mapping reply failed: {:?}", e))?;

            let per = map.keysyms_per_keycode as usize;
            let mut found_keycode: Option<u8> = None;
            for i in 0..len as usize {
                let base = i * per;
                for j in 0..per {
                    if (map.keysyms[base + j] as u32) == keysym {
                        found_keycode = Some(min_keycode.wrapping_add(i as u8));
                        break;
                    }
                }
                if found_keycode.is_some() { break; }
            }

            if let Some(kc) = found_keycode {
                if self.xtest_present {
                    use x11rb::protocol::xtest;
                    let evtype = if pressed { xproto::KEY_PRESS_EVENT } else { xproto::KEY_RELEASE_EVENT };
                    // fake_input requires time, root, x, y, deviceid arguments after detail
                    let _cookie = xtest::fake_input(conn_ref, evtype as u8, kc, 0u32, x11rb::NONE, 0i16, 0i16, 0u8)
                        .map_err(|e| format!("xtest fake_input failed: {:?}", e))?;
                    conn_ref.flush().map_err(|e| format!("flush failed: {:?}", e))?;
                    Ok(())
                } else {
                    // No XTest: not implemented in PoC
                    Ok(())
                }
            } else {
                Err("keysym not found in keymap".to_string())
            }
        }

        pub fn inject_pointer_event(&mut self, x: i32, y: i32, button_mask: u32) -> Result<(), String> {
            let conn_ref: &RustConnection = &*self.conn;
            let setup = conn_ref.setup();
            let screen = &setup.roots[self.screen_num];
            let root = screen.root;

            conn_ref.warp_pointer(x11rb::NONE, root, 0, 0, 0, 0, x as i16, y as i16)
                .map_err(|e| format!("warp pointer failed: {:?}", e))?;

            if self.xtest_present {
                use x11rb::protocol::xtest;
                let _cookie = xtest::fake_input(conn_ref, xproto::MOTION_NOTIFY_EVENT as u8, 0u8, 0u32, x11rb::NONE, 0i16, 0i16, 0u8)
                    .map_err(|e| format!("xtest fake_input failed: {:?}", e))?;
            }

            let changed = button_mask ^ self.last_button_mask;
            if changed != 0 {
                for i in 0..8 {
                    let bit = 1u32 << i;
                    if changed & bit != 0 {
                        let button = (i + 1) as u8;
                        if button_mask & bit != 0 {
                            if self.xtest_present {
                                use x11rb::protocol::xtest;
                                let _ = xtest::fake_input(conn_ref, xproto::BUTTON_PRESS_EVENT as u8, button, 0u32, x11rb::NONE, 0i16, 0i16, 0u8)
                                    .map_err(|e| format!("xtest fake_input failed: {:?}", e))?;
                            }
                        } else {
                            if self.xtest_present {
                                use x11rb::protocol::xtest;
                                let _ = xtest::fake_input(conn_ref, xproto::BUTTON_RELEASE_EVENT as u8, button, 0u32, x11rb::NONE, 0i16, 0i16, 0u8)
                                    .map_err(|e| format!("xtest fake_input failed: {:?}", e))?;
                            }
                        }
                    }
                }
            }
            // update state and flush
            self.last_button_mask = button_mask;
            conn_ref.flush().map_err(|e| format!("flush failed: {:?}", e))?;
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
        match DisplayHandle::connect(None) {
            Ok(d) => {
                let buf = d.capture_root(8, 4).unwrap();
                assert_eq!(buf.len(), 8 * 4 * 3);
                // check first pixel matches expected gradient
                assert_eq!(&buf[..3], &[0u8, 0u8, 0u8]);
            }
            Err(e) => {
                // No X server available in the test environment — skip.
                eprintln!("skipping connect test: {}", e);
            }
        }
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
