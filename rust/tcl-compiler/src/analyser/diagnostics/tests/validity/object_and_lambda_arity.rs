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

//! Arity of `TclOO` methods, forwards, constructors and `next`, and of `apply` lambdas.

use super::*;

// TclOO method / `forward` arity (generalises E002/E003 to object
// dispatch), all verified against real tclsh 9.0.4.

fn e00x_codes_for(src: &str) -> Vec<String> {
    e00x_codes_for_in(src, "tcl")
}

fn e00x_codes_for_in(src: &str, dialect: &str) -> Vec<String> {
    let mut a = Analyser::new();
    let r = a.analyse(src, dialect);
    let mut codes: Vec<String> = r
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .filter(|c| c == "E002" || c == "E003")
        .collect();
    codes.sort();
    codes
}

#[test]
fn tcloo_method_arity_fires_and_stays_silent() {
    let src = |call: &str| {
        format!(
            "oo::class create Widget {{ method bar {{x y}} {{ return \"$x+$y\" }} }}\n\
             set f1 [Widget new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$f1 bar 1")), vec!["E002".to_owned()]);
    assert_eq!(e00x_codes_for(&src("$f1 bar 1 2")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$f1 bar 1 2 3")),
        vec!["E003".to_owned()]
    );
}

#[test]
fn tcloo_loop_installed_method_arity_abstains() {
    // A literal `foreach`-installed member's parameter list is
    // deliberately never re-derived (`params_computed`), so a call of any
    // arity must draw neither E002 nor E003 — tclsh 9.0.4 / 8.6.16 both
    // agree `alpha` really does take any number of arguments (`args`), but
    // even if it took none this must still abstain: the point is that the
    // analyser never claims to know.
    let src = |call: &str| {
        format!(
            "oo::class create Widget {{\n\
                 foreach m {{alpha beta gamma}} {{ method $m {{args}} {{ return $args }} }}\n\
             }}\n\
             set f1 [Widget new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$f1 alpha")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$f1 alpha 1 2 3 4 5")),
        Vec::<String>::new()
    );
    // Non-vacuity: an *ordinary*, non-loop-installed method with the exact
    // same call shape still fires — proving the abstention is specific to
    // the loop-installed shape (a fixed two-parameter method really does
    // draw E002/E003 on the same harness).
    let ordinary = |call: &str| {
        format!(
            "oo::class create Widget2 {{ method bar {{x y}} {{ return \"$x+$y\" }} }}\n\
             set f1 [Widget2 new]\n{call}\n"
        )
    };
    assert_eq!(
        e00x_codes_for(&ordinary("$f1 bar 1")),
        vec!["E002".to_owned()]
    );
}

#[test]
fn tcloo_method_with_default_and_trailing_args_is_unbounded() {
    // tclsh 9.0.4: `method baz {x {y 1} args}` accepts any count >= 1.
    let src = |call: &str| {
        format!(
            "oo::class create Widget {{ method baz {{x {{y 1}} args}} {{ return $x }} }}\n\
             set f1 [Widget new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$f1 baz 1")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$f1 baz 1 2 3 4")),
        Vec::<String>::new()
    );
    assert_eq!(e00x_codes_for(&src("$f1 baz")), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_forward_arity_is_shifted_by_prepended_args() {
    // tclsh 9.0.4: `forward fwd target_orig` (no prepended args) inherits
    // target_orig's own 3-arg arity exactly.
    let src = |call: &str| {
        format!(
            "proc target_orig {{a b c}} {{}}\n\
             oo::class create Widget {{ forward fwd target_orig }}\n\
             set f1 [Widget new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$f1 fwd 1 2")), vec!["E002".to_owned()]);
    assert_eq!(e00x_codes_for(&src("$f1 fwd 1 2 3")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$f1 fwd 1 2 3 4")),
        vec!["E003".to_owned()]
    );
}

#[test]
fn tcloo_forward_to_bare_method_name_abstains() {
    // A bare method name is *never* a resolvable `forward` target — tclsh
    // 9.0.4 fails at run time with `invalid command name "base"`, since
    // `forward`'s TARGET is looked up as an ordinary command and a
    // `method` never creates one. Must abstain, not guess an arity.
    let src = "\
oo::class create Bad { method base {a b c} {} forward fwdBad base }
set b1 [Bad new]
$b1 fwdBad 1 2 3 4 5
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_forward_via_my_resolves_sibling_method_arity() {
    // `forward NAME my TARGET ?ARG…?` is the documented, working idiom
    // for forwarding to a sibling method (`self` is not usable here —
    // tclsh 9.0.4: it errors "self may only be called from inside a
    // method" since forwarding doesn't run inside a method body). `my`'s
    // target resolves through the receiver's own MRO, arity-shifted by
    // any args placed after it — confirmed against tclsh 9.0.4.
    let src = |call: &str| {
        format!(
            "oo::class create Widget {{\n\
               method base {{a b c}} {{ return \"$a$b$c\" }}\n\
               forward fwd my base\n\
               forward fwdShift my base fixedarg\n\
             }}\n\
             set f1 [Widget new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$f1 fwd 1 2")), vec!["E002".to_owned()]);
    assert_eq!(e00x_codes_for(&src("$f1 fwd 1 2 3")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$f1 fwd 1 2 3 4")),
        vec!["E003".to_owned()]
    );
    assert_eq!(
        e00x_codes_for(&src("$f1 fwdShift 1")),
        vec!["E002".to_owned()]
    );
    assert_eq!(
        e00x_codes_for(&src("$f1 fwdShift 1 2")),
        Vec::<String>::new()
    );
    assert_eq!(
        e00x_codes_for(&src("$f1 fwdShift 1 2 3")),
        vec!["E003".to_owned()]
    );
}

#[test]
fn tcloo_forward_via_my_resolves_inherited_method_arity() {
    // `my`'s target resolution walks the receiver's full MRO, so a
    // `forward … my …` declared on a subclass reaches a method defined
    // only on a superclass — confirmed against tclsh 9.0.4.
    let src = |call: &str| {
        format!(
            "oo::class create Base {{ method base {{a b c}} {{ return \"$a$b$c\" }} }}\n\
             oo::class create Derived {{ superclass Base\n forward fwd my base }}\n\
             set d1 [Derived new]\n{call}\n"
        )
    };
    assert_eq!(e00x_codes_for(&src("$d1 fwd 1 2")), vec!["E002".to_owned()]);
    assert_eq!(e00x_codes_for(&src("$d1 fwd 1 2 3")), Vec::<String>::new());
    assert_eq!(
        e00x_codes_for(&src("$d1 fwd 1 2 3 4")),
        vec!["E003".to_owned()]
    );
}

#[test]
pub(in super::super) fn tcloo_method_arity_abstains_for_ambiguous_receiver_class() {
    // `$f1` could hold either `A` or `B` — disjoint arities for `same` —
    // must not guess; no E002/E003 either way.
    let src = "\
oo::class create A { method same {x} {} }
oo::class create B { method same {x y} {} }
if {$cond} { set f1 [A new] } else { set f1 [B new] }
$f1 same 1
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_instance_dispatch_abstains_for_class_side_method() {
    // A `self method` (stored in `class_methods`) is called on the class
    // object, never on an instance — confirmed against tclsh 9.0.4:
    // `set o [C new]; $o make 1` fails "unknown method", while `C make
    // 1 2` succeeds. Must abstain (no E002/E003), not compute arity from
    // a signature the instance dispatch could never actually reach.
    let src = "\
oo::class create C {
    self method make {a b} { return \"$a$b\" }
}
set o [C new]
$o make 1
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

// -- TclOO constructor call-site arity (`ClassName new` / `ClassName create`)

// naming.source.original-class-constructor-call
// docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
// Conditional constructor-source controls select the C9 formal grammar used
// by their cited CLI examples. No constructor entry or actual argv is proved.
fn tcloo_constructor_codes_for(src: &str) -> Vec<String> {
    e00x_codes_for_in(src, "tcl9.0")
}

#[test]
fn tcloo_constructor_signature_advice_keeps_unknown_formal_grammar_separate_from_authored_names() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let plain =
        tcl_registry::InvocationDialect::of_profile(tcl_dialect::DialectProfile::plain_tcl());
    assert!(plain.authored_name_policy().is_some());
    assert!(plain.native_name_protocol().is_none());
    assert!(plain.parameter_grammar().is_none());
    let source = "oo::class create C {constructor {a b} { }}\nC new 1";
    assert_eq!(e00x_codes_for(source), [] as [std::string::String; 0]);
    assert_eq!(tcloo_constructor_codes_for(source), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_constructor_new_arity_fires_and_stays_silent() {
    let src =
        |call: &str| format!("oo::class create Widget {{ constructor {{a b}} {{ }} }}\n{call}\n");
    assert_eq!(
        tcloo_constructor_codes_for(&src("Widget new 1")),
        vec!["E002".to_owned()]
    );
    assert_eq!(
        tcloo_constructor_codes_for(&src("Widget new 1 2")),
        Vec::<String>::new()
    );
    assert_eq!(
        tcloo_constructor_codes_for(&src("Widget new 1 2 3")),
        vec!["E003".to_owned()]
    );
}

#[test]
fn tcloo_constructor_create_arity_accounts_for_mandatory_name() {
    // `create` consumes one mandatory word (the object name) ahead of the
    // constructor's own parameters — confirmed against tclsh 9.0.4:
    // `Widget create fido 1` fails "should be Widget create objectName a b".
    let src =
        |call: &str| format!("oo::class create Widget {{ constructor {{a b}} {{ }} }}\n{call}\n");
    assert_eq!(
        tcloo_constructor_codes_for(&src("Widget create fido 1")),
        vec!["E002".to_owned()]
    );
    assert_eq!(
        tcloo_constructor_codes_for(&src("Widget create fido 1 2")),
        Vec::<String>::new()
    );
    assert_eq!(
        tcloo_constructor_codes_for(&src("Widget create")),
        vec!["E002".to_owned()],
        "the mandatory object-name word itself must be enforced"
    );
}

#[test]
fn tcloo_constructor_createwithnamespace_arity_accounts_for_two_mandatory_words() {
    // `createWithNamespace` consumes two mandatory words (the object name
    // and the target namespace) ahead of the constructor's own
    // parameters — same word layout as the sibling class-*definition*
    // shape `oo::class createWithNamespace Name ::ns body` (see
    // `oo_class_arg_roles`), just with constructor args standing in for
    // the definition body. Unlike `new`/`create`, `createWithNamespace` is
    // unexported by default (confirmed against tclsh 9.0.4), so the class
    // object must `self export` it for an external
    // call to even reach the constructor — see the companion
    // `..._is_not_checked_when_not_exported` test for the unexported case.
    let src = |call: &str| {
        format!(
            "oo::class create Widget {{ constructor {{a b}} {{ }}; self export createWithNamespace }}\n{call}\n"
        )
    };
    assert_eq!(
        tcloo_constructor_codes_for(&src("Widget createWithNamespace fido ::ns 1")),
        vec!["E002".to_owned()]
    );
    assert_eq!(
        tcloo_constructor_codes_for(&src("Widget createWithNamespace fido ::ns 1 2")),
        Vec::<String>::new()
    );
    assert_eq!(
        tcloo_constructor_codes_for(&src("Widget createWithNamespace fido ::ns 1 2 3")),
        vec!["E003".to_owned()]
    );
    assert_eq!(
        tcloo_constructor_codes_for(&src("Widget createWithNamespace fido ::ns")),
        vec!["E002".to_owned()],
        "the mandatory object-name and namespace words themselves must be enforced"
    );
}

#[test]
fn tcloo_constructor_createwithnamespace_is_not_checked_when_not_exported() {
    // `createWithNamespace` is unexported by default (confirmed against
    // tclsh 9.0.4) — an external call to a class that
    // never `export`s it raises "unknown method" at run time and never
    // reaches the constructor, so it must not be arity-checked. This is
    // a false positive.
    let src = "\
oo::class create Widget { constructor {a b} { } }
Widget createWithNamespace fido ::ns 1
";
    assert_eq!(tcloo_constructor_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_constructor_arity_is_inherited_through_superclass() {
    // `Sub` declares no constructor of its own — `Base`'s is inherited
    // (confirmed against tclsh 9.0.4: a subclass with no `constructor`
    // block uses the nearest ancestor's).
    let src = "\
oo::class create Base { constructor {a b} { } }
oo::class create Sub { superclass Base }
Sub new 1
";
    assert_eq!(tcloo_constructor_codes_for(src), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_constructor_own_overrides_inherited() {
    let src = "\
oo::class create Base { constructor {a b} { } }
oo::class create Sub { superclass Base; constructor {a} { } }
Sub new 1 2
";
    assert_eq!(
        tcloo_constructor_codes_for(src),
        vec!["E003".to_owned()],
        "Sub's own 1-arg constructor must win over Base's 2-arg one"
    );
}

#[test]
fn tcloo_no_explicit_constructor_anywhere_is_never_arity_checked() {
    // TclOO's default (inherited from `oo::object`) constructor accepts and
    // ignores any number of arguments — confirmed against tclsh 9.0.4:
    // `oo::class create Foo {}` then `Foo new 1 2 3` succeeds.
    let src = "oo::class create Widget { method bar {} { } }\nWidget new 1 2 3 4 5\n";
    assert_eq!(tcloo_constructor_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_create_mandatory_name_is_checked_even_without_a_constructor() {
    // A class with no explicit constructor anywhere in its MRO must not
    // abstain from arity-checking `create` entirely, only from the
    // constructor's own (unconstrained) parameters. `create`'s mandatory
    // leading object-name word is enforced by the dispatcher itself,
    // independent of the constructor -- confirmed against tclsh 9.0.4:
    // `oo::class create Foo {}` then `Foo create` (no name) still raises
    // "wrong # args", even though any number of trailing args succeeds.
    let src = |call: &str| format!("oo::class create Widget {{ method bar {{}} {{ }} }}\n{call}\n");
    assert_eq!(
        e00x_codes_for_in(&src("Widget create"), "tcl8.6"),
        vec!["E002".to_owned()],
        "the mandatory object-name word must still be enforced"
    );
    assert_eq!(
        e00x_codes_for_in(&src("Widget create fido"), "tcl8.6"),
        Vec::<String>::new()
    );
    assert_eq!(
        e00x_codes_for_in(&src("Widget create fido 1 2 3 4 5"), "tcl8.6"),
        Vec::<String>::new(),
        "the unconstrained default constructor still accepts any trailing args"
    );
    assert_eq!(
        e00x_codes_for_in(&src("Widget new"), "tcl8.6"),
        Vec::<String>::new(),
        "`new` has no mandatory name word, so it stays unchecked as before"
    );
}

#[test]
fn tcloo_constructor_arity_snit_new_is_not_checked() {
    // A snit type's `new`/`create` (if it even has one — snit instantiates
    // via `TypeName instanceName ?args?`, not `new`/`create`) must never be
    // arity-checked against a `TclOO` constructor; snit is a wholly
    // different object system.
    let src = "\
snit::type Widget {
    constructor {a b} { }
}
Widget new 1
";
    assert_eq!(tcloo_constructor_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_constructor_arity_dynamic_expand_abstains() {
    // `{*}`-expanded args can't be counted statically — matches the same
    // any-uncertainty-abstains convention as every other arity path here.
    let src = "\
oo::class create Widget { constructor {a b} { } }
set args {1}
Widget new {*}$args
";
    assert_eq!(tcloo_constructor_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_constructor_arity_forward_reference_in_proc_body_resolves() {
    // A proc body runs after the whole file loads, so a class defined
    // *later* in the file still resolves — not order-gated, mirroring
    // `queue_user_call_arity_candidate`'s proc-body convention.
    let src = "\
proc make {} { Widget new 1 }
oo::class create Widget { constructor {a b} { } }
";
    assert_eq!(tcloo_constructor_codes_for(src), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_constructor_arity_via_oo_define_after_class_create() {
    // The constructor need not be declared inline in `oo::class create` —
    // a later `oo::define ClassName { constructor {...} {...} }` populates
    // the same `ClassDef::constructors` slot the arity check reads.
    let src = "\
oo::class create Widget {}
oo::define Widget {
    constructor {a b} { }
}
Widget new 1
";
    assert_eq!(tcloo_constructor_codes_for(src), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_constructor_arity_empty_body_is_not_a_real_constructor() {
    // `constructor {a b} {}` (a literally empty body) is TclOO's own way
    // of writing "no constructor" — confirmed against tclsh 9.0.4:
    // `info class constructor` returns empty and `new` with any argument
    // count succeeds; the arity check must not treat this as a real,
    // arity-enforcing constructor.
    let src = "oo::class create Widget { constructor {a b} {} }\nWidget new 1 2 3\n";
    assert_eq!(tcloo_constructor_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_constructor_arity_whitespace_or_comment_body_is_still_real() {
    // By contrast, ANY body content — even a single space or a
    // comment-only body — keeps the constructor's arity fully enforced
    // (confirmed against tclsh 9.0.4). Must not over-correct the empty-body
    // exclusion into treating every trivial-looking body as absent.
    let src = "oo::class create Widget { constructor {a b} { } }\nWidget new 1\n";
    assert_eq!(tcloo_constructor_codes_for(src), vec!["E002".to_owned()]);
    let src_comment = "oo::class create Widget {\n    constructor {a b} {\n        # just a comment\n    }\n}\nWidget new 1\n";
    assert_eq!(
        tcloo_constructor_codes_for(src_comment),
        vec!["E002".to_owned()]
    );
}

#[test]
fn tcloo_constructor_arity_honours_definition_order_for_top_level_call() {
    // A top-level call made *before* a
    // later `oo::define` adds the constructor sees the class as it stood
    // at that point — no constructor yet, so TclOO's permissive default
    // applies (confirmed against tclsh 9.0.4).
    let src = "\
oo::class create Widget {}
Widget new 1
oo::define Widget {
    constructor {a b} { }
}
";
    assert_eq!(
        tcloo_constructor_codes_for(src),
        Vec::<String>::new(),
        "the call precedes the constructor's own definition, not just the class's"
    );
}

#[test]
fn tcloo_constructor_arity_redefinition_uses_the_one_in_effect_at_the_call() {
    // A call between two constructor redefinitions sees the one that was
    // in effect at that point, not the file's final constructor —
    // mirrors `resolve_indirect_call_target`'s identical convention for a
    // same-file proc redefinition.
    let src = "\
oo::class create Widget {
    constructor {a} { }
}
Widget new 1
oo::define Widget {
    constructor {a b} { }
}
Widget new 1 2
";
    assert_eq!(tcloo_constructor_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_constructor_arity_proc_body_call_not_order_gated_against_later_define() {
    // The genuine uncalled procedure body has conditional future-source
    // applicability against the authored class graph. This signature advice
    // does not establish when the procedure or constructor will execute.
    let src = "\
oo::class create Widget {}
proc make {} { Widget new 1 }
oo::define Widget {
    constructor {a b} { }
}
";
    assert_eq!(tcloo_constructor_codes_for(src), vec!["E002".to_owned()]);
}

#[test]
fn apply_lambda_arity_suppressed_when_apply_itself_is_shadowed() {
    // `apply` is an ordinary command name
    // and can be shadowed by a user proc — confirmed against tclsh 9.0.4,
    // a user-defined `apply` resolves ahead of the language builtin. The
    // lambda-literal-shaped argument must not be arity-checked against a
    // builtin `apply` that this call never actually reaches.
    let src = "proc apply {lambda x} { return $x }\napply {{a b} {}} 1\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn apply_lambda_arity_still_checked_when_apply_is_not_shadowed() {
    // Regression guard: the shadowing fix above must not silence the
    // ordinary, non-shadowed case.
    let src = "apply {{a b} {return [expr {$a+$b}]}} 1\n";
    assert_eq!(e00x_codes_for_in(src, "tcl8.6"), vec!["E002".to_owned()]);
}

#[test]
fn tcloo_constructor_arity_top_level_before_class_definition_abstains() {
    // A top-level call textually before the class exists fails at run time
    // ("invalid command name"), not with a constructor-arity mismatch —
    // matches the same order-gating convention as a shadowing proc.
    let src = "\
Widget new 1
oo::class create Widget { constructor {a b} { } }
";
    assert_eq!(tcloo_constructor_codes_for(src), Vec::<String>::new());
}

// -- TclOO `next` / `nextto` call-site arity

#[test]
fn tcloo_next_arity_checked_against_superclass_override() {
    // `next` inside `Derived::speak` invokes `Base::speak` — a 2-param
    // method, so `next` itself needs exactly 2 arguments — confirmed
    // against tclsh 9.0.4.
    let src = |call: &str| {
        format!(
            "oo::class create Base {{ method speak {{a b}} {{ return \"$a$b\" }} }}\n\
             oo::class create Derived {{\n\
               superclass Base\n\
               method speak {{a b}} {{ {call} }}\n\
             }}\n\
             [Derived new] speak x y\n"
        )
    };
    assert_eq!(
        e00x_codes_for_in(&src("next 1"), "tcl8.6"),
        vec!["E002".to_owned()]
    );
    assert_eq!(
        e00x_codes_for_in(&src("next 1 2"), "tcl8.6"),
        Vec::<String>::new()
    );
    assert_eq!(
        e00x_codes_for_in(&src("next 1 2 3"), "tcl8.6"),
        vec!["E003".to_owned()]
    );
}

#[test]
fn tcloo_nextto_arity_checked_against_named_target() {
    // `nextto Root` jumps straight to `Root::speak` (1 param) rather than
    // walking the MRO from `Derived`, skipping `Mid` — confirmed against
    // tclsh 9.0.4.
    let src = |call: &str| {
        format!(
            "oo::class create Root {{ method speak {{a}} {{ return $a }} }}\n\
             oo::class create Mid {{ superclass Root\n method speak {{a b}} {{ return \"$a$b\" }} }}\n\
             oo::class create Derived {{\n\
               superclass Mid\n\
               method speak {{a b}} {{ {call} }}\n\
             }}\n\
             [Derived new] speak x y\n"
        )
    };
    assert_eq!(
        e00x_codes_for_in(&src("nextto ::Root 1 2"), "tcl8.6"),
        vec!["E003".to_owned()]
    );
    assert_eq!(
        e00x_codes_for_in(&src("nextto ::Root 1"), "tcl8.6"),
        Vec::<String>::new()
    );
}

#[test]
fn tcloo_next_arity_silent_when_no_further_provider() {
    // `Base` declares no superclass override of `speak` — `next` here
    // has no provider to resolve arity against (a real `next` in this
    // position is itself a runtime error, "no next" — not an arity
    // mismatch this check models). Must not invent E002/E003 either way.
    let src =
        "oo::class create Base { method speak {a b} { next 1 2 3 4 5 } }\n[Base new] speak x y\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_next_arity_silent_outside_method_body() {
    // A bareword `next` at top level (or inside an ordinary proc) is not
    // inside any method's calling frame — confirmed against tclsh 9.0.4:
    // it fails "next may only be called from inside a method", not an
    // arity mismatch. `current_method_context` returns `None`, so the
    // candidate is dropped, not checked against some unrelated method.
    let src = "\
oo::class create Base { method speak {a b} { return \"$a$b\" } }
proc helper {} { next 1 2 3 4 5 }
helper
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_nextto_arity_silent_for_unresolvable_target_class() {
    // `nextto` naming a class the analyser doesn't locally know (an
    // external / cross-file / dynamically-loaded class) must abstain
    // rather than guess.
    let src = "\
oo::class create Base { method speak {a b} { next 1 2 } }
oo::class create Derived {
    superclass Base
    method speak {a b} { nextto SomeExternalClass 1 2 3 }
}
[Derived new] speak x y
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_next_arity_expansion_never_false_fires_too_few() {
    // `{*}`-expanded args make the true count a lower bound only — must
    // never false-fire E002, matching every other arity check's
    // `{*}`-expansion convention.
    let src = "\
oo::class create Base { method speak {a b} { return \"$a$b\" } }
oo::class create Derived {
    superclass Base
    method speak {a b} { set rest {1}; next {*}$rest }
}
[Derived new] speak x y
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn tcloo_next_arity_trait_bit_does_not_collide_with_structurally_checked_arity() {
    // `Traits::TCLOO_NEXT_CHAIN` must not share a bit with
    // `Traits::STRUCTURALLY_CHECKED_ARITY`: every command carrying the
    // latter (e.g. `if`) would also read as carrying the former. An ordinary
    // `if` inside a TclOO method body would then get queued as a bogus
    // next/nextto arity candidate and checked against the superclass
    // override's arity — here `Base::speak` takes 5 params, so a
    // 2-argument `if $a {puts hi}` would misfire E002 ("too few
    // arguments") on a collision.
    let src = "\
oo::class create Base { method speak {a b c d e} { return $a } }
oo::class create Derived {
    superclass Base
    method speak {a b c d e} { if {$a} { puts hi } }
}
[Derived new] speak 1 2 3 4 5
";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

// -- `apply {{params} body}` direct-call arity

#[test]
fn apply_lambda_arity_fires_and_stays_silent() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    assert_eq!(
        e00x_codes_for_in("apply {{a b} {return [expr {$a+$b}]}} 1\n", "tcl8.6"),
        vec!["E002".to_owned()]
    );
    assert_eq!(
        e00x_codes_for_in("apply {{a b} {return [expr {$a+$b}]}} 1 2\n", "tcl8.6"),
        Vec::<String>::new()
    );
    assert_eq!(
        e00x_codes_for_in("apply {{a b} {return [expr {$a+$b}]}} 1 2 3\n", "tcl8.6"),
        vec!["E003".to_owned()]
    );
}

#[test]
fn apply_lambda_with_default_param_is_silent_when_omitted() {
    let src = "apply {{a {b 2}} {return [expr {$a+$b}]}} 1\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn apply_lambda_with_args_catchall_is_unbounded() {
    let src = "apply {{a args} {return $a}} 1 2 3 4 5\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn apply_dynamic_lambda_abstains() {
    // `apply $lambda …` — the lambda literal isn't statically visible, so
    // nothing here can be counted; must not false-fire.
    let src = "set lambda {{a b} {return $a}}\napply $lambda 1\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}

#[test]
fn apply_expand_args_abstains_too_few() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let src = "set rest {1}\napply {{a b} {return $a}} {*}$rest\n";
    assert_eq!(e00x_codes_for(src), Vec::<String>::new());
}
