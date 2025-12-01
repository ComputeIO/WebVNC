// Adapter to make `crate::x11::DisplayHandle` implement `InputInjector`.
// This is feature-gated on the `x11` feature to avoid pulling X11 code
// into non-X builds.

#![cfg(feature = "x11")]

use std::io;
use crate::inject::InputInjector;
use crate::x11::DisplayHandle;

impl InputInjector for DisplayHandle {
    fn send_key(&mut self, keycode: u16, pressed: bool) -> io::Result<()> {
        // `DisplayHandle::inject_key_event` returns Result<(), String> in the
        // existing codebase; adapt to io::Result.
        let res = self.inject_key_event(keycode as u32, pressed);
        match res {
            Ok(()) => Ok(()),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    fn send_pointer(&mut self, dx: i32, dy: i32, button_mask: u32) -> io::Result<()> {
        let res = self.inject_pointer_event(dx, dy, button_mask);
        match res {
            Ok(()) => Ok(()),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        // DisplayHandle likely doesn't buffer; return Ok.
        Ok(())
    }
}
