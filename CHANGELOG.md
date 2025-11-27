# Changelog — src/rust (Rust re-write of x11vnc)

Date: 2025-11-27

This file summarizes the current state of the Rust rewrite contained in `src/rust`.

## Goals
- Port the original `src/` logic from C into idiomatic, well-tested Rust
- Maintain `libvncserver`/`libvncclient` as backend via a minimal FFI layer while reducing unsafe code
- Incremental, test-driven replacement of subsystems (screen, event loop, input, networking)

## Completed / Present State
- A PoC Rust workspace with a functional skeleton binary `x11vnc` and test harness
- Feature-gated `libvnc` integration with a thin FFI wrapper (`libvnc_wrapper.rs`)
- Safe Rust equivalents for utilities and helpers (`safe_utils.rs`) and a pure-Rust `Screen` framebuffer implementation (`screen.rs`)
- Pixel format handling & conversions covering RGB24, common 4-byte layouts, RGB565/RGB555 with stride and endianness handling
- Unit tests added covering format conversions and core functionality; `cargo test` passes for the default build
- E2E helpers support ephemeral port allocation to make CI-friendly tests
- Added a `Connections` manager module implementing basic client bookkeeping,
  access checks and a `run_user_command` helper that sets RFB_* environment
  variables and safely invokes external commands. This is the first non-trivial
  port of `connections.c` functionality and includes unit tests.

## Files of interest
- `Cargo.toml`, `build.rs` — workspace & feature gating for `libvnc`
- `src/libvnc_wrapper.rs` — unsafe FFI isolated to a single module
- `src/safe_utils.rs` — safe helpers and conversions
- `src/screen.rs` — framebuffer and pixel handling
- `src/event_loop.rs`, `src/main.rs` — small event loop harness and CLI

## Known limitations / next steps
1. Make ephemeral port handling race-free by binding a socket and handing it to native `libvncserver` (if supported)
2. Add CI coverage for `--features libvnc` builds / tests on a runner that provides `libvncserver` libraries and headers
3. Continue porting additional C subsystems from `src/` into Rust incrementally and adding tests

---

If you need a deeper breakdown (per-file or per-module diffs, recommended next PR sizes, or test suggestions) I can produce that next.
