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

//! Fixed Tcl 9.0 original-source fixtures for introspection tests.

#[cfg(test)]
pub(crate) fn original_info_invocation(
    source: &str,
) -> Option<crate::registry_invocation::ResolvedStatementInvocation> {
    let context = tcl_registry::model::ingress::static_context_for("tcl9.0");
    let registry = context.commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
        source,
        registry,
        false,
        context
            .commands()
            .profile()
            .expect("fixture context retains its selected Tcl profile"),
    );
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_module(
        registry,
        &unit.ir_module,
    )?;
    crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
        registry,
        Some(metadata),
        unit.ir_module.top_level.statements.first()?,
    )
}

#[cfg(test)]
pub const FRAME_INSPECTING_SUBCOMMANDS: &[&str] = &[
    "level",
    "frame",
    "vars",
    "locals",
    "coroutine",
    "errorstack",
];
#[cfg(test)]
pub const INTERPRETER_GLOBAL_SUBCOMMANDS: &[&str] = &[
    "body",
    "args",
    "default",
    "commands",
    "procs",
    "class",
    "functions",
    "cmdtype",
    "patchlevel",
    "tclversion",
    "nameofexecutable",
    "sharedlibextension",
    "library",
    "hostname",
    "script",
    "cmdcount",
    "complete",
    "cancel",
    "loaded",
    "object",
    "constant",
    "constants",
    "consts",
    "globals",
    "exists",
];
#[must_use]
#[cfg(test)]
pub fn is_safe_info_subcommand(subcmd: &str) -> bool {
    original_info_invocation(&format!("info {subcmd}")).is_some_and(|invocation| {
        matches!(
            invocation.facts.subcommand,
            tcl_registry::OwnedSubcommandResolution::Exact { .. }
                | tcl_registry::OwnedSubcommandResolution::UniquePrefix { .. }
        ) && !invocation
            .facts
            .traits
            .contains(tcl_registry::prelude::Traits::CURRENT_FRAME_INTROSPECTION)
    })
}

/// Return true if `info <subcmd>` enumerates or reads frame state.
#[must_use]
#[cfg(test)]
pub fn is_frame_inspecting_info_subcommand(subcmd: &str) -> bool {
    original_info_invocation(&format!("info {subcmd}")).is_some_and(|invocation| {
        invocation
            .facts
            .traits
            .contains(tcl_registry::prelude::Traits::CURRENT_FRAME_INTROSPECTION)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_inspecting_set_is_pessimistic() {
        assert!(is_frame_inspecting_info_subcommand("level"));
        assert!(is_frame_inspecting_info_subcommand("frame"));
        assert!(is_frame_inspecting_info_subcommand("vars"));
        assert!(is_frame_inspecting_info_subcommand("locals"));
        assert!(is_frame_inspecting_info_subcommand("coroutine"));
        assert!(is_frame_inspecting_info_subcommand("errorstack"));
    }

    #[test]
    fn interpreter_global_set_is_safe() {
        for sub in [
            "body",
            "args",
            "commands",
            "procs",
            "tclversion",
            "patchlevel",
            "loaded",
            "exists",
            "globals",
            "constants",
        ] {
            assert_eq!(
                is_safe_info_subcommand(sub),
                sub != "constants",
                "{sub} classification"
            );
        }
    }

    #[test]
    fn unknown_subcommand_neither_safe_nor_frame_inspecting() {
        assert!(!is_safe_info_subcommand("totally_made_up"));
        assert!(!is_safe_info_subcommand("cancel"));
        assert!(!is_safe_info_subcommand("constants"));
        assert!(is_safe_info_subcommand("consts"));
        assert!(!is_frame_inspecting_info_subcommand("totally_made_up"));
    }

    #[test]
    fn frame_and_safe_sets_are_disjoint() {
        for f in FRAME_INSPECTING_SUBCOMMANDS {
            assert!(
                !INTERPRETER_GLOBAL_SUBCOMMANDS.contains(f),
                "{f} appears in both sets"
            );
        }
    }
}
