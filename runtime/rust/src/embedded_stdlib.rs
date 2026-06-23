//! The Tcl 9 standard-library files embedded in the runtime binary, seeded into
//! the WASM [`MemFs`](crate::mem_fs::MemFs) so the filesystem-backed startup path
//! works with no host filesystem.
//!
//! These are the **real** C-Tcl-9 library files (vendored verbatim under
//! `runtime/rust/vendor/tcl_library/`, license in `license.terms`). The set is
//! exactly the read-closure of bootstrapping `init.tcl` and loading the
//! `tcltest` package — `init.tcl` itself, the auto-load index, the `package`
//! and Tcl-module (`tm.tcl`) machinery, the per-package `pkgIndex.tcl` files the
//! `package unknown` scan sources, and the `tcltest` package. The large data
//! trees (`tzdata`, `encoding`, `msgs`) are never read by this path and are
//! omitted to keep the binary small.
//!
//! Gated behind the `wasm_stdlib` feature so non-WASM consumers of the runtime
//! crate do not carry the ~250 KB of embedded Tcl.

use crate::mem_fs::MemFs;

/// The mount point of the embedded library — reported as `$TCL_LIBRARY` by the
/// WASM hosts, so `init_library()` sources `$TCL_LIBRARY/init.tcl` from the VFS.
pub const TCL_LIBRARY_MOUNT: &str = "/tcl/library";

macro_rules! embed {
    ($($rel:literal),* $(,)?) => {
        &[ $( ($rel, include_bytes!(concat!("../vendor/tcl_library/", $rel)) as &[u8]) ),* ]
    };
}

/// `(path relative to the mount, file bytes)` for each embedded library file.
static FILES: &[(&str, &[u8])] = embed![
    "init.tcl",
    "package.tcl",
    "tm.tcl",
    "parray.tcl",
    "tclIndex",
    "tcltest/pkgIndex.tcl",
    "tcltest/tcltest.tcl",
    "cookiejar/pkgIndex.tcl",
    "dde/pkgIndex.tcl",
    "http/pkgIndex.tcl",
    "msgcat/pkgIndex.tcl",
    "opt/pkgIndex.tcl",
    "platform/pkgIndex.tcl",
    "registry/pkgIndex.tcl",
];

/// Seed `fs` with the embedded standard library under [`TCL_LIBRARY_MOUNT`].
pub fn seed(fs: &MemFs) {
    for (rel, bytes) in FILES {
        fs.insert(&format!("{TCL_LIBRARY_MOUNT}/{rel}"), bytes.to_vec());
    }
}
