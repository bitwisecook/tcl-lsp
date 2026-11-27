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

//! `namespace which` and `namespace origin`: lookup order, cached command names and byte-valued names.

use super::*;

#[test]
fn original_jim_which_helper_preserves_public_and_direct_cold_and_hot_getter_order() {
    use tcl_runtime_api::Namespaces;
    let evidence = include_str!(
        "../../../tests/data/native_jim_namespace_canonical/which-helper-20261015.tsv"
    );
    let row = |kind: &str, label: &str| {
        evidence
            .lines()
            .find(|line| {
                let mut fields = line.split('|');
                fields.next() == Some(kind) && fields.next() == Some(label)
            })
            .unwrap()
            .split('|')
            .collect::<Vec<_>>()
    };
    for (label, option, case, direct) in [
        (
            "PUBLIC_VARIABLE_ABSOLUTE",
            b"-variable".as_slice(),
            0,
            false,
        ),
        (
            "PUBLIC_VARIABLE_RELATIVE",
            b"-variable".as_slice(),
            1,
            false,
        ),
        ("PUBLIC_COMMAND_ABSOLUTE", b"-command".as_slice(), 2, false),
        ("PUBLIC_COMMAND_RELATIVE", b"-command".as_slice(), 3, false),
        ("DIRECT_VARIABLE_ABSOLUTE", b"-variable".as_slice(), 0, true),
        ("DIRECT_VARIABLE_RELATIVE", b"-variable".as_slice(), 1, true),
        ("DIRECT_COMMAND_ABSOLUTE", b"-command".as_slice(), 2, true),
        ("DIRECT_COMMAND_RELATIVE", b"-command".as_slice(), 3, true),
    ] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        crate::cmd_proc::install_stock_scripted_library(
            &mut interp,
            tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers,
        );
        let context = interp.native_jim_object_context().unwrap();
        let namespace = obj::Owned::fresh(obj::new_wide_int_obj(42));
        crate::native_source::bind_context(namespace.as_ptr(), &context).unwrap();
        let name = obj::Owned::fresh(obj::alloc_typed(&WHICH_PROBE_NAME, case));
        crate::native_source::bind_context(name.as_ptr(), &context).unwrap();
        let head = obj::Owned::fresh(obj::new_string_bytes(if direct {
            b"namespace which"
        } else {
            b"namespace"
        }));
        let which = obj::Owned::fresh(obj::new_string_bytes(b"which"));
        let option = obj::Owned::fresh(obj::new_string_bytes(option));
        let argv = if direct {
            vec![head.as_ptr(), option.as_ptr(), name.as_ptr()]
        } else {
            vec![
                head.as_ptr(),
                which.as_ptr(),
                option.as_ptr(),
                name.as_ptr(),
            ]
        };
        let body = obj::Owned::fresh(interp.new_list_object(&argv));
        WHICH_NAMESPACE.with(|slot| slot.set(namespace.as_ptr()));
        for hot in [false, true] {
            let phase = format!("{label}_{}", if hot { "HOT" } else { "COLD" });
            if hot {
                assert_eq!(
                    obj::with_jim_command_cache(head.as_ptr(), |cache| cache.namespace.as_ptr()),
                    Some(namespace.as_ptr()),
                );
                obj::invalidate_string(namespace.as_ptr());
                obj::invalidate_string(name.as_ptr());
            }
            let current = interp
                .namespaces_mut()
                .retain_unmaterialized_jim_namespace(obj::Owned::retain(namespace.as_ptr()));
            assert_eq!(
                obj::has_string_rep(namespace.as_ptr()),
                row("NAMESPACE_BEFORE", &phase)[2] == "1"
            );
            assert!(interp.namespaces().jim_namespace_bytes(current).is_none());
            let lookup = interp.namespace_variable_name_bytes_checked(
                tcl_runtime_api::NsId(current as u32),
                b"absent",
            );
            assert!(lookup.is_err());
            assert!(!obj::has_string_rep(namespace.as_ptr()));
            WHICH_NAME_OBSERVATIONS.with(|observations| observations.borrow_mut().clear());
            assert_eq!(
                interp.ns_eval_in_token(
                    current,
                    &[],
                    None,
                    false,
                    Some((
                        tcl_native_model::native_eval_object::EvalObjectPurpose::NamespaceBody,
                        body.as_ptr()
                    )),
                    None
                ),
                Code::Ok,
                "{phase}"
            );
            assert_eq!(
                WHICH_NAME_OBSERVATIONS.with(|observations| observations.borrow().clone()),
                vec![row("NAMESPACE_AT_NAME_GETTER", &phase)[2] == "1"]
            );
            assert_eq!(
                obj::has_string_rep(namespace.as_ptr()),
                row("NAMESPACE_AFTER", &phase)[2] == "1"
            );
            assert_eq!(
                obj::obj_type_ptr(name.as_ptr()),
                &WHICH_PROBE_NAME as *const obj::TclObjType
            );

            let expected = row("RESULT", &phase)[3]
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(interp.result_bytes(), expected, "{phase}");
            assert!(interp.native_execution_refusal().is_none());
        }
        interp.set_current_ns(crate::namespace::GLOBAL);
        WHICH_NAMESPACE.with(|slot| slot.set(std::ptr::null_mut()));
    }
}

#[test]
fn jim_namespace_which_dispatches_the_actual_replaceable_helper_allocation() {
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    crate::cmd_proc::install_stock_scripted_library(
        &mut interp,
        tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers,
    );
    let helper = interp
        .resolve_cmd_token(b"namespace which")
        .expect("actual helper allocation");
    assert!(matches!(
        interp
            .namespaces()
            .command_by_generation(helper)
            .map(|(_, command)| command),
        Some(Command::Proc(_))
    ));
    assert_eq!(
        interp.eval_str(b"namespace which -variable absent"),
        Code::Ok
    );
    assert_eq!(interp.result_bytes(), b"::absent");
    assert_eq!(interp.eval_str(b"rename {namespace which} saved"), Code::Ok);
    assert_eq!(interp.resolve_cmd_token(b"saved"), Some(helper));
    assert_eq!(
        interp.eval_str(
            b"proc {namespace which} args {return OVERRIDE}; namespace which -variable absent"
        ),
        Code::Ok
    );
    assert_eq!(interp.result_bytes(), b"OVERRIDE");
    let replacement = interp.resolve_cmd_token(b"namespace which").unwrap();
    assert_ne!(replacement, helper);
    interp.install_stock_jim_namespace_helpers();
    assert_eq!(
        interp.resolve_cmd_token(b"namespace which"),
        Some(replacement)
    );
    assert_eq!(
        interp.eval_str(b"{namespace which} -variable absent"),
        Code::Ok
    );
    assert_eq!(interp.result_bytes(), b"OVERRIDE");
    assert_eq!(
        interp.eval_str(b"rename {namespace which} {}; namespace which -variable absent"),
        Code::Error
    );
    assert!(interp.resolve_cmd_token(b"namespace which").is_none());
    assert!(interp.native_execution_refusal().is_none());
}

#[test]
fn jim_namespace_which_refresh_preserves_the_actual_replacement() {
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interp.eval_str(b"proc {namespace which} args {return SELECTED}"),
        Code::Ok
    );
    let selected = interp.resolve_cmd_token(b"namespace which").unwrap();
    interp.set_dialect_profile(crate::environment::profile_for_dialect("tcl8.6"));
    assert_eq!(interp.resolve_cmd_token(b"namespace which"), Some(selected));
    interp.set_dialect_profile(crate::environment::profile_for_dialect("jimtcl"));
    assert_eq!(interp.resolve_cmd_token(b"namespace which"), Some(selected));
    assert_eq!(
        interp.eval_str(b"namespace which -variable ignored"),
        Code::Ok
    );
    assert_eq!(interp.result_bytes(), b"SELECTED");
    assert!(interp.native_execution_refusal().is_none());
}

#[test]
fn origin_reuses_the_original_stringless_command_cache() {
    for version in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"proc p {} {return P}"), Code::Ok);
        let words = [b"namespace".as_slice(), b"origin", b"p"]
            .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)));
        let argv = words.each_ref().map(|word| word.as_ptr());
        assert_eq!(super::super::ns_origin(&mut interp, &argv), Code::Ok);
        let cache = obj::native_command_name_cache(argv[2]).unwrap();
        obj::invalidate_string(argv[2]);
        assert_eq!(super::super::ns_origin(&mut interp, &argv), Code::Ok);
        assert_eq!(interp.result_bytes(), b"::p");
        assert!(!obj::has_string_rep(argv[2]));
        assert_eq!(obj::native_command_name_cache(argv[2]), Some(cache));
        assert!(!interp.host_refusal_pending());
    }
}

#[test]
fn origin_refuses_a_stringless_original_namespace_name_before_lookup() {
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(interp.eval_str(b"namespace current"), Code::Ok);
    let original = obj::Owned::retain(interp.result_obj());
    assert!(obj::native_namespace_name::cache(original.as_ptr()).is_some());
    obj::invalidate_string(original.as_ptr());
    interp.set_result_bytes(b"BEFORE");
    let before = interp.result_obj();
    let head = obj::Owned::fresh(obj::new_string_bytes(b"namespace"));
    let subcommand = obj::Owned::fresh(obj::new_string_bytes(b"origin"));
    assert_eq!(
        super::super::ns_origin(
            &mut interp,
            &[head.as_ptr(), subcommand.as_ptr(), original.as_ptr()]
        ),
        Code::Error
    );
    assert!(interp.host_refusal_pending());
    assert_eq!(interp.result_obj(), before);
    assert!(!obj::has_string_rep(original.as_ptr()));
}

#[test]
fn original_namespace_origin_matches_five_native_import_chains() {
    // Native proof: naming.namespace-origin.script-import-chain
    // docs/design/analysis/name-resolution-proofs/namespace-origin.script-import-chain.md
    fn decode(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut compared = 0;
    let fixture = include_str!("../../../tests/data/native_namespace_origin/windows.tsv");
    for (_, fields) in tcl_test_support::fixture::tsv_rows::<4>(fixture) {
        let release =
            tcl_test_support::fixture::oracle_release(fields[0]).expect("modelled release");
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(release.dialect_profile_name()),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interp.eval_str(&decode(fields[1])).as_int(),
            fields[2].parse::<i64>().unwrap(),
            "{}",
            fields[0]
        );
        assert_eq!(interp.result_bytes(), decode(fields[3]), "{}", fields[0]);
        assert!(!interp.host_refusal_pending());
        compared += 1;
    }
    assert_eq!(compared, 5);
}

#[test]
fn which_resolves_command_fqn() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"namespace which -command set"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::set");
        assert_eq!(i.eval_str(b"namespace which nope"), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
        // Prefix abbreviation of the flags (`-com`, `-var`).
        assert_eq!(i.eval_str(b"namespace which -com set"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::set");
    });
}

#[test]
fn which_variable_resolves_namespace_var_fqn() {
    leak_free(|i| {
        // `namespace which -variable` resolves a namespace variable to its
        // FQN (ignoring local proc links); a missing one yields "".
        assert_eq!(
            i.eval_str(b"namespace eval ::n { variable gv 1 }"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"namespace which -variable ::n::gv"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::n::gv");
        assert_eq!(i.eval_str(b"namespace which -var ::n::gv"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::n::gv");
        assert_eq!(i.eval_str(b"namespace which -variable ::n::nope"), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
    });
}

// Each expectation below is pinned against tclsh 8.6.16 and 9.0.4; the
// interpreter emulates 9.0 by default, so a release-axis vector says so.

/// `namespace which -variable` is `Tcl_FindNamespaceVar`: namespace
/// variable tables only, never a call frame.
#[test]
fn which_variable_never_answers_with_a_proc_local() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(b"proc t {} {set loc 1; namespace which -variable loc}; t"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"");
        // A local shadowing a real namespace variable still reports the
        // namespace one.
        assert_eq!(
            i.eval_str(
                b"namespace eval ns2 {variable shadow 5\n\
                      proc q {} {set shadow 9; namespace which -variable shadow}}\n\
                      ns2::q"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::ns2::shadow");
    });
}

/// `namespace origin` follows `namespace import` links to their source
/// through the shared `TclGetOriginalCommand` core.
#[test]
fn origin_follows_import_chains() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval src {namespace export p; proc p {} {return P}}\n\
                      namespace eval dst {namespace import ::src::p}\n\
                      namespace origin ::dst::p"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::src::p");
        assert_eq!(i.eval_str(b"namespace origin set"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::set");
    });
}

/// `namespace which -variable` is byte-preserving: a name carrying bytes
/// that are not valid UTF-8 round-trips, and two such names stay distinct
/// (a lossy map would collapse both onto U+FFFD).
#[test]
fn which_variable_preserves_byte_valued_names() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"namespace eval nb {}"), Code::Ok);
        assert_eq!(
            i.eval_str(b"namespace eval nb [list variable [binary format H* ff41] 7]"),
            Code::Ok
        );
        assert_eq!(
            i.eval_str(
                b"binary scan [namespace eval nb [list namespace which -variable \
                       [binary format H* ff41]]] H* h; set h"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"3a3a6e623a3aff41");
        // Two distinct invalid-UTF-8 names must not collide.
        assert_eq!(
            i.eval_str(
                b"namespace eval nb [list variable [binary format H* ff] 1]\n\
                       namespace eval nb [list variable [binary format H* fe] 2]\n\
                       namespace eval nb {list [set [binary format H* ff]] \
                       [set [binary format H* fe]]}"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"1 2");
    });
}

/// The same byte fidelity for the *command* half of the namespace surface:
/// `namespace origin`, `namespace which -command`, and the TclOO
/// object-name resolution behind them. These reach their tables through
/// the shared core's byte-valued entry points, so an invalid-UTF-8 name is
/// never routed through `str`. tclsh 9.0.4-pinned (`binary encode hex` of
/// each answer).
#[test]
fn origin_and_which_command_preserve_byte_valued_names() {
    leak_free(|i| {
        // An imported command whose simple name is the single byte 0xFF.
        assert_eq!(
            i.eval_str(
                b"set n [binary format H* ff]\n\
                      namespace eval src [list namespace export $n]\n\
                      namespace eval src [list proc $n {} {return P}]\n\
                      namespace eval dst [list namespace import ::src::$n]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.eval_str(b"binary encode hex [namespace origin ::dst::$n]"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"3a3a7372633a3aff");
        assert_eq!(i.eval_str(b"::dst::$n"), Code::Ok);
        assert_eq!(i.result_bytes(), b"P");
        // Two distinct byte names must stay distinct through `origin` —
        // a lossy map would collapse both onto U+FFFD and collide them.
        assert_eq!(
            i.eval_str(
                b"set a [binary format H* ff]; set b [binary format H* fe]\n\
                      namespace eval s2 [list namespace export $a $b]\n\
                      namespace eval s2 [list proc $a {} {return A}]\n\
                      namespace eval s2 [list proc $b {} {return B}]\n\
                      list [binary encode hex [namespace origin ::s2::$a]] \
                           [binary encode hex [namespace origin ::s2::$b]]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"3a3a73323a3aff 3a3a73323a3afe");
        // `namespace which -command` over the same name.
        assert_eq!(
            i.eval_str(b"binary encode hex [namespace which -command ::s2::$a]"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"3a3a73323a3aff");
        i.eval_str(b"unset -nocomplain n a b");
    });
}
