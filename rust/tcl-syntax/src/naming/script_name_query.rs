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

//! Original script query-family selection, separate from Native entry authority.

/// Select the audited script family; unsupported query versions refuse.
/// Version observations cannot issue a Native identity or allocation receipt.
#[must_use]
pub const fn original_script_name_query_support() -> &'static str {
    SUPPORT
}

const SUPPORT: &str = r#"namespace eval ::__tcl_name_query {}
proc ::__tcl_name_query::family {} {
    if {![catch {::info tclversion} version] && [lsearch -exact {8.4 8.5 8.6 9.0 9.1} $version] >= 0} {
        return c
    }
    if {![catch {::info version} version] && $version eq "0.84"} {return jim}
    error "name query has no selected interpreter grammar"
}
"#;
