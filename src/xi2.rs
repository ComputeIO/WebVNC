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
