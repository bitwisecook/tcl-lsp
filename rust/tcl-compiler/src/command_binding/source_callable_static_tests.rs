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

fn analyse(source: &str) -> SourceCommandBindings {
    let context = tcl_registry::model::ingress::static_context_for("jim");
    let registry = context.commands();
    SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
        registry,
        SourceAnalysisOptions {
            invocation_dialect: registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile),
            ..SourceAnalysisOptions::default()
        },
    )
}

fn assert_saved_native_set(source: &str) {
    let bindings = analyse(source);
    let site = u32::try_from(source.rfind("saved x").unwrap()).unwrap();
    let target = bindings.invocation_at_source("saved", site);
    assert!(
        target
            .proved_execution_target()
            .is_some_and(|target| target.registry_backed && target.command == "::set"),
        "{target:#?}"
    );
}

#[test]
fn static_formals_update_one_retained_slot_across_actual_calls() {
    assert_saved_native_set(
        "proc p {x} {{x OLD}} {return $x}; p OTHER; eval [p {rename set saved}]; saved x 1",
    );
}

#[test]
fn captured_raw_wrapper_observes_later_alias_retargeting() {
    assert_saved_native_set(
        "set a OLD; set b {rename set saved}; upvar 0 a x; proc p {} {&x} {return $x}; upvar 0 b x; eval [p]; saved x 1",
    );
}

#[test]
fn raw_capture_retains_old_cell_after_global_name_recreation() {
    assert_saved_native_set(
        "set x {rename set saved}; proc p {} {&x} {return $x}; unset x; set x OTHER; eval [p]; saved x 1",
    );
}
