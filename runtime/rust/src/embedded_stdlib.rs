// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
//! The bytes are gated behind the `wasm_stdlib` feature so non-WASM consumers
//! of the runtime crate do not carry the ~250 KB of embedded Tcl; the table of
//! what the library defines ([`DEFINED_COMMANDS`]) is not, because a test holds
//! it to the vendored files in every build.

#[cfg(feature = "wasm_stdlib")]
use crate::mem_fs::MemFs;

/// The mount point of the embedded library — reported as `$TCL_LIBRARY` by the
/// WASM hosts, so `init_library()` sources `$TCL_LIBRARY/init.tcl` from the VFS.
pub const TCL_LIBRARY_MOUNT: &str = "/tcl/library";

/// The core commands the library defines, each with the library file that
/// defines it (relative to the mount): what a runtime that embeds the library
/// reports as a command backed by the library once `init_library()` has
/// sourced it. Names are as the registry spells them, without a leading `::`.
///
/// `pkg::create` is the alias `package.tcl` makes to `::tcl::Pkg::Create`; the
/// rest are procedures. A test sources the vendored files and holds this table
/// to what they define.
pub const DEFINED_COMMANDS: &[(&str, &str)] = &[
    ("auto_execok", "init.tcl"),
    ("auto_import", "init.tcl"),
    ("auto_load", "init.tcl"),
    ("auto_load_index", "init.tcl"),
    ("auto_qualify", "init.tcl"),
    ("parray", "parray.tcl"),
    ("pkg::create", "package.tcl"),
    ("tclLog", "init.tcl"),
    ("tclPkgSetup", "package.tcl"),
    ("tclPkgUnknown", "package.tcl"),
    ("unknown", "init.tcl"),
];

#[cfg(feature = "wasm_stdlib")]
macro_rules! embed {
    ($($rel:literal),* $(,)?) => {
        &[ $( ($rel, include_bytes!(concat!("../vendor/tcl_library/", $rel)) as &[u8]) ),* ]
    };
}

/// `(path relative to the mount, file bytes)` for each embedded library file.
#[cfg(feature = "wasm_stdlib")]
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
#[cfg(feature = "wasm_stdlib")]
pub fn seed(fs: &MemFs) {
    for (rel, bytes) in FILES {
        fs.insert(&format!("{TCL_LIBRARY_MOUNT}/{rel}"), bytes.to_vec());
    }
}

#[cfg(all(test, have_tommath))]
mod tests {
    use super::DEFINED_COMMANDS;
    use crate::interp::{Code, Command, Interp};
    use crate::namespace::GLOBAL;
    use std::rc::Rc;
    use tcl_host_native::NativeHost;
    use tcl_platform::Host;

    /// The library sourced from the vendored files leaves each command the
    /// table reports defined, once the auto-load index has been asked for any
    /// it names: a procedure, or for the one alias an alias, and the file the
    /// table names is the one whose text defines it. The report's `Stdlib`
    /// answer rests on this.
    #[test]
    fn the_library_defines_the_commands_it_reports() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/tcl_library");
        let host = Rc::new(NativeHost::new());
        host.env().set("TCL_LIBRARY", dir);
        let mut interp = Interp::with_host(host);
        assert_eq!(
            interp.init_library(),
            Code::Ok,
            "init.tcl: {}",
            String::from_utf8_lossy(&interp.result_bytes())
        );
        for (name, file) in DEFINED_COMMANDS {
            let text = std::fs::read_to_string(format!("{dir}/{file}")).expect("a library file");
            // A command the library's auto-load index names (`parray`) is
            // defined the first time it is wanted.
            let _ = interp.eval_str(format!("auto_load {name}").as_bytes());
            let resolved = interp.namespaces().resolve(GLOBAL, name.as_bytes());
            let definition = match resolved {
                Some(Command::Proc(_)) => format!("proc {name} "),
                Some(Command::Alias { .. }) => format!("interp alias {{}} ::{name} "),
                other => panic!("{name}: expected a definition, found {}", other.is_some()),
            };
            assert!(
                text.contains(&definition),
                "{file} holds no `{definition}`, so it is not where {name} is defined"
            );
        }
    }
}
