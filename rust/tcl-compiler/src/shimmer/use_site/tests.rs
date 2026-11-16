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
use crate::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

fn captured_read_summary(
    function: &crate::compilation_unit::FunctionUnit,
    registry: &CommandRegistry,
) -> Vec<String> {
    let mut summary = Vec::new();
    for (block, body) in &function.ssa.blocks {
        for index in 0..body.statements.len() {
            let view = crate::ssa::SsaSourceView::at_statement(&function.ssa, *block, index);
            let Some(tokens) = view.source_tokens() else {
                summary.push(format!("{block:?}/{index}: no carrier"));
                continue;
            };
            for access in &tokens.variable_accesses {
                let read = view.read_reference(&access.source, &access.original_spelling);
                let semantic = read.and_then(|read| {
                    read.version
                        .and_then(|version| function.types.get(&(read.symbol, version)))
                });
                let contexts: Vec<_> = access
                    .context_alternatives()
                    .iter()
                    .map(|context| {
                        let place = crate::var_resolve::resolve_substitution_access(
                            &access.original_spelling,
                            context,
                            registry,
                            tcl_registry::TraceOperation::Read,
                        );
                        (
                            context.read_produces_value(&place, registry),
                            context.contents_representation_at(&place),
                            context.container_representation_alternatives_at(&place),
                            context.contents_origin(&place),
                        )
                    })
                    .collect();
                summary.push(format!(
                        "{block:?}/{index} {:?} {:?}: read={read:?}, semantic={semantic:?}, residual={:?}, contexts={contexts:?}",
                        access.source.span,
                        access.original_spelling,
                        access.context_residual(),
                    ));
            }
        }
    }
    summary.sort();
    summary
}

#[test]
fn unchanged_store_bytes_do_not_preserve_a_callback_coerced_representation() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (callback, dictionary_shimmer) in [
        ("", true),
        (
            "upvar 1 dictionary destination; llength $destination",
            false,
        ),
    ] {
        let source = format!(
            "proc observe args {{{callback}}}; proc f {{}} {{set dictionary [dict create k 1]; upvar 0 dictionary view; trace add variable view write observe; set dictionary [dict create k 2]; llength $view}}"
        );
        let unit = CompilationUnit::build_for(&source, registry, false);
        let function = unit.function("::f").unwrap();
        let warnings = use_site_shimmers(function, registry);
        assert_eq!(
            warnings.iter().any(|warning| warning.variable == "view"
                && warning.command == "llength"
                && warning.from_type == TclType::Dict),
            dictionary_shimmer,
            "callback {callback:?}: {warnings:?}; reads={:?}",
            captured_read_summary(function, registry),
        );
    }
}

#[test]
fn reselected_alias_reads_use_the_actual_statement_cell() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc f {} {\nset dictionary [dict create a 1]\nset sequence [list a b]\nupvar 0 dictionary view\nllength $view\nupvar 0 sequence view\nllength $view\n}";
    let cu = CompilationUnit::build_for(source, registry, false);
    let function = cu.function("::f").expect("procedure");
    let mut reads = Vec::new();
    for (block, body) in &function.ssa.blocks {
        for (index, statement) in body.statements.iter().enumerate() {
            let Some(invocation) = crate::registry_invocation::normal_statement_representation(
                registry,
                None,
                &statement.statement,
            ) else {
                continue;
            };
            if invocation.diagnostic_command() != "llength" {
                continue;
            }
            let word = &invocation.effective_words().words[1];
            if word
                .sole_variable_substitution()
                .is_none_or(|(spelling, _)| crate::naming::normalise_var_name(spelling) != "view")
            {
                continue;
            }
            let read = crate::ssa::SsaSourceView::at_statement(&function.ssa, *block, index)
                .read_word(word)
                .expect("proved selected alias read");
            assert!(read.version.is_some_and(|version| version > 0));
            assert_eq!(statement.uses.get(&read.symbol).copied(), read.version);
            reads.push((
                statement.statement.span().start(),
                read.symbol,
                read.version,
            ));
        }
    }
    reads.sort_by_key(|(offset, ..)| *offset);
    assert_eq!(
        reads.len(),
        2,
        "both original alias reads remain represented",
    );
    assert_ne!(reads[0].1, reads[1].1, "retargeting selects distinct cells");
    let warnings = use_site_shimmers(function, registry);
    assert!(
        warnings.iter().any(|warning| warning.command == "llength"
            && warning.variable == "view"
            && warning.from_type == TclType::Dict),
        "the selected dictionary cell retains its producer representation: reads={reads:?}, types={:?}, warnings={warnings:?}",
        function.types
    );
    assert_eq!(
        warnings
            .iter()
            .filter(|warning| warning.command == "llength" && warning.variable == "view")
            .count(),
        1
    );
}

/// The shipped catalogue projected for Tcl 8.6, the release these
/// fixtures analyse.
fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap())
}

#[test]
fn selected_length_cost_distinguishes_conversion_from_preserved_numeric_cache() {
    let source = "proc f {} {set x 0; incr x; llength $x; incr x}";
    for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let unit = CompilationUnit::build_for_profile(source, registry, false, profile);
        let function = unit.function("::f").unwrap();
        let warnings = use_site_shimmers(function, registry);
        for command in ["llength", "incr"] {
            assert_eq!(
                warnings.iter().any(|warning| warning.command == command),
                name == "tcl8.6",
                "{name}: {command}: {warnings:?}"
            );
        }
    }
}

/// Compute commit facts + run the use-site detector for one function unit
/// — the production wiring `find_shimmer_warnings` performs.
fn use_site_shimmers(
    fu: &crate::compilation_unit::FunctionUnit,
    registry: &CommandRegistry,
) -> Vec<ShimmerWarning> {
    let ctx = super::super::commit::CommitCtx {
        registry,
        context: crate::shimmer::ShimmerContext::standalone(registry),
        ssa: &fu.ssa,
        source: crate::ssa::SsaSourceView::unpositioned(&fu.ssa),
        types: &fu.types,
        values: &fu.sccp.values,
        folded: &fu.sccp.folded_types,
    };
    let facts = super::super::ShimmerFacts {
        commit: super::super::commit::compute_commit_facts(
            &fu.cfg,
            &ctx,
            &fu.sccp.executable_blocks,
            &fu.sccp.executable_edges,
        ),
        loop_blocks: super::super::graph::loop_body_blocks(&fu.cfg),
    };
    let in_force = crate::type_infer::types_in_force(&fu.types, &fu.sccp);
    find_use_site_shimmers(
        &fu.cfg,
        &ctx,
        (&fu.sccp.executable_blocks, &in_force),
        &facts,
    )
}

/// A computed constant never hides a conversion
/// (`docs/design/compiler/value-transfers-examples.md` § S100–S110):
/// `[string length $s]` and `incr` build an int, whatever the lattice
/// knows of its string, so reading it as a list re-represents it — the
/// value's representation evidence, not the literal rule, decides. tclsh
/// 8.6, 9.0 and 9.1 agree (`tcl::unsupported::representation`: `int`
/// before the `lindex`, `list` after). A literal of the same spelling
/// is a pure string whose first list conversion is free, and stays
/// silent.
#[test]
fn a_computed_constant_never_hides_a_conversion() {
    let r = registry();
    let src = "proc computed {} {\n    set s abc\n    set n [string length $s]\n    \
                   lindex $n 0\n}\nproc counted {} {\n    set i 0\n    incr i\n    \
                   lindex $i 0\n}\nproc literal {} {\n    set n 3\n    lindex $n 0\n}\n";
    let cu = CompilationUnit::build_for(src, &r, false);
    let shimmers = |name: &str| use_site_shimmers(cu.function(name).unwrap(), &r);
    for (name, var) in [("::computed", "n"), ("::counted", "i")] {
        let found = shimmers(name);
        assert!(
            found.iter().any(|w| w.variable == var
                && w.command == "lindex"
                && w.from_type == TclType::Int
                && w.to_type == TclType::List),
            "{name}: {found:?}"
        );
    }
    assert!(
        shimmers("::literal").is_empty(),
        "{:?}",
        shimmers("::literal")
    );
}

/// A String variable passed to `incr` triggers S100.
#[test]
fn shimmer_detected_for_string_used_with_incr() {
    let cu = CompilationUnit::build_for("set x \"hello\"\nincr x", &registry(), false);
    let fu = cu.function("::top").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings
        .iter()
        .find(|w| w.command == "incr" && w.from_type == TclType::String);
    assert!(
        w.is_some(),
        "expected incr/String shimmer, got: {warnings:?}"
    );
    assert_eq!(w.unwrap().to_type, TclType::Int);
}

/// A hex literal string (`0x80`) classifies as
/// String but promotes cleanly to Int at `incr` — no shimmer.
#[test]
fn no_shimmer_for_hex_literal_string_used_with_incr() {
    let cu = CompilationUnit::build_for("set n 0x80\nincr n", &registry(), false);
    let fu = cu.function("::top").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let incr_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "incr").collect();
    assert!(
        incr_shimmers.is_empty(),
        "0x80 promotes cleanly to int — no incr shimmer expected: {incr_shimmers:?}",
    );
}

/// An Int variable passed to `incr` has no shimmer.
#[test]
fn no_shimmer_for_int_used_with_incr() {
    let cu = CompilationUnit::build_for("set x 5\nincr x", &registry(), false);
    let fu = cu.function("::top").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let incr_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "incr").collect();
    assert!(
        incr_shimmers.is_empty(),
        "unexpected incr shimmer for Int: {incr_shimmers:?}"
    );
}

/// A *committed* Int variable passed to `lindex` triggers a shimmer
/// (Int → List at arg 0). The Int comes from `[llength]` (a committed
/// numeric intrep), not a bare literal — `set x 5; lindex $x 0` is a *pure*
/// value that promotes for free (see `no_shimmer_for_pure_literal_as_list`).
#[test]
fn shimmer_detected_for_int_used_with_lindex() {
    let cu = CompilationUnit::build_for(
        "proc f {lst} {\n set x [llength $lst]\n lindex $x 0\n}",
        &registry(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings.iter().find(|w| w.command == "lindex");
    assert!(
        w.is_some(),
        "expected lindex shimmer for committed Int var, got: {warnings:?}"
    );
    assert_eq!(w.unwrap().from_type, TclType::Int);
    assert_eq!(w.unwrap().to_type, TclType::List);
}

/// TN: a *pure* literal — even a number-shaped or word-shaped
/// one — used as a list must NOT shimmer. `set a 1; foreach b $a` and `set x
/// 5; lindex $x 0` promote a pure string to a list for free (oracle: `set x
/// 5` is a pure string, `lindex` parses it into a 1-element list once). The
/// define-time shape (`1` looks integer-ish) does not make the value any
/// less a valid list.
#[test]
fn no_shimmer_for_pure_literal_as_list() {
    for src in [
        "set a 1\nforeach b $a {}",
        "set x 5\nlindex $x 0",
        "set p 3.14\nlindex $p 0",
    ] {
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        assert!(
            warnings.is_empty(),
            "pure literal used as a list must not shimmer for {src:?}: {warnings:?}"
        );
    }
}

/// The warning's span is tight around the offending `$x` argument, not
/// the whole `lindex $x 0` invocation — the developer's eye should land
/// on the variable, not have to scan the whole call.
#[test]
fn shimmer_span_is_tight_around_call_argument() {
    let src = "set x [dict create a 1 b 2]\nlindex $x 0";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::top").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings
        .iter()
        .find(|w| w.command == "lindex")
        .unwrap_or_else(|| panic!("expected lindex shimmer, got: {warnings:?}"));
    let text = &src[w.span.start() as usize..w.span.end() as usize];
    assert_eq!(
        text, "$x",
        "span should cover only the '$x' argument, got {text:?} from {w:?}"
    );
}

/// A `[cmd …]` in a call argument reads its
/// arguments exactly as the same command on its own line does — Tcl runs
/// it either way.
#[test]
fn use_site_shimmer_fires_in_a_nested_call_argument() {
    let cu = CompilationUnit::build_for(
        "proc f {lst} {\n set x [llength $lst]\n puts [lindex $x 0]\n}",
        &registry(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    let w = use_site_shimmers(fu, &registry());
    let hit = w.iter().find(|w| w.command == "lindex");
    assert!(hit.is_some(), "nested lindex is still a list read: {w:?}");
    assert_eq!(hit.unwrap().from_type, TclType::Int);
    assert_eq!(hit.unwrap().to_type, TclType::List);
}

/// The braced spelling of that word is a literal Tcl never runs, and the
/// flat argument text cannot tell the two apart — only `word_exprs` can.
#[test]
fn no_use_site_shimmer_in_a_braced_call_argument() {
    let cu = CompilationUnit::build_for(
        "proc f {lst} {\n set x [llength $lst]\n puts {[lindex $x 0]}\n}",
        &registry(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    let w = use_site_shimmers(fu, &registry());
    assert!(
        !w.iter().any(|w| w.command == "lindex"),
        "a braced word is data, not a call: {w:?}"
    );
}

/// The braced / quoted / bare spellings of one argument
/// word, pinned against each other for a command that consumes its word
/// as **data** (`lindex`) and one that re-evaluates it **in the caller's
/// frame** (`expr`) — the two halves of the rule, so neither can drift.
///
/// tclsh 9.0.4, `set x 5`: `lindex {$x} 0` yields the two characters
/// `$x` (no read at all) while `lindex "$x" 0` and `lindex $x 0` both
/// yield `5`; `expr {$x} + 1` yields `6`, so every `expr` spelling reads
/// `x`. A statement's IR `args` hold the *de-braced* word, so all six
/// spellings look alike there — the answer comes from the segmenter's
/// token kinds plus the registry's argument role.
#[test]
fn a_braced_argument_reads_only_where_the_callee_evaluates_it_in_frame() {
    for dialect in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = registry.profile().expect("actual native fixture profile");
        for (body, reads) in [
            ("set x [llength $lst]\n lindex {$x} 0", false),
            ("set x [llength $lst]\n lindex \"$x\" 0", true),
            ("set x [llength $lst]\n lindex $x 0", true),
            ("set x [lrange $lst 0 1]\n expr {$x} + 1", true),
            ("set x [lrange $lst 0 1]\n expr \"$x\" + 1", true),
            ("set x [lrange $lst 0 1]\n expr $x + 1", true),
        ] {
            // Ordinary input closes C 9 abstract-list callbacks, which can
            // otherwise retarget x before its result store. A nonempty
            // ordinary range also supplies an actual List result.
            let input = "set constructor list; set lst [$constructor 7];";
            let src = format!("proc f {{lst}} {{\n {input} {body}\n}}");
            let cu = CompilationUnit::build_for_profile(&src, registry, false, profile);
            let w = super::super::find_shimmer_warnings_for_cu(&cu, registry);
            assert_eq!(
                !w.is_empty(),
                reads,
                "expected reads={reads} for {src:?}, got: {w:?}; captured={:?}",
                captured_read_summary(cu.function("::f").unwrap(), registry)
            );
        }
    }
}

#[test]
fn an_unknown_range_result_does_not_donate_a_list_conversion_warning() {
    let unit = CompilationUnit::build_for(
        "proc f {lst} {set x [lrange $lst 0 1]; expr {$x} + 1}",
        &registry(),
        false,
    );
    let warnings = super::super::find_shimmer_warnings_for_cu(&unit, &registry());
    assert!(
        warnings.is_empty(),
        "unknown/empty/abstract range: {warnings:?}"
    );
    for dialect in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let unit = CompilationUnit::build_for_profile(
            "proc f {lst} {set x [lrange $lst 0 1]; expr {$x} + 1}",
            registry,
            false,
            registry.profile().expect("actual native fixture profile"),
        );
        let warnings = super::super::find_shimmer_warnings_for_cu(&unit, registry);
        assert!(
            warnings.is_empty(),
            "{dialect}: unknown range: {warnings:?}"
        );
    }
}

#[test]
fn an_unknown_or_replaced_constructor_cannot_donate_native_range_representation() {
    let source = "proc f {} {set constructor list; set lst [$constructor 7]; set x [lrange $lst 0 1]; expr {$x} + 1}";
    let registry = registry();
    // Generic authoring retains an actual selected native entry; the
    // absence of an explicit profile argument is not an unknown engine.
    let unit = CompilationUnit::build_for(source, &registry, false);
    let warnings = super::super::find_shimmer_warnings_for_cu(&unit, &registry);
    assert!(!warnings.is_empty(), "actual authoring entry: {warnings:?}");
    let entry = crate::command_binding::SourceAnalysisEntry {
        unknown_entry: true,
        ..Default::default()
    };
    let unknown = CompilationUnit::build_with_source_entry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::default(),
            dialect: None,
            external_call_sites: None,
            declared_commands: None,
        },
        &entry,
    );
    let warnings = super::super::find_shimmer_warnings_for_cu(&unknown, &registry);
    assert!(warnings.is_empty(), "unknown entry: {warnings:?}");
    for dialect in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let replaced = format!("rename list native_list; proc list args {{return 7}}; {source}");
        let unit = CompilationUnit::build_for_profile(
            &replaced,
            registry,
            false,
            registry.profile().expect("actual native fixture profile"),
        );
        let warnings = super::super::find_shimmer_warnings_for_cu(&unit, registry);
        assert!(
            warnings.is_empty(),
            "{dialect}: replaced creator: {warnings:?}"
        );
    }
}

#[test]
fn an_unknown_abstract_input_does_not_close_the_result_store() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
    let unit = CompilationUnit::build_for_profile(
        "proc f {lst} {set x [llength $lst]; lindex $x 0}",
        registry,
        false,
        registry.profile().expect("actual native fixture profile"),
    );
    let warnings = super::super::find_shimmer_warnings_for_cu(&unit, registry);
    assert!(warnings.is_empty(), "unknown callback world: {warnings:?}");
}

/// The same braced word must not move the **committed-intrep state**
/// either. `commit.rs` emits nothing itself, so a gate only in the
/// emitting pass left `lindex {$x} 0` recording a list commitment that
/// made the genuine read on the next line report a conversion the
/// runtime never performs.
#[test]
fn a_braced_argument_commits_no_intrep_for_later_reads() {
    let cu = CompilationUnit::build_for(
        "proc f {lst} {\n set x [llength $lst]\n lindex {$x} 0\n expr {$x + 1}\n}",
        &registry(),
        false,
    );
    let w = super::super::find_shimmer_warnings_for_cu(&cu, &registry());
    assert!(
        w.is_empty(),
        "the int `x` is only ever read as a number here: {w:?}"
    );
}

/// The two statement kinds that carry substitutable words
/// must agree about which commands a word runs. `puts <word>` and
/// `set r <word>` differ only in the outer command, and neither outer
/// command changes what Tcl evaluates inside the word — so any word that
/// shimmers in a call argument shimmers identically in an assignment
/// value, and any word that does not, does not.
///
/// Written as a paired assertion rather than two independent expectations
/// on purpose: it is the *symmetry* that regressed (the `AssignValue` arm
/// parsed only its outermost `[cmd …]` and so went silent below depth
/// one), and only a test that compares the paths can catch them drifting
/// apart again.
#[test]
fn assign_value_lifts_the_same_substitutions_a_call_does() {
    // (word, does its `lindex` run?) — a bare substitution, then nested
    // depths, then the braced spelling Tcl never substitutes.
    for (word, runs) in [
        ("[lindex $x 0]", true),
        ("[list [lindex $x 0]]", true),
        ("[list \"[lindex $x 0]\"]", true),
        ("[list a[lindex $x 0]b]", true),
        ("[list {[lindex $x 0]}]", false),
    ] {
        let mut verdicts = Vec::new();
        for stmt in [format!("puts {word}"), format!("set r {word}")] {
            let src = format!("proc f {{lst}} {{\n set x [llength $lst]\n {stmt}\n}}");
            let cu = CompilationUnit::build_for(&src, &registry(), false);
            let fu = cu.function("::f").unwrap();
            let w = use_site_shimmers(fu, &registry());
            let hit = w.iter().find(|w| w.command == "lindex").cloned();
            if let Some(hit) = hit.as_ref() {
                assert_eq!(hit.from_type, TclType::Int, "in {stmt:?}");
                assert_eq!(hit.to_type, TclType::List, "in {stmt:?}");
            }
            verdicts.push((stmt, hit.is_some()));
        }
        let [(call, call_fired), (assign, assign_fired)] = verdicts.as_slice() else {
            unreachable!("one verdict per spelling")
        };
        assert_eq!(
            call_fired, assign_fired,
            "{call:?} and {assign:?} run the same commands and must report alike"
        );
        assert_eq!(*call_fired, runs, "wrong verdict for {call:?}");
    }
}

/// The lift is the *only* path into the `AssignValue` arm, so the
/// depth-zero substitution must be reported exactly once — a second,
/// arm-local parse of the outermost `[cmd …]` would double every
/// `set b [lindex $x 0]` diagnostic.
#[test]
fn assign_value_substitution_is_reported_once() {
    let cu = CompilationUnit::build_for(
        "proc f {lst} {\n set x [llength $lst]\n set b [lindex $x 0]\n}",
        &registry(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    let w = use_site_shimmers(fu, &registry());
    assert_eq!(
        w.iter().filter(|w| w.command == "lindex").count(),
        1,
        "one substitution, one report: {w:?}"
    );
}

/// The span stays tight around the `$x` of the *nested* command, not the
/// outer `[list …]` and not the whole `set` — the developer's eye has to
/// land on the read that actually converts.
#[test]
fn assign_value_nested_substitution_span_is_tight() {
    let src = "proc f {lst} {\n set x [llength $lst]\n set r [list [lindex $x 0]]\n}";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings
        .iter()
        .find(|w| w.command == "lindex")
        .unwrap_or_else(|| panic!("expected nested lindex shimmer, got: {warnings:?}"));
    let text = &src[w.span.start() as usize..w.span.end() as usize];
    assert_eq!(text, "$x", "got {text:?} from {w:?}");
}

/// The conversion a nested substitution performs must also reach the
/// commit state, or every later read of the same variable is judged
/// against a stale representation. Before this, the `incr` here reported
/// nothing at all — while the identical code with `lindex $x 0` on its own
/// line reported it.
#[test]
fn a_nested_conversion_is_visible_to_later_reads() {
    let cu = CompilationUnit::build_for(
        "proc f {lst} {\n set x [llength $lst]\n puts [lindex $x 0]\n incr x\n}",
        &registry(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    let w = use_site_shimmers(fu, &registry());
    let incr = w.iter().find(|w| w.command == "incr");
    assert!(incr.is_some(), "the incr after a nested list read: {w:?}");
    assert_eq!(
        incr.unwrap().from_type,
        TclType::List,
        "it converted back from the list the nested `lindex` installed"
    );
}

/// The commit half: the same must hold when the nested substitution sits
/// in an assignment value. An `AssignValue` transfer function that walks
/// only the outermost `[cmd …]` moves no state for `[list […]]`, so the
/// following `incr` is judged against the stale `Int` and reports
/// nothing.
#[test]
fn a_nested_conversion_in_an_assignment_is_visible_to_later_reads() {
    let cu = CompilationUnit::build_for(
        "proc f {lst} {\n set x [llength $lst]\n set r [list [lindex $x 0]]\n incr x\n}",
        &registry(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    let w = use_site_shimmers(fu, &registry());
    let incr = w.iter().find(|w| w.command == "incr");
    assert!(incr.is_some(), "the incr after a nested list read: {w:?}");
    assert_eq!(
        incr.unwrap().from_type,
        TclType::List,
        "it converted back from the list the nested `lindex` installed"
    );
}

/// Same tightness property through the `[cmd …]`-in-`AssignValue` path
/// (`set b [lindex $x 0]`) — the span still lands on `$x`, not the
/// whole `set b […]` statement.
#[test]
fn shimmer_span_is_tight_around_assign_value_substitution_argument() {
    let src = "set x [dict create a 1 b 2]\nset b [lindex $x 0]";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::top").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings
        .iter()
        .find(|w| w.command == "lindex")
        .unwrap_or_else(|| panic!("expected lindex shimmer, got: {warnings:?}"));
    let text = &src[w.span.start() as usize..w.span.end() as usize];
    assert_eq!(
        text, "$x",
        "span should cover only the '$x' argument, got {text:?} from {w:?}"
    );
}

/// Two shimmering arguments of the *same* call each get their *own*
/// tight span — proves the per-index span lookup, not just "arg 0's
/// span reused everywhere". `linsert list index element`: `list_var`
/// (committed Dict, arg 0) expects List; `index_var` (String `world`, arg 1)
/// expects Int — `world` is not a valid integer, so it still fires.
#[test]
fn shimmer_span_is_tight_per_argument_not_reused_across_args() {
    let src = "set list_var [dict create a 1 b 2]\n\
                    set index_var world\n\
                    linsert $list_var $index_var x";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::top").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let list_w = warnings
        .iter()
        .find(|w| w.command == "linsert" && w.variable == "list_var")
        .unwrap_or_else(|| panic!("expected linsert shimmer for list_var, got: {warnings:?}"));
    let list_text = &src[list_w.span.start() as usize..list_w.span.end() as usize];
    assert_eq!(list_text, "$list_var", "got {list_w:?}");

    let index_w = warnings
        .iter()
        .find(|w| w.command == "linsert" && w.variable == "index_var")
        .unwrap_or_else(|| panic!("expected linsert shimmer for index_var, got: {warnings:?}"));
    let index_text = &src[index_w.span.start() as usize..index_w.span.end() as usize];
    assert_eq!(index_text, "$index_var", "got {index_w:?}");
}

/// TN: a `foreach` element variable is typed String — each element is a
/// *pure* string, and reading it as a list via `[lindex $x 0]` is a free
/// first conversion (oracle: a list element goes pure → list once, per
/// value). It is NOT a per-iteration shimmer: no cached intrep is thrown
/// away, because a fresh element value arrives each pass. The substitution
/// lives in an `AssignValue` value, so this also guards the `AssignValue`
/// arm against re-flagging pure elements.
#[test]
fn no_shimmer_for_foreach_element_used_as_list_in_cmd_sub() {
    let src = "proc f {l} {\n    foreach x $l {\n        set b [lindex $x 0]\n    }\n}\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    assert!(
        warnings.iter().all(|w| w.command != "lindex"),
        "a pure foreach element read as a list must not shimmer: {warnings:?}"
    );
}

/// TP control (the committed counterpart): a *committed* value re-read as a
/// list inside a loop, via the same `[cmd …]`-in-`AssignValue` path, DOES
/// shimmer per iteration (S101). `[dict create k $x]` builds a fresh dict
/// each pass (defined in the loop, so not the loop-invariant downgrade), and
/// `[llength $d]` re-represents it list-ward every iteration.
#[test]
fn shimmer_detected_for_committed_value_as_list_in_cmd_sub() {
    let src = "proc f {l} {\n    foreach x $l {\n        set d [dict create k $x]\n        set n [llength $d]\n    }\n}\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings
        .iter()
        .find(|w| w.command == "llength" && w.variable == "d")
        .unwrap_or_else(|| panic!("expected llength S101 for committed dict, got: {warnings:?}"));
    assert_eq!(w.code, DiagCode::S101);
    assert_eq!(w.from_type, TclType::Dict);
    assert_eq!(w.to_type, TclType::List);
}

/// TP: `foreach`'s own list argument shimmers when the variable holds a
/// *committed* non-list intrep — the CFG builder lowers the header to a
/// synthetic `Statement::Call` (`command="foreach", args=[list_arg]`) that
/// never reaches the per-index `arg_types` path, so this is the
/// `IterationBindings` input-representation proof path specifically. A committed `Dict`
/// (from `[dict create]`) genuinely re-represents when foreach reads it as a
/// list; a *pure* string would be a free first conversion (see the
/// `no_shimmer_for_foreach_header_with_pure_string` TN below).
#[test]
fn shimmer_detected_for_foreach_header_list_argument() {
    let src = "proc f {} {\n    set l [dict create a 1 b 2]\n    foreach x $l { puts $x }\n}\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings
        .iter()
        .find(|w| w.command == "foreach" && w.variable == "l")
        .unwrap_or_else(|| panic!("expected foreach header shimmer, got: {warnings:?}"));
    assert_eq!(w.from_type, TclType::Dict);
    assert_eq!(w.to_type, TclType::List);
}

/// TN control: a genuine list variable in `foreach` must not shimmer.
#[test]
fn no_shimmer_for_foreach_header_with_real_list() {
    let src = "proc f {} {\n    set l [list 1 2 3]\n    foreach x $l { puts $x }\n}\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    assert!(
        warnings.iter().all(|w| w.command != "foreach"),
        "unexpected foreach header shimmer for a real list: {warnings:?}"
    );
}

/// TN: a *pure* string literal in `foreach` must not shimmer —
/// `{a b c}` (and any well-formed list literal) is a valid list, and Tcl
/// parses it into a list intrep once, for free (oracle: the value goes pure
/// → list).
#[test]
fn no_shimmer_for_foreach_header_with_pure_string() {
    for literal in ["{10.0 12.0 16.0 24.0}", "hello", "{}", "1"] {
        let src =
            format!("proc f {{}} {{\n    set l {literal}\n    foreach x $l {{ puts $x }}\n}}\n");
        let cu = CompilationUnit::build_for(&src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        assert!(
            warnings.iter().all(|w| w.variable != "l"),
            "pure string literal {literal:?} in foreach must not shimmer: {warnings:?}"
        );
    }
}

/// TP: `lmap`'s list argument shares the same synthetic header shape.
#[test]
fn shimmer_detected_for_lmap_header_list_argument() {
    let src = "proc f {} {\n    set l [dict create a 1 b 2]\n    lmap x $l { set x $x }\n}\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings
        .iter()
        .find(|w| w.command == "lmap" && w.variable == "l")
        .unwrap_or_else(|| panic!("expected lmap header shimmer, got: {warnings:?}"));
    assert_eq!(w.from_type, TclType::Dict);
    assert_eq!(w.to_type, TclType::List);
}

/// TP: `dict for`'s dict argument shimmers to Dict — the compound
/// two-word command name (`"dict for"`) routes through the `dict`
/// subcommand table via `IterationBindings` input-representation proof's split.
#[test]
fn shimmer_detected_for_dict_for_header_argument() {
    let src = "proc f {} {\n    set d hello\n    dict for {k v} $d { puts $k }\n}\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings
        .iter()
        .find(|w| w.command == "dict for" && w.variable == "d")
        .unwrap_or_else(|| panic!("expected dict-for header shimmer, got: {warnings:?}"));
    assert_eq!(w.from_type, TclType::String);
    assert_eq!(w.to_type, TclType::Dict);
}

/// TP: `dict map` shares `dict for`'s compound-name dispatch.
#[test]
fn shimmer_detected_for_dict_map_header_argument() {
    let src = "proc f {} {\n    set d hello\n    dict map {k v} $d { set v $v }\n}\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings
        .iter()
        .find(|w| w.command == "dict map" && w.variable == "d")
        .unwrap_or_else(|| panic!("expected dict-map header shimmer, got: {warnings:?}"));
    assert_eq!(w.from_type, TclType::String);
    assert_eq!(w.to_type, TclType::Dict);
}

/// A loop-invariant variable (defined outside the loop) coerced to a
/// single intrep inside the loop converts once and is cached — that is a
/// one-time S100, not the per-iteration S101.
#[test]
fn loop_invariant_single_target_downgrades_to_s100() {
    for dialect in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = registry.profile().expect("actual native fixture profile");
        let src = "proc f {} {\n  set data [dict create a 1 b 2]\n  for {set i 0} {$i < 3} {incr i} {\n    set x [lindex $data $i]\n  }\n}\n";
        let cu = CompilationUnit::build_for_profile(src, registry, false, profile);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, registry);
        let w = warnings
            .iter()
            .find(|w| w.command == "lindex" && w.variable == "data");
        assert!(
            w.is_some(),
            "expected lindex shimmer for data: {warnings:?}"
        );
        assert_eq!(
            w.unwrap().code,
            DiagCode::S100,
            "loop-invariant single-target use is one-time (S100): {warnings:?}"
        );
    }
}

/// A conversion performed by a *nested* `[cmd …]` is paid on
/// every iteration exactly as the direct spelling is, so it classifies the
/// same way.
///
/// `record_use_targets` read only the flat argument text, so for
/// `puts [list [llength $x]]` the single word `[list [llength $x]]` did not
/// start with `$`, nothing was recorded, `use_targets` stayed below the two
/// distinct targets `effective_in_loop` needs, and a genuinely
/// per-iteration conversion was reported S100 instead of S101.
#[test]
fn a_nested_conversion_in_a_loop_classifies_as_s101_like_the_direct_one() {
    for dialect in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = registry.profile().expect("actual native fixture profile");
        for (first, second) in [
            // Direct statements — the row that was already right.
            ("llength $x", "dict size $x"),
            // Nested inside a `Statement::Call`.
            ("puts [list [llength $x]]", "puts [list [dict size $x]]"),
            // Nested inside a `Statement::AssignValue`.
            ("set a [list [llength $x]]", "set b [list [dict size $x]]"),
        ] {
            let src = format!(
                "proc f {{l}} {{\n  set x [dict create a 1 b 2]\n  foreach i $l {{\n    \
                 {first}\n    {second}\n  }}\n  return $x\n}}\n"
            );
            let cu = CompilationUnit::build_for_profile(&src, registry, false, profile);
            let fu = cu.function("::f").unwrap();
            let warnings = use_site_shimmers(fu, registry);
            let on_x: Vec<&ShimmerWarning> =
                warnings.iter().filter(|w| w.variable == "x").collect();
            assert!(
                !on_x.is_empty(),
                "expected shimmer warnings on x for {first:?} / {second:?}: {warnings:?}; captured={:?}",
                captured_read_summary(fu, registry),
            );
            assert!(
                on_x.iter().all(|w| w.code == DiagCode::S101),
                "a per-iteration conversion is S101 however it is spelled; \
                 {first:?} / {second:?} gave {on_x:?}",
            );
        }
    }
}

/// The same brace gate the emitting passes apply: a
/// braced word substitutes nothing, so it contributes no use target and
/// cannot push a loop-invariant value over the two-target threshold.
#[test]
fn a_braced_argument_contributes_no_use_target() {
    let src = "proc f {l} {\n  set x [dict create a 1 b 2]\n  foreach i $l {\n    \
                   llength {$x}\n    dict size $x\n  }\n  return $x\n}\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    assert!(
        warnings
            .iter()
            .filter(|w| w.variable == "x")
            .all(|w| w.code != DiagCode::S101),
        "`llength {{$x}}` reads nothing, so only one target is live and the \
             conversion is one-time: {warnings:?}",
    );
}

/// `incr n $step` where `$step` holds a String shimmers on the increment
/// argument (Tcl coerces it to int).
#[test]
fn incr_amount_string_var_shimmers() {
    let cu = CompilationUnit::build_for(
        "set n 0\nset step \"hello\"\nincr n $step",
        &registry(),
        false,
    );
    let fu = cu.function("::top").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings
        .iter()
        .find(|w| w.command == "incr" && w.variable == "step");
    assert!(
        w.is_some(),
        "expected incr-amount shimmer for step: {warnings:?}"
    );
    assert_eq!(w.unwrap().from_type, TclType::String);
    assert_eq!(w.unwrap().to_type, TclType::Int);
}

/// Array elements collapse onto one SSA symbol (the `(key)` suffix is
/// stripped before interning), so two individually-stable but different
/// elements must not use-site shimmer against each other (FP-SH-13):
/// `set arr(n) 5; set arr(label) "text"; incr arr(n)` must not report
/// `arr` as "string used with incr" — `arr(n)` is always int.
#[test]
fn no_use_site_shimmer_for_array_element_conflation() {
    let cu = CompilationUnit::build_for(
        "proc f {} { set arr(n) 5\n set arr(label) \"text\"\n incr arr(n) }",
        &registry(),
        false,
    );
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    assert!(
        warnings.is_empty(),
        "array-element conflation must not use-site shimmer: {warnings:?}"
    );
}

/// TP control: the identical shape on a plain scalar still fires — the
/// array guard must not blanket-silence use-site shimmer.
/// `string length`/`index`/`range` install the string intrep on their
/// subject (tclsh-verified: a list becomes `string`) — genuine shimmer —
/// but a pure byte array short-circuits and keeps its rep
/// (`transparent_from`), so it must stay silent.
#[test]
fn string_subject_shimmers_list_but_not_bytearray() {
    // TP: a List-typed subject is coerced to the string intrep.
    let cu = CompilationUnit::build_for("set l [list 1 2 3]\nstring length $l", &registry(), false);
    let fu = cu.function("::top").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    assert!(
        warnings
            .iter()
            .any(|w| w.variable == "l" && w.to_type == TclType::String),
        "list subject of `string length` must shimmer: {warnings:?}"
    );

    // FP guard: a ByteArray-typed subject passes through untouched.
    let cu2 = CompilationUnit::build_for(
        "set ba [binary format c* {200 201}]\nstring length $ba\nstring range $ba 0 1",
        &registry(),
        false,
    );
    let fu2 = cu2.function("::top").unwrap();
    let warnings2 = use_site_shimmers(fu2, &registry());
    assert!(
        !warnings2.iter().any(|w| w.variable == "ba"),
        "byte-array subject is transparent — no shimmer: {warnings2:?}"
    );
}

#[test]
fn use_site_shimmer_still_fires_for_scalar_control() {
    let cu = CompilationUnit::build_for("proc f {} { set n hello\n incr n }", &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    assert!(
        warnings
            .iter()
            .any(|w| w.command == "incr" && w.variable == "n"),
        "plain scalar must still use-site shimmer: {warnings:?}"
    );
}

/// Variables with Unknown type do not produce false-positive shimmers.
#[test]
fn no_shimmer_for_unknown_type() {
    // `set x $other` — type of x is Unknown (other has no known type).
    let cu = CompilationUnit::build_for("set x $other\nlindex $x 0", &registry(), false);
    let fu = cu.function("::top").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    // x has Unknown type; should not produce a shimmer.
    let lindex_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "lindex").collect();
    assert!(
        lindex_shimmers.is_empty(),
        "unexpected shimmer for Unknown type: {lindex_shimmers:?}"
    );
}

/// TP: a call through an `interp alias` to a shimmering builtin is still
/// detected — the registry lookup keys off the resolved
/// `canonical_command`, not the alias's source spelling. `$x` holds a
/// *committed* Dict so the list conversion is a genuine shimmer (a pure
/// string would promote for free through the alias too).
#[test]
fn shimmer_detected_through_interp_alias_to_lindex() {
    let cu = CompilationUnit::build_for(
        "interp alias {} myindex {} ::lindex\nset x [dict create a 1 b 2]\nmyindex $x 0",
        &registry(),
        false,
    );
    let fu = cu.function("::top").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings.iter().find(|w| w.variable == "x");
    assert!(
        w.is_some(),
        "expected shimmer through interp alias, got: {warnings:?}"
    );
    let w = w.unwrap();
    assert_eq!(w.from_type, TclType::Dict);
    assert_eq!(w.to_type, TclType::List);
    // The message keeps the alias spelling the user wrote, not the
    // resolved canonical target.
    assert_eq!(w.command, "myindex");
}

/// Helper for the string-subcommand family tests below: all use-site
/// warnings for `src`'s `::top`.
fn warnings_for(src: &str) -> Vec<ShimmerWarning> {
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::top").unwrap();
    use_site_shimmers(fu, &registry())
}

/// `string first`/`last` convert BOTH the needle and the haystack to the
/// string intrep; a pure byte array stays silent (the 9.0-safe
/// under-approximation — see the registry note). tclsh 8.6.14 probe:
///
/// ```text
/// string first pat SUBJ   list:  list -> string
/// string first SUBJ hay   list:  list -> string
/// string last  pat SUBJ   list:  list -> string
/// string last  SUBJ hay   list:  list -> string
/// string first pat SUBJ   bytes: bytearray -> string   (silenced: 9.0
/// string first SUBJ hay   bytes: bytearray -> string    keeps the rep
///                                                       for a both-
///                                                       bytearray pair)
/// ```
#[test]
fn string_first_last_needle_and_haystack_shimmer_list_not_bytearray() {
    for sub in ["first", "last"] {
        // TP: a List-typed needle and a List-typed haystack each fire.
        let warnings = warnings_for(&format!(
            "set n [list 1 2]\nset h [list 3 4]\nstring {sub} $n $h",
        ));
        for var in ["n", "h"] {
            assert!(
                warnings
                    .iter()
                    .any(|w| w.variable == var && w.to_type == TclType::String),
                "string {sub}: list-typed '{var}' must shimmer to string: {warnings:?}"
            );
        }
        // FP guard: byte-array operands stay silent.
        let warnings = warnings_for(&format!(
            "set n [binary format c* {{200}}]\nset h [binary format c* {{199 200}}]\nstring {sub} $n $h",
        ));
        assert!(
            warnings
                .iter()
                .all(|w| w.variable != "n" && w.variable != "h"),
            "string {sub}: byte-array operands must stay silent: {warnings:?}"
        );
    }
}

/// `string map` subject: converts to the string intrep from a List AND
/// from a pure `ByteArray` (no fast path in 8.6 or 9.0 — the subject is
/// read via `Tcl_GetUnicodeFromObj` unconditionally). tclsh 8.6.14:
///
/// ```text
/// string map {a b} SUBJ   list:  list -> string
/// string map {a b} SUBJ   bytes: bytearray -> string
/// ```
#[test]
fn string_map_subject_shimmers_list_and_bytearray() {
    let warnings = warnings_for("set s [list 1 2 3]\nstring map {a b} $s");
    assert!(
        warnings
            .iter()
            .any(|w| w.variable == "s" && w.to_type == TclType::String),
        "list-typed map subject must shimmer to string: {warnings:?}"
    );
    let warnings = warnings_for("set s [binary format c* {200 201}]\nstring map {a b} $s");
    assert!(
        warnings.iter().any(|w| w.variable == "s"
            && w.from_type == TclType::ByteArray
            && w.to_type == TclType::String),
        "byte-array map subject genuinely converts — must fire: {warnings:?}"
    );
}

/// `string map` mapping: the dict iteration path keeps a pure dict, and
/// `TclListObjGetElements` keeps a list — neither may fire. tclsh
/// 8.6.14 (probe refuting the former `expected: Dict` hint):
///
/// ```text
/// map mapping list:   list -> list
/// map mapping dict:   dict -> dict
/// map mapping string: pure -> list
/// ```
#[test]
fn string_map_mapping_list_and_dict_stay_silent() {
    for mapping in ["[list a b]", "[dict create a b]"] {
        let warnings = warnings_for(&format!("set m {mapping}\nstring map $m xyz"));
        assert!(
            warnings.iter().all(|w| w.variable != "m"),
            "mapping {mapping} keeps its intrep — must stay silent: {warnings:?}"
        );
    }
}

/// The `-nocase` option shifts `string map`'s positionals by one; the
/// leading-option skip keeps the mapping/subject hints aligned, so the
/// mapping stays silent and the subject still fires.
#[test]
fn string_map_nocase_keeps_hint_alignment() {
    let warnings = warnings_for("set m [list a b]\nset s [list 1 2]\nstring map -nocase $m $s");
    assert!(
        warnings.iter().all(|w| w.variable != "m"),
        "-nocase mapping keeps its list intrep — must stay silent: {warnings:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.variable == "s" && w.to_type == TclType::String),
        "-nocase subject must still shimmer: {warnings:?}"
    );
}

/// `string replace`/`reverse` subjects: List converts to the string
/// intrep, a pure byte array keeps its rep in both 8.6 and 9.0. tclsh
/// 8.6.14:
///
/// ```text
/// string replace SUBJ 0 0 Z   list:  list -> string
/// string replace SUBJ 0 0 Z   bytes: bytearray -> bytearray
/// string reverse SUBJ         list:  list -> string
/// string reverse SUBJ         bytes: bytearray -> bytearray
/// ```
#[test]
fn string_replace_reverse_subject_shimmers_list_not_bytearray() {
    for call in ["string replace $s 0 0 Z", "string reverse $s"] {
        let warnings = warnings_for(&format!("set s [list 1 2 3]\n{call}"));
        assert!(
            warnings
                .iter()
                .any(|w| w.variable == "s" && w.to_type == TclType::String),
            "{call}: list subject must shimmer: {warnings:?}"
        );
        let warnings = warnings_for(&format!("set s [binary format c* {{200 201}}]\n{call}"));
        assert!(
            warnings.iter().all(|w| w.variable != "s"),
            "{call}: byte-array subject is transparent — silent: {warnings:?}"
        );
    }
}

/// Negatives verified on tclsh 8.6.14 (a list subject stays `list`
/// through every one of these — the subject is read via its string rep
/// only, dual-ported):
///
/// ```text
/// string compare SUBJ y / y SUBJ    list -> list
/// string equal SUBJ y               list -> list
/// string match p* SUBJ / SUBJ s     list -> list
/// string repeat SUBJ 2              list -> list
/// string tolower|toupper|totitle    list -> list   (also with indices)
/// string trim|trimleft|trimright    list -> list
/// string wordend|wordstart SUBJ 1   list -> list   (9.0 converts — see
///                                                   the registry note)
/// string cat SUBJ / SUBJ y          list -> list
/// ```
#[test]
fn dual_ported_string_subcommands_stay_silent_for_list_subject() {
    for call in [
        "string compare $s y",
        "string compare y $s",
        "string equal $s y",
        "string match p* $s",
        "string match $s y",
        "string repeat $s 2",
        "string tolower $s",
        "string toupper $s 0 1",
        "string totitle $s",
        "string trim $s",
        "string trimleft $s x",
        "string trimright $s",
        "string wordend $s 1",
        "string wordstart $s 1",
        "string cat $s y",
    ] {
        let warnings = warnings_for(&format!("set s [list 1 2 3]\n{call}"));
        assert!(
            warnings.iter().all(|w| w.variable != "s"),
            "{call}: dual-ported read keeps the list intrep — silent: {warnings:?}"
        );
    }
}

/// TN control: an alias to a *non*-shimmering command (`puts`) must not
/// spuriously fire just because the alias itself resolved.
#[test]
fn no_shimmer_through_interp_alias_to_non_shimmering_command() {
    let cu = CompilationUnit::build_for(
        "interp alias {} say {} ::puts\nset x hello\nsay $x",
        &registry(),
        false,
    );
    let fu = cu.function("::top").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    assert!(
        warnings.iter().all(|w| w.variable != "x"),
        "unexpected shimmer through non-shimmering alias: {warnings:?}"
    );
}

/// `regexp -all -inline` returns the matched substrings as a
/// list, so iterating the result converts nothing.  A trimmed report
/// shape: it drew "variable 'vinfo' has int intrep but 'foreach'
/// expects list (argument 1)".
#[test]
fn no_shimmer_iterating_a_regexp_inline_result() {
    let src = "proc f {s} {\n\
                   set vinfo [regexp -all -inline {(?:^|\\s)(\\d+)(?:\\.(\\d+))?} $s]\n\
                   foreach {match v1 v2} $vinfo { lappend out $v1$v2 }\n\
                   }";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let on_vinfo: Vec<_> = warnings.iter().filter(|w| w.variable == "vinfo").collect();
    assert!(
        on_vinfo.is_empty(),
        "regexp -inline returns a list — iterating it is not a shimmer: {on_vinfo:?}"
    );
}

/// The control for the case above: a bare `regexp` really does return a
/// match flag, so consuming it as a list is a genuine conversion.  Proves
/// the per-form typing reads the switches rather than blanket-silencing
/// `regexp`.
#[test]
fn shimmer_still_detected_iterating_a_bare_regexp_result() {
    let src = "proc f {s} {\n\
                   set m [regexp {(\\d+)} $s]\n\
                   foreach v $m { lappend out $v }\n\
                   }";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let w = warnings.iter().find(|w| w.variable == "m");
    assert!(
        w.is_some(),
        "an int match flag used as a list must still shimmer: {warnings:?}"
    );
    assert_eq!(w.unwrap().from_type, TclType::Int);
    assert_eq!(w.unwrap().to_type, TclType::List);
}

/// The same defect on `lsearch`: `-all` returns the list of matching
/// indices, not one index.
#[test]
fn no_shimmer_iterating_an_lsearch_all_result() {
    let src = "proc f {l} {\n\
                   set hits [lsearch -all $l a]\n\
                   foreach i $hits { lappend out $i }\n\
                   }";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let on_hits: Vec<_> = warnings.iter().filter(|w| w.variable == "hits").collect();
    assert!(
        on_hits.is_empty(),
        "lsearch -all returns a list of indices: {on_hits:?}"
    );
}

/// And on `regsub`: with no `varName` the result is the substituted
/// string, so measuring its length converts nothing.
#[test]
fn no_shimmer_measuring_a_varname_less_regsub_result() {
    let src = "proc f {s} {\n\
                   set out [regsub -all a $s b]\n\
                   set n [string length $out]\n\
                   }";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = cu.function("::f").unwrap();
    let warnings = use_site_shimmers(fu, &registry());
    let on_out: Vec<_> = warnings.iter().filter(|w| w.variable == "out").collect();
    assert!(
        on_out.is_empty(),
        "a varName-less regsub returns the substituted string: {on_out:?}"
    );
}
