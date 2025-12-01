// Linux uinput implementation (feature = "uinput").
// Provides a small, safe wrapper over `/dev/uinput` to create a virtual
// input device and send keyboard / pointer events. The implementation aims
// to be minimal and usable as a headless injection fallback.

#![cfg(all(feature = "uinput", target_os = "linux"))]

use std::ffi::CString;
use std::io;
use std::mem::{self, zeroed};
use std::os::unix::io::RawFd;
use std::ptr;
use crate::inject::InputInjector;

const UINPUT_PATH: &str = "/dev/uinput";

// ioctl / uinput constants (common values from linux/uinput.h)
const UI_DEV_CREATE: libc::c_ulong = 0x5501;
const UI_DEV_DESTROY: libc::c_ulong = 0x5502;
const UI_SET_EVBIT: libc::c_ulong = 0x4004_5564;
const UI_SET_KEYBIT: libc::c_ulong = 0x4004_5565;
const UI_SET_RELBIT: libc::c_ulong = 0x4004_5566;

// event types
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;

const SYN_REPORT: u16 = 0;
const REL_X: u16 = 0x00;
const REL_Y: u16 = 0x01;

// common buttons
const BTN_LEFT: u16 = 0x110;
const BTN_RIGHT: u16 = 0x111;
const BTN_MIDDLE: u16 = 0x112;

#[repr(C)]
#[allow(non_camel_case_types)]
struct input_id {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[repr(C)]
struct uinput_user_dev {
    name: [u8; 80],
    id: input_id,
    ff_effects_max: i32,
    absmax: [i32; 64],
    absmin: [i32; 64],
    absfuzz: [i32; 64],
    absflat: [i32; 64],
}

#[repr(C)]
struct input_event {
    tv: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

pub struct UInputDevice {
    fd: RawFd,
}

impl UInputDevice {
    pub fn create(name: &str) -> io::Result<Self> {
        // open /dev/uinput
        let cpath = CString::new(UINPUT_PATH).unwrap();
        let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_WRONLY | libc::O_NONBLOCK) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        // enable event types and common codes
        unsafe {
            if libc::ioctl(fd, UI_SET_EVBIT, EV_KEY as libc::c_ulong) < 0 {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(e);
            }
            if libc::ioctl(fd, UI_SET_EVBIT, EV_REL as libc::c_ulong) < 0 {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(e);
            }
            if libc::ioctl(fd, UI_SET_EVBIT, EV_SYN as libc::c_ulong) < 0 {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(e);
            }

            // enable relative axes
            if libc::ioctl(fd, UI_SET_RELBIT, REL_X as libc::c_ulong) < 0 {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(e);
            }
            if libc::ioctl(fd, UI_SET_RELBIT, REL_Y as libc::c_ulong) < 0 {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(e);
            }

            // enable common buttons
            if libc::ioctl(fd, UI_SET_KEYBIT, BTN_LEFT as libc::c_ulong) < 0 {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(e);
            }
            if libc::ioctl(fd, UI_SET_KEYBIT, BTN_RIGHT as libc::c_ulong) < 0 {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(e);
            }
            if libc::ioctl(fd, UI_SET_KEYBIT, BTN_MIDDLE as libc::c_ulong) < 0 {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(e);
            }

            // prepare and write uinput_user_dev
            let mut dev: uinput_user_dev = zeroed();
            let bytes = name.as_bytes();
            let len = bytes.len().min(dev.name.len());
            dev.name[..len].copy_from_slice(&bytes[..len]);
            dev.id = input_id { bustype: 0x03, vendor: 0x1234, product: 0x5678, version: 1 };
            dev.ff_effects_max = 0;

            let write_size = mem::size_of::<uinput_user_dev>();
            let ret = libc::write(fd, &dev as *const _ as *const libc::c_void, write_size);
            if ret as usize != write_size {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(e);
            }

            // create the device
            if libc::ioctl(fd, UI_DEV_CREATE, 0) < 0 {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(e);
            }
        }

        Ok(UInputDevice { fd })
    }

    pub fn send_key_event(&mut self, keycode: u16, pressed: bool) -> io::Result<()> {
        let mut ev: input_event = unsafe { zeroed() };
        // timeval zeroed is acceptable for synthetic events
        ev.type_ = EV_KEY;
        ev.code = keycode;
        ev.value = if pressed { 1 } else { 0 };

        let res = unsafe {
            libc::write(self.fd, &ev as *const _ as *const libc::c_void, mem::size_of::<input_event>())
        };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }

        // send SYN
        let mut syn: input_event = unsafe { zeroed() };
        syn.type_ = EV_SYN;
        syn.code = SYN_REPORT;
        syn.value = 0;
        let res2 = unsafe {
            libc::write(self.fd, &syn as *const _ as *const libc::c_void, mem::size_of::<input_event>())
        };
        if res2 < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Send a relative pointer update and optional button mask.
    /// `button_mask` uses bit 0 = left, bit 1 = right, bit 2 = middle.
    pub fn send_pointer_event(&mut self, dx: i32, dy: i32, button_mask: u32) -> io::Result<()> {
        // relative X
        if dx != 0 {
            let mut ev: input_event = unsafe { zeroed() };
            ev.type_ = EV_REL;
            ev.code = REL_X;
            ev.value = dx;
            let r = unsafe { libc::write(self.fd, &ev as *const _ as *const libc::c_void, mem::size_of::<input_event>()) };
            if r < 0 { return Err(io::Error::last_os_error()); }
        }
        if dy != 0 {
            let mut ev: input_event = unsafe { zeroed() };
            ev.type_ = EV_REL;
            ev.code = REL_Y;
            ev.value = dy;
            let r = unsafe { libc::write(self.fd, &ev as *const _ as *const libc::c_void, mem::size_of::<input_event>()) };
            if r < 0 { return Err(io::Error::last_os_error()); }
        }

        // buttons
        let buttons = [BTN_LEFT, BTN_RIGHT, BTN_MIDDLE];
        for (i, &code) in buttons.iter().enumerate() {
            let pressed = (button_mask & (1 << i)) != 0;
            let mut ev: input_event = unsafe { zeroed() };
            ev.type_ = EV_KEY;
            ev.code = code;
            ev.value = if pressed { 1 } else { 0 };
            let r = unsafe { libc::write(self.fd, &ev as *const _ as *const libc::c_void, mem::size_of::<input_event>()) };
            if r < 0 { return Err(io::Error::last_os_error()); }
        }

        // final SYN
        let mut syn: input_event = unsafe { zeroed() };
        syn.type_ = EV_SYN;
        syn.code = SYN_REPORT;
        syn.value = 0;
        let r = unsafe { libc::write(self.fd, &syn as *const _ as *const libc::c_void, mem::size_of::<input_event>()) };
        if r < 0 { return Err(io::Error::last_os_error()); }
        Ok(())
    }
}

impl Drop for UInputDevice {
    fn drop(&mut self) {
        unsafe {
            let _ = libc::ioctl(self.fd, UI_DEV_DESTROY, 0);
            let _ = libc::close(self.fd);
        }
    }
}

impl InputInjector for UInputDevice {
    fn send_key(&mut self, keycode: u16, pressed: bool) -> io::Result<()> {
        self.send_key_event(keycode, pressed)
    }

    fn send_pointer(&mut self, dx: i32, dy: i32, button_mask: u32) -> io::Result<()> {
        self.send_pointer_event(dx, dy, button_mask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_does_not_panic() {
        // Attempt to create; may return Err if /dev/uinput missing or permission denied.
        let _ = UInputDevice::create("webvnc-test");
    }
}
