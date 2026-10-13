// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
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
