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

use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out = Path::new(&out_dir);

    for (name, cfg) in [
        ("tcl-lsp-server", "bundled_lsp"),
        ("tcl-mcp", "bundled_mcp"),
    ] {
        println!("cargo:rustc-check-cfg=cfg({cfg})");
        let src = Path::new("bundled").join(name);
        println!("cargo:rerun-if-changed={}", src.display());
        if src.exists() {
            fs::copy(&src, out.join(name)).expect("failed to copy bundled asset");
            println!("cargo:rustc-cfg={cfg}");
        }
    }
}
