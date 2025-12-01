# webvnc-rs (Rust PoC)

This crate is a Rust proof-of-concept port of parts of the `webvnc`/`x11vnc`
project. It focuses on a minimal, test-driven subset useful for experimentation
and incremental porting.

## uinput fallback

When compiled with the `uinput` feature enabled, `webvnc-rs` will provide a
Linux `/dev/uinput` based headless input fallback for injecting keyboard and
pointer events when X11 is not available. To enable the feature:

```bash
cargo test --features uinput
```

Notes and permissions:
- Creating a uinput device typically requires elevated privileges or a
  suitable udev rule that allows the current user to open `/dev/uinput`.
- The crate's `uinput` module is feature-gated to avoid adding platform
  dependencies for users who don't need headless injection.

## XI2 multi-pointer support (status)

There is a scaffold in `src/xi2.rs` that will be expanded to port XI2
device creation from the original C implementation. The current scaffold
is a no-op unless the `x11` feature is enabled. Work to fully implement
master/slave device creation and client mapping is ongoing.

## Integration tests (Xvfb)

An optional integration test exists at `tests/integration_xvfb.rs`. To run
it, ensure `Xvfb` is installed and set the environment variable
`RUN_XVFB_TESTS=1` before running tests:

```bash
RUN_XVFB_TESTS=1 cargo test
```

The test is conservative and will be skipped when the env var is not set.

## Contributing

If you plan to work on XI2 support or pointer remap semantics, start by
reading the C sources in the original `webvnc` tree to understand the
expected runtime behavior and edge cases. Please open issues/PRs for
non-trivial design changes.
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
