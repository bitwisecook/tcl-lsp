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

//! Computed metaclass names: provenance through helpers and call sites, command tables, loops, namespace normalisers and variable command heads.

use super::*;

#[test]
fn a_computed_metaclass_name_resolves_through_a_helper_proved_at_its_call() {
    // TP — the same chain with the helper's first argument read in the
    // manufacturer's own frame. Each head (`NSNormalize`, `string`,
    // `regsub`) is the command its source position is proved to
    // dispatch to, so the name folds on a release profile.
    //
    // tclsh 8.6.18, 9.0.4, 9.1.0: after this source,
    // `info commands ::T::D::*` answers `::T::D::class`, and
    // `::T::D::class create ::T::W {method go {} {return went}}` then
    // `[[::T::W new] go]` answers `went`.
    let src = COMPUTED_METACLASS.replace("[uplevel 1 {namespace current}]", "[namespace current]");
    for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = analysis(&src, release);
        assert!(
            result.all_classes.contains_key("::T::D::class"),
            "{release}: got {:?}",
            result.all_classes.keys().collect::<Vec<_>>()
        );
    }
    // TN — once the helper is moved, its call position proves nothing.
    let moved = src.replace(
        "::T::mkdialect ::T::D\n",
        "rename ::T::NSNormalize ::T::MovedNormalize\n::T::mkdialect ::T::D\n",
    );
    assert!(
        !analysis(&moved, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );
}

#[test]
fn a_computed_metaclass_name_resolves_through_a_literal_call_site() {
    // TP — the literal argument proves `${ns}::class` is
    // `::T::D::class`, so the metaclass enters the factory index and
    // everything it later manufactures becomes visible.
    let result = analysis(COMPUTED_METACLASS, "tcl9.0");
    assert!(
        result.all_classes.contains_key("::T::D::class"),
        "expected ::T::D::class; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
    assert!(
        result.class_factories().contains_key("::T::D::class"),
        "the resolved metaclass must publish a factory"
    );
}

#[test]
fn disjoint_dynamic_command_table_names_preserve_provenance() {
    // TP/FP control: the alias target words retain the fixed
    // `::define::` fragment, so neither the earlier nor later mutation
    // can denote NSNormalize, string, or regsub. This is the command-table
    // shape used by tcllib's dialect factory.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    set method current_class\n",
            "    interp alias {} ${NSPACE}::define::$method {} puts\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
            "    interp alias {} ${NSPACE}::define::later {} puts\n",
        ),
    );
    assert!(
        analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "disjoint dynamic command names must not poison every identity"
    );
}

#[test]
fn dormant_dynamic_command_table_words_do_not_mutate_load_time_identity() {
    // TN control: declaring a proc does not execute its body. A broad
    // dynamic command-table target in that dormant body can therefore
    // coexist with the proved helper identity; the exact tcllib corpus
    // contains this shape in unrelated factory bodies.
    let src = COMPUTED_METACLASS.replace(
        "::T::mkdialect ::T::D\n",
        concat!(
            "proc ::T::dormant {name} { proc ${name} {} {} }\n",
            "::T::mkdialect ::T::D\n",
        ),
    );
    assert!(
        analysis(&src, "tcl9.0")
            .class_factories()
            .contains_key("::T::D::class"),
        "a dormant body must not mutate the load-time command table"
    );
}

#[test]
fn unrelated_multi_arm_writes_preserve_provenance() {
    // TN control: branch selection is unknown, but every arm writes only
    // `other`. The dominating NSPACE fact remains valid.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    if {$runtime} {set other a} else {set other b}\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        ),
    );
    assert!(
        analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "a branch writing an unrelated local must preserve NSPACE"
    );
}

#[test]
fn a_multi_arm_write_to_the_provenance_variable_abstains() {
    // FP guard: either arm may replace the computed name.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    if {$runtime} {set NSPACE ::Bad} else {set other b}\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        ),
    );
    assert!(
        !analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "a branch write to NSPACE must invalidate its provenance"
    );
}

#[test]
fn an_unknown_same_frame_branch_write_abstains() {
    // FP guard: eval's registry-declared same-frame body may write any
    // local, so a dynamic script invalidates NSPACE.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    if {$runtime} {eval $script}\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        ),
    );
    assert!(
        !analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "an unknown same-frame write must invalidate provenance"
    );
}

#[test]
pub(super) fn a_collection_loop_in_the_dominating_prefix_preserves_provenance() {
    // TP — `foreach` iterates a value the invocation itself
    // supplies, so it runs a bounded number of times and control reaches
    // the creation below it exactly when the loop body falls through.
    // Nothing is read *out* of the loop: the body writes only its own
    // iteration variable and an unrelated local, so NSPACE's provenance
    // survives. This is tcllib clay's `foreach command [info commands
    // ::oo::define::*] {…}`, which must not abstain the whole walk
    // merely by standing between `set NSPACE …` and the creation.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    foreach command [info commands ::oo::define::*] {\n",
            "        set procname [namespace tail $command]\n",
            "    }\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        ),
    );
    let result = analysis(&src, "tcl9.0");
    assert!(
        result.all_classes.contains_key("::T::D::class"),
        "a bounded collection loop must not abstain the walk; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
    assert!(
        result.class_factories().contains_key("::T::D::class"),
        "the resolved metaclass must still publish a factory"
    );
}

#[test]
fn a_collection_loop_writing_the_provenance_variable_abstains() {
    // FP guard for the above — the relaxation covers *reachability*
    // only. Which iteration ran, or whether any did, stays undecidable,
    // so a loop body that writes NSPACE destroys the fact exactly as an
    // unknown-selection `if` arm does.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    foreach command {a b} {\n",
            "        set NSPACE $command\n",
            "    }\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        ),
    );
    assert!(
        !analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "a loop write to NSPACE must invalidate its provenance"
    );
}

#[test]
fn a_collection_loop_whose_body_diverges_abstains() {
    // FP guard: "bounded" buys the termination fact, nothing else. If the
    // body cannot reach its own end, the loop reaches the statement after
    // it only when the collection was empty — which is exactly what this
    // walk cannot decide — so the creation below stays unproved.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    foreach command {a b} {\n",
            "        return\n",
            "    }\n",
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        ),
    );
    assert!(
        !analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "a loop body that cannot fall through must abstain"
    );
}

#[test]
fn a_creation_inside_a_collection_loop_body_abstains() {
    // FP guard on the *selection* axis: a bounded loop is still not a
    // selectable arm. A creation written inside the body runs once per
    // element and not at all for an empty collection, so the walk must
    // not claim it the way it claims a creation in the unconditional
    // statement stream.
    let src = COMPUTED_METACLASS.replace(
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        concat!(
            "    foreach x {a} {\n",
            "        ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
            "    }\n",
        ),
    );
    assert!(
        !analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "a creation inside a loop body must not be claimed"
    );
}

#[test]
fn a_condition_loop_in_the_dominating_prefix_abstains() {
    // FP guard for the same relaxation, on the other axis: a `while` /
    // `for` trip count depends on state no descriptor here can read, so
    // the loop may never terminate and nothing is claimed about the
    // statement after it. Only *collection* loops carry the termination
    // guarantee the relaxation rests on.
    for loop_text in [
        "    while {[incr n] < $limit} {\n        set zz 1\n    }\n",
        "    for {set i 0} {$i < $limit} {incr i} {\n        set zz 1\n    }\n",
    ] {
        let src = COMPUTED_METACLASS.replace(
                "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
                &format!(
                    "{loop_text}    ::T::Mother create ${{NSPACE}}::class {{ superclass ::T::Mother }}\n"
                ),
            );
        assert!(
            !analysis(&src, "tcl9.0")
                .all_classes
                .contains_key("::T::D::class"),
            "an unbounded loop must abstain: {loop_text:?}"
        );
    }
}

#[test]
fn each_literal_call_site_proves_its_own_metaclass() {
    // TP — two call sites, two literals, two real metaclasses.
    // This is the clay / practcl shape: one `oo::dialect::create` proc
    // manufacturing a per-dialect metaclass for each caller.
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mkdialect {ns} {\n",
        "    ::T::Mother create ${ns}::class { superclass ::T::Mother }\n",
        "}\n",
        "::T::mkdialect ::T::D\n",
        "::T::mkdialect ::T::E\n",
    );
    let result = analysis(src, "tcl9.0");
    for want in ["::T::D::class", "::T::E::class"] {
        assert!(
            result.all_classes.contains_key(want),
            "expected {want}; got {:?}",
            result.all_classes.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn helper_proc_branch_result_drives_computed_creation() {
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::choose {enabled yes no} {\n",
        "    if {$enabled} { return $yes } else { return $no }\n",
        "}\n",
        "proc ::T::mk {name} {\n",
        "    set selected [::T::choose 1 $name ::T::Wrong]\n",
        "    ::T::Mother create ${selected}::class {}\n",
        "}\n",
        "::T::mk ::T::Right\n",
    );
    let result = analysis(src, "tcl9.0");
    assert!(result.all_classes.contains_key("::T::Right::class"));
    assert!(!result.all_classes.contains_key("::T::Wrong::class"));
}

#[test]
fn malformed_helper_branch_cannot_fabricate_a_computed_creation_name() {
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::choose {name} {\n",
        "    if {1} then\n",
        "    return $name\n",
        "}\n",
        "proc ::T::mk {name} {\n",
        "    set selected [::T::choose $name]\n",
        "    ::T::Mother create ${selected}::class {}\n",
        "}\n",
        "::T::mk ::T::Ghost\n",
    );
    assert!(
        !analysis(src, "tcl9.0")
            .all_classes
            .contains_key("::T::Ghost::class")
    );
}

#[test]
fn deep_helper_call_chain_cannot_exhaust_the_provenance_budget() {
    use std::fmt::Write as _;

    let mut src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
    )
    .to_owned();
    for index in 0..10 {
        let next = index + 1;
        let _ = writeln!(
            src,
            "proc ::T::step{index} {{name}} {{ return [::T::step{next} $name] }}"
        );
    }
    src.push_str("proc ::T::step10 {name} { return $name }\n");
    src.push_str(concat!(
        "proc ::T::mk {name} {\n",
        "    set selected [::T::step0 $name]\n",
        "    ::T::Mother create ${selected}::class {}\n",
        "}\n",
        "::T::mk ::T::Ghost\n",
    ));

    assert!(
        !analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::Ghost::class"),
        "a helper chain beyond the bounded proof budget must abstain"
    );
}

#[test]
fn computed_creation_does_not_carry_proc_locals_across_namespace_eval() {
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name} {\n",
        "    namespace eval ::Other {\n",
        "        ::T::Mother create ${name}::class {}\n",
        "    }\n",
        "}\n",
        "::T::mk ::T::Ghost\n",
    );
    assert!(
        !analysis(src, "tcl9.0")
            .all_classes
            .contains_key("::T::Ghost::class")
    );
}

#[test]
fn computed_creation_uses_default_proc_formals() {
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {{name ::T::D}} {\n",
        "    ::T::Mother create ${name}::class {}\n",
        "}\n",
        "::T::mk\n",
    );
    assert!(
        analysis(src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );
}

#[test]
fn a_renamed_namespace_normaliser_abstains() {
    // TN/rename guard — registry-derived command-table trust, not
    // textual resemblance, controls the helper call. Once its binding is
    // moved, the evaluator must not run the old body under the new name.
    let src = COMPUTED_METACLASS.replace(
        "::T::mkdialect ::T::D\n",
        "rename ::T::NSNormalize ::T::MovedNormalize\n::T::mkdialect ::T::D\n",
    );
    assert!(
        !analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "a renamed helper cannot prove the computed metaclass"
    );
}

#[test]
fn a_wildcard_compatible_later_rename_abstains() {
    // FP guard: a fully dynamic target is compatible with every command
    // identity. The trust scan is deliberately flow-insensitive, so a
    // later mutation blocks the proof too.
    let src = COMPUTED_METACLASS.replace(
        "::T::mkdialect ::T::D\n",
        "::T::mkdialect ::T::D\nrename $victim ::T::Moved\n",
    );
    assert!(
        !analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "a compatible later rename must block provenance"
    );
}

#[test]
fn an_aliased_namespace_normaliser_abstains() {
    // TN/alias guard — an alias can replace the helper's result
    // relation, so registry/user-proc provenance must decline it.
    let src = COMPUTED_METACLASS
        .replace(
            "set NSPACE [NSNormalize [uplevel 1 {namespace current}] $name]",
            "set NSPACE [AliasNormalize [uplevel 1 {namespace current}] $name]",
        )
        .replace(
            "::T::mkdialect ::T::D\n",
            concat!(
                "interp alias {} ::T::AliasNormalize {} ::T::NSNormalize\n",
                "::T::mkdialect ::T::D\n",
            ),
        );
    assert!(
        !analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "an aliased helper cannot inherit the original proc's provenance"
    );
}

#[test]
fn a_wildcard_compatible_earlier_alias_abstains() {
    // FP guard: `$victim` could bind the normaliser or either folded
    // builtin, so the proof must not depend on its runtime value.
    let src = COMPUTED_METACLASS.replace(
        "::T::mkdialect ::T::D\n",
        "interp alias {} $victim {} puts\n::T::mkdialect ::T::D\n",
    );
    assert!(
        !analysis(&src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "a compatible dynamic alias name must block provenance"
    );
}

#[test]
fn an_unknown_namespace_normaliser_abstains() {
    // TN/unknown guard — dependency on the literal argument alone
    // is not an identity proof; an unresolved helper may return anything.
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mkdialect {name} {\n",
        "    set NSPACE [UnknownNormalize $name]\n",
        "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
        "}\n",
        "::T::mkdialect ::T::D\n",
    );
    assert!(
        !analysis(src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "an unknown transform cannot prove an identity result"
    );
}

#[test]
fn a_dynamic_call_site_argument_proves_nothing() {
    // TN — the sole call site passes a runtime value, so no name
    // is knowable and the pass must record nothing. Abstention is the
    // documented contract; a guess here would invent a class.
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mkdialect {ns} {\n",
        "    ::T::Mother create ${ns}::class { superclass ::T::Mother }\n",
        "}\n",
        "set target [getNamespace]\n",
        "::T::mkdialect $target\n",
    );
    let result = analysis(src, "tcl9.0");
    let recorded: Vec<&String> = result
        .all_classes
        .keys()
        .filter(|k| k.ends_with("::class") && !k.contains("${"))
        .collect();
    assert!(
        recorded.is_empty(),
        "a dynamic call-site argument must prove no class name; got {recorded:?}"
    );
}

#[test]
fn a_call_site_inside_another_proc_body_proves_nothing() {
    // TN — the call runs at *call* time, if the enclosing proc is
    // ever invoked, so sourcing the file creates nothing. Same load-level
    // rule the destruction filter applies.
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mkdialect {ns} {\n",
        "    ::T::Mother create ${ns}::class { superclass ::T::Mother }\n",
        "}\n",
        "proc ::T::setup {} { ::T::mkdialect ::T::D }\n",
    );
    assert!(
        !analysis(src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "a call inside a proc body must not prove a load-time creation"
    );
}

#[test]
fn a_relative_computed_name_still_abstains() {
    // TN — the resolved name has no absolute written form, so
    // homing it needs the call site's namespace, which this pass does not
    // model. Abstaining beats homing it into the wrong namespace.
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mkdialect {ns} {\n",
        "    ::T::Mother create ${ns}::class { superclass ::T::Mother }\n",
        "}\n",
        "::T::mkdialect Rel\n",
    );
    let result = analysis(src, "tcl9.0");
    let recorded: Vec<&String> = result
        .all_classes
        .keys()
        .filter(|k| k.contains("Rel"))
        .collect();
    assert!(
        recorded.is_empty(),
        "a relative resolved name must abstain; got {recorded:?}"
    );
}

#[test]
fn a_proc_nobody_calls_proves_nothing() {
    // TN — no call site, no binding, nothing proved. The
    // regression guard for the whole pass: a workspace whose procs are
    // never called must analyse exactly as it did before.
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mkdialect {ns} {\n",
        "    ::T::Mother create ${ns}::class { superclass ::T::Mother }\n",
        "}\n",
    );
    let result = analysis(src, "tcl9.0");
    let recorded: Vec<&String> = result
        .all_classes
        .keys()
        .filter(|k| !k.contains("${") && k.ends_with("::class"))
        .collect();
    assert!(
        recorded.is_empty(),
        "an uncalled proc must prove no class name; got {recorded:?}"
    );
}

#[test]
fn a_braced_variable_command_head_resolves_through_its_constant() {
    // TP: the head's *variable* is `ns`, not the whole token
    // text `ns}::setdef` the lexer hands over for a braced composite
    // word.  Reading the true source bytes makes the call resolve to
    // the proc it really dispatches to (tclsh 9.0.4 / 8.6.16 both run
    // `::tc::setdef`).
    let src = concat!(
        "namespace eval tc { proc setdef {a b} { return 1 } }\n",
        "set ns tc\n",
        "${ns}::setdef x y\n",
    );
    let resolved: Vec<String> = analysis(src, "tcl8.6")
        .command_invocations
        .iter()
        .filter(|inv| inv.name.contains("${ns}"))
        .filter_map(|inv| inv.resolved_qualified_name.clone())
        .collect();
    assert_eq!(resolved, ["::tc::setdef"]);
}

#[test]
fn a_resolved_braced_variable_head_is_a_reference_not_a_rename_target() {
    // FP guard — the head's span is `${ns}::setdef`, which spells only
    // the tail.  Rewriting that span with a new name would splice it
    // over the substitution and corrupt the source,
    // so the invocation is marked `indirect`: references report it,
    // rename skips it.
    let src = concat!(
        "namespace eval tc { proc setdef {a b} { return 1 } }\n",
        "set ns tc\n",
        "${ns}::setdef x y\n",
    );
    let r = analysis(src, "tcl8.6");
    let inv = r
        .command_invocations
        .iter()
        .find(|inv| inv.name.contains("${ns}"))
        .expect("the composite head is recorded");
    assert!(inv.indirect, "{inv:?}");
}

#[test]
fn a_whole_word_variable_head_is_left_to_the_flow_sensitive_engine() {
    // TN — `$cmd` is M7's shape, settled from the CFG/SSA value model
    // rather than the walk's lexical map.  The walk must not
    // pre-resolve it from a last-write-wins constant, or the two
    // disagree about which sites are safe to rewrite.
    let src = "proc target {} { return hi }\nset cmd target\n$cmd\n";
    let r = analysis(src, "tcl8.6");
    let inv = r
        .command_invocations
        .iter()
        .find(|inv| inv.name == "${cmd}")
        .expect("the variable head is recorded");
    assert_eq!(
        inv.resolved_qualified_name.as_deref(),
        Some("::${cmd}"),
        "the walk leaves this head unfolded for M7 to settle: {inv:?}",
    );
}

#[test]
fn a_braced_variable_command_head_abstains_on_a_branch_binding() {
    // TN — a head whose variable is written differently on two
    // branches has no single dominating value, so resolution must not
    // pin either one.
    let src = concat!(
        "namespace eval a { proc go {} { return 1 } }\n",
        "namespace eval b { proc go {} { return 1 } }\n",
        "set ns a\n",
        "if {$cond} { set ns b }\n",
        "${ns}::go\n",
    );
    let resolved: Vec<String> = analysis(src, "tcl8.6")
        .command_invocations
        .iter()
        .filter(|inv| inv.name.contains("${ns}"))
        .filter_map(|inv| inv.resolved_qualified_name.clone())
        .collect();
    assert_eq!(
        resolved,
        ["::${ns}::go"],
        "an unresolved head stays as written"
    );
}
