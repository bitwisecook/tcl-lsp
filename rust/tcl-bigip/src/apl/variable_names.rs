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

//! APL field convention, independent of variable lookup or appliance publication.

pub(super) struct AplVariableConvention {
    pub(super) absolute: bool,
    pub(super) apl_name: String,
}

pub(super) fn project_variable_root(tcl_var: &str) -> Option<AplVariableConvention> {
    let parts = tcl_syntax::naming::written_command_parts(tcl_var.as_bytes());
    if !parts.qualifiers.is_empty() || parts.simple.is_empty() {
        return None;
    }
    let simple = std::str::from_utf8(parts.simple).ok()?;
    simple.contains("__").then(|| AplVariableConvention {
        absolute: parts.absolute,
        apl_name: simple.replace("__", "."),
    })
}

/// Project one simple variable root into the APL field convention.
/// Namespace qualifiers are outside this convention; lone colons remain data.
#[must_use]
pub fn tcl_var_to_apl_name(tcl_var: &str) -> Option<String> {
    project_variable_root(tcl_var).map(|name| name.apl_name)
}

/// Conventional rooted export report; no native variable or source-address proof.
#[must_use]
pub fn apl_name_to_tcl_var(apl_name: &str) -> String {
    tcl_syntax::naming::root_unrooted_key(&apl_name.replace('.', "__"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_convention_uses_shared_root_geometry() {
        for root in ["basic__addr", "::basic__addr", ":::basic__addr"] {
            assert_eq!(tcl_var_to_apl_name(root).as_deref(), Some("basic.addr"));
        }
        assert_eq!(
            tcl_var_to_apl_name(":basic__addr").as_deref(),
            Some(":basic.addr")
        );
        for root in ["::ns::basic__addr", "basic__addr::", "::plain"] {
            assert_eq!(tcl_var_to_apl_name(root), None);
        }
    }
}
