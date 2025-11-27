//! src_utils crate root — minimal, focused PoC runtime helpers and modules.
//!
//! This crate keeps a small surface area — the previous large collection of
//! C-FFI helpers has been replaced by `safe_utils.rs`. Export explicit
//! modules here so other crates/integration tests can use them.

pub mod event_loop;
pub mod libvnc_wrapper;
pub mod safe_utils;
pub mod screen;
pub mod x11;
pub mod connections;
pub mod userinput;
