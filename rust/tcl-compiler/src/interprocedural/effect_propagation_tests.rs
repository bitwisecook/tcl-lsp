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

use super::test_body_identity::fixture_proc;
use super::*;
use tcl_registry::CommandRegistry;

fn irules_registry() -> std::sync::Arc<CommandRegistry> {
    tcl_registry::model::ingress::static_context_for("f5-irules")
        .commands()
        .clone()
}

fn analyse_irules(source: &str, registry: &CommandRegistry) -> InterproceduralAnalysis {
    let profile = registry.profile().expect("selected F5 provider profile");
    let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
        source, registry, false, profile,
    );
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        source,
        unit.ir_module.lexer_config,
        registry,
        &unit.ir_module.source_entry,
    );
    build_interprocedural_analysis(
        &unit.ir_module,
        registry,
        Some(profile),
        ObjectTypeMap::none(),
        &identities,
        None,
    )
}

/// Reached substitutions contribute their effects in assignments, returns,
/// conditions and ordinary command operands. A braced value is inert.
#[test]
fn nested_substitution_effects_propagate_only_when_evaluated() {
    let reg = irules_registry();

    for src in [
        "proc p {} { set x [matchclass [HTTP::uri] equals $::l] }",
        "proc p {} { return [matchclass [HTTP::uri] equals $::l] }",
        "proc p {} { if {[matchclass [HTTP::uri] equals $::l]} {} }",
        "proc p {} { matchclass [HTTP::uri] equals $::l }",
    ] {
        let ia = analyse_irules(src, &reg);
        let s = fixture_proc(&ia, "::p").expect("proc ::p in IA");
        assert!(
            s.effect_reads.contains(EffectRegion::HTTP_STATE),
            "evaluated nested [HTTP::uri] should propagate HTTP_STATE; src={src:?} reads={:?}",
            s.effect_reads
        );
    }

    let ia = analyse_irules("proc q {} { matchclass {[HTTP::uri]} equals $::l }", &reg);
    let s = fixture_proc(&ia, "::q").expect("proc ::q in IA");
    assert!(
        !s.effect_reads.contains(EffectRegion::HTTP_STATE),
        "inert braced operand must not propagate HTTP_STATE; reads={:?}",
        s.effect_reads
    );
}
