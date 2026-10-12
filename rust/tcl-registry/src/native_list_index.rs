// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native list-index argument diagnostics, independent of compiler selection.

impl crate::InvocationDialect {
    /// Actual list-index usage tail for a selected native interpreter.
    #[must_use]
    pub fn list_index_usage(self) -> Option<&'static str> {
        if let Some(version) = self.tcl_version {
            Some(if version < tcl_dialect::TclVersion::V8_6 {
                "lindex list ?index...?"
            } else {
                "lindex list ?index ...?"
            })
        } else {
            self.core_point
                .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84)
                .then_some("lindex list ?index ...?")
        }
    }
}
