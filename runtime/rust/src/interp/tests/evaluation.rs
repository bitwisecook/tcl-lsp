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

//! Evaluating scripts end to end: nesting limits, variables and arrays, command lookup and unknown handling, tailcalls, expressions and completion codes.

use super::*;

/// The native Runtime returns its explicit host evaluation-limit error
/// before synchronously nested generic callbacks exhaust the host stack.
/// This host boundary is separate from the original native completions.
#[test]
fn deeply_nested_foreach_reaches_the_explicit_host_evaluation_limit() {
    // Native proof: naming.runtime.original-nested-foreach-source-completion
    // docs/design/analysis/name-resolution-proofs/runtime-original-nested-foreach-source-completion.md
    // The identical 300-body source completes in all six native providers.
    // These assertions exercise this Runtime's separately bounded host path.
    leak_free(|i| {
        let mut src = String::new();
        for _ in 0..300 {
            src.push_str("foreach x {1} {\n");
        }
        src.push_str("set done 1\n");
        for _ in 0..300 {
            src.push_str("}\n");
        }
        assert_eq!(i.eval_str(src.as_bytes()), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"too many nested evaluations (infinite loop?)"
        );
    });
}

/// Defence in depth for the alias trampoline. `interp alias`
/// and `rename` both refuse a cycle at definition time, so plant one
/// straight into the command table — bypassing that gate the way only a
/// bug could — and confirm the dispatch bound turns what would otherwise be
/// unbounded native recursion (`invoke` → `dispatch_alias` → `invoke`, i.e.
/// stack exhaustion / a WASM trap) into a catchable Tcl error.
#[test]
fn a_planted_alias_cycle_errors_instead_of_exhausting_the_stack() {
    leak_free(|i| {
        let alias = |target: &[u8]| Command::Alias {
            original_prefix: Rc::new(NativeAliasHeader::new(
                i.generated_alias_prefix(target, &[]),
            )),
            registration_name: Some(Rc::new(NativeAliasHeader::new(obj::Owned::fresh(
                new_string(target),
            )))),
            publication_name: target.to_vec(),
            identity: Rc::new(()),
        };
        {
            let mut namespaces = i.namespaces.borrow_mut();
            namespaces.register(b"loop_a", alias(b"loop_b"));
            namespaces.register(b"loop_b", alias(b"loop_a"));
        }
        assert_eq!(i.eval_str(b"loop_a"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"too many nested alias invocations (infinite loop?)"
        );
        // The counter unwinds with the failed call, so a later alias call
        // is unaffected.
        assert_eq!(i.eval_str(b"interp alias {} = {} set"), Code::Ok);
        assert_eq!(i.eval_str(b"= v 1"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
    });
}

/// A moderately nested `foreach` (well under `NATIVE_EVAL_DEPTH_LIMIT`)
/// still runs to completion — the safety net must not fire on realistic
/// nesting depths.
#[test]
fn moderately_nested_foreach_still_runs() {
    leak_free(|i| {
        let mut src = String::new();
        for _ in 0..50 {
            src.push_str("foreach x {1} {\n");
        }
        src.push_str("set done 1\n");
        for _ in 0..50 {
            src.push_str("}\n");
        }
        assert_eq!(i.eval_str(src.as_bytes()), Code::Ok);
        assert_eq!(i.eval_str(b"set done"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
    });
}

/// A recursive proc with no base case relies purely on a recursion cap
/// to terminate. Before `NATIVE_EVAL_DEPTH_LIMIT`, this overflowed the
/// native stack well before the pre-existing `RECURSION_LIMIT` (1000)
/// was ever reached — a real, unguarded crash on ordinary recursive Tcl
/// code, not just pathological control-flow nesting.
#[test]
fn unbounded_proc_recursion_errors_instead_of_crashing() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"proc r {} { r }"), Code::Ok);
        assert_eq!(i.eval_str(b"r"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"too many nested evaluations (infinite loop?)"
        );
    });
}

#[test]
fn set_and_read_back() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set x 5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        assert_eq!(i.eval_str(b"set y $x"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
    });
}

#[test]
fn command_substitution_closes_the_seam() {
    // [set y 42] evaluates the inner command; x becomes its result.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set x [set y 42]"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        assert_eq!(i.eval_str(b"set x"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
    });
}

#[test]
fn incr_arithmetic() {
    leak_free(|i| {
        i.eval_str(b"set n 10");
        assert_eq!(i.eval_str(b"incr n"), Code::Ok);
        assert_eq!(i.result_bytes(), b"11");
        assert_eq!(i.eval_str(b"incr n 5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"16");
        // incr of an unset var starts from 0
        assert_eq!(i.eval_str(b"incr fresh"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
    });
}

#[test]
fn undefined_variable_is_an_error() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set y $nope"), Code::Error);
        assert_eq!(i.result_bytes(), b"can't read \"nope\": no such variable");
    });
}

#[test]
fn unknown_command_is_an_error() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"frobnicate a b"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"frobnicate\"");
    });
}

#[test]
fn missing_alias_target_uses_custom_unknown_with_target_and_args() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(b"proc unknown {cmd args} {list unknown $cmd $args}"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"interp alias {} la {} nosuch"), Code::Ok);
        assert_eq!(i.eval_str(b"la a b"), Code::Ok);
        assert_eq!(i.result_bytes(), b"unknown nosuch {a b}");
    });
}

#[test]
fn missing_alias_target_without_unknown_preserves_target_error_identity() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"interp alias {} la {} nosuch"), Code::Ok);
        assert_eq!(i.eval_str(b"la a b"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"nosuch\"");
    });
}

#[test]
fn missing_alias_target_preserves_prefix_arguments_for_unknown() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(b"proc unknown {cmd args} {list $cmd $args}"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"interp alias {} la {} nosuch prefix"), Code::Ok);
        assert_eq!(i.eval_str(b"la one two"), Code::Ok);
        assert_eq!(i.result_bytes(), b"nosuch {prefix one two}");
    });
}

#[test]
fn missing_alias_target_selects_the_native_handler_context() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
    {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(
                i.eval_str(b"proc unknown {cmd args} {list global $cmd $args}"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"namespace eval n {proc u {cmd args} {list ns $cmd $args}; namespace unknown ::n::u; interp alias {} la {} nosuch; la x}"), Code::Ok);
            let expected: &[u8] = if version == tcl_dialect::TclVersion::V8_5 {
                b"global nosuch x"
            } else {
                b"ns nosuch x"
            };
            assert_eq!(i.result_bytes(), expected);
        });
    }
}

#[test]
fn original_command_getter_uses_the_retained_lookup_namespace() {
    for version in tcl_dialect::TclVersion::ALL {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(i.eval_str(b"namespace eval A {proc p {} {return A}}; namespace eval B {proc p {} {return B}}"), Code::Ok);
            let head = crate::obj::Owned::fresh(new_string(b"p"));
            for name in [b"::A".as_slice(), b"::B", b"::A"] {
                let namespace = i.namespaces.borrow().find_namespace(GLOBAL, name).unwrap();
                let (command, generation) = i
                    .resolve_original_command_at(namespace, head.as_ptr())
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    i.invoke_bound(command, generation, &[head.as_ptr()]),
                    Code::Ok
                );
                assert_eq!(i.result_bytes(), &name[2..]);
            }
        });
    }
}

#[test]
fn tailcall_retired_namespace_relookup_observes_deletion_and_recreation() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_6)
    {
        for recreate in [false, true] {
            leak_free(|i| {
                i.set_runtime_version(version);
                let source: &[u8] = if recreate {
                    b"namespace eval N {proc issue {} {tailcall target}; proc target {} {return OLD}}; proc leave args {namespace delete N; namespace eval N {proc target {} {return NEW}}}; trace add execution N::issue leave leave; N::issue"
                } else {
                    b"namespace eval N {proc issue {} {tailcall target}; proc target {} {return OLD}}; proc leave args {namespace delete N}; trace add execution N::issue leave leave; N::issue"
                };
                assert_eq!(
                    i.eval_str(source),
                    if recreate { Code::Ok } else { Code::Error }
                );
                assert_eq!(
                    i.result_bytes(),
                    if recreate {
                        b"NEW".as_slice()
                    } else {
                        b"namespace \"::N\" not found"
                    }
                );
            });
        }
    }
}

#[test]
fn tailcall_selected_miss_does_not_relookup_the_callers_command() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_6)
    {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(i.eval_str(b"namespace eval Iss {proc p {} {tailcall missing}}; namespace eval Caller {proc missing {} {return WRONG}; proc u {cmd args} {return HANDLER}; namespace unknown ::Caller::u; ::Iss::p}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"HANDLER");
        });
    }
}

#[test]
fn missing_alias_unknown_cycle_is_bounded() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"interp alias {} la {} nosuch"), Code::Ok);
        assert_eq!(i.eval_str(b"proc unknown {cmd args} {la}"), Code::Ok);
        assert_eq!(i.eval_str(b"la"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"too many nested evaluations (infinite loop?)"
        );
    });
}

#[test]
fn release_hidden_alias_target_uses_unknown_under_tcl84() {
    use tcl_dialect::TclVersion;

    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(
            i.eval_str(b"proc unknown {cmd args} {list hidden $cmd $args}"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"interp alias {} la {} lassign"), Code::Ok);
        assert_eq!(i.eval_str(b"la {a b} x y"), Code::Ok);
        assert_eq!(i.result_bytes(), b"hidden lassign {{a b} x y}");
    });
}

#[test]
fn absolute_qualified_command_resolves() {
    leak_free(|i| {
        // `::set` is the global `set`, reached via the namespace resolver.
        assert_eq!(i.eval_str(b"::set x 5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        assert_eq!(i.eval_str(b"set y $x"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        // an unknown namespace qualifier is an error
        assert_eq!(i.eval_str(b"::nosuch::cmd a"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"::nosuch::cmd\"");
    });
}

#[cfg(have_tommath)]
#[test]
fn expr_command_end_to_end() {
    leak_free(|i| {
        // braced arithmetic with precedence
        assert_eq!(i.eval_str(b"expr {2 + 3 * 4}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"14");
        // a variable resolved through the frame store (object-preserving)
        assert_eq!(i.eval_str(b"set x 20"), Code::Ok);
        assert_eq!(i.eval_str(b"expr {$x * 2 + 2}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        // overflow promotes to a bignum, then a command substitution feeds back in
        assert_eq!(i.eval_str(b"expr {2 ** 64}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"18446744073709551616");
        assert_eq!(i.eval_str(b"expr {[set x] < 100}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        // divide by zero surfaces the verbatim error
        assert_eq!(i.eval_str(b"expr {1 / 0}"), Code::Error);
        assert_eq!(i.result_bytes(), b"divide by zero");
    });
}

#[cfg(have_tommath)]
#[test]
fn incr_promotes_to_bignum() {
    leak_free(|i| {
        // incr starts at 0 for an unset var
        assert_eq!(i.eval_str(b"incr n"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        // incr past a wide promotes to a bignum (never wraps)
        assert_eq!(i.eval_str(b"set big 9223372036854775807"), Code::Ok); // i64::MAX
        assert_eq!(i.eval_str(b"incr big"), Code::Ok);
        assert_eq!(i.result_bytes(), b"9223372036854775808");
        // incrementing a bignum cell keeps working, and demotes when it fits
        assert_eq!(i.eval_str(b"incr big -1"), Code::Ok);
        assert_eq!(i.result_bytes(), b"9223372036854775807");
        // a non-integer value is rejected verbatim
        assert_eq!(i.eval_str(b"set f 1.5"), Code::Ok);
        assert_eq!(i.eval_str(b"incr f"), Code::Error);
        assert_eq!(i.result_bytes(), b"expected integer but got \"1.5\"");
    });
}

/// The `incr` adapter's element + const paths, exercised through the shared
/// `ValueOps::int_add` seam: array elements increment (and widen) correctly,
/// an unset element starts at 0, and a `const` scalar is rejected before the
/// read-modify-write (the const check stays runtime-side).
#[cfg(have_tommath)]
#[test]
fn incr_element_and_const_via_seam() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set a(k) 10"), Code::Ok);
        assert_eq!(i.eval_str(b"incr a(k) 5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"15");
        // an unset element starts at 0
        assert_eq!(i.eval_str(b"incr a(fresh)"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        // an element past a wide promotes to a bignum (never wraps)
        assert_eq!(i.eval_str(b"set a(big) 9223372036854775807"), Code::Ok);
        assert_eq!(i.eval_str(b"incr a(big)"), Code::Ok);
        assert_eq!(i.result_bytes(), b"9223372036854775808");
        // a const scalar cannot be incremented
        assert_eq!(i.eval_str(b"const c 7"), Code::Ok);
        assert_eq!(i.eval_str(b"incr c"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't incr \"c\": variable is a constant"
        );
    });
}

#[test]
fn qualified_global_aliases_plain_at_top_level() {
    // `::pinged` and `pinged` resolve to the SAME global.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set ::pinged 1"), Code::Ok);
        assert_eq!(i.eval_str(b"set pinged"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(i.eval_str(b"set pinged 2"), Code::Ok);
        assert_eq!(i.eval_str(b"set ::pinged"), Code::Ok);
        assert_eq!(i.result_bytes(), b"2");
        assert_eq!(i.eval_str(b"unset ::pinged"), Code::Ok);
    });
}

#[test]
fn qualified_var_resolves_through_namespace_table() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"namespace eval a {}"), Code::Ok);
        // qualified write lands in ::a's var table …
        assert_eq!(i.eval_str(b"set ::a::x 5"), Code::Ok);
        // … visible as the unqualified `x` from inside ::a …
        assert_eq!(i.eval_str(b"namespace eval a { set x }"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        // … and through `$::a::x` substitution.
        assert_eq!(i.eval_str(b"set y $::a::x"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        assert_eq!(i.eval_str(b"unset ::a::x"), Code::Ok);
    });
}

#[test]
fn qualified_set_into_missing_namespace_errors() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set ::nosuch::x 1"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't set \"::nosuch::x\": parent namespace doesn't exist"
        );
        // a read of the same name reports the ordinary no-such-variable.
        assert_eq!(i.eval_str(b"set ::nosuch::x"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't read \"::nosuch::x\": no such variable"
        );
    });
}

#[test]
fn namespace_eval_body_set_is_a_namespace_var() {
    // `set x` inside `namespace eval` (a non-proc context) creates a ns var,
    // not a global.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"namespace eval b { set v 10 }"), Code::Ok);
        assert_eq!(i.eval_str(b"set ::b::v"), Code::Ok);
        assert_eq!(i.result_bytes(), b"10");
        assert_eq!(i.eval_str(b"set v"), Code::Error); // not a global
        assert_eq!(i.eval_str(b"unset ::b::v"), Code::Ok);
    });
}

#[test]
fn array_element_via_set_and_subst() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set a(k) hello"), Code::Ok);
        assert_eq!(i.eval_str(b"set out $a(k)"), Code::Ok);
        assert_eq!(i.result_bytes(), b"hello");
    });
}

#[test]
fn literal_element_references_match_the_selected_native_interpreter() {
    let mut engines: Vec<_> = tcl_test_support::available_tclshs()
        .into_iter()
        .map(|engine| (engine.path, Some(engine.version)))
        .collect();
    if let Some(engine) = tcl_test_support::locate_jimsh().expect("Jim oracle discovery") {
        engines.push((engine.path, None));
    }
    let scripts = [
            "set arr(k) ELEMENT; list ${arr(k)} $arr(k)",
            "set i k; set arr(k) ELEMENT; set {arr($i)} LITERAL; list ${arr($i)} $arr($i)",
            "set arr(k) ELEMENT; list prefix${arr(k)}suffix [subst {${arr(k)}}]",
            "set arr(k) ELEMENT; catch {set result ${arr(missing)}} result; set result",
            "proc probe {} {set arr(k) ELEMENT; set {arr($i)} LITERAL; set i k; list ${arr(k)} ${arr($i)} $arr($i)}; probe",
            "set code [catch {proc probe {{arr(k)}} {list [info exists arr] ${arr(k)} $arr(k) [set arr]}; probe FORMAL} result]; list $code $result",
        ];
    for (path, version) in engines {
        for script in scripts {
            let expected = tcl_test_support::run_script(
                &path,
                format!("set c [catch {{{script}}} r]; puts [list $c $r]\n").as_bytes(),
            )
            .expect("native variable reference execution");
            assert!(
                expected.success() && expected.stderr.is_empty(),
                "{expected:?}"
            );
            counters::reset();
            {
                let profile = version.map_or("jim", tcl_dialect::TclVersion::dialect_profile_name);
                let mut interp = Interp::with_native_core(
                    default_host(),
                    crate::environment::profile_for_dialect(profile),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .expect("selected original native interpreter constructor");
                let code = interp.eval_str(script.as_bytes());
                assert!(
                    !interp.host_refusal_pending(),
                    "{path:?}: {script}: {:?}",
                    interp.native_access_refusal(),
                );
                let observed = jim_list_bytes([
                    code.as_int().to_string().into_bytes(),
                    interp.result_bytes(),
                ]);
                assert_eq!(
                    observed,
                    expected
                        .stdout
                        .strip_suffix(b"\n")
                        .unwrap_or(&expected.stdout),
                    "{path:?}: {script}"
                );
                assert!(!interp.host_refusal_pending(), "{path:?}: {script}");
            }
            assert_eq!(
                counters::finalize(),
                0,
                "{path:?}: native source owner leaks"
            );
            assert_eq!(counters::double_free_count(), 0);
        }
    }
}

#[test]
#[cfg(not(have_tommath))]
fn an_unavailable_sugar_engine_cannot_be_caught_as_a_guest_error() {
    leak_free(|interp| {
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        let code =
            interp.eval_str(b"set prior 1; catch {set destination $(1+2)} captured; set after 1");
        assert_eq!(code, Code::Error);
        assert_eq!(
            interp.native_access_refusal(),
            Some(tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable)
        );
        assert_eq!(interp.var_get(b"prior").map(obj_bytes), Some(b"1".to_vec()));
        assert!(interp.var_get(b"captured").is_none());
        assert!(interp.var_get(b"destination").is_none());
        assert!(interp.var_get(b"after").is_none());
    });
}

#[test]
#[cfg(not(have_tommath))]
fn unavailable_safe_integer_engine_stops_after_the_reached_operand() {
    leak_free(|interp| {
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        let code =
            interp.eval_str(b"set prior 1; catch {string repeat x {1+1}} captured; set after 1");
        assert_eq!(code, Code::Error);
        assert_eq!(
            interp.native_access_refusal(),
            Some(tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable)
        );
        assert_eq!(interp.var_get(b"prior").map(obj_bytes), Some(b"1".to_vec()));
        assert!(interp.var_get(b"captured").is_none());
        assert!(interp.var_get(b"after").is_none());
    });
}

#[test]
#[cfg(have_tommath)]
fn jim_expression_sugar_bypasses_command_lookup_and_keeps_original_expression() {
    let jim = tcl_test_support::require_jimsh().expect("Jim expression sugar oracle");
    let scripts = [
            "rename expr oldexpr; proc expr args {return WRONG}; set x 3; list $(1+2) \"pre$($x+1)post\" [subst {$(1+2)}] [subst -novariables {$(1+2)}] [subst -nocommands {$(1+2)}]",
            "set seen 0; proc once {} {incr ::seen; return 4}; list $([once]+1) $seen",
            "set prior 1; set code [catch {set destination $(k)} result]; list $code $result $prior [info exists destination]",
            "set value $(1+(2*3)); list $value $()",
        ];
    for script in scripts {
        let expected = tcl_test_support::run_script(
            &jim.path,
            format!("set c [catch {{{script}}} r]; puts [list $c $r]\n").as_bytes(),
        )
        .expect("native sugar execution");
        assert!(
            expected.success() && expected.stderr.is_empty(),
            "{expected:?}"
        );
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            let code = interp.eval_str(script.as_bytes());
            let observed = jim_list_bytes([
                code.as_int().to_string().into_bytes(),
                interp.result_bytes(),
            ]);
            assert_eq!(
                observed,
                expected
                    .stdout
                    .strip_suffix(b"\n")
                    .unwrap_or(&expected.stdout),
                "{script}"
            );
            assert!(!interp.host_refusal_pending(), "{script}");
        });
    }
}

#[test]
#[cfg(have_tommath)]
fn jim_raw_expression_comparisons_errors_and_numeric_nul_match_native() {
    use tcl_test_support::expressions::{
        JIM_RAW_EXPRESSION_ERROR_SCRIPTS, JIM_RAW_EXPRESSION_VALUE_SCRIPTS,
        NUMERIC_NUL_EXPRESSION_OBSERVATION_SCRIPTS,
    };
    let jim = tcl_test_support::require_jimsh().expect("required Jim expression oracle");
    for script in JIM_RAW_EXPRESSION_VALUE_SCRIPTS
        .iter()
        .chain(JIM_RAW_EXPRESSION_ERROR_SCRIPTS)
        .chain(NUMERIC_NUL_EXPRESSION_OBSERVATION_SCRIPTS)
    {
        let observation = format!(
                "set ::errorCode NONE; set c [catch {{{script}}} r]; binary scan $r H* hx; list $c $hx $::errorCode"
            );
        let expected =
            tcl_test_support::run_script(&jim.path, format!("puts [{observation}]\n").as_bytes())
                .expect("native raw expression observation");
        assert!(
            expected.success() && expected.stderr.is_empty(),
            "{script}: {expected:?}"
        );
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            assert_eq!(
                interp.eval_str(observation.as_bytes()),
                Code::Ok,
                "{script}"
            );
            assert!(!interp.host_refusal_pending(), "{script}");
            assert_eq!(
                interp.result_bytes(),
                expected
                    .stdout
                    .strip_suffix(b"\n")
                    .unwrap_or(&expected.stdout),
                "{script}"
            );
        });
    }
}

#[test]
#[cfg(have_tommath)]
fn numeric_nul_expression_observations_match_every_selected_native_engine() {
    let scripts = tcl_test_support::expressions::NUMERIC_NUL_EXPRESSION_OBSERVATION_SCRIPTS;
    assert_eq!(scripts.len(), 18);
    let mut engines: Vec<_> = tcl_test_support::available_tclshs()
        .into_iter()
        .map(|engine| (engine.path, Some(engine.version)))
        .collect();
    let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(tcl_test_support::require_jimsh().expect("required Jim numeric oracle"))
    } else {
        tcl_test_support::locate_jimsh().expect("Jim numeric oracle discovery")
    };
    if let Some(jim) = jim {
        engines.push((jim.path, None));
    }
    let mut failures = Vec::new();
    for (path, version) in engines {
        for (index, script) in scripts.iter().enumerate() {
            let expected =
                tcl_test_support::run_script(&path, format!("puts [{script}]\n").as_bytes())
                    .expect("native numeric NUL observation");
            assert!(
                expected.success() && expected.stderr.is_empty(),
                "{script}: {expected:?}"
            );
            leak_free(|interp| {
                match version {
                    Some(version) => interp.set_runtime_version(version),
                    None => {
                        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"))
                    }
                }
                let code = interp.eval_str(script.as_bytes());
                let observed = interp.result_bytes();
                let wanted = expected
                    .stdout
                    .strip_suffix(b"\n")
                    .unwrap_or(&expected.stdout);
                if code != Code::Ok || interp.host_refusal_pending() || observed != wanted {
                    failures.push(format!("{version:?} case {index}: {script}\ncode {code:?}, observed {observed:?}, expected {wanted:?}"));
                }
            });
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn original_variable_word_vectors_match_the_selected_native_interpreter() {
    use tcl_syntax::execution_conformance::{vectors, ExecutionDomain};
    let cases: Vec<_> = vectors(ExecutionDomain::CommandBinding)
        .into_iter()
        .filter(|case| case.id.starts_with("variable_word_"))
        .collect();
    assert_eq!(
        cases.len(),
        12,
        "retain every shared original-variable control"
    );
    let mut engines: Vec<_> = tcl_test_support::available_tclshs()
        .into_iter()
        .map(|engine| (engine.path, Some(engine.version)))
        .collect();
    let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(tcl_test_support::require_jimsh().expect("required Jim variable-word oracle"))
    } else {
        tcl_test_support::locate_jimsh().expect("Jim variable-word oracle discovery")
    };
    if let Some(engine) = jim {
        engines.push((engine.path, None));
    }
    let mut failures = Vec::new();
    for (path, version) in engines {
        for case in &cases {
            let expected = tcl_test_support::run_script(&path, case.script().as_bytes())
                .expect("native original-variable execution");
            assert!(
                expected.success() && expected.stderr.is_empty(),
                "{expected:?}"
            );
            leak_free(|interp| {
                match version {
                    Some(version) => interp.set_runtime_version(version),
                    None => interp.set_dialect_profile(
                        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
                    ),
                }
                let code = interp.eval_str(case.source.as_bytes());
                let observed = jim_list_bytes([
                    code.as_int().to_string().into_bytes(),
                    interp.result_bytes(),
                ]);
                let wanted = expected
                    .stdout
                    .strip_suffix(b"\n")
                    .unwrap_or(&expected.stdout);
                if observed != wanted || interp.host_refusal_pending() {
                    failures.push(format!(
                        "{path:?} {}: observed {observed:?}, expected {wanted:?}, host refusal {}",
                        case.id,
                        interp.host_refusal_pending(),
                    ));
                }
            });
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn expand_marker_splits_arguments() {
    // A recording builtin proves {*} produced the right argv.
    leak_free(|i| {
        fn record(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
            let joined: Vec<Vec<u8>> = argv[1..].iter().map(|&o| obj_bytes(o)).collect();
            interp.set_result_bytes(&joined.join(&b'|'));
            Code::Ok
        }
        i.register_builtin(b"record", record);
        i.eval_str(b"set lst {a b c}");
        assert_eq!(i.eval_str(b"record {*}$lst tail"), Code::Ok);
        assert_eq!(i.result_bytes(), b"a|b|c|tail");
    });
}

#[test]
fn return_sets_code() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"return done"), Code::Return);
        assert_eq!(i.result_bytes(), b"done");
    });
}

#[test]
fn multi_command_script() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set a 1; set b 2\nset c $a$b"), Code::Ok);
        assert_eq!(i.result_bytes(), b"12");
    });
}

#[test]
fn direct_dispatch_resets_the_prior_result_at_the_central_boundary() {
    leak_free(|i| {
        i.register_builtin(b"resultless", resultless_builtin);
        assert_eq!(i.eval_str(b"set stale prior"), Code::Ok);
        assert_eq!(i.result_bytes(), b"prior");

        let command = new_string(b"resultless");
        // `dispatch` borrows argv whose owners retain each object.
        unsafe { obj::incr_ref_count(command) };
        assert_eq!(i.dispatch(&[command]), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
        unsafe { obj::decr_ref_count(command) };
    });
}

#[test]
fn command_substitution_propagates_non_error_codes() {
    // A non-OK code other than `Error` (here `[return]`) propagates out of
    // `[...]` rather than being treated as an ordinary value (C Tcl). The
    // path is uniform (`code != Ok → Err(code)`), so this also covers
    // `break`/`continue` once those commands land.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set x [return foo]"), Code::Return);
        assert_eq!(i.result_bytes(), b"foo");
    });
}

#[test]
fn scalar_read_of_array_reports_variable_is_array() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set a(k) v"), Code::Ok);
        assert_eq!(i.eval_str(b"set a"), Code::Error);
        assert_eq!(i.result_bytes(), b"can't read \"a\": variable is array");
    });
}

#[test]
fn expand_split_error_names_the_right_failure() {
    // `{*}` over a value whose list form has an unmatched quote reports the
    // quote failure, not a hardcoded brace message.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set s {\"abc}"), Code::Ok); // s = "abc
        assert_eq!(i.eval_str(b"list {*}$s"), Code::Error);
        assert_eq!(i.result_bytes(), b"unmatched open quote in list");
    });
}
