//! Input injection abstraction
//!
//! Defines a small trait `InputInjector` that abstracts over different
//! backends capable of injecting keyboard and pointer events (X11/XTest or
//! Linux uinput). Implementations are feature-gated so the crate can compile
//! on systems without those backends.

use std::io;

/// A minimal injector trait used by higher-level code to deliver input.
pub trait InputInjector: Send {
    /// Send a key event. `keycode` follows the backend's expected keycode
    /// numbering (e.g., Linux input event keycodes or X keycodes).
    fn send_key(&mut self, keycode: u16, pressed: bool) -> io::Result<()>;

    /// Send a relative pointer motion and a button mask. `dx`/`dy` are
    /// relative movements. `button_mask` uses bit0=left, bit1=right, bit2=middle.
    fn send_pointer(&mut self, dx: i32, dy: i32, button_mask: u32) -> io::Result<()>;

    /// Optional: flush pending events if the backend buffers them.
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

/// A small helper that forwards injection calls to boxed closures.
pub struct CallbackInjector {
    pub key_cb: Box<dyn FnMut(u16, bool) -> io::Result<()> + Send>,
    pub ptr_cb: Box<dyn FnMut(i32, i32, u32) -> io::Result<()> + Send>,
}

impl InputInjector for CallbackInjector {
    fn send_key(&mut self, keycode: u16, pressed: bool) -> io::Result<()> {
        (self.key_cb)(keycode, pressed)
    }

    fn send_pointer(&mut self, dx: i32, dy: i32, button_mask: u32) -> io::Result<()> {
        (self.ptr_cb)(dx, dy, button_mask)
    }
}

/// Concrete, ergonomic injector that avoids boxing in the common case.
pub enum Injector {
    Callback(CallbackInjector),
    #[cfg(feature = "x11")]
    X11(crate::x11::DisplayHandle),
    #[cfg(feature = "uinput")]
    UInput(crate::uinput::UInputDevice),
}

impl std::fmt::Debug for Injector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Injector::Callback(_) => f.debug_tuple("Injector::Callback").finish(),
            #[cfg(feature = "x11")]
            Injector::X11(_) => f.debug_tuple("Injector::X11").finish(),
            #[cfg(feature = "uinput")]
            Injector::UInput(_) => f.debug_tuple("Injector::UInput").finish(),
        }
    }
}

impl Injector {
    pub fn send_key(&mut self, keycode: u16, pressed: bool) -> io::Result<()> {
        match self {
            Injector::Callback(c) => c.send_key(keycode, pressed),
            #[cfg(feature = "x11")]
            Injector::X11(d) => d.inject_key_event(keycode as u32, pressed).map_err(|e| io::Error::new(io::ErrorKind::Other, e)),
            #[cfg(feature = "uinput")]
            Injector::UInput(u) => u.send_key(keycode, pressed),
        }
    }

    pub fn send_pointer(&mut self, dx: i32, dy: i32, button_mask: u32) -> io::Result<()> {
        match self {
            Injector::Callback(c) => c.send_pointer(dx, dy, button_mask),
            #[cfg(feature = "x11")]
            Injector::X11(d) => d.inject_pointer_event(dx, dy, button_mask).map_err(|e| io::Error::new(io::ErrorKind::Other, e)),
            #[cfg(feature = "uinput")]
            Injector::UInput(u) => u.send_pointer(dx, dy, button_mask),
        }
    }
    
        // NOTE: This module previously used boxed trait objects in several call sites.
        // The concrete `Injector` enum avoids per-call boxing and reduces
        // allocation / dynamic dispatch overhead. The remaining boxed closures are
        // only inside the `CallbackInjector` helper where boxing is appropriate.

    pub fn flush(&mut self) -> io::Result<()> {
        match self {
            Injector::Callback(c) => c.flush(),
            #[cfg(feature = "x11")]
            Injector::X11(_d) => Ok(()),
            #[cfg(feature = "uinput")]
            Injector::UInput(_u) => Ok(()),
        }
    }
}

impl From<CallbackInjector> for Injector {
    fn from(c: CallbackInjector) -> Self { Injector::Callback(c) }
}

#[cfg(feature = "x11")]
impl From<crate::x11::DisplayHandle> for Injector {
    fn from(d: crate::x11::DisplayHandle) -> Self { Injector::X11(d) }
}

#[cfg(feature = "uinput")]
impl From<crate::uinput::UInputDevice> for Injector {
    fn from(u: crate::uinput::UInputDevice) -> Self { Injector::UInput(u) }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy;

    impl InputInjector for Dummy {
        fn send_key(&mut self, _keycode: u16, _pressed: bool) -> io::Result<()> { Ok(()) }
        fn send_pointer(&mut self, _dx: i32, _dy: i32, _button_mask: u32) -> io::Result<()> { Ok(()) }
    }

    #[test]
    fn dummy_injector_works() {
        let mut d = Dummy;
        d.send_key(30, true).unwrap();
        d.send_pointer(10, -5, 1).unwrap();
    }
}
