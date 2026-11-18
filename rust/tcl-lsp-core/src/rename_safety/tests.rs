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
use tcl_compiler::analyser::Analyser;

fn analyse(source: &str) -> AnalysisResult {
    analyse_as(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

fn analyse_as(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, dialect.name).clone()
}

fn hazard(source: &str, family: &[&str], method: &str, is_classmethod: bool) -> Option<String> {
    hazard_to(
        source,
        family,
        method,
        is_classmethod,
        "Renamed",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

/// [`hazard`] with the requested new name (and dialect) spelled out — the
/// destination-sensitive arms need it.  `hazard` passes a name no fixture
/// declares, so the destination arms stay silent for the older cases.
fn hazard_to(
    source: &str,
    family: &[&str],
    method: &str,
    is_classmethod: bool,
    new_name: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Option<String> {
    let analysis = analyse_as(source, dialect);
    let owned: Vec<String> = family.iter().map(|s| (*s).to_string()).collect();
    method_rename_hazard(
        source,
        dialect,
        &analysis,
        MethodRenameTarget {
            family: &owned,
            method,
            is_classmethod,
            new_name,
        },
        &LineIndex::new(source),
    )
    .map(|r| r.reason)
}

/// The untracked-receiver hazard in its own shape.
///
/// nico-robert/tomato's `Vector3d.tcl` copy-constructor: `$other` is
/// `[lindex $args 0]` guarded by a runtime `info object isa` test, so it
/// really is a `Vector3d` at run time (tclsh 9.0.4 / 8.6.16 both print
/// `v2 = 7 9`) but the analyser has no binding for it.  An edit set
/// touching only the declaration, applied and re-run, gives on both
/// interpreters:
///
/// ```text
/// unknown method "X": must be Get, GetX, Y or destroy
///     while executing
/// "$other X"
/// ```
///
/// FP guard: the gate must refuse.
#[test]
fn fp_refuses_a_method_dispatched_on_an_untracked_receiver() {
    let src = "oo::class create Vector3d {\n\
                   \x20   variable _x\n\
                   \x20   constructor {args} {\n\
                   \x20       set other [lindex $args 0]\n\
                   \x20       if {[info object isa typeof $other ::Vector3d]} {\n\
                   \x20           set _x [$other X]\n\
                   \x20       }\n\
                   \x20   }\n\
                   \x20   method X {} { return $_x }\n\
                   }\n";
    let reason = hazard(src, &["::Vector3d"], "X", false)
        .expect("an untracked `$other X` dispatch must refuse the rename");
    assert!(
        reason.contains("not tracked"),
        "refusal must name the untracked receiver: {reason}"
    );
}

/// FN guard: the ordinary, provable shape must still rename.  `$v` is
/// bound by `set v [Dog new]`, so its dispatch is found and rewritten by
/// the reference scan — nothing to refuse.
#[test]
fn fn_guard_allows_a_method_whose_receivers_are_all_tracked() {
    let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   }\n\
                   set v [Dog new]\n\
                   puts [$v speak]\n";
    assert_eq!(hazard(src, &["::Dog"], "speak", false), None);
}

/// FN guard: an untracked receiver dispatching some *other* member is not
/// a hazard for this rename — refusing on it would block almost every
/// real rename, and it cannot affect the member being renamed.
#[test]
fn fn_guard_allows_when_the_untracked_receiver_names_another_member() {
    let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   \x20   method walk {} { return ok }\n\
                   \x20   constructor {other} { $other walk }\n\
                   }\n";
    assert_eq!(hazard(src, &["::Dog"], "speak", false), None);
}

/// TN: a receiver tracked to an *unrelated* class provably dispatches
/// that class's table, so a same-named member elsewhere is untouched and
/// unblocked.  tclsh 9.0.4 / 8.6.16 confirm the two `speak`s are
/// independent methods on independent objects.
#[test]
fn tn_allows_when_the_receiver_is_tracked_to_an_unrelated_class() {
    let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   }\n\
                   oo::class create Cat {\n\
                   \x20   method speak {} { return meow }\n\
                   }\n\
                   set c [Cat new]\n\
                   puts [$c speak]\n";
    assert_eq!(hazard(src, &["::Dog"], "speak", false), None);
}

/// FP guard: a member name computed at run time, dispatched on a receiver
/// of this very class.  No edit can keep `$v $m` consistent with a
/// renamed declaration, and nothing proves `$m` is not the renamed
/// member.
#[test]
fn fp_refuses_a_computed_member_name_on_a_tracked_receiver() {
    let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   }\n\
                   set v [Dog new]\n\
                   set m speak\n\
                   puts [$v $m]\n";
    let reason = hazard(src, &["::Dog"], "speak", false)
        .expect("a computed member name on an instance of the class must refuse");
    assert!(
        reason.contains("computed at run time"),
        "refusal must name the computed dispatch: {reason}"
    );
}

/// FP guard: the same, through the registry's own self-dispatch keyword
/// inside the class's own bodies (`my $m`) — recognised via
/// [`tcl_registry::Traits::TCLOO_SELF_DISPATCH`], never a literal
/// `== "my"`.
#[test]
fn fp_refuses_a_computed_member_name_dispatched_through_my() {
    let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   \x20   method run {m} { my $m }\n\
                   }\n";
    let reason = hazard(src, &["::Dog"], "speak", false)
        .expect("a `my $m` dispatch inside the class must refuse");
    assert!(reason.contains("computed at run time"), "{reason}");
}

#[test]
fn incomplete_method_selector_census_refuses_a_negative_result() {
    let src = "oo::class create Dog {method speak {} {}}; if {1} $body";
    let reason = hazard(src, &["::Dog"], "speak", false)
        .expect("an incomplete method-selector census cannot clear rename safety");
    assert!(reason.contains("potential source coverage"), "{reason}");
}

#[test]
fn positive_computed_method_hazard_survives_incomplete_source_inventory() {
    let src = "oo::class create Dog {method speak {} {}}; set v [Dog new]; set m speak; $v $m; if {1} $body";
    let reason = hazard(src, &["::Dog"], "speak", false)
        .expect("a selected computed method remains a concrete hazard");
    assert!(reason.contains("computed method name"), "{reason}");
}

/// FN guard: a plain `my speak` is exactly what the reference scan finds
/// and rewrites — never a refusal.
#[test]
fn fn_guard_allows_a_literal_my_dispatch() {
    let src = "oo::class create Dog {\n\
                   \x20   method speak {} { return woof }\n\
                   \x20   method run {} { my speak }\n\
                   }\n";
    assert_eq!(hazard(src, &["::Dog"], "speak", false), None);
}

/// FP guard, the object-command half: two classes binding the
/// *same qualified* object command.  `::a::Factory create rex` twice over
/// (here with two different classes in one namespace) leaves which class
/// a later `rex make` reaches a runtime fact — the second `create`
/// replaces the first command.
#[test]
fn fp_refuses_when_two_classes_bind_the_same_qualified_object_command() {
    let src = "namespace eval ::a {\n\
                   \x20   oo::class create Factory { method make {} { return f } }\n\
                   \x20   oo::class create Widget { method make {} { return w } }\n\
                   \x20   if {[gets stdin] eq \"a\"} {Factory create rex} else {Widget create rex}\n\
                   \x20   puts [rex make]\n\
                   }\n";
    let reason = hazard(src, &["::a::Factory"], "make", false)
        .expect("an object command bound by two classes must refuse");
    assert!(
        reason.contains("more than one class"),
        "refusal must name the ambiguity: {reason}"
    );
}

/// FN guard: the same bare name bound in *different* namespaces is not
/// ambiguous at all — `::a::rex` and `::b::rex` are two distinct commands
/// (tclsh 9.0.4 / 8.6.16: `rex make` inside `::a` prints `a-made`, inside
/// `::b` prints `b-made`, and both object commands coexist).
#[test]
fn fn_guard_allows_same_bare_object_command_in_sibling_namespaces() {
    let src = "namespace eval ::a {\n\
                   \x20   oo::class create Factory { method make {} { return a } }\n\
                   \x20   Factory create rex\n\
                   }\n\
                   namespace eval ::b {\n\
                   \x20   oo::class create Widget { method make {} { return b } }\n\
                   \x20   Widget create rex\n\
                   }\n";
    assert_eq!(hazard(src, &["::a::Factory"], "make", false), None);
}

/// Software authoring advice only; actual Native edit identity stays
/// independent and has separate refusal controls below.
fn var_hazard(source: &str, cell: &str) -> Option<String> {
    var_hazard_as(source, cell, "tcl8.6")
}

/// Supply the actual availability and independently selected close-rule
/// config with a genuinely Logical source input. The label supplies no
/// Native naming policy or interpreter observation.
fn var_hazard_as(source: &str, cell: &str, dialect: &str) -> Option<String> {
    let environment = tcl_registry::model::ingress::resolve_environment(dialect);
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::for_profile(Some(environment.unit_profile())),
    );
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    assert!(analysis.allows_retained_logical_declaration_advice());
    namespace_variable_rename_hazard(source, profile, &analysis, cell, &LineIndex::new(source))
        .map(|r| r.reason)
}

/// The rename gate reads a computed name's `${…}` extent under the
/// **document's** close rule, not the 9.x default.
///
/// The literal characters around a substitution are the whole bound this
/// gate rests on, so the two close rules move the decision in opposite
/// directions. These software controls retain Logical source input and
/// the independently selected exact config and availability either way:
///
/// - **9.x** — `${a{b}c}` is one wildcard (the nesting rule consumes the
///   inner pair), which can spell any cell, so the rename is refused.
/// - **8.x** — the name ends at the first `}`, leaving the wildcard
///   followed by the literal `c}`; no ordinary cell name ends that way, so
///   the word provably cannot reach `::ns::v` and the rename is safe.
///
/// Reading an 8.x document with the 9.x default therefore refuses a rename
/// that is provably safe. Oracle for the underlying split:
/// `set {a{b}c} HIT; subst {${a{b}c}}` is `HIT` on tclsh 9.0.4 and
/// `can't read "a{b"` on 8.6.16 (`Tcl_ParseVarName`, `tclParse.c:1315` vs
/// `:1398`).
#[test]
fn the_computed_name_bound_follows_the_document_close_rule() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {} { set ${a{b}c} 2 }\n\
                   }\n";

    let reason = var_hazard_as(src, "::ns::v", "tcl9.0")
        .expect("9.x reads `${a{b}c}` as one wildcard, which can spell the cell");
    assert!(reason.contains("computed at run time"), "{reason}");

    assert_eq!(
        var_hazard_as(src, "::ns::v", "tcl8.6"),
        None,
        "8.x ends the name at the first `}}`, so the trailing literal `c}}` \
             puts `::ns::v` out of reach and the rename is safe",
    );

    // Control: with the 8.x remainder actually present in the cell name the
    // narrower pattern *does* match, so 8.x still refuses — the bound is a
    // narrowing, not a disabling.
    let reason = var_hazard_as(src, "::ns::xc}", "tcl8.6")
        .expect("`*c}` really can spell a cell ending in `c}`");
    assert!(reason.contains("computed at run time"), "{reason}");
}

/// FP guard: a computed variable name in a registry-declared variable-name
/// argument position may be naming the very cell being renamed, and there
/// is no word to rewrite.
#[test]
fn fp_refuses_a_namespace_variable_rename_beside_a_computed_name() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {n} { set $n 2 }\n\
                   }\n";
    let reason = var_hazard(src, "::ns::v")
        .expect("a `set $n` in the same document must refuse the cell rename");
    assert!(reason.contains("computed at run time"), "{reason}");
}

/// FP guard (#2259): the `otherVar` of `namespace upvar` and `upvar` names
/// the cell the pair links. A computed one may be the cell being renamed,
/// which the registry's cell role, not a separate alias record, reports.
#[test]
fn fp_refuses_a_namespace_variable_rename_beside_a_computed_upvar_cell() {
    for link in ["namespace upvar ::ns $v local", "upvar #0 $v local"] {
        let src = format!(
            "namespace eval ::ns {{\n    variable version 1\n}}\n\
                 proc p {{v}} {{ {link}; return $local }}\n"
        );
        let reason = var_hazard(&src, "::ns::version")
            .unwrap_or_else(|| panic!("`{link}` may link `::ns::version`"));
        assert!(reason.contains("computed at run time"), "{link}: {reason}");
    }
    let src = "namespace eval ::ns {\n    variable version 1\n}\n\
                   proc p {} { namespace upvar ::ns version local; return $local }\n";
    assert_eq!(
        var_hazard(src, "::ns::version"),
        None,
        "a literal cell is rewritable"
    );
}

/// FP guard: a computed namespace beside the literal tail may still be the
/// renamed cell. tclsh 8.6/9.0: `namespace eval mypkg {variable version 1};
/// proc show {ns} {namespace upvar $ns version local; return $local};
/// show ::mypkg` returns 1. Another tail never reaches the cell.
#[test]
fn fp_refuses_a_namespace_variable_rename_beside_a_computed_upvar_namespace() {
    let doc = |tail: &str| {
        format!(
            "namespace eval ::ns {{\n    variable version 1\n}}\n\
             proc p {{ns}} {{ namespace upvar $ns {tail} local; return $local }}\n"
        )
    };
    let reason = var_hazard(&doc("version"), "::ns::version")
        .expect("`namespace upvar $ns version local` may link `::ns::version`");
    assert!(reason.contains("computed at run time"), "{reason}");
    assert_eq!(var_hazard(&doc("counter"), "::ns::version"), None);
}

/// FN guard: the ordinary namespace-variable document has no dynamic
/// name and must rename freely.
#[test]
fn fn_guard_allows_a_namespace_variable_rename_with_static_names() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {} { variable v; return $v }\n\
                   }\n\
                   puts $::ns::v\n";
    assert_eq!(var_hazard(src, "::ns::v"), None);
}

// Per-site provenance.  The refusal asks whether *this* word can spell
// *this* cell, rather than refusing on any dynamic word anywhere in the
// document.

/// TN: a dynamic word under a different, statically-written namespace
/// cannot spell a cell in `::ns`, so the rename proceeds.
///
/// tclsh-proof (8.6.14): `namespace eval ::ns {variable v 1}; namespace
/// eval ::other {}; set n {::ns::v}; set ::other::$n 99` fails with
/// `can't set "::other::::ns::v": parent namespace doesn't exist` — the
/// value lands *under* the written prefix, so no value of `$n` reaches
/// `::ns::v`.
#[test]
fn tn_a_dynamic_name_under_another_namespace_no_longer_refuses() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   }\n\
                   namespace eval ::other {\n\
                   \x20   proc p {n} { set ::other::$n 2 }\n\
                   }\n";
    assert_eq!(
        var_hazard(src, "::ns::v"),
        None,
        "`::other::$n` cannot name `::ns::v`",
    );
}

/// TP control for the pair above: the same word *under the renamed
/// namespace* still refuses — the narrowing is a narrowing, not a
/// disabling.
#[test]
fn tp_a_dynamic_name_under_the_renamed_namespace_still_refuses() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {n} { set ::ns::$n 2 }\n\
                   }\n";
    let reason = var_hazard(src, "::ns::v").expect("`::ns::$n` can name the cell");
    assert!(reason.contains("computed at run time"), "{reason}");
}

/// TN: a written affix bounds the name too — `${p}_count` can never spell
/// `v` (tclsh-proof, 8.6.14: `set j 1; set v$j 2` leaves `::ns::total`
/// untouched and creates only `::v1`).
#[test]
fn tn_a_dynamic_name_with_an_unsatisfiable_affix_no_longer_refuses() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {pfx} { set ${pfx}_count 2 }\n\
                   }\n";
    assert_eq!(var_hazard(src, "::ns::v"), None);
}

/// TP (CRITICAL): the issue's own oracle shape must stay refused — a bare
/// `$n` is a lone wildcard and really does reach the cell.
///
/// tclsh-proof (8.6.14): `namespace eval ns {variable v 1; proc bump {n}
/// {variable $n; set $n 2}}; ns::bump v` leaves `set ::ns::v` -> 2.
#[test]
fn tp_the_bare_dynamic_name_oracle_still_refuses() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc bump {n} { variable $n; set $n 2 }\n\
                   }\n";
    let reason = var_hazard(src, "::ns::v").expect("`variable $n` really does reach the cell");
    assert!(reason.contains("computed at run time"), "{reason}");
}

// Per-site *value* provenance.  The text bound cannot see what a lone
// `$n` holds; the analyser's dominating-constant lattice can, and carries
// the answer out on `dynamic_variable_names`.

/// TN: a `$n` whose dominating constant is provably a different name does
/// not refuse.
///
/// tclsh-proof (8.6.14): `namespace eval ::ns {variable v 1; proc p {}
/// {set n other; set $n 2}}; ::ns::p; info exists ::ns::v` -> 1 and
/// `set ::ns::v` -> 1 — the write landed on `other`, never on `v`.
#[test]
fn tn_a_dominating_constant_name_no_longer_refuses() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {} { set n other ; set $n 2 }\n\
                   }\n";
    assert_eq!(
        var_hazard(src, "::ns::v"),
        None,
        "`$n` is provably `other` at that site",
    );
}

/// TP control: the same shape whose constant **is** the cell's tail still
/// refuses — the provenance narrows, it does not disable.
#[test]
fn tp_a_dominating_constant_naming_the_cell_still_refuses() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {} { set n v ; set $n 2 }\n\
                   }\n";
    let reason = var_hazard(src, "::ns::v").expect("`$n` is provably `v` at that site");
    assert!(reason.contains("computed at run time"), "{reason}");
}

/// TP: a **branch-dependent** binding dominates nothing, so the site
/// resolves to `None` and the refusal stands.
///
/// tclsh-proof (8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0): `namespace eval ::ns {variable v 1; proc p {c}
/// {set n other; if {$c} {set n v}; variable v; set $n 2}}; ::ns::p 1`
/// leaves `set ::ns::v` -> 2 — the conditional branch really does reach
/// the cell.
#[test]
fn tp_a_branch_dependent_constant_still_refuses() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {c} { set n other ; if {$c} { set n v } ; variable v ; set $n 2 }\n\
                   }\n";
    let reason = var_hazard(src, "::ns::v").expect("a branch-dependent value proves nothing");
    assert!(reason.contains("computed at run time"), "{reason}");
}

/// TP: a parameter-fed name proves nothing either — the caller chooses it.
#[test]
fn tp_a_parameter_fed_name_still_refuses() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {n} { set $n 2 }\n\
                   }\n";
    let reason = var_hazard(src, "::ns::v").expect("a parameter proves nothing");
    assert!(reason.contains("computed at run time"), "{reason}");
}

#[test]
fn original_dynamic_name_value_narrowing_stays_in_its_logical_purpose() {
    // naming.core.original-dynamic-name-value-purpose
    // docs/design/analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md
    let source = "set n other\nset $n 2";
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jimtcl",
        "f5-irules",
    ] {
        let analysis = Analyser::new().analyse(source, dialect);
        assert!(!analysis.allows_lexical_declaration_advice(), "{dialect}");
        assert_eq!(analysis.dynamic_variable_names.len(), 1, "{dialect}");
        for site in &analysis.dynamic_variable_names {
            assert!(
                !site_resolution_rules_out(
                    source,
                    &analysis,
                    site.span,
                    "target",
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile()
                ),
                "{dialect}"
            );
        }
    }
    let logical = Analyser::new().analyse(source, "tcl");
    assert!(logical.allows_lexical_declaration_advice());
    let site = logical.dynamic_variable_names.first().unwrap();
    assert_eq!(site.resolved.as_deref(), Some("other"));
    assert!(site_resolution_rules_out(
        source,
        &logical,
        site.span,
        "target",
        tcl_dialect::DialectProfile::plain_tcl()
    ));
}

/// The provenance table itself: a resolvable site carries its value, an
/// unresolvable one carries `None`, and a **braced** name word — which
/// substitutes nothing — is not a dynamic site at all.
///
/// tclsh-proof (8.6.14): `set {$n} v; info exists {$n}` -> 1 while
/// `info exists n` -> 0.
#[test]
fn the_provenance_table_records_each_site_once() {
    let analysis = analyse(
        "proc a {} { set n other ; set $n 2 }\n\
             proc b {p} { set $p 2 }\n\
             proc c {} { set {$n} 2 }\n",
    );
    let resolutions: Vec<Option<&str>> = analysis
        .dynamic_variable_names
        .iter()
        .map(|s| s.resolved.as_deref())
        .collect();
    assert_eq!(
        resolutions,
        [Some("other"), None],
        "one row per computed name word; a braced literal name is not one",
    );
    assert!(
        analysis.dynamic_variable_names.iter().all(|s| s.writes),
        "both sites are `set` targets",
    );
}

// A moved member's declaration site is the
// `renamemethod`'s destination word, so renaming it rewrites that word and
// some new names produce a body real Tcl refuses to run.  The gate reads
// the same `RenamedMember` fold the W315 diagnostic does.

/// TP: `renamemethod old new` + renaming `new` back to `old` would emit
/// `renamemethod old old`, which fails `cannot rename method to itself`
/// and creates no class (tclsh 9.0.4 / 8.6.14).
#[test]
fn tp_refuses_renaming_a_moved_member_to_its_own_source() {
    let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   renamemethod old new\n\
                   }\n";
    let reason = hazard_to(
        src,
        &["::C"],
        "new",
        false,
        "old",
        crate::profile_for_dialect("tcl9.0"),
    )
    .expect("renaming the moved member to its source must refuse");
    assert!(
        reason.contains("cannot rename method to itself"),
        "{reason}"
    );
}

/// TP: renaming it onto a **live same-side sibling** would emit
/// `renamemethod old sib` -> `method called sib already exists`.
#[test]
fn tp_refuses_renaming_a_moved_member_onto_a_live_sibling() {
    let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   method sib {} { return 2 }\n\
                   \x20   renamemethod old new\n\
                   }\n";
    let reason = hazard_to(
        src,
        &["::C"],
        "new",
        false,
        "sib",
        crate::profile_for_dialect("tcl9.0"),
    )
    .expect("renaming the moved member onto a live sibling must refuse");
    assert!(
        reason.contains("method called sib already exists"),
        "{reason}"
    );
}

/// TP, the mirror: renaming the ordinary sibling *into* the destination a
/// later `renamemethod` needs free collides at that same site.
#[test]
fn tp_refuses_renaming_a_sibling_onto_a_later_rename_destination() {
    let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   method sib {} { return 2 }\n\
                   \x20   renamemethod old new\n\
                   }\n";
    let reason = hazard_to(
        src,
        &["::C"],
        "sib",
        false,
        "new",
        crate::profile_for_dialect("tcl9.0"),
    )
    .expect("renaming a sibling onto a later rename destination must refuse");
    assert!(
        reason.contains("method called new already exists"),
        "{reason}"
    );
}

/// TN: a fresh destination is a legal rename and must not be refused.
#[test]
fn tn_allows_renaming_a_moved_member_to_a_fresh_name() {
    let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   method sib {} { return 2 }\n\
                   \x20   renamemethod old new\n\
                   }\n";
    assert_eq!(
        hazard_to(
            src,
            &["::C"],
            "new",
            false,
            "fresh",
            crate::profile_for_dialect("tcl9.0")
        ),
        None
    );
}

/// TN (CRITICAL): a name live only on the **other** side is not a
/// collision — `method old {} {…}` + `self { method sib {} {…} }` +
/// `renamemethod old sib` runs fine on 9.0.4 and 8.6.14.
#[test]
fn tn_allows_renaming_a_moved_member_onto_a_cross_side_name() {
    let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   self { method sib {} { return 2 } }\n\
                   \x20   renamemethod old new\n\
                   }\n";
    assert_eq!(
        hazard_to(
            src,
            &["::C"],
            "new",
            false,
            "sib",
            crate::profile_for_dialect("tcl9.0")
        ),
        None
    );
}

/// TN: a destination deleted *before* the move is free — the snapshot is
/// taken at the move's own point in the body, exactly as Tcl reads it.
#[test]
fn tn_allows_renaming_a_moved_member_onto_an_earlier_deleted_name() {
    let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   method sib {} { return 2 }\n\
                   \x20   deletemethod sib\n\
                   \x20   renamemethod old new\n\
                   }\n";
    assert_eq!(
        hazard_to(
            src,
            &["::C"],
            "new",
            false,
            "sib",
            crate::profile_for_dialect("tcl9.0")
        ),
        None
    );
}

/// TN (CRITICAL): the hazard is **side-local**. An
/// instance-side `renamemethod old same` says nothing about a
/// class-object-side `same`, so renaming that class-side member to `old` is
/// legal and must not be refused. Oracle, byte-identical on tclsh 9.0.4 and
/// 8.6.14 — the post-edit body creates the class and both members dispatch:
///
/// ```tcl
/// oo::class create ::P2 { method old {} { return inst-old }
///                         renamemethod old same
///                         self { method old {} { return cls-old } } }
/// info class methods ::P2   ;# -> same        [::P2 new] same -> inst-old
/// info object methods ::P2  ;# -> old         ::P2 old        -> cls-old
/// ```
#[test]
fn tn_an_instance_side_move_does_not_gate_a_class_side_rename() {
    let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   renamemethod old same\n\
                   \x20   self { method same {} { return 2 } }\n\
                   }\n";
    assert_eq!(
        hazard_to(
            src,
            &["::C"],
            "same",
            true,
            "old",
            crate::profile_for_dialect("tcl9.0")
        ),
        None,
        "an instance-side move must not refuse a class-side rename",
    );
}

/// TN, the mirror: a class-object-side move says nothing about an
/// instance-side member of the same name.
#[test]
fn tn_a_class_side_move_does_not_gate_an_instance_side_rename() {
    let src = "oo::class create C {\n\
                   \x20   self { method old {} { return 1 }\n\
                   \x20          renamemethod old same }\n\
                   \x20   method same {} { return 2 }\n\
                   }\n";
    assert_eq!(
        hazard_to(
            src,
            &["::C"],
            "same",
            false,
            "old",
            crate::profile_for_dialect("tcl9.0")
        ),
        None,
        "a class-side move must not refuse an instance-side rename",
    );
}

/// TP control for the pair above: on the **same** side the refusal still
/// fires, so the side filter narrowed the arm rather than disabling it.
#[test]
fn tp_the_side_filter_keeps_the_same_side_refusal() {
    let src = "oo::class create C {\n\
                   \x20   self { method old {} { return 1 }\n\
                   \x20          renamemethod old same }\n\
                   }\n";
    let reason = hazard_to(
        src,
        &["::C"],
        "same",
        true,
        "old",
        crate::profile_for_dialect("tcl9.0"),
    )
    .expect("a class-side move must still gate a class-side rename");
    assert!(
        reason.contains("cannot rename method to itself"),
        "{reason}"
    );
}

/// TN: the mirror arm is side-local too — an instance-side sibling renamed
/// into a *class-side* move's destination collides with nothing.
#[test]
fn tn_the_mirror_arm_is_side_local() {
    let src = "oo::class create C {\n\
                   \x20   self { method old {} { return 1 }\n\
                   \x20          method sib {} { return 2 }\n\
                   \x20          renamemethod old new }\n\
                   \x20   method sib {} { return 3 }\n\
                   }\n";
    assert_eq!(
        hazard_to(
            src,
            &["::C"],
            "sib",
            false,
            "new",
            crate::profile_for_dialect("tcl9.0")
        ),
        None,
        "an instance-side `sib` is not the class-side move's blocker",
    );
}

/// TN: a class with no `renamemethod` at all is untouched by the new arm.
#[test]
fn tn_allows_an_ordinary_member_rename_with_no_renamemethod_in_the_body() {
    let src = "oo::class create C {\n\
                   \x20   method old {} { return 1 }\n\
                   \x20   method sib {} { return 2 }\n\
                   }\n";
    assert_eq!(
        hazard_to(
            src,
            &["::C"],
            "old",
            false,
            "sib",
            crate::profile_for_dialect("tcl9.0")
        ),
        None
    );
}
