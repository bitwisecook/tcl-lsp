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

//! O127: a computed single-use assignment forwarded into its use, and what between them stops it.

use super::*;

/// Run the whole optimiser (raw, unfiltered) so the registry is
/// threaded — needed for the O127 store-to-load-forwarding pass,
/// which bails without a registry.
fn o127(source: &str) -> Vec<Optimisation> {
    crate::optimiser::optimise_raw_for_profile(
        source,
        &registry(),
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
    )
    .into_iter()
    .filter(|o| o.code == DiagCode::O127)
    .collect()
}

#[test]
fn o127_forwards_single_use_computed_assignment() {
    // `set x [llength $y]` is computed and used once → inline at the
    // use site + delete the store, both O127 in one group.
    let src = "proc p {y} {\n    set x [llength $y]\n    puts $x\n}\n";
    let opts = o127(src);
    assert_eq!(opts.len(), 2, "{opts:?}");
    // Both edits share a group id (all-or-nothing).
    assert_eq!(opts[0].group, opts[1].group);
    assert!(opts[0].group.is_some());
    // One edit inlines `[set x [llength $y]]`; the other deletes.
    assert!(
        opts.iter()
            .any(|o| o.replacement.contains("[set x [llength $y]]"))
    );
    assert!(opts.iter().any(|o| o.replacement.is_empty()));
}

#[test]
fn o127_operand_selection_follows_the_actual_alias_cell() {
    use crate::ssa::SsaSourceView;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for names in [("value", "pointer"), ("pointer", "value")] {
        let source = format!(
            "proc p {{input}} {{set {} [llength $input]; set other [llength $input]; upvar 0 {} {}; puts ${{{}}}}}",
            names.0, names.0, names.1, names.1
        );
        let unit = CompilationUnit::build_for(&source, registry, false);
        let function = unit.function("::p").unwrap();
        let mut checked = false;
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let Some(tokens) = statement.statement.tokens() else {
                    continue;
                };
                if tokens.argv_texts.first().map(String::as_str) != Some("puts") {
                    continue;
                }
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                let operand = &tokens.words()[1];
                let read = view.read_word(operand).unwrap();
                let version = read.version.unwrap();
                let (span, prior_effect) = locate_use_var(
                    tokens,
                    view,
                    read.symbol,
                    version,
                    registry,
                    unit.ir_module.lexer_config,
                )
                .unwrap();
                assert!(!prior_effect);
                let span = tcl_lexer::word_span_at(&source, span);
                assert_eq!(
                    &source[span.start() as usize..span.end() as usize],
                    format!("${{{}}}", names.1)
                );
                let other = view.reaching_binding("other", registry).unwrap().symbol;
                assert_ne!(other, read.symbol);
                assert!(
                    locate_use_var(
                        tokens,
                        view,
                        other,
                        version,
                        registry,
                        unit.ir_module.lexer_config
                    )
                    .is_none()
                );
                checked = true;
            }
        }
        assert!(checked);
        assert!(
            o127(&source).is_empty(),
            "an aliased receiver cannot move its store"
        );
    }
}

#[test]
fn o127_observed_alias_receiver_keeps_its_original_store() {
    let source = "proc observe args {}; proc p {input} {set value [llength $input]; upvar 0 value pointer; trace add variable pointer write observe; puts $pointer}";
    assert_eq!(o127(source), [] as [crate::optimiser::Optimisation; 0]);
}

#[test]
fn o127_skips_intervening_side_effect() {
    // The intervening `puts` is impure → forwarding past it is
    // unsafe.
    let src = "proc p {y} {\n    set x [llength $y]\n    puts hi\n    puts $x\n}\n";
    assert_eq!(o127(src).len(), 0, "{:?}", o127(src));
}

#[test]
fn o127_skips_intervening_upframe() {
    // An intervening `uplevel` lowers to a `Statement::UpFrame`,
    // whose opaque body can reach into / out of this frame and
    // mutate any name the forwarded `[llength $y]` reads.  It must
    // suppress the forward exactly like a barrier — otherwise the
    // re-evaluated inline could compute a different value.
    let src = "proc p {y} {\n    set x [llength $y]\n    uplevel 1 {incr ::n}\n    puts $x\n}\n";
    assert_eq!(o127(src).len(), 0, "{:?}", o127(src));
}

#[test]
fn o127_skips_command_substitution_before_endpoint_use() {
    // Tcl substitutes words left-to-right. Moving the `llength` past the
    // preceding substitution would make it observe the emptied `y`.
    let src = "proc p {y} {\n    set x [llength $y]\n    puts [set y {}] $x\n}\n";
    assert_eq!(o127(src).len(), 0, "{:?}", o127(src));
}

#[test]
fn o127_skips_multiple_uses() {
    // Two uses of `$x` → not single-use, no forwarding.
    let src = "proc p {y} {\n    set x [llength $y]\n    puts $x\n    puts $x\n}\n";
    assert_eq!(o127(src).len(), 0, "{:?}", o127(src));
}

#[test]
fn o127_forwards_past_extra_terminator_use() {
    // `$x` has one operand use (`baz $x`) plus a terminator use
    // (`return $x`). Def-use records the terminator use, but it must not
    // block the forward — the forward still fires into the operand use
    // while preserving the assignment for the `return`. The inlined
    // `[set x …]` keeps `x` defined for the trailing `return`.
    let src = "proc baz {value} {return $value}\nproc p {y} {\n    set x [llength $y]\n    baz $x\n    return $x\n}\n";
    let opts = o127(src);
    assert_eq!(opts.len(), 2, "{opts:?}");
    assert!(
        opts.iter()
            .any(|o| o.replacement.contains("[set x [llength $y]]")),
        "{opts:?}"
    );
}

#[test]
fn o127_declines_a_later_unproved_call_after_an_opaque_endpoint() {
    let source = "proc p {y} {set x [llength $y]; baz $x; return $x}";
    assert_eq!(o127(source), [] as [crate::optimiser::Optimisation; 0]);
}

#[test]
fn o127_skips_literal_assignment() {
    // A literal `set x 5` is the O102 path, not O127.
    let src = "proc p {} {\n    set x 5\n    puts $x\n}\n";
    assert_eq!(o127(src).len(), 0, "{:?}", o127(src));
}
