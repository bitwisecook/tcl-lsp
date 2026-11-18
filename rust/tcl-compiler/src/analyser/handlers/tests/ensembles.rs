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

//! `namespace ensemble create` and `configure`, and the call sites that dispatch through them.

use super::*;

// handle_namespace_ensemble

#[test]
fn handle_namespace_ensemble_records_in_set() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(
        &["ensemble".to_string(), "create".to_string()],
        &[],
        &[0],
        None,
    );
    assert!(a.ensemble_namespaces.contains("::myns"));
}

#[test]
fn handle_namespace_ensemble_command_option_recorded() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    // `-command` recognition is registry-driven (`ENSEMBLE_CREATE_OPTIONS`).
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(
        &[
            "ensemble".to_string(),
            "create".to_string(),
            "-command".to_string(),
            "::ens".to_string(),
        ],
        &[],
        &[0],
        None,
    );
    assert!(a.ensemble_namespaces.contains("::myns"));
    assert!(a.ensemble_namespaces.contains("::ens"));
}

#[test]
fn handle_namespace_ensemble_command_option_qualifies_relative_name() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    // `-command` recognition is registry-driven (`ENSEMBLE_CREATE_OPTIONS`).
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(
        &[
            "ensemble".to_string(),
            "create".to_string(),
            "-command".to_string(),
            "ens".to_string(),
        ],
        &[],
        &[0],
        None,
    );
    assert!(a.ensemble_namespaces.contains("::myns::ens"));
}

#[test]
fn handle_namespace_ensemble_command_option_dynamic_value_not_recorded() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(
        &[
            "ensemble".to_string(),
            "create".to_string(),
            "-command".to_string(),
            "$dyn".to_string(),
        ],
        &[],
        &[0],
        None,
    );
    assert_eq!(
        a.ensemble_namespaces,
        std::collections::HashSet::from(["::myns".to_string()])
    );
}

#[test]
fn namespace_ensemble_map_and_subcommands_record_command_references() {
    // `-map {get ::foo::getImpl}` — the odd element is the target command;
    // `-subcommands {show}` maps `show` to `::foo::show`.  Both become
    // command references so navigation reaches the implementing procs.
    let mut a = Analyser::new();
    let src = "namespace eval ::foo {\n    \
                   proc getImpl {} {}\n    \
                   proc show {} {}\n    \
                   namespace ensemble create -map {get ::foo::getImpl} -subcommands {show}\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::foo::getImpl"),
        "the -map target should be a command reference: {resolved:?}",
    );
    assert!(
        resolved.contains(&"::foo::show"),
        "the -subcommands name should map to `<ns>::show`: {resolved:?}",
    );
}

/// A tk-shaped ensemble: the ensemble-creating command runs inside
/// a proc whose *qualified name* homes it to `::tk`, but which is declared
/// at the top level with no enclosing `namespace eval`.  `namespace
/// current` there is `::tk` (tclsh 8.6.16 / 9.0.4-verified: the ensemble
/// created is `::tk` and `tk alpha` dispatches to `::tk::alpha`), so the
/// subcommand must map to `::tk::alpha`; a purely lexical namespace walk
/// skips proc scopes and would map it to `::alpha`.
#[test]
fn namespace_ensemble_create_in_a_qualified_name_proc_homes_to_that_namespace_923_idx85() {
    let mut a = Analyser::new();
    let src = "namespace eval ::tk {}\n\
                   proc ::tk::alpha {} {}\n\
                   proc ::tk::SetupEnsemble {} {\n    \
                   namespace ensemble create -subcommands {alpha}\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::tk::alpha"),
        "the subcommand should map to the proc's defining namespace: {resolved:?}",
    );
    assert!(
        !resolved.contains(&"::alpha"),
        "the lexical (global) namespace is the wrong home: {resolved:?}",
    );
    assert!(
        a.ensemble_namespaces.contains("::tk"),
        "the ensemble command itself is `::tk`: {:?}",
        a.ensemble_namespaces,
    );
}

/// TN control for the above: the ordinary lexical case — the ensemble is
/// created directly inside `namespace eval ::foo`, so both the lexical and
/// the command-resolution namespace agree on `::foo`.  Pinned so the common
/// shape cannot regress.
#[test]
fn namespace_ensemble_create_lexically_inside_namespace_eval_is_unchanged_923_idx85() {
    let mut a = Analyser::new();
    let src = "namespace eval ::foo {\n    \
                   proc alpha {} {}\n    \
                   namespace ensemble create -subcommands {alpha}\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::foo::alpha"),
        "lexical case still maps to `::foo::alpha`: {resolved:?}",
    );
    assert!(a.ensemble_namespaces.contains("::foo"));
}

/// The definition-homing sweep: every analyser site that asks "which
/// namespace is current here?" answers with
/// [`Analyser::command_resolution_namespace`], so a definition made inside
/// a qualified-name proc's body homes to that proc's *defining* namespace.
/// These are cases (A)–(D) from `docs/design/analysis/name-resolution.md`
/// §3.5, each with the real interpreter's answer as the expectation.
#[test]
fn definitions_inside_a_qualified_name_proc_home_to_its_defining_namespace_923_idx85() {
    // (A) The already-correct lexical control.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::x { proc mk {} { proc helper {} {} } }\n",
        "tcl8.6",
    );
    assert!(
        r.all_procs.contains_key("::x::helper"),
        "(A) lexical case: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );

    // (B) A qualified-name proc with no enclosing `namespace eval`. Each
    // qualifier exists first: tclsh 8.6/9.0 refuse `proc ::x::mk` with
    // `unknown namespace` otherwise.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::x {}\nproc ::x::mk {} { proc helper {} {} }\n",
        "tcl8.6",
    );
    assert!(
        r.all_procs.contains_key("::x::helper"),
        "(B) qualified encloser: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );

    // (C) A qualified-name proc whose own name overrides the lexical
    // `namespace eval` it sits in.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::y {}\nnamespace eval ::x { proc ::y::mk {} { proc helper {} {} } }\n",
        "tcl8.6",
    );
    assert!(
        r.all_procs.contains_key("::y::helper"),
        "(C) absolute name rebases: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );

    // (D) The same rule for a class, not a proc.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::x {}\nproc ::x::mk {} { oo::class create Helper {} }\n",
        "tcl8.6",
    );
    assert!(
        r.all_classes.contains_key("::x::Helper"),
        "(D) class create: {:?}",
        r.all_classes.keys().collect::<Vec<_>>(),
    );
}

/// A nested definition that mis-homes to `::` collides with a real global
/// of the same name, and one silently overwrites the other.  Both must
/// survive under their own keys.
#[test]
fn a_nested_definition_no_longer_overwrites_a_same_named_global_923_idx85() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc helper {} { return global }\n\
             proc ::x::mk {} { proc helper {} { return nested } }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::helper"),
        "global lost: {keys:?}"
    );
    assert!(
        r.all_procs.contains_key("::x::helper"),
        "nested lost: {keys:?}",
    );
}

/// `namespace import` written inside a qualified-name proc imports into
/// that proc's defining namespace, not the global one.
#[test]
fn namespace_import_inside_a_qualified_name_proc_targets_that_namespace_923_idx85() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::src { proc thing {} {}; namespace export thing }\n\
             namespace eval ::dst {}\n\
             proc ::dst::setup {} { namespace import ::src::thing }\n",
        "tcl8.6",
    );
    let targets: Vec<&str> = r.namespace_imports.iter().map(|i| i.ns.as_str()).collect();
    assert!(
        targets.contains(&"::dst"),
        "the import lands in the proc's defining namespace: {targets:?}",
    );
}

/// A `-map` target inside the same qualified-name proc resolves the same
/// way — the `-map` half.
#[test]
fn namespace_ensemble_map_in_a_qualified_name_proc_resolves_targets_923_idx85() {
    let mut a = Analyser::new();
    let src = "namespace eval ::tk {}\n\
                   proc ::tk::getImpl {} {}\n\
                   proc ::tk::SetupEnsemble {} {\n    \
                   namespace ensemble create -map {get getImpl}\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::tk::getImpl"),
        "the -map target resolves in the proc's defining namespace: {resolved:?}",
    );
}

// The tk-shaped ensemble's *call-site* half.  Everything above pins the
// ensemble's own declaration (where the `-map`/`-subcommands` targets
// home to); these pin the downstream `<ensemble> <sub>` dispatch sites,
// which is what find-references / rename / code-lens / call-hierarchy
// enumerate.  Oracle — tclsh 8.6.16 and 9.0.4 both print
// `shown` then `configured:-x 1` for `VIAPROC_SRC`, so
// `::app::widget show` really does dispatch to `::app::widget::Show`.

/// The shape: `namespace ensemble create -map` inside a
/// proc declared with a fully-qualified name at top level, with no
/// enclosing `namespace eval`, and the dispatch call sites written after
/// it — one nested in a `[…]` substitution, one at the top level.
const VIAPROC_SRC: &str = "namespace eval ::app::widget {\n    \
         variable state 0\n\
         }\n\
         proc ::app::widget::Setup {} {\n    \
         namespace ensemble create -map {\n        \
         show      ::app::widget::Show\n        \
         configure ::app::widget::Configure\n    \
         }\n\
         }\n\
         proc ::app::widget::Show {} { puts \"shown\" }\n\
         proc ::app::widget::Configure {args} { puts \"configured:$args\" }\n\
         ::app::widget::Setup\n\
         puts [::app::widget show]\n\
         ::app::widget configure -x 1\n";

/// Every `(written name, resolved name)` pair recorded as an
/// existence-probed reference — the shape
/// `record_ensemble_subcommand_invocation` gives a subcommand word.
fn probe_refs(r: &super::super::super::types::AnalysisResult) -> Vec<(String, String)> {
    r.command_invocations
        .iter()
        .filter(|i| i.existence_probe)
        .map(|i| {
            (
                i.name.clone(),
                i.resolved_qualified_name.clone().unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn ensemble_call_sites_resolve_when_the_ensemble_is_created_in_a_qualified_proc_923_idx85() {
    let r = Analyser::new().analyse(VIAPROC_SRC, "tcl8.6").clone();
    let probes = probe_refs(&r);
    assert!(
        probes.contains(&("show".to_owned(), "::app::widget::Show".to_owned())),
        "the `[::app::widget show]` dispatch must be a reference to Show: {probes:?}",
    );
    assert!(
        probes.contains(&(
            "configure".to_owned(),
            "::app::widget::Configure".to_owned()
        )),
        "the top-level `::app::widget configure` dispatch must be a reference \
             to Configure: {probes:?}",
    );
}

#[test]
fn per_item_records_the_same_ensemble_call_sites_as_the_whole_file_walk_923_idx85() {
    // The per-item shell pass defers `::app::widget::Setup`'s body, so the
    // ensemble map is empty when the two call sites below it are
    // walked: without the deferred replay neither subcommand reference
    // is recorded — go-to-definition (an on-demand lookup against the
    // finished analysis) still answers, find-references cannot.  Comparing
    // the two walk strategies directly is the assertion that cannot drift.
    let whole = Analyser::new().analyse(VIAPROC_SRC, "tcl8.6").clone();
    let per_item = Analyser::new().analyse_per_item(VIAPROC_SRC, "tcl8.6");
    assert_eq!(
        probe_refs(&per_item),
        probe_refs(&whole),
        "per-item must record the same ensemble dispatch references as the \
             whole-file walk",
    );
    assert!(
        probe_refs(&per_item)
            .iter()
            .any(|(_, res)| res == "::app::widget::Show"),
        "and both must actually find Show's call site: {:?}",
        probe_refs(&per_item),
    );
}

#[test]
fn a_dynamic_ensemble_map_records_no_call_site_reference_923_idx85() {
    // TN — deliberate abstention.  `-map [list …]` is not a literal
    // list, so no `subcommand -> target` fact exists at all; the
    // dispatch site must stay unattributed rather than being guessed
    // at from the surrounding text.  Both walk strategies abstain.
    let src = "proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map [list show ::app::widget::Show]\n\
                   }\n\
                   proc ::app::widget::Show {} {}\n\
                   ::app::widget::Setup\n\
                   ::app::widget show\n";
    for r in [
        Analyser::new().analyse(src, "tcl8.6").clone(),
        Analyser::new().analyse_per_item(src, "tcl8.6"),
    ] {
        assert!(
            !probe_refs(&r)
                .iter()
                .any(|(_, res)| res == "::app::widget::Show"),
            "a dynamic -map must not attribute the dispatch site: {:?}",
            probe_refs(&r),
        );
    }
}

#[test]
fn an_ensemble_call_site_above_its_declaration_is_not_attributed_923_idx85() {
    // TN — the whole-file DFS walks a proc body at its *definition*
    // point, so a dispatch written above the proc that creates the
    // ensemble sees no map.  The deferred replay reproduces that
    // visibility rule rather than widening it, which is what keeps the
    // two walk strategies byte-identical.
    let src = "proc ::app::widget::Show {} {}\n\
                   ::app::widget show\n\
                   proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map {show ::app::widget::Show}\n\
                   }\n";
    for r in [
        Analyser::new().analyse(src, "tcl8.6").clone(),
        Analyser::new().analyse_per_item(src, "tcl8.6"),
    ] {
        assert!(
            !probe_refs(&r)
                .iter()
                .any(|(_, res)| res == "::app::widget::Show"),
            "a dispatch above the declaration must not be attributed: {:?}",
            probe_refs(&r),
        );
    }
}

// `namespace ensemble configure`: the real
// `tk/library/systray.tcl` idiom splices new subcommands onto a
// *pre-existing* ensemble via `configure`, not `create`, so
// `handle_namespace_ensemble` has to recognise both.

#[test]
fn namespace_ensemble_configure_extends_a_preexisting_ensembles_map() {
    // TP — the mechanism in isolation, with a literal `-map` value and a
    // deliberately non-tk ensemble name (`myens`), proving the mechanism is
    // registry-agnostic rather than hardcoded to `tk`.
    let mut a = Analyser::new();
    let src = "namespace eval ::myens {\n    \
                   namespace ensemble create -subcommands {}\n\
                   }\n\
                   proc ::myens::extra {args} {}\n\
                   namespace ensemble configure ::myens -map {extra ::myens::extra}\n\
                   myens extra\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::myens::extra"),
        "a configure-spliced -map target should be a command reference: {resolved:?}",
    );
}

#[test]
fn namespace_ensemble_configure_dynamic_name_is_not_recorded() {
    // FP-guard — `namespace ensemble configure $x -map {...}` can't be
    // resolved statically; the whole call must abstain rather than
    // recording a `-map` splice under a guessed/wrong key.
    let mut a = Analyser::new();
    a.handle_namespace_ensemble(
        &[
            "ensemble".to_string(),
            "configure".to_string(),
            "$x".to_string(),
            "-map".to_string(),
            "sub".to_string(),
            "::real::target".to_string(),
        ],
        &[],
        &[],
        None,
    );
    assert_eq!(a.result.ensemble_subcommand_targets.len(), 0);
}

#[test]
fn namespace_ensemble_configure_with_no_name_argument_is_a_no_op() {
    // TN — the bare query form `namespace ensemble configure` (no NAME,
    // no options) must not panic on `args[2]` indexing.
    let mut a = Analyser::new();
    a.handle_namespace_ensemble(
        &["ensemble".to_string(), "configure".to_string()],
        &[],
        &[],
        None,
    );
    assert_eq!(a.result.ensemble_subcommand_targets.len(), 0);
}

#[test]
fn namespace_ensemble_configure_dict_merge_literal_tail_is_extracted() {
    // TP — the real, exact idiom:
    // `namespace ensemble configure NAME -map [dict merge [namespace
    // ensemble configure NAME -map] {literal}]`. The self-referential
    // query argument is left unknown, but the literal tail's own pairs
    // are statically known regardless of what it evaluates to.
    let mut a = Analyser::new();
    let src = "namespace eval ::myens {\n    \
                   namespace ensemble create -subcommands {}\n\
                   }\n\
                   proc ::myens::extra {args} {}\n\
                   proc ::myens::other {args} {}\n\
                   namespace ensemble configure ::myens -map \
                   [dict merge [namespace ensemble configure ::myens -map] \
                   {extra ::myens::extra other ::myens::other}]\n\
                   myens extra\n\
                   myens other\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::myens::extra"),
        "the dict-merge literal tail's first pair should resolve: {resolved:?}",
    );
    assert!(
        resolved.contains(&"::myens::other"),
        "the dict-merge literal tail's second pair should resolve too, \
             proving the extraction isn't overfit to a single pair: {resolved:?}",
    );
}

#[test]
fn namespace_ensemble_configure_unrecognised_dynamic_map_shape_abstains_safely() {
    // Safety guard: a `-map` value that is itself one whole
    // dynamic `[...]` substitution NOT matching the narrow `dict merge
    // ARG {literal}` shape must abstain entirely — not naively
    // word-split the expression's own source text into bogus
    // subcommand/target pairs. `[linsert {} 0 foo bar]` evaluates to
    // the *string* "foo bar" at runtime (one call to `linsert`; "foo"/
    // "bar" are plain data words, never independently invoked) — were
    // the value's raw text naively whitespace-split instead of
    // abstained on, "foo" would land at an odd (target) index and
    // wrongly resolve to the real `::myens::foo` defined below,
    // recording a spurious command reference to it.
    let mut a = Analyser::new();
    let src = "namespace eval ::myens {\n    \
                   namespace ensemble create -subcommands {}\n\
                   }\n\
                   proc ::myens::foo {} {}\n\
                   namespace ensemble configure ::myens -map [linsert {} 0 foo bar]\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.ensemble_subcommand_targets
            .get("::myens")
            .is_none_or(std::collections::HashMap::is_empty),
        "an unrecognised dynamic -map shape must record no subcommand \
             targets: {:?}",
        r.ensemble_subcommand_targets.get("::myens"),
    );
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        !resolved.contains(&"::myens::foo"),
        "must not record a spurious command reference from splitting \
             the dynamic expression's own text: {resolved:?}",
    );
}

#[test]
fn ensemble_subcommand_targets_record_which_option_declared_them() {
    // The two options bind the subcommand word to its
    // target in opposite ways, so the recorded fact has to say which one
    // wrote it. Oracle (tclsh 8.6.14 / 9.0.4, identical): with `-map
    // {show ::app::widget::Show}` the call `::app::widget Show` is
    // `unknown or ambiguous subcommand "Show": must be show` — the key is
    // arbitrary; with `-subcommands {alpha}` the ensemble derives
    // `::app::widget::alpha` from the entry, and a `-subcommands {alpha}`
    // whose proc is named `beta` is `invalid command name "alpha"`.
    use crate::signature_scan::types::EnsembleSubcommandProvenance;

    let mut a = Analyser::new();
    let map_src = "namespace eval ::app::widget {}\n\
                       proc ::app::widget::Show {} {}\n\
                       namespace ensemble create -command ::app::widget \
                       -map {show ::app::widget::Show}\n";
    let r = a.analyse(map_src, "tcl8.6");
    let entry = r
        .ensemble_subcommand_targets
        .get("::app::widget")
        .and_then(|subs| subs.get("show"))
        .expect("the literal -map pair is recorded");
    assert_eq!(entry.target, "::app::widget::Show");
    assert_eq!(
        entry.provenance,
        EnsembleSubcommandProvenance::Map,
        "a -map pair must be tagged as such",
    );

    let mut a = Analyser::new();
    let sub_src = "namespace eval ::app::widget {\n    \
                       proc alpha {} {}\n    \
                       namespace ensemble create -command ::app::widget \
                       -subcommands {alpha}\n\
                       }\n";
    let r = a.analyse(sub_src, "tcl8.6");
    let entry = r
        .ensemble_subcommand_targets
        .get("::app::widget")
        .and_then(|subs| subs.get("alpha"))
        .expect("the literal -subcommands entry is recorded");
    assert_eq!(entry.target, "::app::widget::alpha");
    assert_eq!(
        entry.provenance,
        EnsembleSubcommandProvenance::Subcommands,
        "a -subcommands entry must be tagged as such",
    );
}

#[test]
fn ensemble_dispatch_call_sites_carry_their_mapping_provenance() {
    // The dispatch word's own invocation record carries the provenance,
    // because only the recording site knows *this span* is
    // a subcommand word: a `-map {Show ::app::widget::Show}` whose key
    // happens to equal the target's tail is textually indistinguishable
    // from an ordinary bare call to the target, so a consumer that
    // re-derived the fact by name would gate the wrong spans.
    use crate::signature_scan::types::EnsembleSubcommandProvenance;

    let mut a = Analyser::new();
    let map_src = "namespace eval ::app::widget {}\n\
                       proc ::app::widget::Show {} {}\n\
                       namespace ensemble create -command ::app::widget \
                       -map {show ::app::widget::Show}\n\
                       ::app::widget show\n";
    let r = a.analyse(map_src, "tcl8.6");
    let tagged: Vec<(&str, Option<EnsembleSubcommandProvenance>)> = r
        .command_invocations
        .iter()
        .filter(|i| i.resolved_qualified_name.as_deref() == Some("::app::widget::Show"))
        .map(|i| (i.name.as_str(), i.ensemble_dispatch))
        .collect();
    assert!(
        tagged.contains(&("show", Some(EnsembleSubcommandProvenance::Map))),
        "the dispatch word is tagged with the -map provenance: {tagged:?}",
    );
    assert!(
        tagged
            .iter()
            .any(|(name, prov)| *name == "::app::widget::Show" && prov.is_none()),
        "the -map *value* is an ordinary reference, not a dispatch word — \
             it carries the target's own name and rename must rewrite it: {tagged:?}",
    );

    let mut a = Analyser::new();
    let sub_src = "namespace eval ::app::widget {\n    \
                       proc alpha {} {}\n    \
                       namespace ensemble create -command ::app::widget \
                       -subcommands {alpha}\n\
                       }\n\
                       ::app::widget alpha\n";
    let r = a.analyse(sub_src, "tcl8.6");
    let tagged: Vec<(&str, Option<EnsembleSubcommandProvenance>)> = r
        .command_invocations
        .iter()
        .filter(|i| i.resolved_qualified_name.as_deref() == Some("::app::widget::alpha"))
        .map(|i| (i.name.as_str(), i.ensemble_dispatch))
        .collect();
    assert!(
        tagged.contains(&("alpha", Some(EnsembleSubcommandProvenance::Subcommands))),
        "the dispatch word is tagged with the -subcommands provenance: {tagged:?}",
    );
}

#[test]
fn handle_namespace_ensemble_other_options_value_word_is_not_mistaken_for_command_flag() {
    // `-map`'s value here is (pathologically, but syntactically legally)
    // the string `-command`. A scan checking *every* word for literal
    // equality with `-command`, another option's own value word
    // included, misreads that value as the
    // `-command` flag itself and steals the *next* word (the real
    // `-command`'s own flag) as if it were a namespace name. Walking
    // by each option's declared value arity instead correctly skips
    // `-map`'s whole value before ever looking for `-command` again,
    // so only the genuine `-command ::real::target` is recorded.
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(
        &[
            "ensemble".to_string(),
            "create".to_string(),
            "-map".to_string(),
            "-command".to_string(),
            "-command".to_string(),
            "::real::target".to_string(),
        ],
        &[],
        &[0],
        None,
    );
    assert_eq!(
        a.ensemble_namespaces,
        std::collections::HashSet::from(["::myns".to_string(), "::real::target".to_string()]),
        "only the genuine -command flag's value must be recorded, \
             not -map's value word that happens to read \"-command\""
    );
}

#[test]
fn handle_namespace_ensemble_global_scope_no_op() {
    let mut a = Analyser::new();
    a.handle_namespace_ensemble(
        &["ensemble".to_string(), "create".to_string()],
        &[],
        &[],
        None,
    );
    assert_eq!(a.ensemble_namespaces.len(), 0);
}

#[test]
fn handle_namespace_ensemble_wrong_subcommand_no_op() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(&["eval".to_string(), "myns".to_string()], &[], &[0], None);
    assert_eq!(a.ensemble_namespaces.len(), 0);
}
