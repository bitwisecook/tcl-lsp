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

//! `namespace export`, `import` and `forget`: import chains, source generations, cycles and the command lifecycle of replaced imports.

use super::*;

#[test]
fn import_redefinition_rename_reexport_and_retirement_follow_dialect() {
    let jim = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    )));
    let script = br#"namespace eval S {proc p {} {return ONE}; namespace export *}; namespace eval D {namespace import ::S::*; namespace export *}; namespace eval E {namespace import ::D::*}; namespace eval S {proc p {} {return TWO}}; set result [list [D::p] [E::p] [namespace origin E::p]]; rename S::p S::moved; lappend result [catch {D::p} value] [catch {namespace origin E::p} value]; proc S::p {} {return THREE}; lappend result [D::p] [E::p]; rename S::moved {}; lappend result [llength [info commands D::p]] [llength [info commands E::p]]; set result"#;
    for profile in tcl_dialect::TclVersion::ALL
        .into_iter()
        .map(|release| {
            tcl_dialect::DialectProfile::find(release.dialect_profile_name()).expect("C profile")
        })
        .chain(std::iter::once(&*jim))
    {
        leak_free_with(
            || {
                let mut interp = Interp::with_native_core(
                    crate::interp::default_host(),
                    profile,
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                crate::cmd_proc::install_stock_scripted_wrappers(&mut interp);
                interp
            },
            |interp| {
                assert_eq!(
                    interp.eval_str(script),
                    Code::Ok,
                    "{}: {:?}",
                    profile.name,
                    interp.result_bytes()
                );
                let expected = if profile.namespace_import_binding()
                    == Some(tcl_dialect::NamespaceImportBinding::SourceName)
                {
                    b"TWO TWO ::S::p 1 1 THREE THREE 1 1".as_slice()
                } else {
                    b"TWO TWO ::S::p 0 0 TWO TWO 0 0".as_slice()
                };
                assert_eq!(interp.result_bytes(), expected, "{}", profile.name);
            },
        );
    }
}

#[test]
fn jim_import_ignores_exports_overwrites_and_retains_missing_source_alias() {
    let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    )));
    leak_free_with(
        || {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            crate::cmd_proc::install_stock_scripted_wrappers(&mut interp);
            interp
        },
        |interp| {
            let script = br#"namespace eval S {proc p {} {return SOURCE}}; namespace eval D {proc p {} {return DESTINATION}; namespace import -force ::S::*; set imported [namespace import]}; rename S::p {}; set result [list [llength [info commands D::p]] [catch {D::p}] [catch {namespace origin D::p}] [namespace eval D {namespace import ::missing::*}] [namespace eval D {namespace export}]]; proc S::p {} {return REVIVED}; lappend result [D::p] [namespace origin D::p] [catch {namespace path}]; set result"#;
            assert_eq!(
                interp.eval_str(script),
                Code::Ok,
                "{:?}",
                interp.result_bytes()
            );
            assert_eq!(interp.result_bytes(), b"1 1 1 {} {} REVIVED ::S::p 1");
        },
    );
}

#[test]
fn export_import_forget_roundtrip() {
    leak_free(|i| {
        // ::lib exports g* ; provide a command ::lib::greet via an alias.
        i.eval_str(b"interp alias {} ::lib::greet {} set greeted");
        i.eval_str(b"namespace eval lib { namespace export g* }");
        // Import into ::app, then call the bare name there.
        assert_eq!(
            i.eval_str(b"namespace eval app { namespace import ::lib::* }"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"namespace eval app { greet hi }"), Code::Ok);
        // `set greeted` ran in ::app (the call's current namespace).
        assert_eq!(i.eval_str(b"set ::app::greeted"), Code::Ok);
        assert_eq!(i.result_bytes(), b"hi");
        // Forget removes the redirect.
        assert_eq!(
            i.eval_str(b"namespace eval app { namespace forget ::lib::* }"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"namespace eval app { greet hi }"), Code::Error);
    });
}

fn forget_native_fields(stdout: &'static str) -> std::collections::BTreeMap<&'static str, Vec<u8>> {
    stdout
        .lines()
        .map(|line| {
            let (name, hex) = line
                .strip_prefix("FIELD ")
                .unwrap()
                .split_once(' ')
                .unwrap();
            let bytes = hex
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            (name, bytes)
        })
        .collect()
}

#[test]
fn native_c86_forget_global_origin_matches_original_issue_witness() {
    let fields = forget_native_fields(include_str!(
            "../../../../../experiments/name-handling/results/namespace-forget-global-origin/results/tcl8.6.18/stdout.txt"
        ));
    leak_free_with(
        || native_namespace_interp(tcl_dialect::TclVersion::V8_6),
        |interp| {
            assert_eq!(fields["setup_code"], b"0");
            let code = interp.eval_str(include_bytes!("../../../../../experiments/name-handling/results/namespace-forget-global-origin/issue-witness.tcl"));
            assert_eq!(
                code.as_int().to_string().as_bytes(),
                fields["positive_code"]
            );
            assert_eq!(interp.result_bytes(), fields["positive_result"]);
            assert!(!interp.host_refusal_pending());
        },
    );
}

#[test]
fn native_c86_forget_global_origin_matches_original_adverse_controls() {
    let fields = forget_native_fields(include_str!(
            "../../../../../experiments/name-handling/results/namespace-forget-global-origin/results/tcl8.6.18/stdout.txt"
        ));
    let probe = include_str!(
        "../../../../../experiments/name-handling/results/namespace-forget-global-origin/probe.tcl"
    );
    let block = |start: &str, end: &str| {
        probe
            .split_once(start)
            .unwrap()
            .1
            .split_once(end)
            .unwrap()
            .0
            .as_bytes()
    };
    let line = |prefix: &str| {
        probe
            .lines()
            .find(|line| line.starts_with(prefix))
            .unwrap()
            .as_bytes()
    };
    leak_free_with(
        || native_namespace_interp(tcl_dialect::TclVersion::V8_6),
        |interp| {
            assert_eq!(
                interp.eval_str(block("set setup_code [catch {\n", "} setup_result]")),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), fields["setup_result"]);
            assert_eq!(
                interp.eval_str(block("set positive_code [catch {\n", "} positive_result]")),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), fields["positive_result"]);
            assert_eq!(
                interp.eval_str(block(
                    "set adverse_setup_code [catch {\n",
                    "} adverse_setup_result]"
                )),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), fields["adverse_setup_result"]);
            assert_eq!(interp.eval_str(line("set adverse_code ")), Code::Ok);
            assert_eq!(interp.result_bytes(), fields["adverse_code"]);
            assert_eq!(interp.eval_str(b"set adverse_result"), Code::Ok);
            assert_eq!(interp.result_bytes(), fields["adverse_result"]);
            assert_eq!(interp.eval_str(b"set ::errorCode"), Code::Ok);
            assert_eq!(interp.result_bytes(), fields["adverse_error_code"]);
            assert_eq!(interp.eval_str(b"llength [info commands ::n::h]"), Code::Ok);
            assert_eq!(interp.result_bytes(), fields["imported_other_present"]);
            let local_forget = line("field local_forget_code ")
                .strip_prefix(b"field local_forget_code [")
                .unwrap()
                .strip_suffix(b"]")
                .unwrap();
            assert_eq!(interp.eval_str(local_forget), Code::Ok);
            assert_eq!(interp.result_bytes(), fields["local_forget_code"]);
            assert_eq!(interp.eval_str(b"set local_forget_result"), Code::Ok);
            assert_eq!(interp.result_bytes(), fields["local_forget_result"]);
            for (query, field) in [
                (
                    b"llength [info commands ::n::local]".as_slice(),
                    "local_present",
                ),
                (
                    b"llength [info commands ::n::h]".as_slice(),
                    "imported_other_after_local_forget",
                ),
                (
                    b"llength [info commands ::g]".as_slice(),
                    "global_source_present",
                ),
            ] {
                assert_eq!(interp.eval_str(query), Code::Ok);
                assert_eq!(interp.result_bytes(), fields[field]);
            }
            assert!(!interp.host_refusal_pending());
        },
    );
}

#[test]
fn native_c86_forget_matches_original_import_chain_controls() {
    let fields = forget_native_fields(include_str!(
            "../../../../../experiments/name-handling/results/namespace-forget-import-chain/results/tcl8.6.18/stdout.txt"
        ));
    let probe = include_str!(
        "../../../../../experiments/name-handling/results/namespace-forget-import-chain/probe.tcl"
    );
    let block = |start: &str, end: &str| {
        probe
            .split_once(start)
            .unwrap()
            .1
            .split_once(end)
            .unwrap()
            .0
            .as_bytes()
    };
    leak_free_with(
        || native_namespace_interp(tcl_dialect::TclVersion::V8_6),
        |interp| {
            let setup = interp.eval_str(block("set setup_code [catch {\n", "} setup_result]"));
            assert_eq!(setup.as_int().to_string().as_bytes(), fields["setup_code"]);
            assert_eq!(interp.result_bytes(), fields["setup_result"]);
            for case in [
                "ultimate",
                "immediate",
                "middle",
                "tail_precedence",
                "own_name",
            ] {
                let start = format!("set {case}_code [catch {{\n");
                let end = format!("}} {case}_result]");
                let code = interp.eval_str(block(&start, &end));
                assert_eq!(
                    code.as_int().to_string().as_bytes(),
                    fields[format!("{case}_code").as_str()],
                    "{case}"
                );
                assert_eq!(
                    interp.result_bytes(),
                    fields[format!("{case}_result").as_str()],
                    "{case}"
                );
                assert!(!interp.host_refusal_pending(), "{case}");
            }
        },
    );
}

#[test]
fn reimport_same_source_is_idempotent() {
    // Re-importing the *same* command from the *same* source is a silent
    // no-op (C's reimport check) — the common case where a file and its
    // sourced helper both `namespace import ::lib::*` (e.g. tcltest). Only a
    // clobber of a *different* command is a conflict (without -force).
    leak_free(|i| {
        i.eval_str(b"namespace eval lib { proc g {} {return G} ; namespace export g }");
        assert_eq!(i.eval_str(b"namespace import ::lib::*"), Code::Ok);
        // Second import of the same command from the same source: no error.
        assert_eq!(i.eval_str(b"namespace import ::lib::*"), Code::Ok);
        assert_eq!(i.eval_str(b"namespace import ::lib::g"), Code::Ok);
        // A different command of the same simple name does conflict.
        i.eval_str(b"namespace eval other { proc g {} {return O} ; namespace export g }");
        assert_eq!(i.eval_str(b"namespace import ::other::*"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't import command \"g\": already exists"
        );
        // -force overrides the clobber.
        assert_eq!(i.eval_str(b"namespace import -force ::other::*"), Code::Ok);
    });
}

#[test]
fn import_only_takes_exported_commands() {
    leak_free(|i| {
        i.eval_str(b"interp alias {} ::lib::secret {} set s");
        i.eval_str(b"interp alias {} ::lib::pub {} set p");
        i.eval_str(b"namespace eval lib { namespace export pub }");
        i.eval_str(b"namespace eval app { namespace import ::lib::* }");
        // `pub` imported, `secret` not.
        assert_eq!(i.eval_str(b"namespace eval app { pub 1 }"), Code::Ok);
        assert_eq!(i.eval_str(b"namespace eval app { secret 1 }"), Code::Error);
    });
}

/// C imports hold the source's command *token*: renaming the source
/// keeps the import working and `namespace origin` reports the NEW name;
/// deleting the source leaves the import dangling. Pinned on tclsh
/// 8.6.16 / 9.0.4 (`rename ::src::e ::src::e2` → `::dst::e` still runs,
/// origin `::src::e2`; `rename ::src2::f ""` → `::dst2::f` errors).
#[test]
fn import_follows_source_rename_but_not_delete() {
    leak_free(|i| {
        i.eval_str(b"namespace eval src { namespace export e; proc e {} { return E } }");
        i.eval_str(b"namespace eval dst { namespace import ::src::e }");
        assert_eq!(i.eval_str(b"rename ::src::e ::src::e2"), Code::Ok);
        assert_eq!(i.eval_str(b"::dst::e"), Code::Ok);
        assert_eq!(i.result_bytes(), b"E");
        assert_eq!(i.eval_str(b"namespace origin ::dst::e"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::src::e2");
        // Delete dangles (lazy miss), as in C.
        assert_eq!(i.eval_str(b"rename ::src::e2 {}"), Code::Ok);
        assert_eq!(i.eval_str(b"::dst::e"), Code::Error);
    });
}

#[test]
fn export_query_returns_patterns() {
    leak_free(|i| {
        i.eval_str(b"namespace eval lib { namespace export a* b* }");
        assert_eq!(
            i.eval_str(b"namespace eval lib { namespace export }"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"a* b*");
    });
}

/// Retained and recreated namespace tokens may expose identical command
/// FQNs simultaneously. Imports, renames, origins, dispatch, and teardown
/// remain qualified by the exact source command generation throughout.
#[test]
fn retained_and_recreated_import_sources_stay_generation_distinct() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval N {
                          proc x {} {return OLD}
                          proc p {} {
                              namespace delete ::N
                              namespace eval ::N {
                                  proc x {} {return NEW}
                                  namespace export x
                              }
                              namespace eval ::B {namespace import ::N::x}
                              rename x y
                              list [namespace origin ::A::x] \
                                   [namespace origin ::B::x] \
                                   [::A::x] [::B::x] [y]
                          }
                          namespace export x p
                      }
                      namespace eval A {namespace import ::N::x}
                      set inside [N::p]
                      list $inside [namespace origin ::B::x] [B::x] \
                           [info commands ::A::x]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"{::N::y ::N::x OLD NEW OLD} ::N::x NEW {}"
        );
    });
}

/// `namespace origin` follows an imported source chain by command token,
/// even while a recreated namespace exposes a direct command at the same
/// intermediate FQN.
#[test]
fn retained_import_origin_chain_ignores_a_recreated_same_fqn_command() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval Root {
                          proc x {} {return ROOT}
                          namespace export x
                      }
                      namespace eval O {
                          namespace import ::Root::x
                          namespace export x
                      }
                      namespace eval N {
                          namespace import ::O::x
                          proc p {} {
                              namespace delete ::N
                              namespace eval ::N {proc x {} {return NEW}}
                              list [namespace origin ::A::x] [::A::x] \
                                   [namespace origin ::N::x] [::N::x]
                          }
                          namespace export x p
                      }
                      namespace eval A {namespace import ::N::x}
                      N::p",
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::Root::x ROOT ::N::x NEW");
    });
}

/// Same-origin reimport compares source generations, not display FQNs. A
/// retained old and recreated new `::N::x` are different import origins.
#[test]
fn same_fqn_different_generation_reimport_is_not_idempotent() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval N {
                          proc x {} {return OLD}
                          proc p {} {
                              namespace delete ::N
                              namespace eval ::N {
                                  proc x {} {return NEW}
                                  namespace export x
                              }
                              namespace eval ::A {
                                  set c [catch {namespace import ::N::x} m]
                                  list $c $m [namespace origin x] [x]
                              }
                          }
                          namespace export x p
                      }
                      namespace eval A {namespace import ::N::x}
                      N::p",
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"1 {can't import command \"x\": already exists} ::N::x OLD"
        );
    });
}

/// Command replacement adopts surviving imports onto the fresh source
/// generation, so its later rename and true deletion remain exact too.
#[test]
fn replacement_import_source_adopts_the_fresh_generation() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval S {
                          proc p {} {return OLD}
                          namespace export p
                      }
                      namespace eval I {namespace import ::S::p}
                      proc ::S::p {} {return NEW}
                      rename ::S::p ::S::q
                      set before [list [I::p] [namespace origin ::I::p]]
                      rename ::S::q {}
                      list $before [info commands ::I::p]",
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"{NEW ::S::q} {}");
    });
}

/// True command deletion removes every import of the source command,
/// including transitive and hidden ordinary-proc aliases. Recreating the
/// source binding does not resurrect any of them. Namespace teardown uses
/// the same generic origin-deletion seam.
#[test]
fn true_source_deletion_purges_generic_import_origins() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval S {
                          namespace export p
                          proc p {} {return OLD}
                      }
                      namespace eval I {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval J {namespace import ::I::p}
                      namespace import ::S::p
                      interp hide {} p heldImport
                      set before [list [namespace origin ::I::p] \
                                           [namespace origin ::J::p] [interp hidden {}]]
                      rename ::S::p {}
                      namespace eval S {proc p {} {return NEW}}
                      list {*}$before [info commands ::I::p] \
                           [info commands ::J::p] [interp hidden {}] \
                           [namespace eval I {namespace import}] \
                           [namespace eval J {namespace import}] \
                           [info commands ::p]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"::S::p ::S::p heldImport {} {} {} {} {} {}"
        );
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval S {
                          namespace export p
                          proc p {} {return OLD}
                      }
                      namespace eval I {namespace import ::S::p}
                      namespace delete ::S
                      namespace eval S {
                          namespace export p
                          proc p {} {return NEW}
                      }
                      list [info commands ::I::p] \
                           [namespace eval I {namespace import}]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"{} {}");
    });
}

/// Each import retains its immediate source command. Replacing the
/// intermediate import changes what downstream aliases invoke and report as
/// their origin; true deletion of that intermediate command removes them.
#[test]
fn transitive_import_retains_intermediate_binding_lifecycle() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval A {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {namespace import ::A::p}
                      proc ::A::p {} {return A}
                      set before [list [::B::p] [namespace origin ::B::p]]
                      rename ::A::p {}
                      list {*}$before [info commands ::B::p] \
                           [namespace eval B {namespace import}]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"A ::A::p {} {}");
    });

    // Independent true-delete path: A is still an imported command when it
    // is deleted. B is therefore deleted with A, while the original source
    // command in S remains live.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval A {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {namespace import ::A::p}
                      rename ::A::p {}
                      list [::S::p] [info commands ::B::p] \
                           [namespace eval B {namespace import}]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"S {} {}");
    });
}

#[test]
fn namespace_import_rejects_a_cycle_before_mutating_the_graph() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval S {
                          proc p {} {return S}
                          namespace export p
                      }
                      namespace eval A {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {
                          namespace import ::A::p
                          namespace export p
                      }
                      set no_force_code [catch {
                          namespace eval S {namespace import ::B::p}
                      } no_force_message no_force_options]
                      set no_force [list $no_force_code $no_force_message \
                          [dict get $no_force_options -errorcode] \
                          [namespace origin ::S::p] \
                          [namespace origin ::A::p] \
                          [namespace origin ::B::p]]
                      set force_code [catch {
                          namespace eval S {namespace import -force ::B::p}
                      } force_message force_options]
                      list {*}$no_force $force_code $force_message \
                           [dict get $force_options -errorcode] \
                           [namespace origin ::S::p] \
                           [namespace origin ::A::p] \
                           [namespace origin ::B::p]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"1 {can't import command \"p\": already exists} \
                  {TCL IMPORT OVERWRITE} ::S::p ::S::p ::S::p \
                  1 {import pattern \"::B::p\" would create a loop containing command \"::S::p\"} \
                  {TCL IMPORT LOOP} ::S::p ::S::p ::S::p"
        );
    });
}

#[test]
fn namespace_import_cycle_reports_the_root_command_from_its_slot() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc p {} {return P}
                      namespace export p
                      namespace eval S {
                          namespace import ::p
                          namespace export p
                      }
                      set code [catch {namespace import -force ::S::p} message options]
                      list $code $message [dict get $options -errorcode] \
                           [namespace origin ::p] [namespace origin ::S::p]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"1 {import pattern \"::S::p\" would create a loop containing command \"::p\"} \
                  {TCL IMPORT LOOP} ::p ::p"
        );
    });
}

/// `namespace import -force` is command replacement, so it uses the same
/// lifecycle as `proc` redefinition: the displaced token's delete trace
/// runs while the old command remains visible, and none of its command or
/// execution trace sidecars transfer to the fresh imported token.
#[test]
fn forced_import_replacement_runs_command_lifecycle() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set seen {}
                      proc deleted {old new op} {
                          lappend ::seen [list delete $old [info commands $old]]
                      }
                      proc entered {cmd op} {
                          lappend ::seen [list enter $cmd $op]
                      }
                      namespace eval S {
                          proc p {} {return SRC}
                          namespace export p
                      }
                      namespace eval D {proc p {} {return OLD}}
                      trace add command ::D::p delete deleted
                      trace add execution ::D::p enter entered
                      namespace eval D {namespace import -force ::S::p}
                      set traces [list [trace info command ::D::p] \
                                           [trace info execution ::D::p]]
                      set result [::D::p]
                      list $seen $traces $result [namespace origin ::D::p]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"{{delete ::D::p ::D::p}} {{} {}} SRC ::S::p"
        );
    });
}

/// Imported-command delete traces run before unbinding, and a command
/// recreated by the trace callback has a new identity that survives the old
/// import's deletion. Downstream imports then resolve through that live
/// intermediate replacement.
#[test]
fn imported_delete_trace_observes_and_can_replace_the_binding() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval I {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {namespace import ::I::p}
                      set seen {}
                      proc cb {old new op} {
                          lappend ::seen [info commands $old]
                          proc $old {} {return REBORN}
                      }
                      trace add command ::I::p delete cb
                      rename ::S::p {}
                      list $seen [::I::p] [::B::p] [namespace origin ::B::p]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::I::p REBORN REBORN ::I::p");
    });

    // Re-importing the same origin from the delete callback creates a fresh
    // imported-command identity. The outer deletion retires only the old
    // import even though both identities carry identical source metadata.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval A {namespace import ::S::p}
                      proc reimport {old new op} {
                          namespace eval ::A {namespace import -force ::S::p}
                      }
                      trace add command ::A::p delete reimport
                      rename ::A::p {}
                      list [info commands ::A::p] [::A::p] \
                           [namespace origin ::A::p]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::A::p S ::S::p");
    });

    // A trace may move the dying import itself. Identity cleanup follows
    // it and silently drops the moved sidecar, so a later unrelated
    // command at that name cannot fire the old trace.
    leak_free(|i| {
        let code = i.eval_str(
            b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval I {namespace import ::S::p}
                      set seen {}
                      proc move_import {old new op} {
                          lappend ::seen [list $old $new $op [info commands $old]]
                          rename $old ::I::q
                      }
                      trace add command ::I::p delete move_import
                      rename ::S::p {}
                      set first $seen
                      proc ::I::q {} {return unrelated}
                      rename ::I::q {}
                      list $first $seen [info commands ::I::p] \
                           [info commands ::I::q]",
        );
        assert_eq!(
            code,
            Code::Ok,
            "{}",
            String::from_utf8_lossy(&i.result_bytes())
        );
        assert_eq!(
            i.result_bytes(),
            b"{{::I::p {} delete ::I::p}} {{::I::p {} delete ::I::p}} {} {}"
        );
    });
}

/// Only `objv[1]` is the `-clear` / `-force` flag: the registry pins
/// `max_leading_option_words: Some(1)` and C tests that one word.
#[test]
fn only_the_first_word_is_the_export_or_import_flag() {
    leak_free(|i| {
        // tclsh 8.6.16 / 9.0.4: `-clear x`.
        assert_eq!(
            i.eval_str(
                b"namespace eval e {namespace export -clear -clear x}\n\
                      namespace eval e {namespace export}"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"-clear x");
        // A trailing `-force` is an ordinary pattern, and an unqualified
        // pattern names no source namespace.
        i.eval_str(b"namespace eval s3 {namespace export q; proc q {} {return Q}}");
        assert_eq!(
            i.eval_str(b"namespace eval t3 {catch {namespace import ::s3::q -force} m; set m}"),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"no namespace specified in import pattern \"-force\""
        );
        // …and the import that preceded it still happened.
        assert_eq!(i.eval_str(b"info commands ::t3::*"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::t3::q");
        i.eval_str(b"unset -nocomplain m");
    });
}

/// `namespace export -clear` empties the list before the patterns that
/// follow it are added.
#[test]
fn export_clear_resets_the_pattern_list() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval e {namespace export a b; namespace export -clear}\n\
                      namespace eval e {namespace export}"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"");
        assert_eq!(
            i.eval_str(
                b"namespace eval f {namespace export a b; namespace export -clear c}\n\
                      namespace eval f {namespace export}"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"c");
    });
}
