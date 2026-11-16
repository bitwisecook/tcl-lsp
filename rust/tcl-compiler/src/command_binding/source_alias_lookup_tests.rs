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

use super::*;

#[test]
fn jim_alias_targets_use_each_actual_callers_namespace() {
    let source = "proc target {} {return ROOT}; namespace eval A {proc target {} {return A}}; alias a target; a; namespace eval A {a}";
    let context = tcl_registry::model::ingress::static_context_for("jim");
    let registry = context.commands();
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
        registry,
        SourceAnalysisOptions {
            invocation_dialect: registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile),
            ..SourceAnalysisOptions::default()
        },
    );
    for (offset, expected) in [
        (source.find("; a;").unwrap() + 2, "::target"),
        (source.rfind("{a}").unwrap() + 1, "::A::target"),
    ] {
        let binding = bindings.invocation_at_source("a", u32::try_from(offset).unwrap());
        assert_eq!(
            binding
                .proved_target()
                .map(|target| target.command.as_str()),
            Some(expected)
        );
    }
}
