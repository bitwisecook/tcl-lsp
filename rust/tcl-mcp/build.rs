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

use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");

    let timestamp = std::env::var("SOURCE_DATE_EPOCH").map_or_else(
        |_| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("build clock precedes the Unix epoch")
                .as_secs()
        },
        |epoch| {
            epoch
                .parse::<u64>()
                .expect("SOURCE_DATE_EPOCH must be Unix seconds")
        },
    );
    println!("cargo:rustc-env=TCL_MCP_BUILD_TIMESTAMP={timestamp}");
}
