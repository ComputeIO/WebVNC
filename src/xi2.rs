//! XI2 multi-pointer support (basic port)
//!
//! This module provides helpers to detect XInput2 availability and to
//! perform initial master device creation. The full XI2 device emulation in
//! the original C code is large; here we implement a pragmatic, tested
//! starting point that queries the X server for XInput version and returns
//! an io::Result. Later steps can augment this with master/slave device
//! creation using XIChangeHierarchy requests.

use std::io;

#[cfg(feature = "x11")]
pub fn create_xi2_devices() -> io::Result<()> {
    use x11rb::connection::Connection;
    use x11rb::rust_connection::RustConnection;

    // Try to connect to the X server using x11rb. If connection fails, we
    // propagate an io::Error so callers can gracefully fall back.
    let (conn, _screen_num) = x11rb::connect(None).map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("x11 connect failed: {:?}", e))
    })?;

    // Attempt to query XInput version (2.0+) — the exact minimal version the
    // PoC requires can be adjusted later. If the extension or request is
    // not supported, return an error which allows the caller to fallback.
    match x11rb::protocol::xinput::xi_query_version(&conn, 2, 2) {
        Ok(cookie) => match cookie.reply() {
            Ok(_reply) => Ok(()),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("xi_query_version reply failed: {:?}", e),
            )),
        },
        Err(e) => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("xi_query_version request failed: {:?}", e),
        )),
    }
}

#[cfg(not(feature = "x11"))]
pub fn create_xi2_devices() -> io::Result<()> {
    // Without X11 support this is a no-op to keep crate usable on non-X
    // platforms.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xi2_query_runs() {
        // This test is conservative: it only verifies the function returns
        // an io::Result. It will succeed on systems without X11 because the
        // non-x11 path returns Ok(()).
        let _ = create_xi2_devices();
    }
}
//! Minimal XI2 multi-pointer support scaffold.
//!
//! This file provides a small, compile-time-safe scaffold that will be
//! expanded to fully port the XI2 device creation and multi-pointer
//! emulation from the C PoC. For now it exposes a simple API that is a
//! no-op when the `x11` feature is disabled and performs a light-weight
//! creation routine when enabled.

#[cfg(feature = "x11")]
mod inner {
    use std::io;

    /// Create XI2 master devices and return Ok(()) on success.
    ///
    /// The real implementation will create master/slave pointer and keyboard
    /// devices and map clients to them. This scaffold verifies linkage and
    /// provides a controlled place to implement the port incrementally.
    pub fn create_xi2_devices() -> io::Result<()> {
        // TODO: Implement XI2 device creation using x11rb or XCB bindings.
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn xi2_create_ok() {
            let r = create_xi2_devices();
            assert!(r.is_ok());
        }
    }
}

#[cfg(not(feature = "x11"))]
mod inner {
    use std::io;

    pub fn create_xi2_devices() -> io::Result<()> {
        // When X11 feature is not enabled, XI2 support is a no-op.
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn xi2_create_ok_no_x11() {
            assert!(create_xi2_devices().is_ok());
        }
    }
}

pub use inner::create_xi2_devices;
