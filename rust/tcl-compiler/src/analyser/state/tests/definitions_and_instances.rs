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

//! What the walk records about definitions: procedure homes, `apply` lambda namespaces and the class of each created instance.

use super::*;

#[test]
fn analyse_records_top_level_proc() {
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set x 1 }", "tcl");
    assert!(r.all_procs.contains_key("::foo"));
}

#[test]
fn apply_body_proc_registers_in_global_namespace_not_caller() {
    // `apply`'s body runs in the *global* namespace (no lambda namespace
    // given), NOT the caller's — so a nested `proc` registers as `::helper`,
    // never `::foo::helper` (Tcl `apply` manual; matches the IR lowering).
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval foo { apply {{} { proc helper {} { return 1 } }} }",
        "tcl8.6",
    );
    assert!(
        r.all_procs.contains_key("::helper"),
        "apply body proc must register globally: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_procs.contains_key("::foo::helper"),
        "apply body proc must NOT inherit the caller namespace: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// TP — the tcllib pki idiom: `proc [namespace current]::_x {...}` inside
/// `::pki`.  The body's own definitions must home to `::pki`, not to a
/// phantom namespace made out of the substitution's source text.
///
/// Oracle (tclsh 8.6.16 and 9.0.4, probe `s5_probe.tcl`):
///
/// ```text
/// inner exists: ::pki::_inner
/// helper homed at: ::pki::helper / global:
/// ```
#[test]
fn a_dynamically_named_procs_body_homes_to_the_lexical_namespace() {
    let src = "namespace eval ::pki {\n\
                       proc _outer {} {\n\
                           proc [namespace current]::_inner {} {\n\
                               proc helper {} { return HELPED }\n\
                           }\n\
                       }\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.all_procs.contains_key("::pki::helper"),
        "the body of a `[namespace current]`-named proc runs in ::pki: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_procs
            .keys()
            .any(|k| k.contains("[namespace current]") && k.ends_with("helper")),
        "no phantom substitution-text namespace may appear: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// TN — a statically-named proc is untouched: the scope name is the
/// resolved name verbatim, so an ordinary qualified-name proc still homes
/// its nested definitions to its own defining namespace.
#[test]
fn a_statically_named_procs_body_still_homes_to_its_defining_namespace() {
    let src = "proc ::pki::_outer {} { proc helper {} {} }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.all_procs.contains_key("::pki::helper"),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// A `$`-spelled dynamic name (`proc _$n {...}`) gives its body no proven
/// namespace: a value with `::` names a child namespace, and the body runs
/// there. tclsh 8.4.20, 8.6 and 9.0 with `suffix` `x` then `a::b` define
/// `::pki::helper` and `::pki::_a::helper`, so no single home is reported.
#[test]
fn a_dollar_named_procs_body_has_no_proven_home() {
    let src = "namespace eval ::pki { proc _$suffix {} { proc helper {} {} } }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.all_procs.keys().any(|name| name.ends_with("::helper")),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn apply_body_proc_honours_explicit_lambda_namespace() {
    // A lambda pinning a namespace (element 2) runs its body there. The
    // namespace exists first (tclsh 8.6/9.0: `namespace "::bar" not found`).
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::bar {}\napply {{} { proc helper {} { return 1 } } ::bar}",
        "tcl8.6",
    );
    assert!(
        r.all_procs.contains_key("::bar::helper"),
        "explicit lambda namespace must be honoured: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// TP — a *relative* (non-`::`-prefixed) lambda namespace element is
/// interpreted against the GLOBAL namespace, never the caller's.
///
/// `doc/apply.n`: "If given, namespace is interpreted relative to the
/// global namespace even if its name does not start with `::`";
/// `tclProc.c`'s `TclNRApplyObjCmd` `::`-prefixes the word before the
/// lookup. tclsh 9.0.4 oracle (probe `a2b_apply.tcl`), with `::sub`,
/// `::pin::sub`, and `::lex::sub` all defined:
///
/// ```text
/// lex-body current=::lex -> ns=::sub who=GLOBAL-SUB
/// proc current=::pin     -> ns=::sub who=GLOBAL-SUB
/// c2 current=::pin       -> ns=::sub who=GLOBAL-SUB
/// ```
#[test]
fn apply_relative_lambda_namespace_homes_globally_not_to_the_caller() {
    let src = "namespace eval ::sub {}\n\
                   namespace eval ::caller::sub {}\n\
                   proc ::caller::p {} {\n\
                       apply {{} { proc helper {} { return 1 } } sub}\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.all_procs.contains_key("::sub::helper"),
        "relative lambda ns must resolve against ::, not the caller: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_procs.contains_key("::caller::sub::helper"),
        "relative lambda ns must NOT pin against the caller namespace: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    let (_, ns) = r
        .namespace_overrides
        .first()
        .expect("the lambda body records a namespace override");
    assert_eq!(ns, "::sub");
}

/// TN — an *absolute* namespace element is unaffected by the
/// global-relative rule: `::caller::sub` stays `::caller::sub`.
#[test]
fn apply_absolute_lambda_namespace_is_unchanged() {
    let src = "namespace eval ::sub {}\n\
                   namespace eval ::caller::sub {}\n\
                   proc ::caller::p {} {\n\
                       apply {{} { proc helper {} { return 1 } } ::caller::sub}\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.all_procs.contains_key("::caller::sub::helper"),
        "absolute lambda ns must be honoured verbatim: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// Control — no namespace element at all still means the global
/// namespace, from inside a qualified-name proc as much as anywhere.
#[test]
fn apply_without_a_namespace_element_stays_global_inside_a_qualified_proc() {
    // `::caller` exists first (tclsh 8.6/9.0: `can't create procedure
    // "::caller::p": unknown namespace`).
    let src = "namespace eval ::caller {}\n\
                   proc ::caller::p {} {\n\
                       apply {{} { proc helper {} { return 1 } }}\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.all_procs.contains_key("::helper"),
        "an absent lambda ns means global: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn apply_with_namespace_element_records_a_namespace_override() {
    // TP (the core mechanism): a literal
    // `apply {{params} body ns}` must record a namespace_overrides
    // entry spanning the body, so `tcl-lsp-core`'s command-resolution
    // lookups can pin bareword calls inside the body to `ns` — the
    // `Scope` subtree `reconstruct_proc_scope` builds for `ns` is
    // rooted under body_span-less wrapper nodes the ordinary lexical
    // walk can never reach.
    let mut a = Analyser::new();
    let src = "apply {{} { cleanup done } ::real}";
    let r = a.analyse(src, "tcl8.6");
    assert_eq!(
        r.namespace_overrides.len(),
        1,
        "{:?}",
        r.namespace_overrides
    );
    let (span, ns) = &r.namespace_overrides[0];
    assert_eq!(ns, "::real");
    // The recorded span is the body token's raw span (opening brace
    // included, per this codebase's lexer-span convention — the
    // closing brace is excluded, one byte short of the token's end).
    assert_eq!(
        &src[span.start() as usize..span.end() as usize],
        "{ cleanup done "
    );
}

#[test]
fn apply_without_namespace_element_does_not_record_an_override() {
    // TN — a 2-element lambda (no namespace argument) defaults to
    // global, which `command_resolution_namespace_at` already reports
    // with no override present; pushing a `(span, "::")` entry would
    // be a no-op relative to today's behaviour, but the plan
    // deliberately still records it (uniform code path) — confirm the
    // recorded namespace is exactly `"::"`, not skipped or wrong.
    let mut a = Analyser::new();
    let r = a.analyse("apply {{} { cleanup done }}", "tcl8.6");
    assert_eq!(
        r.namespace_overrides.len(),
        1,
        "{:?}",
        r.namespace_overrides
    );
    assert_eq!(r.namespace_overrides[0].1, "::");
}

#[test]
fn analyse_records_multiple_top_level_commands() {
    let mut a = Analyser::new();
    let r = a.analyse("set x 1\nproc foo {} {}\nglobal a b", "tcl");
    assert!(r.all_procs.contains_key("::foo"));
    assert!(r.global_scope.variables.contains_key("x"));
    assert!(r.global_scope.variables.contains_key("a"));
    assert!(r.global_scope.variables.contains_key("b"));
}

#[test]
fn analyse_records_instance_class_set_new() {
    // `set d [Dog new]` maps `d` -> `::Dog`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} {} }\nset d [Dog new]\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("d").map(String::as_str),
        Some("::Dog")
    );
}

/// A rooted `::set d [Dog new]` binds `d` as `set` does: the
/// instance tracking reads `set`'s handle-binding layout, which the
/// rooted spelling resolves to, rather than comparing the spelling `set`
/// (tclsh 8.6.18 to 9.1b0: `$d bark` dispatches to `::Dog`).
#[test]
fn analyse_records_instance_class_rooted_set_new() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} {} }\n::set d [Dog new]\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("d").map(String::as_str),
        Some("::Dog")
    );
}

#[test]
fn analyse_records_instance_class_create_named() {
    // `Dog create rex` maps `rex` -> `::Dog`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} {} }\nDog create rex\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("rex").map(String::as_str),
        Some("::Dog"),
    );
}

#[test]
fn analyse_records_instance_class_set_create() {
    // `set d [Dog create rex]` maps `d` -> `::Dog`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog {}\nset d [Dog create rex]\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("d").map(String::as_str),
        Some("::Dog")
    );
}

#[test]
fn analyse_records_instance_class_through_a_var_headed_create() {
    // TP: tcllib's `httpd/httpd.tcl` flows the
    // constructor's class name through a single, unconditional `set`
    // one line earlier (`set class ::Derived; set obj [$class create
    // NAME]`) rather than writing the class as a literal bareword.
    // Verified against real tclsh9.0/8.6: `$obj`'s methods dispatch to
    // `Dog` either way, so the analyser must bind `obj` -> `::Dog`
    // exactly like the literal-bareword shape already does.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} {} }\nset class Dog\nset obj [$class create rex]\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("obj").map(String::as_str),
        Some("::Dog"),
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_records_instance_class_through_a_var_headed_new() {
    // TP — same gap, the `new` constructor spelling.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} {} }\nset class Dog\nset obj [$class new]\n",
        "tcl8.6",
    );
    assert_eq!(
        r.instance_classes.get("obj").map(String::as_str),
        Some("::Dog"),
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_rejects_a_var_headed_non_manufacturer_word() {
    // FP guard: the shape parser accepts any literal method so that the
    // registry can decide after `$class` resolves. A normal method must
    // not become a constructor merely because it follows a class handle.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Dog { method inspect {} { return value } }\nset class Dog\nset obj [$class inspect]\n",
            "tcl",
        );
    assert_eq!(
        r.instance_classes.get("obj"),
        None,
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_rejects_unexported_create_with_namespace_dispatch() {
    // TN/FP guard pinned against tclsh 9.0.4: `Dog
    // createWithNamespace x ::xns` fails with TCL LOOKUP METHOD. The
    // registry retains its structural layout for self-dispatch, but an
    // ordinary `$class` command cannot use it as construction evidence.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog {}\nset class Dog\nset obj [$class createWithNamespace x ::xns]\n",
        "tcl",
    );
    assert_eq!(
        r.instance_classes.get("obj"),
        None,
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_abstains_on_a_branch_ambiguous_class_var() {
    // TN — a class variable whose reaching definitions genuinely
    // disagree (one arm `Dog`, the other `Cat`) is unprovable at the
    // constructor call: binding *either* class would be a guess, so
    // the analyser must abstain (no `instance_classes` entry) rather
    // than pick one arbitrarily.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Dog {}\noo::class create Cat {}\nif {$flag} { set class Dog } else { set class Cat }\nset obj [$class create x]\n",
            "tcl",
        );
    assert_eq!(
        r.instance_classes.get("obj"),
        None,
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_abstains_on_a_genuinely_dynamic_class_var() {
    // TN — a class variable fed by a computed value (no exact
    // constant) can't be proven at all; must abstain exactly like the
    // pre-fix behaviour rather than binding a wrong/empty class.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog {}\nset class [someFactory]\nset obj [$class create x]\n",
        "tcl",
    );
    assert_eq!(
        r.instance_classes.get("obj"),
        None,
        "{:?}",
        r.instance_classes
    );
}

#[test]
fn analyse_does_not_record_class_definition_as_instance() {
    // `oo::class create Dog` defines a class, not an
    // instance — `Dog` must not appear in instance_classes.
    let mut a = Analyser::new();
    let r = a.analyse("oo::class create Dog {}\n", "tcl");
    assert!(
        !r.instance_classes.contains_key("Dog"),
        "class definition leaked into instance_classes: {:?}",
        r.instance_classes,
    );
}

#[test]
fn analyse_ignores_instance_of_unknown_class() {
    // `set d [Widget new]` where Widget isn't a user class
    // records nothing.
    let mut a = Analyser::new();
    let r = a.analyse("set d [Widget new]\n", "tcl");
    assert_eq!(r.instance_classes.len(), 0, "{:?}", r.instance_classes);
}

#[test]
fn analyse_namespace_eval_opens_scope() {
    let mut a = Analyser::new();
    let r = a.analyse("namespace eval ns1 { }", "tcl");
    assert_eq!(r.global_scope.children.len(), 1);
    assert_eq!(r.global_scope.children[0].name, "ns1");
}
