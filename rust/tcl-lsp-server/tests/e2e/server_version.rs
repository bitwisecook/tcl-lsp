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

//! Native port of `tests/lsp_e2e/test_server_version.py`.
//!
//! The smallest real round-trip: boot the live server and read the version it
//! reports in the `initialize` response's `serverInfo`. Guards against the
//! banner regressing to a `dev` fallback, and pins the reported version to
//! `tcl_version::VERSION` — the release version resolved from the tag, which is
//! what the compiled banner carries. The workspace manifest's `0.1.0` is never
//! bumped, so `CARGO_PKG_VERSION` is deliberately not the source here.

use crate::common::Lsp;

#[test]
fn initialize_reports_version() {
    let lsp = Lsp::tcl();
    let info = lsp
        .server_info()
        .expect("initialize result had no serverInfo — cannot read the version banner");
    let reported = info
        .get("version")
        .and_then(|v| v.as_str())
        .expect("server reported no version banner");
    assert!(
        !reported.is_empty() && reported != "vdev" && reported != "dev",
        "server fell back to a dev version banner: {reported:?}"
    );
    // The native binary reports the tag-resolved release version verbatim.
    // Both the test and the binary compile against the same `tcl-version` crate,
    // so this holds for a release build (`2.1.5`) and a working-tree build
    // (`2.1.5-3-gabc1234`) alike.
    let expected = tcl_version::VERSION;
    assert_eq!(
        reported, expected,
        "native server reported version {reported:?}, expected the tag-resolved \
         release version {expected:?} (tcl_version::VERSION)"
    );
}

#[test]
fn server_info_name() {
    let lsp = Lsp::tcl();
    let info = lsp.server_info().expect("serverInfo present");
    assert_eq!(
        info.get("name").and_then(|v| v.as_str()),
        Some("tcl-lsp"),
        "server identifies as tcl-lsp"
    );
}
