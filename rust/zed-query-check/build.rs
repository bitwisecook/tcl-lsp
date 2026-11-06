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

// Compile every tree-sitter grammar whose Zed queries this crate validates:
//
//   * the vendored `tree-sitter-tcl`, pinned to the rev in `vendor/REV` /
//     `editors/zed/extension.toml`;
//   * the two grammars this repo owns — `tree-sitter-apl` and
//     `tree-sitter-bigip`, which back Zed's APL and BIG-IP languages, each
//     with their own queries to validate — as opposed to pointing at the
//     *Tcl* grammar and shipping no queries at all.
fn main() {
    let vendor = std::path::Path::new("vendor");
    println!("cargo:rerun-if-changed=vendor");
    cc::Build::new()
        .include(vendor)
        .file(vendor.join("parser.c"))
        .file(vendor.join("scanner.c"))
        .warnings(false)
        .compile("tree_sitter_tcl");

    for (name, dir) in [
        ("tree_sitter_apl", "../../grammars/tree-sitter-apl"),
        ("tree_sitter_bigip", "../../grammars/tree-sitter-bigip"),
    ] {
        let src = std::path::Path::new(dir).join("src");
        println!("cargo:rerun-if-changed={}", src.display());
        let mut build = cc::Build::new();
        build
            .include(&src)
            .file(src.join("parser.c"))
            .warnings(false);
        let scanner = src.join("scanner.c");
        if scanner.is_file() {
            build.file(scanner);
        }
        build.compile(name);
    }
}
