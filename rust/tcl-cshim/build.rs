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

//! Compiles the C test extension (`tests/c/pkga.c`) against the shim header.
//!
//! The object is bundled into the crate's rlib; the linker pulls it into a
//! binary only when something references `Pkga_Init`, which only the crate's
//! own integration tests do — so ordinary consumers carry nothing. Windows is
//! skipped: the tests that need the extension are gated on the
//! `cshim_c_tests` cfg this script sets, and the Rust-defined extension tests
//! cover that platform.

fn main() {
    println!("cargo:rustc-check-cfg=cfg(cshim_c_tests)");
    println!("cargo:rerun-if-changed=tests/c/pkga.c");
    println!("cargo:rerun-if-changed=include/tclshim.h");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        return;
    }
    cc::Build::new()
        .file("tests/c/pkga.c")
        .include("include")
        .flag_if_supported("-std=c99")
        .warnings(true)
        .compile("tclshim_pkga");
    println!("cargo:rustc-cfg=cshim_c_tests");
}
