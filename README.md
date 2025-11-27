# x11vnc (Rust rewrite) — src/rust

This folder is a work-in-progress rewrite of the original `x11vnc` server's `src/` code in pure Rust.

Goals:
- Keep libvncserver/libvncclient as the RFB backend for now (via a thin FFI) to shorten the migration path.
- Re-implement `src/` logic in idiomatic, safe Rust with as little unsafe code as possible.
- Provide incremental, tested replacements for modules (utilities, networking, X11 glue).

What you can run now:

Build and run tests for the Rust PoC:

```bash
cd src/rust
cargo test --release
```

Build the PoC binary skeleton (this will only start a placeholder loop):

```bash
cargo build --release --bin x11vnc
./target/release/x11vnc --status
```

Next steps:
1. Provide a safe wrapper around libvncserver (bindgen + safe layer).
2. Port core modules to Rust (screen/input, event loop) using safe crates where possible.
3. Add tests, CI and packaging.

Libvnc feature notes
---------------------

The crate now includes a feature-gated integration with `libvncserver` behind the `libvnc` feature. When enabled the crate will attempt to:

- discover `libvncserver` via pkg-config
- run bindgen to generate narrow bindings to the `rfb` types used by the PoC
- provide `VncServer::attach_framebuffer` and `update_framebuffer` which allocate a C framebuffer, copy application pixels into it and mark the rect as modified so connected clients will see updates.

If `libvncserver` and headers are not available, the build script produces safe fallback stubs so the crate stays buildable for development. To use the real native integration you must have `libvncserver` and its headers installed and then build with:

```bash
cargo build --features libvnc --release --bin x11vnc
```

Keep in mind native linking will only succeed on machines that have `libvncserver` installed and available via pkg-config. The PoC keeps unsafe FFI confined to `src/libvnc_wrapper.rs` and uses safe wrappers everywhere else.
