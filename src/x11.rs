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
        xtest_present: bool,
        last_button_mask: u8,
    }

    impl DisplayHandle {
        pub fn connect(display_name: Option<&str>) -> Result<Self, String> {
            let (conn, screen_num) =
                x11rb::connect(display_name).map_err(|e| format!("connect: {:?}", e))?;
            let conn = Arc::new(conn);
            let name = display_name
                .map(|s| s.to_string())
                .unwrap_or_else(|| "x11-display".to_string());
            // check for XTest presence
            let mut xtest_flag = false;
            if let Ok(cookie) = conn.query_extension(b"XTEST") {
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
            #[cfg(not(feature = "x11"))]
            {
                Ok(())
            }
            #[cfg(feature = "x11")]
            {
                // Try to convert keysym to keycode and send via XTest if available,
                // otherwise use a generic send_event for KeyPress/KeyRelease.
                use x11rb::protocol::xproto::{EventMask, KeyPressEvent, KeyReleaseEvent};
                use x11rb::protocol::xproto::ConnectionExt as _;

                let conn = &*self.conn;
                let setup = conn.setup();
                let screen = &setup.roots[self.screen_num];
                let root = screen.root;

                // Map keysym to keycode by querying keyboard mapping.
                let min_keycode = setup.min_keycode;
                let max_keycode = setup.max_keycode;
                let len = (max_keycode - min_keycode + 1) as u8;
                let map = conn
                    .get_keyboard_mapping(min_keycode, len)
                    .map_err(|e| format!("get_keyboard_mapping failed: {:?}", e))?
                    .reply()
                    .map_err(|e| format!("get_keyboard_mapping reply failed: {:?}", e))?;

                // Each keycode has `map.keysyms_per_keycode` keysyms
                let per = map.keysyms_per_keycode as usize;
                let mut found_keycode: Option<u8> = None;
                for i in 0..len as usize {
                    let base = i * per;
                    for j in 0..per {
                        if (map.keysyms[base + j] as u32) == _keysym {
                            found_keycode = Some(min_keycode + (i as u8));
                            break;
                        }
                    }
                    if found_keycode.is_some() { break; }
                }
                if let Some(kc) = found_keycode {
                    // Build minimal KeyPress or KeyRelease event and send via SendEvent.
                    use x11rb::protocol::xproto::SendEvent;
                    use x11rb::protocol::xproto::ClientMessageEvent;
                    use x11rb::protocol::xproto::ConnectionExt;
                    // send key event by generating a KeyPress or KeyRelease event
                    // NOTE: using SendEvent may not reach all clients, but it's a
                    // pragmatic step until we can use XTest or XI2.
                    let ev = if _pressed {
                        KeyPressEvent {
                            response_type: x11rb::protocol::xproto::KEY_PRESS_EVENT,
                            detail: kc,
                            sequence: 0,
                            time: 0,
                            root,
                            event: root,
                            child: x11rb::NONE,
                            root_x: 0,
                            root_y: 0,
                            event_x: 0,
                            event_y: 0,
                            state: 0,
                            same_screen: 1,
                        }
                    } else {
                        KeyReleaseEvent {
                            response_type: x11rb::protocol::xproto::KEY_RELEASE_EVENT,
                            detail: kc,
                            sequence: 0,
                            time: 0,
                            root,
                            event: root,
                            child: x11rb::NONE,
                            root_x: 0,
                            root_y: 0,
                            event_x: 0,
                            event_y: 0,
                            state: 0,
                            same_screen: 1,
                        }
                    };
                    // Use send_event; build the raw bytes with appropriate event packing.
                    let mask = EventMask::KEY_PRESS | EventMask::KEY_RELEASE;
                    // Prefer XTest fake_input if available
                    if self.xtest_present {
                        use x11rb::protocol::xtest;
                        // Attempt to use FakeInput (request-level API) to generate the key event,
                        // this wraps the platform-specific XTest semantics.
                        let evtype = if _pressed { x11rb::protocol::xproto::KEY_PRESS_EVENT } else { x11rb::protocol::xproto::KEY_RELEASE_EVENT };
                        let cookie = xtest::fake_input(&**conn, evtype as u8, kc, 0);
                        let _ = cookie
                            .map_err(|e| format!("xtest fake_input failed: {:?}", e))?
                            .reply()
                            .map_err(|e| format!("xtest fake_input reply failed: {:?}", e))?;
                    } else {
                        let res = if _pressed {
                            conn.send_event(false, root, mask, &ev).map_err(|e| format!("send_event failed: {:?}", e))?
                        } else {
                            conn.send_event(false, root, mask, &ev).map_err(|e| format!("send_event failed: {:?}", e))?
                        };
                    }
                    // flush
                    conn.flush().map_err(|e| format!("flush failed: {:?}", e))?;
                    Ok(())
                } else {
                    Err("keysym not found in keymap".to_string())
                }
            }
        }

        /// Inject a pointer event (x,y and button_mask) into the display
        pub fn inject_pointer_event(&self, _x: i32, _y: i32, _button_mask: u8) -> Result<(), String> {
            #[cfg(not(feature = "x11"))]
            {
                Ok(())
            }
            #[cfg(feature = "x11")]
            {
                let conn = &*self.conn;
                let setup = conn.setup();
                let screen = &setup.roots[self.screen_num];
                let root = screen.root;
                // Warp pointer to x,y first
                conn.warp_pointer(x11rb::NONE, root, 0, 0, 0, 0, _x as i16, _y as i16)
                    .map_err(|e| format!("warp pointer failed: {:?}", e))?;
                if self.xtest_present {
                    use x11rb::protocol::xtest;
                    // issue a fake motion event
                    let cookie = xtest::fake_input(&**conn, x11rb::protocol::xproto::MOTION_NOTIFY as u8, 0, 0);
                    let _ = cookie
                        .map_err(|e| format!("xtest fake_input failed: {:?}", e))?
                        .reply()
                        .map_err(|e| format!("xtest fake_input reply failed: {:?}", e))?;
                }
                // Compare button mask with last known state and generate
                // press/release events as appropriate (with xtest if present).
                let changed = _button_mask ^ self.last_button_mask;
                if changed != 0 {
                    for i in 0..8 {
                        let bit = 1u8 << i;
                        if changed & bit != 0 {
                            let button = (i + 1) as u8; // X11 buttons supported from 1
                            if _button_mask & bit != 0 {
                                // now down
                                if self.xtest_present {
                                    use x11rb::protocol::xtest;
                                    let _ = xtest::fake_input(&**conn, x11rb::protocol::xproto::BUTTON_PRESS_EVENT as u8, button, 0)
                                        .map_err(|e| format!("xtest fake_input failed: {:?}", e))?;
                                } else {
                                    // store as no-op (we have no generic fallback)
                                }
                            } else {
                                // now up
                                if self.xtest_present {
                                    use x11rb::protocol::xtest;
                                    let _ = xtest::fake_input(&**conn, x11rb::protocol::xproto::BUTTON_RELEASE_EVENT as u8, button, 0)
                                        .map_err(|e| format!("xtest fake_input failed: {:?}", e))?;
                                } else {
                                    // no-op fallback
                                }
                            }
                        }
                    }
                }
                self.last_button_mask = _button_mask;
                conn.flush().map_err(|e| format!("flush failed: {:?}", e))?;
                // For button events, ideally use XTest FakeButtonEvent; fallback to send_event
                Ok(())
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
