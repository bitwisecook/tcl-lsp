// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact command inventory patterns for original scripts in the root context.

/// Separate C namespace/tail and Jim flat-table inventory patterns.
/// These describe script queries, not live command identity or Native admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRootCommandPresencePlan {
    c_pattern: String,
    jim_pattern: String,
}

impl ScriptRootCommandPresencePlan {
    /// Retain C qualifiers literally; escape only the original simple tail.
    /// Embedded NUL needs a separate original-byte transport and is refused.
    #[must_use]
    pub fn new(original: &str) -> Option<Self> {
        if original.contains('\0') {
            return None;
        }
        let tail_start = super::written_separator_extent(original.as_bytes())
            .map_or(0, |separator| separator.end);
        let mut c_pattern = original[..tail_start].to_owned();
        c_pattern.push_str(&crate::glob::literal_pattern(&original[tail_start..]));
        Some(Self {
            c_pattern,
            jim_pattern: crate::glob::literal_pattern(original),
        })
    }

    #[must_use]
    pub fn c_pattern(&self) -> &str {
        &self.c_pattern
    }

    #[must_use]
    pub fn jim_pattern(&self) -> &str {
        &self.jim_pattern
    }
}

/// Select an audited script query grammar and run it in the root context.
/// The original command word stays with the caller for autoload and invocation.
#[must_use]
pub const fn original_command_presence_support() -> &'static str {
    SUPPORT
}

const SUPPORT: &str = r#"namespace eval ::__tcl_name_presence {}
proc ::__tcl_name_presence::in_root {cpattern jimpattern supported} {
    if {!$supported} {error "command presence has no original-byte script transport"}
    switch -- [::__tcl_name_query::family] {
        c {set pattern $cpattern}
        jim {set pattern $jimpattern}
    }
    return [expr {[llength [uplevel #0 [list ::info commands $pattern]]] > 0}]
}
"#;
