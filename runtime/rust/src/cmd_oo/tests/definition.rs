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

//! `oo::define` and `oo::objdefine`: the definition context, abbreviation, `definitionnamespace`, method definitions and definition-script errors.

use super::*;

#[test]
fn define_context_is_frame_scoped() {
    leak_free(|i| {
        // A user proc in ::oo::define is reachable as a definition command,
        // and runs with the definition context suspended (frame-scoped): its
        // `variable` is the ordinary command, and `uplevel 1` re-enters the
        // definition body's context (oo-36.9, oo-43.x).
        ok(i, b"oo::class create C");
        // A non-command in a define body routes through the global `unknown`
        // proc, whose `variable ::tcl::UnknownPending` must resolve to the
        // ordinary command (the define context is suspended in that nested
        // frame) — yielding the proper "invalid command name", not a spurious
        // declared-variable error.
        assert_eq!(i.eval_str(b"oo::define C nonesuch"), Code::Error);
        assert!(
            contains(&i.result_bytes(), b"invalid command name \"nonesuch\""),
            "got {:?}",
            i.result_bytes()
        );
        // A user proc in ::oo::define is reachable as a definition command;
        // direct `self` errors at the proc level, `uplevel 1 self` re-enters
        // the definition body's context and yields the class.
        ok(
                i,
                b"proc ::oo::objdefine::probe {} { list [catch {::oo::objdefine::self} m] [catch {uplevel 1 self} m2] $m2 }",
            );
        assert_eq!(ok(i, b"oo::objdefine C probe"), b"1 0 ::C");
    });
}

#[test]
fn namespaced_define_command_context_error() {
    leak_free(|i| {
        // Outside a definition the namespaced define subcommands error.
        assert_eq!(i.eval_str(b"oo::define::private error xyz"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"this command may only be called from within the context of an \
                  ::oo::define or ::oo::objdefine command"
        );
        assert_eq!(i.eval_str(b"oo::objdefine::method m {} {}"), Code::Error);
        // Inside a definition they dispatch normally.
        ok(i, b"oo::class create C");
        ok(i, b"oo::define C { ::oo::define::method m {} {return ok} }");
        ok(i, b"C create o");
        assert_eq!(ok(i, b"o m"), b"ok");
    });
}

#[test]
fn define_context_self_arity() {
    // naming.tcloo.original-definition-context-self
    // docs/design/analysis/name-resolution-proofs/tcloo-original-definition-context-self.md
    leak_free(|i| {
        ok(i, b"oo::class create Cls");
        ok(i, b"Cls create obj");
        // `oo::define`'s `self` accepts class-side directives; `self` alone
        // returns the class.
        assert_eq!(ok(i, b"oo::define Cls {set ::r [self]}; set ::r"), b"::Cls");
        ok(
            i,
            b"oo::define Cls { self { method cm {} {return classside} } }",
        );
        assert_eq!(ok(i, b"Cls cm"), b"classside");
        // `oo::objdefine`'s `self` takes no arguments (oo-36.8).
        assert_eq!(ok(i, b"oo::objdefine obj {self}"), b"::obj");
        assert_eq!(
            i.eval_str(b"oo::objdefine obj {self anything}"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"wrong # args: should be \"self\"");
    });
}

#[test]
fn define_subcommand_abbrev_and_extras() {
    leak_free(|i| {
        // Abbreviated definition subcommands (unique prefixes).
        ok(i, b"oo::class create A");
        ok(i, b"oo::class create C { meth foo {} {return X} }");
        assert_eq!(ok(i, b"[C new] foo"), b"X");
        ok(i, b"oo::define C { super A }");
        assert_eq!(ok(i, b"info class superclasses C"), b"::A");
        // An ambiguous prefix is not a define subcommand.
        assert_eq!(i.eval_str(b"oo::define C { m foo {} {} }"), Code::Error);
        // deletemethod / renamemethod.
        ok(
            i,
            b"oo::class create D { method foo {} {return F}; method bar {} {return B} }",
        );
        ok(i, b"oo::define D deletemethod foo");
        assert_eq!(i.eval_str(b"[D new] foo"), Code::Error);
        ok(i, b"oo::define D renamemethod bar baz");
        assert_eq!(ok(i, b"[D new] baz"), b"B");
        assert_eq!(i.eval_str(b"oo::define D deletemethod nope"), Code::Error);
        // objdefine class — change an object's class.
        ok(i, b"oo::class create P { method m {} {return P} }");
        ok(i, b"oo::class create Q { method m {} {return Q} }");
        ok(i, b"set o [P new]");
        ok(i, b"oo::objdefine $o class Q");
        assert_eq!(ok(i, b"$o m"), b"Q");
    });
}

#[test]
fn method_definition_retains_original_default_objects_without_rendering() {
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect(dialect));
            assert_eq!(interp.eval_str(b"oo::class create C"), Code::Ok);
            let default = obj::Owned::fresh(obj::new_wide_int_obj(17));
            let parameters = [Param {
                name: b"x".to_vec(),
                default: Some(default.clone()),
            }];
            let procedure = interp.create_native_callable_from_chosen(
                parameters.to_vec(),
                obj::Owned::fresh(obj::new_string_bytes(b"return $x")),
                crate::namespace::GLOBAL,
                None,
                0,
            );
            let definition = obj::Owned::fresh(super::super::list_procedure_body(
                interp,
                &procedure.declaration(),
            ));
            assert!(!obj::has_string_rep(definition.as_ptr()));
            let elements = list::native_list_backing(definition.as_ptr()).unwrap();
            let formals = list::native_list_backing(elements.elements().unwrap()[0]).unwrap();
            let parameter = list::native_list_backing(formals.elements().unwrap()[0]).unwrap();
            assert_eq!(parameter.elements().unwrap()[1], default.as_ptr());
            assert!(!obj::has_string_rep(default.as_ptr()));
            interp.set_result(definition.as_ptr());
            assert_eq!(interp.result_bytes(), b"{{x 17}} {return $x}");
        });
    }
}

#[test]
fn definitionnamespace_semantics() {
    // naming.tcloo.original-definitionnamespace-source-control
    // docs/design/analysis/name-resolution-proofs/tcloo-original-definitionnamespace-source-control.md
    leak_free(|i| {
        // TIP 524: a metaclass's -class definition namespace becomes the
        // resolution scope when defining its instances (classes).
        ok(i, b"oo::class create parent");
        ok(
            i,
            b"namespace eval foodef { proc sparkle {} { return ok } }",
        );
        ok(
            i,
            b"oo::class create foocls { superclass oo::class parent; definitionnamespace foodef }",
        );
        ok(
            i,
            b"oo::class create foo { superclass parent; self class foocls }",
        );
        // `sparkle` (a proc in foodef) is now reachable in foo's definition.
        assert_eq!(ok(i, b"oo::define foo { sparkle }"), b"ok");
        // Round-trip introspection of an explicitly-set namespace.
        ok(i, b"namespace eval ::nd {}");
        ok(
            i,
            b"oo::class create D; oo::define D { definitionnamespace ::nd }",
        );
        assert_eq!(ok(i, b"info class definitionnamespace D"), b"::nd");
        assert_eq!(ok(i, b"info class definitionnamespace D -instance"), b"");
        // The root classes reject definition-namespace changes.
        assert_eq!(
            i.eval_str(b"oo::define oo::object { definitionnamespace ::nd }"),
            Code::Error,
        );
    });
}

#[test]
fn definitionnamespace_uses_original_namespace_object_in_each_outer_context() {
    for version in [tcl_dialect::TclVersion::V9_0, tcl_dialect::TclVersion::V9_1] {
        leak_free(|i| {
            i.set_runtime_version(version);
            ok(i, b"namespace eval ::dest {}; namespace eval ::outer {namespace eval dest {}; oo::class create C}; namespace eval ::other {namespace eval dest {}; oo::class create C}; set name dest");
            ok(
                i,
                b"namespace eval ::outer {oo::define C definitionnamespace $::name}",
            );
            assert_eq!(
                ok(i, b"info class definitionnamespace ::outer::C"),
                b"::outer::dest"
            );
            ok(i, b"set name");
            assert!(matches!(
                obj::native_object_snapshot(i.result_obj()).unwrap().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::NamespaceName {
                    resolved: true,
                    ..
                }
            ));
            ok(
                i,
                b"namespace eval ::other {oo::define C definitionnamespace $::name}",
            );
            assert_eq!(
                ok(i, b"info class definitionnamespace ::other::C"),
                b"::other::dest"
            );
            ok(
                i,
                b"namespace eval ::outer {oo::define C definitionnamespace {}}",
            );
            assert_eq!(ok(i, b"info class definitionnamespace ::outer::C"), b"");
        });
    }
}

#[test]
fn original_renamed_definition_matches_native_source_and_raw_return_boundaries() {
    // naming.class.original-renamed-definition-error-context
    // docs/design/analysis/name-resolution-proofs/class-original-renamed-definition-error-context.md
    // Whole public completion/errorInfo only; no private class/header
    // identity follows. C8.4/8.5 stock TclOO is unavailable: their C API
    // settles top-level return to OK, while this raw evaluator retains RETURN.
    macro_rules! provider {
        ($engine:literal, $raw_return:literal) => {
            (
                $engine,
                $raw_return,
                include_str!(concat!(
                    "../../../../../rust/tcl-registry/tests/data/native_oo_renamed_definition321/",
                    $engine,
                    "/renamed-definition/stdout"
                )),
            )
        };
    }
    let source = include_bytes!(
        "../../../../../rust/tcl-registry/tests/data/native_oo_renamed_definition321/source.tcl"
    );
    for (engine, raw_return, stdout) in [
        provider!("tcl8.4", true),
        provider!("tcl8.5", true),
        provider!("tcl8.6", false),
        provider!("tcl9.0", false),
        provider!("tcl9.1", false),
        provider!("jim", false),
    ] {
        let (expected_code, original) = stdout
            .lines()
            .find_map(|line| line.strip_prefix("ORIGINAL|"))
            .unwrap()
            .split_once('|')
            .unwrap();
        let expected: Vec<u8> = original
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            // The native provider starts with its distribution extensions loaded.
            crate::cmd_proc::install_stock_scripted_wrappers(&mut interp);
            let code = interp.eval_str(source);
            assert!(
                !interp.host_refusal_pending(),
                "{engine}: {:?}",
                interp.native_access_refusal()
            );
            if raw_return {
                assert_eq!(expected_code, "0", "{engine}: native embedding boundary");
                assert_eq!(code, Code::Return, "{engine}: raw evaluator boundary");
            } else {
                assert_eq!(code.as_int().to_string(), expected_code, "{engine}");
            }
            assert_eq!(interp.result_bytes(), expected, "{engine}");
        }
        assert_eq!(counters::finalize(), 0, "{engine}");
        assert_eq!(counters::double_free_count(), 0, "{engine}");
    }
}

#[test]
fn define_script_errorinfo_and_super_mixin_messages() {
    // naming.tcloo.original-nonclass-superclass-mixin-errors
    // docs/design/analysis/name-resolution-proofs/tcloo-original-nonclass-superclass-mixin-errors.md
    leak_free(|i| {
        // superclass/mixin: not-an-object vs not-a-class messages (as-written
        // name; matches C's Tcl_GetObjectFromObj + ClassSuperSet/MixinSet).
        ok(i, b"oo::object create anobj");
        assert_eq!(
            i.eval_str(b"oo::class create c1 { superclass nosuch }"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"nosuch does not refer to an object");
        assert_eq!(
            i.eval_str(b"oo::class create c2 { superclass anobj }"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"only a class can be a superclass");
        assert_eq!(
            i.eval_str(b"oo::class create c3 { mixin nosuch }"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"nosuch does not refer to an object");
        assert_eq!(
            i.eval_str(b"oo::class create c4 { mixin anobj }"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"only a class can be a mixin");
        // naming.class.original-renamed-definition-error-context
        // docs/design/analysis/name-resolution-proofs/class-original-renamed-definition-error-context.md
        // The define-script errorInfo frame uses the *current* (renamed) name.
        ok(i, b"oo::class create base");
        assert_eq!(
            i.eval_str(b"oo::class create abc { superclass base; rename abc def; error foo }"),
            Code::Error
        );
        let ei = ok(i, b"set errorInfo");
        assert!(
            ei.windows(b"class \"::oo::define::def\"".len())
                .any(|w| w == b"class \"::oo::define::def\""),
            "errorInfo should name the renamed class ::oo::define::def, got {ei:?}"
        );
    });
}

#[test]
fn define_single_command_wrong_args_names_whole_command() {
    leak_free(|i| {
        ok(i, b"oo::class create Foo");
        // Single-command form: the message names the whole original command.
        assert_eq!(
            i.eval_str(b"oo::define Foo method missingArgs"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"wrong # args: should be \"oo::define Foo method name ?option? args body\""
        );
        assert_eq!(
            i.eval_str(b"oo::objdefine Foo method missingArgs"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"wrong # args: should be \"oo::objdefine Foo method name ?option? args body\""
        );
        // Body form keeps the bare usage (no rewrite prefix).
        assert_eq!(
            i.eval_str(b"oo::define Foo { method missingArgs }"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"wrong # args: should be \"method name ?option? args body\""
        );
    });
}

// Needs the numeric tower: `forward` targets `::tcl::mathop::+`.
#[cfg(have_tommath)]
#[test]
fn objdefine_forward_mixin_unexport_copy() {
    leak_free(|i| {
        ok(i, b"oo::class create Base { method m {} {return base}; method hide {} {return H}; unexport hide }");
        ok(i, b"set o [Base new]");
        // unexported method: external call fails, `my` works.
        assert_eq!(i.eval_str(b"$o hide"), Code::Error);
        // per-object method via objdefine
        ok(i, b"oo::objdefine $o { method extra {} {return per-obj} }");
        assert_eq!(ok(i, b"$o extra"), b"per-obj");
        // forward
        ok(i, b"oo::class create F { forward add ::tcl::mathop::+ 10 }");
        assert_eq!(ok(i, b"[F new] add 5"), b"15");
        // mixin
        ok(
            i,
            b"oo::class create Mix { method mixed {} {return MIXED} }",
        );
        ok(i, b"oo::objdefine $o { mixin Mix }");
        assert_eq!(ok(i, b"$o mixed"), b"MIXED");
        // info introspection
        assert_eq!(ok(i, b"info object class $o"), b"::Base");
        assert_eq!(ok(i, b"info object isa object $o"), b"1");
        let object = ok(i, b"set o");
        assert_eq!(ok(i, b"info class instances Base"), object);
    });
}
