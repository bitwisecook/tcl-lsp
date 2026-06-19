//! Re-export of the shared std-backed [`NativeHost`].
//!
//! The implementation moved to the value-agnostic `tcl-host-native` crate so the
//! bytecode VM and `runtime/rust`'s native test builds share one std-backed
//! [`Host`](tcl_platform::Host). This module re-exports it so existing
//! `tcl_vm::host_native::NativeHost` paths (the `Vm`'s default host, the
//! capability tests) keep resolving unchanged.

pub use tcl_host_native::NativeHost;
