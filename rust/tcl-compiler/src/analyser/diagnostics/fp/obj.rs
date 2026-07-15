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

//! OBJ family — object dispatch (W307/W308) + snit / `TclOO` modelling.
//! Pairs to `tests/test_fp_obj.py`, `tests/test_fp_obj_var_as_cmd.py`, and
//! the §OBJ entries in `docs/design/compiler/FP.md`.

use super::{D, codes, fires};

// ---------------------------------------------------------------------------
// W250 — instantiating an `oo::abstract` class (new / create is a runtime
// error; abstract classes must be subclassed).
// ---------------------------------------------------------------------------

#[test]
fn w250_fires_on_abstract_new_and_create() {
    for shape in ["Base new", "Base create obj", "set o [Base new]"] {
        let src = format!("oo::abstract create Base {{}}\n{shape}\n");
        assert!(
            fires(&src, D, "W250"),
            "abstract instantiation `{shape}` did not fire W250: {:?}",
            codes(&src, D)
        );
    }
}

#[test]
fn w250_not_on_definition_or_concrete_subclass() {
    // The `oo::abstract create Base` definition must not fire; a concrete
    // subclass `Sub` (metaclass oo::class) instantiated normally must not.
    let src = "oo::abstract create Base {}\noo::class create Sub {\n    superclass Base\n}\nSub new\nset s [Sub create obj]\n";
    assert!(
        !fires(src, D, "W250"),
        "W250 false positive on definition / concrete subclass: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-01 — snit self-references ($self/$type/$selfns/$win) are method
// dispatch on the current object, not stray non-literal command words.
// ---------------------------------------------------------------------------

#[test]
fn fp_obj_01_snit_self_references_no_w307() {
    for r in ["self", "type", "selfns", "win"] {
        let src = format!("snit::type T {{\n method m {{}} {{ ${r} foo }}\n}}");
        assert!(
            !fires(&src, D, "W307"),
            "${r} foo in snit body fired W307: {:?}",
            codes(&src, D)
        );
    }
}

#[test]
fn fp_obj_01_self_ref_outside_snit_still_w307() {
    // TP control: the same names in a vanilla proc ARE stray dispatch.
    for r in ["self", "type", "selfns", "win", "hull"] {
        let src = format!("proc f {{}} {{ set {r} [getThing]\n ${r} foo }}");
        assert!(
            fires(&src, D, "W307"),
            "${r} foo outside snit did not fire W307"
        );
    }
}

// ---------------------------------------------------------------------------
// FP-OBJ-02 — $hull dispatch (widgetadaptor)
// ---------------------------------------------------------------------------

#[test]
fn fp_obj_02_widgetadaptor_hull_no_w307() {
    // FP: $hull configure is the canonical widgetadaptor delegation.
    let src = "snit::widgetadaptor W {\n method m {} { $hull configure -bg red }\n}";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-02: $hull configure in widgetadaptor must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-03 — snit component dispatch (instance-var method dispatch)
// ---------------------------------------------------------------------------

const FP_OBJ_03_REPRO_COMPONENT: &str = "\
snit::type T {
    component myexporter
    method run {fmt} {
        return [$myexporter export object $self $fmt]
    }
}
";

#[test]
fn fp_obj_03_component_dispatch_no_w307() {
    // FP-OBJ-03: a snit component is an object handle — dispatching on it inside
    // a method body is method dispatch.
    assert!(
        !fires(FP_OBJ_03_REPRO_COMPONENT, D, "W307"),
        "FP-OBJ-03: component dispatch must NOT fire W307; emitted: {:?}",
        codes(FP_OBJ_03_REPRO_COMPONENT, D)
    );
}

#[test]
fn fp_obj_03_constructor_dispatch_no_w307() {
    // FP-OBJ-03: same exemption applies in constructors.
    let src = "snit::type T {\n    variable handler\n    constructor {args} {\n        $handler reset\n    }\n}";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-03: constructor dispatch must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_03_typemethod_dispatch_no_w307() {
    // FP-OBJ-03: typevariable dispatch inside a typemethod body.
    let src = "snit::type T {\n    typevariable registry\n    typemethod lookup {k} {\n        return [$registry get $k]\n    }\n}";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-03: typemethod typevariable dispatch must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-04 — namespaced-factory provenance
// ---------------------------------------------------------------------------

const FP_OBJ_04_REPRO: &str = "\
proc f {} {
    # ::struct::tree is a namespaced factory; $t is an object handle.
    set t [::struct::tree mytree]
    $t walk root
}
";

#[test]
fn fp_obj_04_namespaced_factory_no_w307() {
    // FP-OBJ-04: a var assigned from a namespaced cmd-sub is treated as an object handle.
    assert!(
        !fires(FP_OBJ_04_REPRO, D, "W307"),
        "FP-OBJ-04: namespaced factory dispatch must NOT fire W307; emitted: {:?}",
        codes(FP_OBJ_04_REPRO, D)
    );
}

#[test]
fn fp_obj_04_short_namespace_form_no_w307() {
    // FP-OBJ-04: same applies to single-segment namespaced factories ([struct::matrix]).
    let src = "proc f {} {\n    set m [struct::matrix]\n    $m add row\n}";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-04: short-namespace factory dispatch must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_04_bare_unknown_command_still_w307() {
    // TP control: a bare (non-namespaced) unknown cmd-sub is not a factory.
    let src = "proc f {} { set x [foo bar]\n $x run }";
    assert!(
        fires(src, D, "W307"),
        "FP-OBJ-04 TP: bare-name cmd-sub must NOT exempt dispatch; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_04_factory_does_not_leak_across_procs() {
    // TP control: a factory assignment in one proc must not silence a same-named dispatch in another.
    let src = "proc a {} { set t [::struct::tree] }\nproc b {x} { set t $x\n $t foo }\n";
    assert!(
        fires(src, D, "W307"),
        "FP-OBJ-04 TP: factory must not propagate across proc boundaries; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_04_namespaced_string_returning_user_proc_fires_w307() {
    // TP: a namespaced user proc that returns a plain string should fire W307.
    let src = "\
namespace eval ::ns { proc make {} { return foo } }
proc f {} {
    set x [::ns::make]
    $x method
}
";
    assert!(
        fires(src, D, "W307"),
        "FP-OBJ-04 TP: string-returning namespaced proc should NOT be treated as factory; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_04_mixed_return_wrapper_fires_w307() {
    // TP: a wrapper proc that returns a factory result on one branch and a plain string on the
    // other should NOT be tagged object-returning.
    let src = "\
proc make {flag} {
    if {$flag} {
        return foo
    } else {
        return [::struct::tree]
    }
}
proc f {} {
    set x [make 1]
    $x method
}
";
    assert!(
        fires(src, D, "W307"),
        "FP-OBJ-04 TP: mixed-return wrapper should NOT be treated as object factory; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-05 — snit instance dispatch (locally-defined snit type)
// ---------------------------------------------------------------------------

const FP_OBJ_05_REPRO: &str = "\
snit::type ::Counter { method bump {} { return 1 } }
proc use {} {
    # `Counter create %AUTO%` returns a snit instance; $a bump is
    # method dispatch, not a stray non-literal command.
    set a [Counter create %AUTO%]
    $a bump
}
";

#[test]
fn fp_obj_05_snit_create_auto_no_w307() {
    // FP-OBJ-05: locally-defined snit-type's create-form returns OBJECT-typed value.
    assert!(
        !fires(FP_OBJ_05_REPRO, D, "W307"),
        "FP-OBJ-05: snit create %AUTO% dispatch must NOT fire W307; emitted: {:?}",
        codes(FP_OBJ_05_REPRO, D)
    );
}

#[test]
fn fp_obj_05_snit_create_named_no_w307() {
    // FP-OBJ-05: same for the named-create form.
    let src = "snit::type ::Counter { method bump {} { return 1 } }\nproc use {} {\n    set b [Counter create mine]\n    $b bump\n}\n";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-05: snit create named dispatch must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_05_snit_create_shorthand_no_w307() {
    // FP-OBJ-05: the create-shorthand `Foo %AUTO%` form.
    let src = "snit::type ::Counter { method bump {} { return 1 } }\nproc use {} {\n    set c [Counter %AUTO%]\n    $c bump\n}\n";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-05: snit shorthand create dispatch must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_05_snit_instance_no_w308() {
    // FP-OBJ-05: snit instances must not get W308 method validation.
    let src = "snit::type ::Foo { method bar {} {} }\nproc u {} { set o [Foo create %AUTO%]\n $o delegated_or_builtin }";
    assert!(
        !fires(src, D, "W308"),
        "FP-OBJ-05: snit instance must NOT fire W308; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-06 — snit private proc body IS analysed
// ---------------------------------------------------------------------------

#[test]
fn fp_obj_06_private_proc_body_analysed() {
    // FP-OBJ-06: a proc declared inside snit::type body is type-private but its body
    // must still be analysed. The `${a}($a)` scalar-vs-array smell inside the proc
    // body must still fire — proves the body isn't silently dropped.
    let src = "snit::type T {\n    proc Helper {a} {\n        return ${a}($a)\n    }\n}";
    let diags = codes(src, D);
    let w216_fires = diags.iter().any(|c| c == "W216");
    assert!(
        w216_fires,
        "FP-OBJ-06: snit private proc body must be analysed (expected W216); emitted: {diags:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-07 — cmd-sub namespaced ensemble [ns_func]::method is dispatch, not stray word
// ---------------------------------------------------------------------------

const FP_OBJ_07_REPRO: &str = "\
namespace eval ::ns {
    proc dispatch {} { return ::ns::sub }
    proc sub::work {arg} { return $arg }
}
[::ns::dispatch]::work hello
";

#[test]
fn fp_obj_07_cmdsub_namespaced_ensemble_no_w307() {
    // FP-OBJ-07: `[cmd]::method` is the namespaced-ensemble dispatch idiom — W307 must NOT fire.
    assert!(
        !fires(FP_OBJ_07_REPRO, D, "W307"),
        "FP-OBJ-07: cmd-sub namespaced ensemble [cmd]::method must NOT fire W307; emitted: {:?}",
        codes(FP_OBJ_07_REPRO, D)
    );
}

#[test]
fn fp_obj_07_bare_cmdsub_dispatch_still_fires() {
    // TP control: a bare `[cmd] $arg` dispatch with NO literal method-name tail still fires.
    let src = "[some_unknown_cmd] $arg\n";
    let diags = codes(src, D);
    let fires_any = diags
        .iter()
        .any(|c| c == "W307" || c == "W101" || c == "W101A");
    assert!(
        fires_any,
        "FP-OBJ-07 TP: bare cmd-sub dispatch must still produce an unresolved-command diagnostic; emitted: {diags:?}"
    );
}

// TP: const-prefix namespaced ensemble dispatch on unknown command (FP-OBJ-07 related)
#[test]
fn fp_obj_07_namespaced_ensemble_const_prefix_fires_w307() {
    // TP: `set ns nope` followed by `${ns}::missing arg` -- tclsh errors. W307 or W123 must fire.
    let src = "proc f {} {\n    set ns nope\n    ${ns}::missing arg\n}\n";
    let diags = codes(src, D);
    let fires_any = diags.iter().any(|c| c == "W307" || c == "W123");
    assert!(
        fires_any,
        "FP-OBJ-07 TP: const-prefix namespaced ensemble on unknown cmd should fire W307 or W123; emitted: {diags:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-08 — W307 suppressed on eval-substituted dispatch (W101 covers it)
// ---------------------------------------------------------------------------

const FP_OBJ_08_REPRO: &str = "\
proc f {cmd args} {
    # eval-substituted dispatch -- W101 already flags the eval-of-
    # substituted-string injection risk.  W307 reporting the same
    # site as \"stray non-literal command word\" is redundant noise.
    eval $cmd $args
}
";

#[test]
fn fp_obj_08_eval_substituted_dispatch_no_w307() {
    // FP-OBJ-08: `eval $cmd ...` is an eval-injection site that W101 already flags.
    // W307 piling on with "stray non-literal command word" is pure duplicate noise.
    assert!(
        !fires(FP_OBJ_08_REPRO, D, "W307"),
        "FP-OBJ-08: W307 must not fire on eval-substituted dispatch (W101 covers it); emitted: {:?}",
        codes(FP_OBJ_08_REPRO, D)
    );
}

#[test]
fn fp_obj_08_eval_substituted_dispatch_still_fires_w101() {
    // TP control: W101 (eval-injection) must still fire on the same site.
    assert!(
        fires(FP_OBJ_08_REPRO, D, "W101"),
        "FP-OBJ-08 TP: W101 must still fire on eval $cmd $args; emitted: {:?}",
        codes(FP_OBJ_08_REPRO, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-09 — W307 multi-dispatch local var (>=2 dispatches on same local)
// ---------------------------------------------------------------------------

const FP_OBJ_09_REPRO: &str = "\
proc analyze {G} {
    set TGraph [createTGraph $G]
    $TGraph node first
    $TGraph dispose
}
";

#[test]
fn fp_obj_09_multi_dispatch_local_no_w307() {
    // FP-OBJ-09: when a single proc dispatches >=2 times on the *same* local var,
    // the user has firmly treated that local as an object handle — W307 must not fire.
    assert!(
        !fires(FP_OBJ_09_REPRO, D, "W307"),
        "FP-OBJ-09: multi-dispatch on same local must NOT fire W307; emitted: {:?}",
        codes(FP_OBJ_09_REPRO, D)
    );
}

#[test]
fn fp_obj_09_single_dispatch_unknown_still_fires() {
    // TP control: a single dispatch on a local set from an unknown source still fires W307.
    let src = "proc f {} { set x [whatever]\n $x op }";
    assert!(
        fires(src, D, "W307"),
        "FP-OBJ-09 TP: single-dispatch on unknown source must still fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_09_const_string_multi_dispatch_fires_w307() {
    // TP / SCCP-evidence guard: `set cmd notacommand` makes `$cmd` a CONST string.
    // Dispatching it twice doesn't make it an object handle — W307 must fire
    // even though the multi-dispatch heuristic would otherwise suppress.
    let src = "proc f {} {\n    set cmd notacommand\n    $cmd a\n    $cmd b\n}\n";
    assert!(
        fires(src, D, "W307"),
        "FP-OBJ-09 TP: const-string multi-dispatch should fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-10 — W307 switch-callback array element ($state(-command))
// ---------------------------------------------------------------------------

const FP_OBJ_10_REPRO: &str = "\
proc h {state token} {
    $state(-command) $token
}
";

#[test]
fn fp_obj_10_dash_prefixed_array_key_callback_no_w307() {
    // FP-OBJ-10: `$state(-command) $token` -- the array element with a dash-prefixed key is
    // a switch-style callback slot. W307 must not fire on this dispatch shape.
    assert!(
        !fires(FP_OBJ_10_REPRO, D, "W307"),
        "FP-OBJ-10: dash-prefixed array key callback must NOT fire W307; emitted: {:?}",
        codes(FP_OBJ_10_REPRO, D)
    );
}

#[test]
fn fp_obj_10_suffix_keyed_callback_no_w307() {
    // FP-OBJ-10: array element keyed by a callback-shaped suffix is also a callback slot.
    let src = "proc h {state arg} { $state(doneCallback) $arg }";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-10: suffix-keyed callback must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_10_const_string_callback_suffix_fires_w307() {
    // TP / SCCP-evidence guard: `set state(doneCallback) notacommand` makes the
    // callback-shaped slot CONST. W307 must fire.
    let src =
        "proc f {} {\n    set state(doneCallback) notacommand\n    $state(doneCallback) a\n}\n";
    assert!(
        fires(src, D, "W307"),
        "FP-OBJ-10 TP: const callback-suffix dispatch should fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_10_const_string_dash_prefix_fires_w307() {
    // TP / SCCP-evidence guard: `set state(-foo) notacommand` -- same SCCP-override
    // applies to the dash-prefixed key heuristic.
    let src = "proc f {} {\n    set state(-foo) notacommand\n    $state(-foo) a\n}\n";
    assert!(
        fires(src, D, "W307"),
        "FP-OBJ-10 TP: const dash-prefix dispatch should fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-11 — W307 interprocedural object-factory tracking
// ---------------------------------------------------------------------------

const FP_OBJ_11_REPRO: &str = "\
proc createGraph {} { return [struct::tree] }
proc f {} { set t [createGraph]
$t op }
";

#[test]
fn fp_obj_11_factory_dispatch_no_w307() {
    // FP-OBJ-11: `createGraph` directly returns `[struct::tree]` (a namespaced object factory).
    // Interproc fixpoint inference marks createGraph as object-returning, so callers are suppressed.
    assert!(
        !fires(FP_OBJ_11_REPRO, D, "W307"),
        "FP-OBJ-11: interprocedural factory dispatch must NOT fire W307; emitted: {:?}",
        codes(FP_OBJ_11_REPRO, D)
    );
}

#[test]
fn fp_obj_11_transitive_factory_no_w307() {
    // FP-OBJ-11: transitive factory chain -- `createGraph` returns `$t` where `$t` was set
    // from another factory. The fixpoint propagates the OBJECT-RETURNING attribute transitively.
    let src = "\
proc factory {} { return [struct::tree] }
proc createGraph {} { set t [factory]
return $t }
proc f {} { set g [createGraph]
$g op }
";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-11: transitive factory dispatch must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-12 — W307 fires on [<cmd-sub>] run in a method body (D3-P3/D4-F5)
// ---------------------------------------------------------------------------

const FP_OBJ_12_REPRO: &str =
    "oo::class create C {\n    method m {} { [format notACommand] run }\n}";

#[test]
fn fp_obj_12_format_in_method_fires() {
    // TP: `[format notACommand] run` inside a method body must fire W307.
    // D4-F5 closure removed the in-method blanket suppression.
    assert!(
        fires(FP_OBJ_12_REPRO, D, "W307"),
        "FP-OBJ-12 TP: [format X] run inside a method body must fire W307 after D4-F5; emitted: {:?}",
        codes(FP_OBJ_12_REPRO, D)
    );
}

#[test]
fn fp_obj_12_known_class_new_in_method_silent() {
    // TN control: `[D new] run` IS suppressed because D's constructor return type is known OBJECT.
    let src = "oo::class create D { method run {} { return ok } }\noo::class create C { method m {} { [D new] run } }\n";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-12: [D new] run must NOT fire W307 when D is a known class; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-13 — W307 fires on [my plain] where plain returns literal (D3-P4)
// ---------------------------------------------------------------------------

const FP_OBJ_13_REPRO: &str = "oo::class create C {\n    method plain {} { return notACommand }\n    method m {} { [my plain] run }\n}";

#[test]
fn fp_obj_13_my_method_returns_literal_fires() {
    // TP: when `[my plain]` resolves to a method whose body is a simple `return <literal>`
    // (no cmd-sub, no var interpolation), the return is provably STRING — fire W307.
    assert!(
        fires(FP_OBJ_13_REPRO, D, "W307"),
        "FP-OBJ-13 TP: [my plain] returning literal must fire W307 after D3-P4 closure; emitted: {:?}",
        codes(FP_OBJ_13_REPRO, D)
    );
}

#[test]
fn fp_obj_13_my_method_returns_object_silent() {
    // TN control: when `[my obj]` body returns `[D new]` (a cmd-sub), the conservative
    // self-dispatch OBJECT suppression holds.
    let src = "oo::class create D { method run {} { return ok } }\noo::class create C {\n    method obj {} { return [D new] }\n    method m {} { [my obj] run }\n}";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-13: [my obj] returning [D new] must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-14 — registered ::ns::cmd / known user proc with non-OBJECT return overrides
// the ::-prefix factory heuristic (D3-P5 PARTIAL / D4-F6)
// ---------------------------------------------------------------------------

const FP_OBJ_14_REPRO: &str = "\
namespace eval ::pkg { proc plain {} { return notACommand } }
proc f {} {
    set x [::pkg::plain]
    $x op
}
";

#[test]
fn fp_obj_14_namespaced_user_proc_non_object_return_fires() {
    // TP: `::pkg::plain` is a known user proc whose interproc fixpoint result is NOT
    // object-returning. D4-F6 partial closure overrides the ::-prefix factory heuristic.
    assert!(
        fires(FP_OBJ_14_REPRO, D, "W307"),
        "FP-OBJ-14 TP: namespaced user proc with plain-string return must fire W307; emitted: {:?}",
        codes(FP_OBJ_14_REPRO, D)
    );
}

#[test]
fn fp_obj_14_namespaced_known_object_factory_silent() {
    // TN control: when `::pkg::Tree` IS a known TclOO class (in known_classes),
    // the `[::pkg::Tree new]` factory site IS typed OBJECT and the dispatch stays suppressed.
    let src =
        "oo::class create ::pkg::Tree {}\nproc f {} {\n    set x [::pkg::Tree new]\n    $x op\n}\n";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-14: known TclOO class factory must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_14_unregistered_external_namespaced_still_silent() {
    // Deferred-coverage TN: an unregistered external `::pkg::plain` with NO proc visible
    // AND no registry spec still suppresses W307.
    let src = "proc f {} {\n    set x [::pkg::plain]\n    $x op\n}\n";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-14: unregistered external ::pkg::plain still suppresses W307 (deferred until D1-11); emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-15 — bare-name [NotAClass new] no longer suppressed (D3-P6/D4-F6)
// ---------------------------------------------------------------------------

const FP_OBJ_15_REPRO: &str = "proc f {} { set x [NotAClass new]; $x method }\n";

#[test]
fn fp_obj_15_unknown_class_new_fires() {
    // TP: `[NotAClass new]` MUST fire W307 -- the analyser has no evidence that
    // NotAClass is an object factory. D4-F6 closure removed the bare-`new`-subcommand heuristic.
    assert!(
        fires(FP_OBJ_15_REPRO, D, "W307"),
        "FP-OBJ-15 TP: [NotAClass new] must fire W307; emitted: {:?}",
        codes(FP_OBJ_15_REPRO, D)
    );
}

#[test]
fn fp_obj_15_known_oo_class_new_silent() {
    // TN control: `[C new]` where C IS a known oo::class correctly suppresses W307.
    let src =
        "oo::class create C { method run {} { return ok } }\nproc f {} { set x [C new]; $x run }\n";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-15: [C new] with known class must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-16 — composed ${ns}::tail ensemble lookup runs unconditionally (D4-F7)
// ---------------------------------------------------------------------------

const FP_OBJ_16_REPRO: &str = "\
namespace eval ::mypkg { proc dowork {arg} {} }
proc f {} { set ns mypkg; ${ns}::dowork arg }
";

#[test]
fn fp_obj_16_const_prefix_resolves_to_known_proc_silent() {
    // FP-OBJ-16: when `${ns}::dowork` resolves (via SCCP CONST(ns='mypkg') composed with
    // the literal tail) to a known proc, W307 must NOT fire.
    assert!(
        !fires(FP_OBJ_16_REPRO, D, "W307"),
        "FP-OBJ-16: composed ${{ns}}::dowork must NOT fire W307 when ::mypkg::dowork is a known proc; emitted: {:?}",
        codes(FP_OBJ_16_REPRO, D)
    );
}

#[test]
fn fp_obj_16_const_prefix_unknown_proc_fires() {
    // TP control: `${ns}::unknownproc` where the composed name has NO known proc must still fire W307.
    let src = "proc f {} { set ns mypkg; ${ns}::unknownproc arg }\n";
    assert!(
        fires(src, D, "W307"),
        "FP-OBJ-16 TP: composed ${{ns}}::unknownproc must fire W307 when not resolvable; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-17 — array set literal-element harvester for callback array (D3-P7)
// ---------------------------------------------------------------------------

const FP_OBJ_17_REPRO: &str =
    "proc f {} { array set state {-command notACommand}; $state(-command) hi }\n";

#[test]
fn fp_obj_17_callback_array_holds_noncommand_fires() {
    // TP: `array set state {-command notACommand}; $state(-command) hi`
    // MUST fire W307 -- the literal element value is a non-command, so the
    // callback-key heuristic suppression is overridden by the SCCP-CONST evidence.
    assert!(
        fires(FP_OBJ_17_REPRO, D, "W307"),
        "FP-OBJ-17 TP: callback array holds non-command literal must fire W307; emitted: {:?}",
        codes(FP_OBJ_17_REPRO, D)
    );
}

#[test]
fn fp_obj_17_callback_array_holds_known_command_silent() {
    // TN control: `array set state {-command puts}` -- the literal value IS a known command,
    // so W307 stays silent.
    let src = "proc f {} { array set state {-command puts}; $state(-command) hi }\n";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-17: callback array holding known command must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-18 — dict with key-value pair harvester for interproc callback (D3-P8)
// ---------------------------------------------------------------------------

const FP_OBJ_18_REPRO: &str = "proc f {d} { dict with d { $cmd hi } }\nf {cmd notACommand}\n";

#[test]
fn fp_obj_18_interproc_dict_with_noncommand_fires() {
    // TP: caller passes literal `{cmd notACommand}`; interproc propagation puts the literal
    // dict in d at v0; dict-with-key harvester registers cmd -> notACommand as CONSTSET;
    // the W307 SCCP-evidence override fires.
    assert!(
        fires(FP_OBJ_18_REPRO, D, "W307"),
        "FP-OBJ-18 TP: interproc-propagated callback non-command must fire W307; emitted: {:?}",
        codes(FP_OBJ_18_REPRO, D)
    );
}

#[test]
fn fp_obj_18_interproc_dict_with_known_command_silent() {
    // TN control: same shape but the unpacked value IS a known command (`puts`). W307 must NOT fire.
    let src = "proc f {d} { dict with d { $cmd hi } }\nf {cmd puts}\n";
    assert!(
        !fires(src, D, "W307"),
        "FP-OBJ-18: interproc dict-with known command must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// Follow-up findings (not tagged FP-OBJ-NN but in test_fp_obj.py)
// ---------------------------------------------------------------------------

#[test]
fn fp_w307_dict_with_does_not_suppress_explicit_local_dispatch() {
    // TP: explicit `set cmd nope` in same proc as `dict with d {}` must still fire W307 on $cmd.
    let src = "proc f {} { set d {}\n dict with d {}\n set cmd nope\n $cmd arg }";
    assert!(
        fires(src, D, "W307"),
        "explicit set cmd in same proc as dict with must still fire W307 on $cmd; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_w307_oo_class_bare_name_factory_propagates() {
    // TN: factory proc returning [C new] (bare TclOO class command) must propagate object
    // provenance and suppress W307.
    let src = "oo::class create C {}\nproc make {} { return [C new] }\nproc f {} { set x [make]\n $x destroy }\n";
    assert!(
        !fires(src, D, "W307"),
        "factory proc returning [C new] must propagate object provenance and suppress W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_w307_oo_class_method_local_literal_fires() {
    // TP: a bare local set to a literal non-command inside an oo::class method body must
    // fire W307 — `in_method` is no longer a blanket suppression.
    let src = "oo::class create C {\n    method m {} { set cmd nope; $cmd arg }\n}";
    assert!(
        fires(src, D, "W307"),
        "local 'set cmd nope' inside method body must fire W307 on $cmd; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_w307_oo_class_instance_var_dispatch_silent() {
    // FP: a dispatch on an oo::class instance variable inside a method body is suppressed.
    let src = "oo::class create C {\n    variable handle\n    method m {} { $handle op }\n}";
    assert!(
        !fires(src, D, "W307"),
        "dispatch on declared instance var must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_w307_snit_typevariable_dispatch_silent() {
    // FP: snit typevariables dispatch inside typemethod stays suppressed.
    let src = "snit::type T {\n    typevariable registry\n    typemethod lookup {k} { return [$registry get $k] }\n}";
    assert!(
        !fires(src, D, "W307"),
        "snit typevariable dispatch must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_w307_itcl_instance_var_dispatch_silent() {
    // FP: a dispatch on an itcl instance variable inside a method body is
    // suppressed, exactly like an oo::class / snit member.
    let src = "itcl::class C {\n    variable handle\n    method m {} { $handle op }\n}";
    assert!(
        !fires(src, D, "W307"),
        "dispatch on an itcl instance var must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_w307_itcl_created_instance_dispatch_silent() {
    // FP: an itcl object captured from a factory call (`ClassName #auto`) and
    // dispatched by `$var method` is a known-created instance — no W307.
    let src = "itcl::class Counter { method bump {} { return 1 } }\nproc use {} {\n    set c [Counter #auto]\n    $c bump\n}\n";
    assert!(
        !fires(src, D, "W307"),
        "dispatch on a created itcl instance must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// VAR-as-command dispatch tests (from test_fp_obj_var_as_cmd.py)
// Pair 1: local literal assignment of a user-proc / builtin name
// ---------------------------------------------------------------------------

#[test]
fn fp_var_as_cmd_literal_user_proc_silent() {
    // TN: `set cmd parse; $cmd x` — `parse` is a user proc, so the dispatch is statically known.
    let src = "proc parse {a} { return $a }\nproc d {} { set cmd parse; $cmd 5 }\n";
    assert!(
        !fires(src, D, "W307"),
        "VAR-as-cmd: set cmd parse; $cmd 5 must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_var_as_cmd_literal_builtin_silent() {
    // TN: `set cmd incr; $cmd v` — `incr` is a registered builtin.
    let src = "proc d {} { set v 0; set cmd incr; $cmd v }\n";
    assert!(
        !fires(src, D, "W307"),
        "VAR-as-cmd: set cmd incr; $cmd v must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_var_as_cmd_literal_non_command_fires() {
    // TP: `set cmd notacommand; $cmd x` — SCCP proves the value is the literal `notacommand`
    // which is not a command.
    let src = "proc d {} { set cmd notacommand; $cmd 5 }\n";
    assert!(
        fires(src, D, "W307"),
        "VAR-as-cmd TP: set cmd notacommand must fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// Pair 2: interprocedural param flow (D3-P2 seeding)

#[test]
fn fp_var_as_cmd_param_flow_single_caller_silent() {
    // TN: every caller passes the literal command name `parse` for the dispatched param.
    let src = "proc parse {a} { return $a }\nproc run {cmd} { $cmd 5 }\nproc d {} { run parse }\n";
    assert!(
        !fires(src, D, "W307"),
        "VAR-as-cmd: interproc param flow with known command must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_var_as_cmd_param_flow_non_command_fires() {
    // TP: the only caller passes `notacommand` for the dispatched param.
    let src = "proc run {cmd} { $cmd 5 }\nproc d {} { run notacommand }\n";
    assert!(
        fires(src, D, "W307"),
        "VAR-as-cmd TP: interproc non-command param must fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// Pair 3: phi join (branch-merged definitions)

#[test]
fn fp_var_as_cmd_phi_two_commands_silent() {
    // TN: both branches assign a known command, so the phi-joined value is a CONSTSET
    // of all-known commands.
    let src = "proc parse {a} { return $a }\nproc lint {a} { return $a }\nproc d {f} {\n    if {$f} { set c parse } else { set c lint }\n    $c 5\n}\n";
    assert!(
        !fires(src, D, "W307"),
        "VAR-as-cmd: phi join of two known commands must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_var_as_cmd_phi_command_and_non_command_fires() {
    // TP: one branch assigns a non-command, so the phi-joined set contains
    // a value that is not a command and the dispatch may be invalid.
    let src = "proc parse {a} { return $a }\nproc d {f} {\n    if {$f} { set c parse } else { set c notacommand }\n    $c 5\n}\n";
    assert!(
        fires(src, D, "W307"),
        "VAR-as-cmd TP: phi join with non-command must fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// Pair 4: per-SSA-version precision (reassignment before dispatch)

#[test]
fn fp_var_as_cmd_reassign_to_command_silent() {
    // TN: `set c notacommand; set c parse; $c x` — only the `parse` version reaches
    // the dispatch, so the earlier `notacommand` write must not keep W307 alive.
    let src = "proc parse {a} { return $a }\nproc d {} {\n    set c notacommand\n    set c parse\n    $c 5\n}\n";
    assert!(
        !fires(src, D, "W307"),
        "VAR-as-cmd: reassigned to known command before dispatch must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_var_as_cmd_reassign_to_non_command_fires() {
    // TP: the last write reaching the dispatch is a non-command.
    let src = "proc parse {a} { return $a }\nproc d {} {\n    set c parse\n    set c notacommand\n    $c 5\n}\n";
    assert!(
        fires(src, D, "W307"),
        "VAR-as-cmd TP: reassigned to non-command before dispatch must fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// Conservatism: mixed callers stay silent (no FP cascade)

#[test]
fn fp_var_as_cmd_mixed_callers_conservative_silent() {
    // TN (intentionally conservative): when distinct callers pass different *known* command
    // names, the CONSTSET is all-known and W307 stays silent.
    let src = "proc parse {a} { return $a }\nproc lint {a} { return $a }\nproc run {c} { $c 5 }\nproc a {} { run parse }\nproc b {} { run lint }\n";
    assert!(
        !fires(src, D, "W307"),
        "VAR-as-cmd: mixed callers with all-known commands must NOT fire W307; emitted: {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-19 — `CLASS create NAME` binds a command NAME; later `NAME method`
// dispatch (and `$var method` where var provably holds NAME) is a real call,
// not an unknown command / stray dispatch.  Issue #777.
// ---------------------------------------------------------------------------

#[test]
fn fp_obj_19_external_class_create_name_no_w123() {
    // FP: `C`/`L` are external-package classes (unknown to the analyser).  The
    // `create NAME` idiom still binds command names `c1`/`l1`, so literal
    // dispatch on them must NOT fire W123 (the unknown *class* command `C`/`L`
    // itself still does — that is a separate, correct diagnostic).
    let src = "C create c1 1 out 0\nc1 configure -c 2\n";
    // `c1` is not among the unresolved-command sites.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(src, D);
    assert!(
        !r.unresolved_command_sites.iter().any(|(_, n)| n == "c1"),
        "created instance command `c1` must not be an unresolved command: {:?}",
        r.unresolved_command_sites,
    );
    // The unknown class command `C` is still reported.
    assert!(
        r.unresolved_command_sites.iter().any(|(_, n)| n == "C"),
        "unknown class command `C` must still be reported: {:?}",
        r.unresolved_command_sites,
    );
}

#[test]
fn fp_obj_19_known_class_create_name_no_w123() {
    // FP: for a known TclOO class, `C create c1` then `c1 method` must be clean.
    let src = "oo::class create C { method configure args {} }\nC create c1\nc1 configure -c 2\n";
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(src, D);
    assert!(
        !r.unresolved_command_sites.iter().any(|(_, n)| n == "c1"),
        "instance command of a known class must not be unresolved: {:?}",
        r.unresolved_command_sites,
    );
}

#[test]
fn fp_obj_19_created_name_via_var_no_w307() {
    // FP: a created command name flowing through a variable — the dispatch is on
    // a value SCCP proves is a known (created) command, so W307 must not fire.
    let src = "C create c1 1 out 0\nset e c1\n$e configure\n";
    assert!(
        !fires(src, D, "W307"),
        "dispatch on a var holding a created command name must NOT fire W307; emitted: {:?}",
        codes(src, D),
    );
}

#[test]
fn fp_obj_19_created_names_via_list_foreach_no_w307() {
    // FP (exact repro of issue #777's screenshot): the created object names are
    // iterated with `foreach elem [list c1 l1 …]` and dispatched via `$elem`.
    // SCCP folds the `[list …]` to the element set, each of which is a created
    // command, so W307 must not fire.
    let src = "\
C create c1 1 out 0 -c 1e-9
L create l1 1 out 0 -l 10e-6
C create c2 2 n002 0 -c 1e-9
foreach elem [list c1 l1 c2] {
    $elem actOnParam -set 1
}
";
    assert!(
        !fires(src, D, "W307"),
        "dispatch over `[list c1 l1 …]` of created names must NOT fire W307; emitted: {:?}",
        codes(src, D),
    );
}

#[test]
fn fp_obj_19_uncreated_name_via_list_foreach_still_w307() {
    // TP control: a `[list …]` containing a name that was never created keeps
    // W307 alive — the element set is not all-known-commands.
    let src = "\
C create c1 1 out 0
foreach elem [list c1 nope] {
    $elem actOnParam
}
";
    assert!(
        fires(src, D, "W307"),
        "a list with an uncreated name must still fire W307; emitted: {:?}",
        codes(src, D),
    );
}

#[test]
fn fp_obj_19_uncreated_name_still_w123() {
    // TP control: registering `c1` must not silence a *different* undefined
    // command `d1` — it is still an unknown command.
    let src = "C create c1 1 out 0\nd1 configure\n";
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(src, D);
    assert!(
        r.unresolved_command_sites.iter().any(|(_, n)| n == "d1"),
        "an uncreated name must still be unresolved: {:?}",
        r.unresolved_command_sites,
    );
}

#[test]
fn fp_obj_19_dict_create_key_is_not_a_command() {
    // TP control: `dict create` builds a dict VALUE — its key words are not
    // command names.  `dict` is a known builtin, so the `create NAME` idiom must
    // not register `foo`; dispatching `foo` still fires W123.
    let src = "dict create foo bar\nfoo x\n";
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(src, D);
    assert!(
        r.unresolved_command_sites.iter().any(|(_, n)| n == "foo"),
        "`dict create foo` must not register `foo` as a command: {:?}",
        r.unresolved_command_sites,
    );
}

// ---------------------------------------------------------------------------
// FP-OBJ-20 — an external (unindexed) mixin contributes methods through the
// MRO exactly as an external superclass does, so the W308 external-class
// escape hatch must cover mixins too.
// ---------------------------------------------------------------------------

const FP_OBJ_20_REPRO: &str = "\
oo::class create Reactive {
    mixin ::somelib::Observable
    method local {} {}
}
set o [Reactive new]
$o subscribe handler
";

#[test]
fn fp_obj_20_external_mixin_method_no_w308() {
    // FP: `subscribe` comes from the external mixin ::somelib::Observable —
    // the class's callable method set is unknowable, so W308 must abstain
    // exactly as it does for an external superclass.
    assert!(
        !fires(FP_OBJ_20_REPRO, D, "W308"),
        "FP-OBJ-20: method from external mixin must NOT fire W308; emitted: {:?}",
        codes(FP_OBJ_20_REPRO, D)
    );
}

#[test]
fn fp_obj_20_external_mixin_cmdsub_dispatch_no_w308() {
    // FP: the same abstention on the `[Reactive new] subscribe` cmd-sub
    // dispatch shape (the `validate_method_on_class` W308 path).
    let src = "oo::class create Reactive {\n    mixin ::somelib::Observable\n    method local {} {}\n}\n[Reactive new] subscribe handler\n";
    assert!(
        !fires(src, D, "W308"),
        "FP-OBJ-20: cmd-sub dispatch of external-mixin method must NOT fire W308; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_20_all_local_class_unknown_method_still_w308() {
    // TP control: with no external superclass or mixin the method set IS
    // fully known — an unknown method must still fire W308.
    let src = "oo::class create Plain { method local {} {} }\nset o [Plain new]\n$o nosuch\n";
    assert!(
        fires(src, D, "W308"),
        "FP-OBJ-20 TP: unknown method on all-local class must fire W308; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_20_local_mixin_unknown_method_still_w308() {
    // TP control: a *local* (indexed) mixin is fully introspectable, so it
    // must not blanket-suppress — an unknown method still fires W308.
    let src = "oo::class create ::Observable { method subscribe {h} {} }\noo::class create Reactive {\n    mixin ::Observable\n}\nset o [Reactive new]\n$o nosuch\n";
    assert!(
        fires(src, D, "W308"),
        "FP-OBJ-20 TP: unknown method on class with all-local mixin must fire W308; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_obj_20_local_mixin_provides_method_silent() {
    // Regression: an in-file mixin providing the method resolves through the
    // MRO — silent, with no reliance on the external-mixin abstention.
    let src = "oo::class create ::Observable { method subscribe {h} {} }\noo::class create Reactive {\n    mixin ::Observable\n    method local {} {}\n}\nset o [Reactive new]\n$o subscribe handler\n";
    assert!(
        !fires(src, D, "W308"),
        "FP-OBJ-20: in-file mixin providing the method must NOT fire W308; emitted: {:?}",
        codes(src, D)
    );
}
