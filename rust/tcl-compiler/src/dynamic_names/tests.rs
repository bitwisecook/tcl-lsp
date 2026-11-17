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

/// These cases carry no `{`/`}`/`\\` in a `${…}` name, so both release
/// rules answer alike; pin them to the default document's rule.
fn dynamic_variable_word_can_spell(word: &str, cell: &str) -> bool {
    super::dynamic_variable_word_can_spell(word, cell, tcl_dialect::BracedVarStyle::default())
}

fn barrier_for(src: &str) -> DynamicNameBarrier {
    barrier_for_dialect(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

/// The barrier for `src` analysed **as** `dialect` — registry, lexer
/// config, and expression grammar all that dialect's, which is the
/// configuration a real host builds.
fn barrier_for_dialect(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> DynamicNameBarrier {
    let registry = tcl_registry::model::ingress::static_context_for_profile(dialect).commands();
    let cu =
        crate::compilation_unit::CompilationUnit::build_for_profile(src, registry, false, dialect);
    let fu = cu.procedures.values().next().unwrap_or(&cu.top_level);
    dynamic_name_barrier(&fu.cfg, registry, lexer_config_for(registry))
}

#[test]
fn literal_name_patterns_keep_closed_element_and_open_root_distinct() {
    // naming.core.original-dynamic-name-value-purpose
    // docs/design/analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md
    // Explicit authored-C lexical compatibility, not a physical name lookup.
    assert!(!names_a_dynamic_variable("a($key)"));
    assert!(names_a_dynamic_variable("a($key"));
    assert!(names_a_dynamic_variable("a($key)tail"));
    assert!(names_a_dynamic_variable("${prefix}::v($key"));
    assert!(dynamic_variable_word_can_spell("a($key", "::ns::a(TAIL"));
    assert!(!dynamic_variable_word_can_spell("a($key)", "::ns::a(TAIL"));
}

#[test]
fn supplied_name_patterns_keep_variable_root_and_namespace_name_purposes() {
    // naming.core.original-dynamic-name-value-purpose
    // docs/design/analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md
    // Conditional Logical source coordinates, independent of target/edit identity.
    use crate::analyser::Analyser;
    use crate::registry_invocation::InvocationMetadataContext;
    use SourceNamePatternPurpose::{NamespaceName, VariableRoot};
    let analysis = Analyser::new().analyse("set value 1", "tcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let metadata = InvocationMetadataContext::for_source_input(
        input.borrowed_context_registry().commands(),
        input,
        input.lexer_config(),
        Some(input.unit_profile()),
    );
    for purpose in [VariableRoot, NamespaceName] {
        assert_eq!(
            source_name_pattern_can_spell("a:::$x:b", "::a::b", purpose, metadata),
            Some(true)
        );
        assert_eq!(
            source_name_pattern_can_spell("a:::$x:b", "::a::OTHER", purpose, metadata),
            Some(false)
        );
    }
    assert_eq!(
        source_name_word_is_dynamic("a($key)", VariableRoot, metadata),
        Some(false)
    );
    assert_eq!(
        source_name_word_is_dynamic("a($key", VariableRoot, metadata),
        Some(true)
    );
    assert_eq!(
        source_name_word_is_dynamic("a($key)", NamespaceName, metadata),
        Some(true)
    );
    for (word, coordinate, expected) in [
        ("::other::$n", "::ns::v", false),
        ("${prefix}::v", "::ns::v", true),
        ("a($key", "::ns::a(TAIL", true),
        ("a($key)", "::ns::a(TAIL", false),
        ("v", "::ns::v", true),
    ] {
        assert_eq!(
            source_name_pattern_can_spell(word, coordinate, VariableRoot, metadata),
            Some(expected),
            "{word}"
        );
    }
    assert_eq!(
        source_name_pattern_can_spell("::ns::a($key)", "::ns::a(TAIL)", NamespaceName, metadata),
        Some(true)
    );
    assert_eq!(
        source_name_pattern_can_spell("::ns::a($key)", "::ns::a(TAIL)", VariableRoot, metadata),
        Some(false)
    );
}

#[test]
fn supplied_name_patterns_do_not_borrow_native_hosted_or_display_identity() {
    // naming.core.original-dynamic-name-value-purpose
    // docs/design/analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md
    use crate::analyser::Analyser;
    use crate::registry_invocation::InvocationMetadataContext;
    use tcl_registry::model::semantic::SemanticContext;
    let purpose = SourceNamePatternPurpose::VariableRoot;
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
    ] {
        let analysis = Analyser::new().analyse("set value 1", dialect);
        let input = analysis.resolved_input.as_ref().unwrap();
        let metadata = InvocationMetadataContext::for_source_input(
            input.borrowed_context_registry().commands(),
            input,
            input.lexer_config(),
            Some(input.unit_profile()),
        );
        assert!(
            metadata.is_some(),
            "actual metadata remains independently available: {dialect}"
        );
        assert_eq!(
            source_name_word_is_dynamic("a($key)", purpose, metadata),
            None,
            "{dialect}"
        );
        assert_eq!(
            source_name_pattern_can_spell("::other::$n", "::ns::v", purpose, metadata),
            None,
            "{dialect}"
        );
    }
    let analysis = Analyser::new().analyse("set value 1", "tcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let supplied_availability = input.borrowed_context_registry().into();
    let standalone = SemanticContext::for_profile(input.unit_profile()).into();
    for metadata in [None, Some(supplied_availability), Some(standalone)] {
        assert_eq!(
            source_name_word_is_dynamic("a($key)", purpose, metadata),
            None
        );
        assert_eq!(
            source_name_pattern_can_spell("::other::$n", "::ns::v", purpose, metadata),
            None
        );
        assert_eq!(
            source_name_pattern_can_spell("v", "::captured::display", purpose, metadata),
            None
        );
    }
    let foreign = tcl_registry::CommandRegistry::build_default();
    assert!(
        InvocationMetadataContext::for_source_input(
            &foreign,
            input,
            input.lexer_config(),
            Some(input.unit_profile()),
        )
        .is_none()
    );
}

#[test]
fn supplied_logical_name_patterns_require_an_actual_authored_c_policy() {
    // naming.core.original-dynamic-name-value-purpose
    // docs/design/analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md
    use crate::analyser::ResolvedAnalysisInput;
    use crate::registry_invocation::InvocationMetadataContext;
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "explicit-pattern-logical-grammar",
        &[],
        "Logical grammar only",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )
    .intern();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        LexerConfig::for_profile(Some(profile)),
    );
    assert!(input.has_logical_source_name_context());
    assert!(
        tcl_registry::InvocationDialect::of_profile(profile)
            .authored_name_policy()
            .is_none()
    );
    let metadata = InvocationMetadataContext::for_source_input(
        input.borrowed_context_registry().commands(),
        &input,
        input.lexer_config(),
        Some(profile),
    );
    assert!(metadata.is_some());
    assert_eq!(
        source_name_pattern_can_spell(
            "::other::$n",
            "::ns::v",
            SourceNamePatternPurpose::VariableRoot,
            metadata
        ),
        None
    );
}

#[test]
fn original_substitution_name_effects_keep_native_child_receipts() {
    assert!(barrier_for("set x [list]; lappend x a; puts $x").is_clear());
    assert!(barrier_for("set x {[set $name 2]}; puts $x").is_clear());
    assert!(barrier_for("set {name[} DATA; puts ${name[}").is_clear());
    assert!(barrier_for("puts [list [set $name 2]]").writes);
    assert!(barrier_for("set x [unknown_child]").reads);
    assert!(barrier_for("set x $a([set $name 2])").writes);
}

#[test]
fn braced_substitution_is_dynamic_plain_name_is_not() {
    // `${x}` is a substitution, not a literal name — tclsh 9.0.4 / 8.6.14:
    // `set x foo; set ${x} bar; info exists foo` → 1.
    assert!(names_a_dynamic_variable("${x}"));
    assert!(!names_a_dynamic_variable("plain"));
    assert!(!names_a_dynamic_variable(""));
}

#[test]
fn qualified_name_with_a_dynamic_namespace_binds_a_static_tail() {
    // `variable ${name}::graphAttr` creates the local `graphAttr`
    // whatever `$name` holds, so no other local becomes reachable.
    assert!(!names_a_dynamic_variable("${name}::graphAttr"));
    assert!(names_a_dynamic_variable("::ns::${tail}"));
}

#[test]
fn array_element_with_dynamic_key_is_not_a_name_barrier() {
    // The base array `a` is statically named; only the element key is
    // computed, which the place model already tracks.
    assert!(!names_a_dynamic_variable("a($k)"));
    assert!(names_a_dynamic_variable("$a(k)"));
}

#[test]
fn substituted_and_bracketed_names_are_dynamic() {
    assert!(names_a_dynamic_variable("$x"));
    assert!(names_a_dynamic_variable("[string trim $x]"));
    assert!(names_a_dynamic_variable("pre$x"));
}

// Per-site provenance: which cells a dynamic name word can actually
// spell.  `true` is the abstention (could reach it), `false` is the proof
// it cannot.

/// TP (must stay refused): a bare substitution is a lone wildcard, so it
/// reaches every spelling of every cell.
#[test]
fn a_bare_substitution_can_spell_any_cell() {
    for word in ["$n", "${n}", "[pick]"] {
        assert!(dynamic_variable_word_can_spell(word, "::ns::v"), "{word}");
        assert!(
            dynamic_variable_word_can_spell(word, "::a::b::deep"),
            "{word}"
        );
    }
}

/// TN: a written namespace prefix confines the name — a substitution
/// cannot escape it.
///
/// tclsh-proof (8.6.14): `namespace eval ::ns {variable v 1}; namespace
/// eval ::other {}; set n {::ns::v}; set ::other::$n 99` fails with
/// `can't set "::other::::ns::v": parent namespace doesn't exist` — the
/// value is appended under the prefix, never resolved as an absolute
/// name.  `set ::other::$n` with `n` = `sub::x` writes `::other::sub::x`.
#[test]
fn a_written_namespace_prefix_confines_the_name() {
    assert!(!dynamic_variable_word_can_spell("::other::$n", "::ns::v"));
    assert!(!dynamic_variable_word_can_spell("::other::${n}", "::ns::v"));
    // …and still reaches cells that really are under it, at any depth.
    assert!(dynamic_variable_word_can_spell("::other::$n", "::other::v"));
    assert!(dynamic_variable_word_can_spell(
        "::other::$n",
        "::other::sub::x"
    ));
}

/// TN: a written affix constrains the name the same way.
///
/// tclsh-proof (8.6.14): `namespace eval ::ns {variable total 5}; set j
/// 1; set v$j 2` leaves `info vars ::v*` = `::v1` and `::ns::total`
/// untouched.
#[test]
fn a_written_affix_constrains_the_name() {
    assert!(!dynamic_variable_word_can_spell("v$j", "::ns::total"));
    // `${j}_count`, not `$j_count` — the latter is the single variable
    // `j_count`, so it is one wildcard with no written affix at all.
    assert!(!dynamic_variable_word_can_spell(
        "${j}_count",
        "::ns::total"
    ));
    assert!(dynamic_variable_word_can_spell("$j_count", "::ns::total"));
    // TP guard: the affix is satisfiable, so the abstention stands — an
    // empty substitution collapses (`set i {}; set v$i 1` -> `info exists
    // v` is 1 on 8.6.14).
    assert!(dynamic_variable_word_can_spell("v$j", "::ns::v"));
    assert!(dynamic_variable_word_can_spell("v$j", "::ns::verbose"));
    assert!(dynamic_variable_word_can_spell("$j", "::ns::total"));
}

/// TN: a *relative* prefix is a prefix too — `b::$m` names something
/// under some `…::b`, so a cell with no `b` segment is out of reach.
#[test]
fn a_relative_prefix_still_excludes_unrelated_cells() {
    assert!(!dynamic_variable_word_can_spell("b::$m", "::ns::v"));
    assert!(dynamic_variable_word_can_spell("b::$m", "::a::b::tail"));
}

/// The relative spellings count: a cell written from inside its own
/// namespace is a bare tail, so a wildcard-tail word reaches it.
#[test]
fn every_relative_spelling_of_the_cell_is_considered() {
    assert!(dynamic_variable_word_can_spell("ns::$t", "::ns::v"));
    assert!(dynamic_variable_word_can_spell("$t", "::ns::v"));
    assert!(!dynamic_variable_word_can_spell("nz::$t", "::ns::v"));
}

/// An element suffix is not part of the variable name.
#[test]
fn the_element_suffix_is_not_part_of_the_name() {
    assert!(dynamic_variable_word_can_spell("$a(k)", "::ns::v"));
    assert!(!dynamic_variable_word_can_spell(
        "::other::$a(k)",
        "::ns::v"
    ));
}

#[test]
fn plain_proc_has_no_barrier() {
    assert!(barrier_for("proc f {} { set a 1; return $a }\n").is_clear());
}

#[test]
fn dynamic_set_sets_the_write_flag_only() {
    let b = barrier_for("proc f {n} { set $n 1; return ok }\n");
    assert!(b.writes, "`set $n 1` is a dynamic write");
    assert!(!b.reads);
    assert!(!b.destroys);
}

#[test]
fn fallback_role_scan_distinguishes_unknown_options_from_ordinary_values() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let config = LexerConfig::from_grammar(profile.grammar);
    let declared = DeclaredFrameEffects::default();
    let commands = Commands::standalone(registry, &declared);
    let scan = |command, arguments: &[&str], braced: &[bool]| {
        let arguments: Vec<_> = arguments.iter().map(|word| (*word).to_owned()).collect();
        let mut barrier = DynamicNameBarrier::default();
        scan_command(
            command,
            &CommandWords {
                args: &arguments,
                braced: Some(braced),
            },
            commands,
            &mut barrier,
            (0, config),
        );
        barrier
    };
    assert_eq!(
        scan(
            "regsub",
            &["$option", "x", "x", "value", "target"],
            &[false; 5]
        ),
        DynamicNameBarrier::OPAQUE_SCRIPT,
        "a computed switch cannot be treated as literal source spelling"
    );
    let ordinary = scan("set", &["$name", "$value"], &[false; 2]);
    assert!(ordinary.writes);
    assert!(!ordinary.reads && !ordinary.destroys);
    assert!(scan("set", &["$name", "$value"], &[true, false]).is_clear());
    let mut quoted_level = DynamicNameBarrier::default();
    scan_script_text("uplevel {0} $body", commands, &mut quoted_level, 0, config);
    assert_eq!(
        quoted_level,
        DynamicNameBarrier::OPAQUE_SCRIPT,
        "the quoted zero still selects the current frame"
    );
    let mut quoted_option = DynamicNameBarrier::default();
    scan_script_text(
        "regsub {-nocase} x x value target",
        commands,
        &mut quoted_option,
        0,
        config,
    );
    assert!(quoted_option.is_clear(), "{quoted_option:?}");
}

#[test]
fn dynamic_one_arg_set_sets_the_read_flag_only() {
    let b = barrier_for("proc f {n} { return [set $n] }\n");
    assert!(b.reads, "`[set $n]` is a dynamic read");
    assert!(!b.writes);
    assert!(!b.destroys);
}

#[test]
fn dynamic_unset_sets_the_destroy_flag_only() {
    let b = barrier_for("proc f {n} { unset $n; return ok }\n");
    assert!(b.destroys, "`unset $n` is a dynamic destroy");
    assert!(!b.writes);
}

#[test]
fn dynamic_subst_template_sets_the_read_flag() {
    let b = barrier_for("proc f {t} { return [subst $t] }\n");
    assert!(b.reads, "`subst $t` can dereference any name");
}

/// The barrier reads the call's template-word plan: a computed
/// template reads any name only when variable or command substitution
/// runs over it, so `subst -nocommands -novariables $t` no longer blinds
/// every read, while `subst -novariables $t` still does — its `[set x]`
/// reads `x` (tclsh 8.4 to 9.1: `proc f {t} {set x 1; return [subst
/// -novariables $t]}; f {[set x]}` is `1`, and removing the `set` as a
/// dead store makes it raise) — as `subst -nocommands $t` and `subst $t`
/// do; a braced template's `[…]` region is script in this frame and is
/// scanned as such.
#[test]
fn a_computed_template_blinds_reads_while_it_substitutes() {
    let off = barrier_for("proc f {t} { return [subst -nocommands -novariables $t] }\n");
    assert!(!off.reads, "no variable of the template is read");
    assert!(barrier_for("proc f {t} { return [subst -novariables $t] }\n").reads);
    assert!(barrier_for("proc f {t} { return [subst -nocommands $t] }\n").reads);
    assert!(barrier_for("proc f {t} { return [subst $t] }\n").reads);
    assert!(
        barrier_for("proc f {n} { return [subst {x[set $n]}] }\n").reads,
        "the region's own dynamic read"
    );
}

#[test]
fn literal_subst_template_is_not_a_barrier() {
    let b = barrier_for("proc f {} { set a 1; return [subst {$a}] }\n");
    assert!(b.is_clear(), "a literal template names its reads");
}

#[test]
fn dynamic_array_set_is_a_write_barrier() {
    let b = barrier_for("proc f {n} { array set $n {x 1} }\n");
    assert!(b.writes);
    let literal = barrier_for("proc f {} { array set table {x 1} }");
    assert!(
        !literal.writes,
        "the selected handler's literal destination does not blind every name"
    );
}

#[test]
fn a_captured_literal_name_is_not_a_source_substitution() {
    let literal = barrier_for("interp alias {} assign {} set {$n}\nproc f {} {assign VALUE}");
    assert!(
        !literal.writes,
        "captured $n is a literal variable name: {literal:?}"
    );
    let dynamic = barrier_for("proc f {n} {set $n VALUE}");
    assert!(dynamic.writes, "the original dynamic name remains a hazard");
}

#[test]
fn a_computed_head_uses_its_retained_handler_for_name_hazards() {
    let barrier = barrier_for("proc f {name} {set operation array; $operation set $name {x 1}}");
    assert!(barrier.writes);
    assert!(!barrier.destroys);
    let unrelated = barrier_for("proc f {array name} {$array set $name {x 1}}");
    assert!(
        !unrelated.writes,
        "a variable name cannot donate array roles"
    );
}

#[test]
fn dynamic_name_nested_in_a_command_substitution_is_seen() {
    let b = barrier_for("proc f {n} { set out {}; lappend out [set $n]; return $out }\n");
    assert!(b.reads, "the dynamic read sits inside a `[…]` argument");
}

#[test]
fn dynamic_name_in_a_branch_condition_is_seen() {
    let b = barrier_for("proc f {n} { if {[set $n] eq {}} { return empty }; return full }\n");
    assert!(b.reads);
}

// A brace-quoted name is a *literal* name.
//
// tclsh 9.0.4 / 8.6.14 (identical):
//   set {$n} v; info exists {$n} → 1 ; info exists n → 0
//   set i 5; set {arr($i)} 1; array names arr → {$i} ; info exists arr(5) → 0

#[test]
fn brace_quoted_write_target_is_not_a_barrier() {
    let b = barrier_for("proc f {} { set {$n} 1; return ok }\n");
    assert!(
        b.is_clear(),
        "`set {{$n}} 1` names the literal variable `$n`, so nothing is \
computed; got {b:?}"
    );
}

#[test]
fn brace_quoted_read_target_is_not_a_barrier() {
    let b = barrier_for("proc f {} { return [set {$n}] }\n");
    assert!(b.is_clear(), "got {b:?}");
}

#[test]
fn brace_quoted_destroy_target_is_not_a_barrier() {
    let b = barrier_for("proc f {} { unset {$n}; return ok }\n");
    assert!(b.is_clear(), "got {b:?}");
}

#[test]
fn unbraced_substituted_target_still_raises_the_write_flag() {
    // TN control for the three above: dropping the braces restores the
    // barrier, so the brace check cannot be over-applied.
    let b = barrier_for("proc f {n} { set $n 1; return ok }\n");
    assert!(b.writes, "`set $n 1` is still a dynamic write; got {b:?}");
}

// A computed command head is an unknown command.

#[test]
fn substituted_command_head_raises_no_flag() {
    // The head's content spelling is `set`, but `string length foo` is
    // what runs. Resolving the lookalike would answer for a command that
    // never executes, so the call contributes nothing — see the module
    // docs for why it raises no flag either.
    let b = barrier_for("proc f {set n} { $set $n 1; return ok }\n");
    assert!(
        b.is_clear(),
        "a computed head is an unknown command, not a computed name; got {b:?}"
    );
}

#[test]
fn literal_head_with_a_computed_name_still_raises_the_write_flag() {
    // TN control: the same argument shape under a *literal* head is the
    // genuine dynamic write.
    let b = barrier_for("proc f {n} { set $n 1; return ok }\n");
    assert!(b.writes, "got {b:?}");
}

// The barrier is computed under the *document's* dialect.
//
// A tokenisation pinned to `LexerConfig::default()` would see word
// boundaries the IR was not built from under a non-default dialect.  The
// barrier is an optimisation-soundness fact — SCCP and every value-motion
// pass abstain on it — so a boundary disagreement that hides a dynamic
// write lets a constant propagate across one.

/// The two shapes the dialects disagree about, as facts about the
/// splitter now shared with the lowering.
#[test]
fn word_splitting_follows_the_dialect() {
    let words = |src: &str, dialect: &'static tcl_dialect::DialectProfile| -> Vec<Vec<String>> {
        crate::ir_helpers::tokenise_command_words(
            src,
            tcl_lexer::LexerConfig::from_grammar(dialect.grammar),
        )
        .into_iter()
        .map(|command| command.into_iter().map(|word| word.text).collect())
        .collect()
    };

    // iRules treats `}{` as a word separator; Tcl concatenates the two
    // braced groups into one word.
    assert_eq!(
        words("cmd {a}{b}", tcl_dialect::DialectProfile::irules()),
        vec![vec!["cmd", "a", "b"]],
        "an iRule splits `{{a}}{{b}}` into two words",
    );
    assert_eq!(
        words(
            "cmd {a}{b}",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
        ),
        vec![vec!["cmd", "ab"]],
        "Tcl welds them into one",
    );

    // `{*}` expands only where the dialect has it — never in 8.4 or an
    // iRule, where it is an ordinary (literal) word.
    assert_eq!(
        words(
            "cmd {*}$args x",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
        ),
        vec![vec!["cmd", "${args}", "x"]],
        "8.5+ reads `{{*}}` as the expansion marker",
    );
    assert_eq!(
        words(
            "cmd {*}$args x",
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile()
        ),
        vec![vec!["cmd", "*${args}", "x"]],
        "tcl8.4 has no `{{*}}` expansion",
    );
    // F5 reclassification (measurements §1/§3 row 6,
    // `docs/design/f5/bigip-irule-parser-measurements.md`): on the F5
    // fork the implicit word break wins over everything — `{*}` is
    // the literal `*` word plus the *separate* unexpanded word
    // (measured `list {*}{a b}` → `* {a b}` on TMM), never the old
    // welded `*${args}` single word and never an expansion.
    assert_eq!(
        words("cmd {*}$args x", tcl_dialect::DialectProfile::irules()),
        vec![vec!["cmd", "*", "${args}", "x"]],
        "the F5 word break splits after the literal `*`",
    );
}

/// The per-word facts `scan_command` reads, as the segmenter now supplies
/// them: the written spelling (which carries the name-position `$`), the
/// brace-literal flag (which says a `$` is *not* a substitution), and the
/// substitution flag (which disqualifies a computed command head).
#[test]
fn word_facts_survive_the_segmenter_mapping() {
    let words = crate::ir_helpers::tokenise_command_words(
        "set $x {a $b}",
        tcl_lexer::LexerConfig::default(),
    )
    .remove(0);
    assert_eq!(
        words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
        vec!["set", "${x}", "a $b"],
    );
    assert_eq!(
        words.iter().map(|w| w.raw.as_str()).collect::<Vec<_>>(),
        vec!["set", "$x", "{a $b}"],
        "the written span keeps both delimiters (`CommandWord::raw`)",
    );
    assert_eq!(
        words.iter().map(|w| w.substituted).collect::<Vec<_>>(),
        vec![false, true, false],
    );
    assert_eq!(
        words.iter().map(|w| w.braced_literal).collect::<Vec<_>>(),
        vec![false, false, true],
    );

    // A compound word substitutes as a whole and is not a brace literal,
    // whichever fragment carries the `$`.
    for src in ["cmd a$b", "cmd {a}$b", "cmd $b-a"] {
        let word =
            crate::ir_helpers::tokenise_command_words(src, tcl_lexer::LexerConfig::default())
                .remove(0)
                .remove(1);
        assert!(word.substituted, "{src}");
        assert!(!word.braced_literal, "{src}");
    }
}

/// The soundness pin: a dynamic write inside a shape whose word
/// boundaries are dialect-dependent still raises the write flag.
#[test]
fn a_dynamic_write_raises_the_barrier_under_every_dialect() {
    for dialect in ["tcl8.6", "tcl8.4", "f5-irules"] {
        let b = barrier_for_dialect(
            "proc f {n} { set $n 1; return ok }\n",
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        assert!(b.writes, "`set $n 1` is a dynamic write under {dialect}");

        // …and through a `[…]` substitution, which is where the barrier
        // walk re-splits script text itself.
        let b = barrier_for_dialect(
            "proc f {n} { set out {}; lappend out [set $n 1]; return $out }\n",
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        assert!(b.writes, "nested dynamic write missed under {dialect}");
    }
}

/// The dialect-dependent word shapes themselves: a `}{` weld (which only
/// an iRule splits) and a `{*}` (which only 8.5+ expands), each wrapped
/// around a nested dynamic write.  Whichever way the words fall, the
/// write inside the `[…]` is still there and the barrier must find it —
/// this is the walk re-splitting script text under the document's own
/// config.
#[test]
fn dialect_specific_word_shapes_do_not_hide_a_dynamic_write() {
    for dialect in ["tcl8.6", "tcl8.4", "f5-irules"] {
        for body in [
            "set out [list {a}{b} [set $n 1]]",
            "set out [list {*}$n [set $n 1]]",
            "set out [concat {a}{b} {*}$n [set $n 1]]",
        ] {
            let b = barrier_for_dialect(
                &format!("proc f {{n}} {{ {body}; return $out }}\n"),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert!(b.writes, "{dialect}: `{body}` hides a dynamic write");
        }
    }
}

/// The discriminating case, and the reason the config is threaded at all:
/// a `{*}` welded to a braced name.  Under **tcl8.4** there is no
/// expansion, so the word is the computed name `*$n` and the `[…]` holds a
/// dynamic write; read under the default (8.5+) grammar the very same text
/// is an expansion marker plus the brace-literal name `{$n}`, which is
/// *static* — so a barrier computed from a differently-configured
/// tokenisation reports "no dynamic write" for a script that has one, and
/// SCCP then propagates constants across it.
#[test]
fn an_84_expansionless_name_word_is_still_a_dynamic_write() {
    let b = barrier_for_dialect(
        "proc f {n} { set out [list [set {*}{$n} 1]]; return $out }\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    assert!(
        b.writes,
        "`{{*}}{{$n}}` names `*$n` under 8.4 — a computed name; got {b:?}",
    );
}

/// TN control: a brace-literal name is static under every dialect, so the
/// dialect threading cannot be over-applied into blanket blindness.
#[test]
fn a_literal_name_stays_clear_under_every_dialect() {
    for dialect in ["tcl8.6", "tcl8.4", "f5-irules"] {
        let b = barrier_for_dialect(
            "proc f {} { set {$n} 1; return ok }\n",
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        assert!(b.is_clear(), "{dialect}: got {b:?}");
    }
}

/// The same expansionless `{*}` word as a *statement* of its own, not
/// buried in a `[…]` the text walk re-segments.
///
/// Under 8.4 / iRules `set {*}$n 1` is the literal `*` welded to `$n`: a
/// computed name.  Specialising it to `AssignConst`, whose name is static
/// by contract, would keep [`scan_statement`] from looking at it and leave
/// the write barrier down — freeing the value-motion passes to move stores
/// across a write that can land on any name.
#[test]
fn an_expansionless_computed_name_statement_raises_the_write_barrier() {
    let b = barrier_for_dialect(
        "proc f {n} { set {*}$n 1; return ok }\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    assert!(
        b.writes,
        "tcl8.4: `set {{*}}$n 1` names `*$n` — a computed name; got {b:?}",
    );
    // F5 reclassification (measurements §1/§3 row 6): under the fork's
    // implicit word break `{*}$n` is TWO words — the literal `*` and
    // `$n` — so there is no welded computed name here any more; the
    // genuinely computed spelling still raises the barrier.
    let b = barrier_for_dialect(
        "proc f {n} { set x$n 1; return ok }\n",
        tcl_dialect::DialectProfile::irules(),
    );
    assert!(
        b.writes,
        "f5-irules: `set x$n 1` is a computed name; got {b:?}",
    );
}

/// The 9.0 grammar reaches the same answer by a different path: `{*}$n`
/// really is an expansion there, so the word is a plain `$n` substitution
/// and `set` is not specialised at all.  Pinned so the expansionless gate
/// cannot be credited with this result, or quietly change it.
#[test]
fn an_expanded_computed_name_statement_still_raises_the_write_barrier() {
    for dialect in ["tcl9.0", "tcl8.6"] {
        let b = barrier_for_dialect(
            "proc f {n} { set {*}$n 1; return ok }\n",
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        assert!(
            b.writes,
            "{dialect}: expanded `{{*}}$n` write missed; got {b:?}"
        );
    }
}

#[test]
fn an_expansion_after_a_fixed_name_does_not_make_the_name_dynamic() {
    for dialect in ["tcl9.0", "tcl8.6"] {
        for body in [
            "set fixed {*}$n",
            "set {*}{fixed} VALUE",
            "set fixed VALUE EXTRA",
        ] {
            let b = barrier_for_dialect(
                &format!("proc f {{n}} {{ {body}; return ok }}\n"),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert!(b.is_clear(), "{dialect}: `{body}` got {b:?}");
        }
    }
}

/// TN control: a fully spelled-out `set` names one variable, so
/// the gate must not blind a function that computes nothing.  The array
/// element with a computed *key* is the near miss — its array is named
/// statically, which is the line [`names_a_dynamic_variable`] draws.
#[test]
fn a_spelled_out_set_stays_clear_under_every_dialect() {
    for dialect in ["tcl9.0", "tcl8.6", "tcl8.4", "f5-irules"] {
        for body in ["set x 1", "set a($i) 1", "set x 1; set y $x"] {
            let b = barrier_for_dialect(
                &format!("proc f {{i}} {{ {body}; return ok }}\n"),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert!(b.is_clear(), "{dialect}: `{body}` got {b:?}");
        }
    }
}

/// `incr $n` is a computed name, not a literal target.  Specialising it
/// to `Statement::Incr { name: "${n}" }`, whose name is static by
/// contract, would keep [`scan_statement`]'s `Incr` arm — which reads no
/// word text at all — from looking at it and leave the write barrier down,
/// freeing the value-motion passes to move stores across a write that can
/// land on any name. Unlike the `{*}$n` shape, a bare `$n` substitutes
/// under every grammar, so all four dialects see the same computed name.
#[test]
fn incr_of_a_computed_name_raises_the_write_barrier() {
    for dialect in ["tcl8.4", "f5-irules", "tcl8.6", "tcl9.0"] {
        let b = barrier_for_dialect(
            "proc f {n} { incr $n; return ok }\n",
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        assert!(
            b.writes,
            "{dialect}: `incr $n` names `$n` — a computed name; got {b:?}",
        );
    }
}

/// TN control: a fully spelled-out `incr` names one variable,
/// and a computed array *element* (`incr a($i)`) is not a computed
/// *name* — the array is named statically, the same line
/// [`names_a_dynamic_variable`] draws for `set` — so neither shape may
/// blind a function that computes nothing.
#[test]
fn a_spelled_out_incr_stays_clear_under_every_dialect() {
    for dialect in ["tcl9.0", "tcl8.6", "tcl8.4", "f5-irules"] {
        for body in ["incr x", "incr a($i)", "incr x 1"] {
            let b = barrier_for_dialect(
                &format!("proc f {{i}} {{ set x 0; {body}; return ok }}\n"),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert!(b.is_clear(), "{dialect}: `{body}` got {b:?}");
        }
    }
}

/// The bracket-depth cap protects the native stack, but it
/// cannot certify that the unread suffix has no dynamic variable access.
/// The source mutation is deliberately under the cap boundary: replacing
/// the dynamic `set $name` with a literal `set x` must not make a bounded
/// scan claim either source is fully known.
#[test]
fn bracket_depth_exhaustion_fails_closed_for_dynamic_name_facts() {
    let nest = |leaf: &str| {
        let mut text = leaf.to_owned();
        for _ in 0..=MAX_BRACKET_TEXT_DEPTH.0 {
            text = format!("[list {text}]");
        }
        text
    };
    let dynamic = barrier_for(&format!(
        "proc f {{name}} {{ set sink {}; return ok }}\n",
        nest("[set $name 2]")
    ));
    let literal = barrier_for(&format!(
        "proc f {{}} {{ set sink {}; return ok }}\n",
        nest("[set x 2]")
    ));
    assert_eq!(dynamic, DynamicNameBarrier::OPAQUE_SCRIPT, "{dynamic:?}");
    assert_eq!(literal, DynamicNameBarrier::OPAQUE_SCRIPT, "{literal:?}");
}

/// Owner ruling, 17 November 2026: a computed variable name is a rename
/// barrier only when it could spell the old or the new cell. tclsh 8.4.20 and
/// 9.0.4: with n `::mypkg::version`, `mypkg::version` or `:::mypkg::version`,
/// `set ::other::$n 2` creates `::other::mypkg::version`; `namespace upvar $ns
/// counter local` with ns `::mypkg` links `counter`.
#[test]
fn a_computed_name_excludes_a_cell_only_by_its_written_prefix_or_tail() {
    let cells = ["::mypkg::version", "::mypkg::release"];
    for dialect in ["tcl8.4", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
        let config = LexerConfig::from_grammar(profile.grammar);
        for word in [
            "::other::$n",
            ":::other::$n",
            "::mypkg::sub::$n",
            "${ns}::counter",
            "${ns}:::counter",
            "[pick]::counter",
        ] {
            assert!(
                computed_variable_name_cannot_spell(word, &cells, config),
                "{dialect}: {word}"
            );
        }
        for word in [
            "$n",
            "::mypkg::$n",
            ":::mypkg::$n",
            "::my$n",
            "::mypkg::ver$n",
            "::mypkg::rel$n",
            "other::$n",
            "mypkg:::$n",
            "${ns}::version",
            "${ns}::release",
            "${x}a:::version",
            "::other:$n",
            "\"::other::$n\"",
            "{::other::$n}",
            "::other::version",
        ] {
            assert!(
                !computed_variable_name_cannot_spell(word, &cells, config),
                "{dialect}: {word} must stay a barrier"
            );
        }
        assert!(
            !computed_variable_name_cannot_spell("${ns}::counter", &["::mypkg::counter"], config),
            "{dialect}: the new name is checked too"
        );
    }
}
