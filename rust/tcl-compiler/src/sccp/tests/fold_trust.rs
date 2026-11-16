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

//! The fold arms answer to the command-binding trust fact: renames, shadowing procedures and unnameable subjects.

use super::*;

// The fold arms answer to the command-binding trust fact.

/// The whole-module mutation summary for `src`.
fn mutations_for(src: &str) -> crate::command_binding::ModuleCommandMutations {
    let reg = registry();
    let ir = crate::lowering::lower_to_ir(src, &reg);
    crate::command_binding::scan_module_command_mutations(&ir, &reg)
}

/// Evaluate `stmt` under the whole-module trust fact `mutations`.
fn evaluate_under_unit(
    stmt: &SsaStatement,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
) -> LatticeValue {
    evaluate_fixture_def_with_folds(
        stmt,
        &HashMap::new(),
        ssa,
        FoldPolicy::default(),
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry,
            mutations,
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::WholeModule,
            proven_pure_parameters: false,
        }),
        tcl_lexer::LexerConfig::default(),
    )
}

/// The `[list …]` / `[format …]` arms run ahead of the registry engine, so
/// they must apply its trust gate themselves — otherwise a unit that
/// renamed `list` still gets the builtin's answer.
///
/// tclsh 8.6.16 / 9.0.4 (identical): after `rename list mylist`, evaluating
/// `list a b c` raises rather than returning `a b c`.
#[test]
fn list_arm_declines_once_the_unit_renames_list() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[list a b c]", 1);

    let untouched = mutations_for("set y 1\n");
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &untouched),
        LatticeValue::Const(ConstValue::String("a b c".into())),
        "an untouched `list` still folds"
    );

    let renamed = mutations_for("rename list mylist\n");
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &renamed),
        LatticeValue::Overdefined,
        "a renamed `list` must not fold to the builtin's answer"
    );
}

/// The same gate, on an arm reached through the head-word check rather
/// than through its own early return — and per name: renaming `llength`
/// must not cost `list` its fold.
#[test]
fn head_word_arms_decline_only_for_the_name_the_unit_touched() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let count = assign_value_stmt(&mut ssa, "n", "[llength {a b c d}]", 1);
    let listing = assign_value_stmt(&mut ssa, "x", "[list a b c]", 2);

    let renamed = mutations_for("rename llength myll\n");
    assert_eq!(
        evaluate_under_unit(&count, &ssa, &reg, &renamed),
        LatticeValue::Overdefined,
        "a renamed `llength` must not fold to the builtin's answer"
    );
    assert_eq!(
        evaluate_under_unit(&listing, &ssa, &reg, &renamed),
        LatticeValue::Const(ConstValue::String("a b c".into())),
        "`list` is untouched by a `rename llength` and keeps its fold"
    );
}

/// Evaluate `stmt` under `mutations` with the shared per-unit lattice's
/// stance ([`FoldTrust::ObservedBindings`]).
fn evaluate_under_lattice_stance(
    stmt: &SsaStatement,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
) -> LatticeValue {
    evaluate_fixture_def_with_folds(
        stmt,
        &HashMap::new(),
        ssa,
        FoldPolicy::default(),
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry,
            mutations,
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
        tcl_lexer::LexerConfig::default(),
    )
}

/// A user `proc` shadowing a builtin is a *named* takeover, so both
/// stances decline it. This is the root of #2164: the shared per-unit
/// lattice took no trust fact at all and answered `3` for a
/// `[llength {a b c}]` the module resolves to a proc returning 99, which
/// the optimiser then published as `return 3` beside its own `set v 99`.
///
/// tclsh 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1b0 (unanimous): with
/// `proc llength {l} {return 99}` in scope, `[llength {a b c}]` is 99.
#[test]
fn both_stances_decline_a_builtin_a_proc_shadows() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    let shadowed = mutations_for("proc llength {l} { return 99 }\n");
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &shadowed),
        LatticeValue::Overdefined,
        "the lattice must not answer with builtin semantics for a shadowed name"
    );
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &shadowed),
        LatticeValue::Overdefined,
        "nor may the rewrite stance"
    );

    // Positive control: the same statement, the same two stances, a module
    // that shadows nothing — both fold, so the declines above are the
    // shadow's doing and not a dead arm.
    let untouched = mutations_for("set y 1\n");
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &untouched),
        LatticeValue::Const(ConstValue::Int(3)),
    );
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &untouched),
        LatticeValue::Const(ConstValue::Int(3)),
    );
}

/// Where the two stances deliberately differ: a command head an autoload
/// index in view may define raises the summary's unbounded `dynamic` top,
/// which names no subject. The rewrite stance declines on it (a rewrite
/// must be certain); the lattice stance keeps folding, or a single
/// unknown library call would cost a file every constant it has.
#[test]
fn only_the_rewrite_stance_declines_on_the_unbounded_top() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    let opaque = mutations_for("someUnknownLibraryCall x\n");
    assert!(
        opaque.has_dynamic_mutation(),
        "an unresolved head is expected to raise the unbounded top"
    );
    assert!(
        opaque.observed_binding_is_the_builtin("llength"),
        "but it names no subject, so `llength`'s observed binding is intact"
    );
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &opaque),
        LatticeValue::Const(ConstValue::Int(3)),
    );
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &opaque),
        LatticeValue::Overdefined,
        "a fold that becomes a rewrite declines on the unbounded top"
    );
}

/// A `rename` whose **subject** the scan cannot name distrusts every
/// builtin, where an unresolved command *head* does not (#2168).
///
/// Both raise the unbounded top, which is why gating the lattice on the
/// whole of it was rejected in #2164 — it would take the fold in
/// [`only_the_rewrite_stance_declines_on_the_unbounded_top`] with it. The
/// distinction is whether something was definitely rebound: `rename $a {}`
/// moved *some* command, so no name is claimable; `someUnknownLibraryCall
/// x` moved nothing.
///
/// tclsh 8.6.18 and 9.0.4 both print 99 for the repro on #2168, where the
/// optimiser rewrote the body to `return 3` — and its own output literally
/// read `rename llength {}`, having constant-propagated the operands in
/// the same run.
#[test]
fn an_unnameable_rename_subject_distrusts_where_an_unknown_head_does_not() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    let computed = mutations_for(
        "set a llength
rename $a {}
",
    );
    assert!(
        computed.has_dynamic_mutation(),
        "a computed rename raises the unbounded top"
    );
    assert!(
        !computed.observed_binding_is_the_builtin("llength"),
        "and, unlike an unknown head, it leaves no name claimable"
    );
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &computed),
        LatticeValue::Overdefined,
        "the lattice must not fold a builtin a computed rename may have moved"
    );

    // Positive control, and the coverage #2164 measured: an unresolved
    // head still raises the top yet still folds, so this change cannot
    // have been bought by gating on `dynamic`.
    let opaque = mutations_for(
        "someUnknownLibraryCall x
",
    );
    assert!(opaque.has_dynamic_mutation());
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &opaque),
        LatticeValue::Const(ConstValue::Int(3)),
    );
}

/// The same rule for a body the source-recursive walk never sees. A proc
/// installed through an alias prefix is recovered only by the closed
/// command lattice, so its unnameable rename subject reaches the optimiser
/// through `mutation_projection` or not at all (#2168).
///
/// Measured end to end before this was joined: the program below prints
/// **99**, and `tcl optimise` rewrote it into one printing **3** on tclsh
/// 8.6.18 and 9.0.4.
#[test]
fn an_unnameable_rename_subject_inside_an_alias_defined_body_also_distrusts() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    // `makep {} BODY` is `proc p {} BODY`; BODY is never walked as source.
    let aliased = mutations_for(
        "proc mylen {l} { return 99 }
interp alias {} makep {} proc p
makep {} {set a llength; set b mylen; rename $a {}; rename $b $a}
p
",
    );
    assert!(
        !aliased.observed_binding_is_the_builtin("llength"),
        "the projection is the only witness that the subject was unnameable"
    );
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &aliased),
        LatticeValue::Overdefined,
        "folding here rewrites a program meaning 99 into one meaning 3"
    );
}
