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

fn main() {
    println!("cargo:rerun-if-changed=src/native_list_sort.c");
    println!("cargo:rustc-check-cfg=cfg(native_list_qsort)");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let architecture = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    // This contract is the local native backend, not a browser or foreign WASI libc.
    if architecture != "wasm32" && matches!(os.as_str(), "linux" | "macos" | "ios" | "freebsd") {
        cc::Build::new()
            .file("src/native_list_sort.c")
            .std("c11")
            .warnings(true)
            .compile("tcl_native_list_sort");
        println!("cargo:rustc-cfg=native_list_qsort");
    }
}
