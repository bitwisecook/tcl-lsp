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

//! `namespace ensemble create`, `configure` and `exists`: option tables, map and prefix values, and error codes.

use super::*;

#[test]
fn ensemble_original_map_queries_do_not_extend_native_owner_lifetime() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
    {
        leak_free(|interp| {
            interp.set_runtime_version(version);
            let arguments: Vec<_> = [
                b"namespace".as_slice(),
                b"ensemble",
                b"create",
                b"-command",
                b"::E",
                b"-map",
                b"a ::target",
            ]
            .into_iter()
            .map(|word| crate::obj::Owned::fresh(crate::obj::new_string_bytes(word)))
            .collect();
            let pointers: Vec<_> = arguments.iter().map(crate::obj::Owned::as_ptr).collect();
            let original = arguments[6].as_ptr();
            assert_eq!(super::super::ens_create(interp, &pointers), Code::Ok);
            let token = interp.ensemble_config_at(b"::E").unwrap();
            let query = token.config();
            assert_eq!(
                crate::ensemble::NativeEnsembleObjects::pointer(&query.originals.map),
                Some(original)
            );
            // SAFETY: argv's owning handle retains this original header.
            let references = unsafe { (*original).ref_count };
            let another_query = query.clone();
            assert_eq!(unsafe { (*original).ref_count }, references);
            let configure: Vec<_> = [
                b"namespace".as_slice(),
                b"ensemble",
                b"configure",
                b"::E",
                b"-map",
            ]
            .into_iter()
            .map(|word| crate::obj::Owned::fresh(crate::obj::new_string_bytes(word)))
            .collect();
            let configure: Vec<_> = configure.iter().map(crate::obj::Owned::as_ptr).collect();
            assert_eq!(super::super::ens_configure(interp, &configure), Code::Ok);
            assert_eq!(interp.result_obj(), original);
            interp.set_result_bytes(b"");
            assert!(interp.delete_command(b"::E"));
            assert!(
                crate::ensemble::NativeEnsembleObjects::pointer(&query.originals.map).is_none()
            );
            assert!(
                crate::ensemble::NativeEnsembleObjects::pointer(&another_query.originals.map)
                    .is_none()
            );
            assert_eq!(unsafe { (*original).ref_count }, references - 1);
        });
    }
}

#[test]
fn original_ensemble_options_use_actual_release_tables() {
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        leak_free(|interp| {
            interp.set_runtime_version(version);
            let arguments: Vec<_> = [
                b"namespace".as_slice(),
                b"ensemble",
                b"create",
                b"-command",
                b"::E",
            ]
            .into_iter()
            .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
            .collect();
            assert_eq!(
                super::super::ens_create(
                    interp,
                    &arguments.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()
                ),
                Code::Ok
            );
            for (option, expected) in [
                (
                    b"-parameters".as_slice(),
                    if version == tcl_dialect::TclVersion::V8_5 {
                        Code::Error
                    } else {
                        Code::Ok
                    },
                ),
                (
                    b"-p".as_slice(),
                    if version == tcl_dialect::TclVersion::V8_5 {
                        Code::Ok
                    } else {
                        Code::Error
                    },
                ),
            ] {
                let arguments: Vec<_> = [
                    b"namespace".as_slice(),
                    b"ensemble",
                    b"configure",
                    b"::E",
                    option,
                ]
                .into_iter()
                .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
                .collect();
                assert_eq!(
                    super::super::ens_configure(
                        interp,
                        &arguments.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()
                    ),
                    expected
                );
            }
            let arguments: Vec<_> = [b"namespace".as_slice(), b"ensemble", b"configure", b"::E"]
                .into_iter()
                .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
                .collect();
            assert_eq!(
                super::super::ens_configure(
                    interp,
                    &arguments.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()
                ),
                Code::Ok
            );
            let protocol = interp
                .native_invocation_dialect()
                .native_string_materialization(None)
                .unwrap()
                .protocol();
            assert_eq!(
                list::list_elements_native_checked(interp.result_obj(), protocol)
                    .unwrap()
                    .len(),
                if version == tcl_dialect::TclVersion::V8_5 {
                    10
                } else {
                    12
                }
            );
        });
    }
}

#[test]
fn explicit_ensemble_members_and_unknown_relative_context_match_native_c() {
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        leak_free(|interp| {
            interp.set_runtime_version(version);
            let source = br#"proc target {} {return GLOBAL_NS}
proc return_relative {args} {return {target}}
namespace eval ::n {
    proc target {} {return ENSEMBLE_NS}
    namespace ensemble create -command ::e -map {a ::n::target b ::n::target} -subcommands a
    namespace ensemble create -command ::fallback -unknown ::return_relative
}
namespace eval ::caller {
    proc target {} {return CALLER_NS}
    list [catch {::e b} result] $result [::fallback absent]
}"#;
            assert_eq!(
                interp.eval_str(source),
                Code::Ok,
                "{version:?}: {:?}",
                interp.result_bytes()
            );
            let selected = if version == tcl_dialect::TclVersion::V8_5 {
                b"GLOBAL_NS".as_slice()
            } else {
                b"ENSEMBLE_NS"
            };
            let mut expected = b"1 {unknown or ambiguous subcommand \"b\": must be a} ".to_vec();
            expected.extend_from_slice(selected);
            assert_eq!(interp.result_bytes(), expected, "{version:?}");
        });
    }
}

#[test]
fn ensemble_configuration_preserves_native_raw_byte_list_words() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
    {
        leak_free(|interp| {
            interp.set_runtime_version(version);
            assert_eq!(
                interp.eval_str(b"namespace ensemble create -command ::E"),
                Code::Ok
            );
            let token = interp.ensemble_config_at(b"::E").unwrap();
            let mut config = token.config();
            let original = b"#\xff\0x y".to_vec();
            config.subcommands = Some(vec![original.clone()]);
            config.map = Some(vec![(
                b"key".to_vec(),
                vec![b"::target".to_vec(), original.clone()],
            )]);
            token.configure(config);
            for (option, expected) in [
                (b"-subcommands".as_slice(), b"{#\xff\0x y}".as_slice()),
                (
                    b"-map".as_slice(),
                    b"key {::target {#\xff\0x y}}".as_slice(),
                ),
            ] {
                let arguments: Vec<_> = [
                    b"namespace".as_slice(),
                    b"ensemble".as_slice(),
                    b"configure".as_slice(),
                    b"::E".as_slice(),
                    option,
                ]
                .into_iter()
                .map(|bytes| crate::obj::Owned::fresh(crate::obj::new_string_bytes(bytes)))
                .collect();
                let pointers: Vec<_> = arguments.iter().map(crate::obj::Owned::as_ptr).collect();
                assert_eq!(super::super::ens_configure(interp, &pointers), Code::Ok);
                // Pinned C8.5/C8.6/C9.0/C9.1 Tcl_EvalObjv observations,
                // including the literal invalid UTF-8 byte and embedded NUL.
                assert_eq!(interp.result_bytes(), expected, "{version:?}");
            }
            assert_eq!(token.config().subcommands, Some(vec![original]));
        });
    }
}

#[test]
fn ensemble_configuration_refuses_an_unselected_list_updater() {
    let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )));
    leak_free(|interp| {
        interp.set_dialect_profile(unknown);
        interp.set_result_bytes(b"UNCHANGED");
        let error = super::super::join_words(interp, &[b"opaque\xff".to_vec()]).unwrap_err();
        assert!(error.native_access_refusal().is_some());
        assert_eq!(interp.result_bytes(), b"UNCHANGED");
    });
}

#[test]
fn jim_namespace_ensemble_uses_the_mutable_helper_and_original_arguments() {
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
            interp.eval_str(br#"proc {namespace ensemble} {subcommand args} {return "[llength $args]:[lindex $args 0]:[lindex $args 1]"}"#),
            Code::Ok
        );
    assert_eq!(
        interp.eval_str(b"namespace ensemble custom {two words} tail"),
        Code::Ok
    );
    assert_eq!(interp.result_bytes(), b"2:two words:tail");
}

/// `-prefixes` reads its value through the runtime's one boolean
/// acceptor, so every spelling `tclsh9.0` accepts here is accepted: a
/// unique word prefix, and any number against zero. Only the ambiguous
/// `o` — shared by `on` and `off` — is refused.
#[test]
fn ensemble_prefixes_accepts_every_boolean_spelling_tclsh_does() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval ens { proc bar {} {}; namespace export bar; \
                      namespace ensemble create }"
            ),
            Code::Ok
        );
        for (spelling, expected) in [
            (&b"tru"[..], b"1".as_slice()),
            (b"ye", b"1"),
            (b"of", b"0"),
            (b"2", b"1"),
            (b"0.0", b"0"),
        ] {
            let mut script = b"namespace ensemble configure ::ens -prefixes ".to_vec();
            script.extend_from_slice(spelling);
            assert_eq!(
                i.eval_str(&script),
                Code::Ok,
                "{}: {}",
                String::from_utf8_lossy(spelling),
                String::from_utf8_lossy(&i.result_bytes())
            );
            assert_eq!(
                i.eval_str(b"namespace ensemble configure ::ens -prefixes"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), expected);
        }
        assert_eq!(
            i.eval_str(b"namespace ensemble configure ::ens -prefixes o"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"expected boolean value but got \"o\"");
    });
}

/// Exact Tcl 9.0.4 lookup/read-only error taxonomy for ensemble configure
/// and namespace origin.
#[test]
fn ensemble_configure_and_origin_error_codes() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc plain {} {}
                      namespace ensemble create -command ::E
                      set out {}
                      foreach script {
                          {namespace ensemble configure ::missing}
                          {namespace ensemble configure ::plain}
                          {namespace ensemble configure ::E -namespace ::N}
                          {namespace origin ::missing}
                      } {
                          catch $script m o
                          lappend out $m [dict get $o -errorcode]
                      }
                      set out"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"{unknown command \"::missing\"} {TCL LOOKUP COMMAND ::missing} \
                  {\"::plain\" is not an ensemble command} {TCL LOOKUP ENSEMBLE ::plain} \
                  {option -namespace is read-only} {TCL ENSEMBLE READ_ONLY} \
                  {invalid command name \"::missing\"} {TCL LOOKUP COMMAND ::missing}"
        );
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set out {}
                      foreach script {
                          {namespace origin {not here}}
                          {namespace ensemble configure {not here}}
                      } {
                          catch $script m o
                          lappend out $m [dict get $o -errorcode] \
                              [llength [dict get $o -errorcode]]
                      }
                      namespace ensemble create -command ::Q \
                          -subcommands {{not here}}
                      catch {::Q {also not here}} m o
                      lappend out $m [dict get $o -errorcode] \
                          [llength [dict get $o -errorcode]]
                      set out"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"{invalid command name \"not here\"} \
                  {TCL LOOKUP COMMAND {not here}} 4 \
                  {unknown command \"not here\"} \
                  {TCL LOOKUP COMMAND {not here}} 4 \
                  {unknown or ambiguous subcommand \"also not here\": must be not here} \
                  {TCL LOOKUP SUBCOMMAND {also not here}} 4"
        );
    });
}

#[test]
fn ensemble_command_option_and_prefixes_off() {
    leak_free(|i| {
        // -command names the ensemble cmd; -prefixes 0 disables prefix match.
        assert_eq!(
                i.eval_str(
                    b"namespace eval q { namespace ensemble create -command ::top -subcommands longname -map {longname ::set} -prefixes 0 }"
                ),
                Code::Ok
            );
        assert_eq!(i.eval_str(b"top longname v 9"), Code::Ok);
        assert_eq!(i.result_bytes(), b"9");
        assert_eq!(i.eval_str(b"top long v 9"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"unknown subcommand \"long\": must be longname"
        );
        i.eval_str(b"unset v");
    });
}

#[test]
fn ensemble_exists() {
    leak_free(|i| {
        i.eval_str(b"namespace eval foo { namespace ensemble create }");
        assert_eq!(i.eval_str(b"namespace ensemble exists foo"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(i.eval_str(b"namespace ensemble exists ::nope"), Code::Ok);
        assert_eq!(i.result_bytes(), b"0");
    });
}

#[test]
fn ensemble_command_is_qualified_relative_to_current_ns() {
    leak_free(|i| {
        // A relative `-command` binds in the current namespace, not global
        // (the `tcl::tm::path` / safe-base case): `-command path` inside
        // `::a::b` creates `::a::b::path`, resolvable by its FQN.
        assert_eq!(
                i.eval_str(
                    b"namespace eval a::b { namespace export path; namespace ensemble create -command path -map {list ::set} }"
                ),
                Code::Ok
            );
        assert_eq!(
            i.eval_str(b"namespace which -command ::a::b::path"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::a::b::path");
        // No bare `::path` leaked into the global namespace.
        assert_eq!(i.eval_str(b"namespace which -command ::path"), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
        assert_eq!(i.eval_str(b"::a::b::path list v 5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        i.eval_str(b"unset v");
    });
}

/// The ensemble option tables come from the shared owner: `create` has no
/// `-namespace`, both tables abbreviate, and the `namespace ensemble`
/// subcommand word abbreviates too.
#[test]
fn ensemble_option_tables_match_c() {
    leak_free(|i| {
        i.eval_str(b"namespace eval e5 {namespace export *; proc go {} {return G}}");
        assert_eq!(
            i.eval_str(b"namespace eval e5 {namespace ensemble create -comm ::ab5 -sub go}"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::ab5");
        assert_eq!(i.eval_str(b"namespace ensemble ex ::ab5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(
            i.eval_str(b"catch {namespace ensemble frobnicate} m; set m"),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"bad subcommand \"frobnicate\": must be configure, create, or exists"
        );
        assert_eq!(
            i.eval_str(b"catch {namespace ensemble create -namespace ::x} m; set m"),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"bad option \"-namespace\": must be -command, -map, -parameters, \
                  -prefixes, -subcommands, or -unknown"
        );
        i.eval_str(b"unset -nocomplain m");
    });
}

/// C's `if (objc & 1)` fires before any option word is looked at, so an
/// odd tail is `wrong # args`, never `bad option`.
#[test]
fn ensemble_create_checks_pair_arity_first() {
    leak_free(|i| {
        for src in [
            &b"catch {namespace ensemble create -command} m; set m"[..],
            b"catch {namespace ensemble create -bogus} m; set m",
        ] {
            assert_eq!(i.eval_str(src), Code::Ok);
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"namespace ensemble create ?option value ...?\""
            );
        }
        i.eval_str(b"unset -nocomplain m");
    });
}

/// `namespace ensemble configure` reads through the shared config table:
/// `-namespace` is readable but never writable, `-command` is not in it,
/// and abbreviations resolve.
#[test]
fn ensemble_configure_uses_the_config_table() {
    leak_free(|i| {
        i.eval_str(
            b"namespace eval e5 {namespace export *; proc go {} {return G}\n\
                  namespace ensemble create -command ::ab5 -subcommands go}",
        );
        assert_eq!(
            i.eval_str(b"namespace ensemble configure ::ab5 -sub"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"go");
        assert_eq!(
            i.eval_str(b"catch {namespace ensemble configure ::ab5 -namespace ::e5} m; set m"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"option -namespace is read-only");
        assert_eq!(
            i.eval_str(b"catch {namespace ensemble configure ::ab5 -command ::zz} m; set m"),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"bad option \"-command\": must be -map, -namespace, -parameters, \
                  -prefixes, -subcommands, or -unknown"
        );
        i.eval_str(b"unset -nocomplain m");
    });
}

/// Configuring an ensemble through a `namespace import` alias configures
/// the ORIGIN, so both spellings observe one config and the alias stays an
/// alias (tclsh 9.0.4-pinned).
#[test]
fn configuring_an_imported_ensemble_updates_the_origin() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval S {namespace export ens\n\
                       proc impl {} {return ORIG}\n\
                       proc impl2 {} {return NEW}\n\
                       namespace ensemble create -command ::S::ens -map {go impl}}\n\
                       namespace eval T {namespace import ::S::ens}"
            ),
            Code::Ok
        );
        assert_eq!(
            i.eval_str(b"namespace eval S {namespace ensemble configure ::T::ens -map {go impl2}}"),
            Code::Ok
        );
        for spelling in [b"::T::ens go".as_slice(), b"::S::ens go"] {
            assert_eq!(i.eval_str(spelling), Code::Ok);
            assert_eq!(i.result_bytes(), b"NEW");
        }
        for spelling in [
            b"namespace ensemble configure ::T::ens -map".as_slice(),
            b"namespace ensemble configure ::S::ens -map",
        ] {
            assert_eq!(i.eval_str(spelling), Code::Ok);
            assert_eq!(i.result_bytes(), b"go ::S::impl2");
        }
        // Still an alias: configuring it did not fork a second ensemble.
        assert_eq!(i.eval_str(b"namespace origin ::T::ens"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::S::ens");
    });
}

/// `-map` is a dict: insertion order round-trips through the read-back and
/// a repeated key keeps its first position while taking the last value
/// (tclsh 9.0.4-pinned).
#[test]
fn ensemble_map_preserves_dict_order_and_collapses_repeats() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval M {namespace export *\n\
                       proc zeta {} {return Z}\n\
                       proc alpha {} {return A}\n\
                       proc mid {} {return M}\n\
                       namespace ensemble create -command ::E -map {zz zeta aa alpha}}"
            ),
            Code::Ok
        );
        // Insertion order, not sorted (`aa` would sort first).
        assert_eq!(
            i.eval_str(b"namespace ensemble configure ::E -map"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"zz ::M::zeta aa ::M::alpha");
        // A repeated key: last value wins, first position kept.
        assert_eq!(
            i.eval_str(
                b"namespace eval M {namespace ensemble configure ::E \
                       -map {zz zeta aa alpha zz mid}}"
            ),
            Code::Ok
        );
        assert_eq!(
            i.eval_str(b"namespace ensemble configure ::E -map"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"zz ::M::mid aa ::M::alpha");
        // Dispatch follows the collapsed entry, not the stale first one.
        assert_eq!(i.eval_str(b"::E zz"), Code::Ok);
        assert_eq!(i.result_bytes(), b"M");
    });
}
