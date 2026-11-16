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

//! Calls through an ensemble: member lookup, prefixes, the unknown handler and the identity of the live command token.

use super::*;

#[test]
fn implicit_ensemble_targets_preserve_the_selected_namespace_context() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
    {
        leak_free_with(
            || native_namespace_interp(version),
            |interp| {
                assert_eq!(
                        interp.eval_str(br#"namespace eval ::M {
                            namespace ensemble create -command path -subcommands {add remove}
                            proc add {path} {return [list ADD [namespace current] $path]}
                        }
                        proc ::add {path} {return [list GLOBAL $path]}
                        proc ::remove {path} {return [list GLOBAL_REMOVE $path]}
                        namespace eval ::Caller {proc add {path} {return [list CALLER $path]}}
                        list [::M::path add x] [namespace eval ::Caller {::M::path add y}] [::add z]"#),
                        Code::Ok,
                        "{version:?}: {:?}",
                        interp.result_bytes(),
                    );
                assert_eq!(
                    interp.result_bytes(),
                    b"{ADD ::M x} {ADD ::M y} {GLOBAL z}",
                    "{version:?}"
                );
                assert_eq!(interp.eval_str(br#"set code [catch {namespace eval ::Caller {::M::path remove q}} result]; list $code $result"#), Code::Ok);
                let expected = if version == tcl_dialect::TclVersion::V8_5 {
                    b"1 {invalid command name \"::M::remove\"}".as_slice()
                } else {
                    b"0 {GLOBAL_REMOVE q}".as_slice()
                };
                assert_eq!(interp.result_bytes(), expected, "{version:?}");
                assert!(interp.native_execution_refusal().is_none());
            },
        );
    }
}

// ensembles (targets are aliases, since procs aren't available yet)

#[test]
fn ensemble_default_dispatches_to_namespace_commands() {
    leak_free(|i| {
        // ::foo::{bar,baz} as alias→set; export + ensemble over ::foo.
        i.eval_str(b"interp alias {} ::foo::bar {} set");
        i.eval_str(b"interp alias {} ::foo::baz {} set");
        assert_eq!(
            i.eval_str(
                b"namespace eval foo { namespace export bar baz; namespace ensemble create }"
            ),
            Code::Ok
        );
        // `foo bar v 42` → ::foo::bar v 42 → set v 42.
        assert_eq!(i.eval_str(b"foo bar v 42"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        assert_eq!(i.eval_str(b"set v"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        i.eval_str(b"unset v");
    });
}

#[test]
fn ensemble_prefix_match_and_ambiguity() {
    leak_free(|i| {
        i.eval_str(b"interp alias {} ::foo::bar {} set");
        i.eval_str(b"interp alias {} ::foo::baz {} set");
        i.eval_str(b"namespace eval foo { namespace export bar baz; namespace ensemble create }");
        // `ba`/`b` are ambiguous between bar and baz.
        assert_eq!(i.eval_str(b"foo ba v 1"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"unknown or ambiguous subcommand \"ba\": must be bar, or baz"
        );
        assert_eq!(i.eval_str(b"foo nope v 1"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"unknown or ambiguous subcommand \"nope\": must be bar, or baz"
        );
        i.eval_str(b"unset v"); // bar/baz never ran; v is unset → ignore result
    });
}

#[test]
fn ensemble_map_and_subcommands() {
    leak_free(|i| {
        // -map a subcommand to a concrete (builtin) target.
        assert_eq!(
            i.eval_str(b"namespace eval m { namespace ensemble create -map {go ::set} }"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"m go v 7"), Code::Ok); // → ::set v 7
        assert_eq!(i.result_bytes(), b"7");
        // a non-mapped subcommand is unknown (the map keys are the set).
        assert_eq!(i.eval_str(b"m set v 7"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"unknown or ambiguous subcommand \"set\": must be go"
        );
        i.eval_str(b"unset v");
    });
}

/// Vectors measured against tclsh 9.0.4: dict validation, callback
/// prefix redispatch/reparse, and the user-facing default target name.
#[test]
fn ensemble_issue_1583_oracle_vectors() {
    leak_free(|i| {
        for (script, message) in [
            (
                &b"namespace ensemble create -map {go}"[..],
                &b"missing value to go with key"[..],
            ),
            (
                b"namespace ensemble create -map {go {}}",
                b"ensemble subcommand implementations must be non-empty lists",
            ),
            (
                b"set badmap \"go \\{\"; namespace ensemble create -map $badmap",
                b"unmatched open brace in dict",
            ),
        ] {
            assert_eq!(i.eval_str(script), Code::Error);
            assert_eq!(i.result_bytes(), message);
        }

        assert_eq!(
            i.eval_str(
                b"proc uh {ens args} {return [list list REPLACED $ens]}
                      namespace eval se5 {
                          namespace ensemble create -command ::se5 -subcommands {} -unknown ::uh
                      }
                      ::se5 nope 1 2"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"REPLACED ::se5 1 2");

        assert_eq!(
            i.eval_str(
                b"proc define {ens args} {
                          namespace eval se6 {
                              proc nope args {return DEFINED}; namespace export nope
                          }
                          return {}
                      }
                      namespace eval se6 {
                          namespace ensemble create -command ::se6 -unknown ::define
                      }
                      ::se6 nope"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"DEFINED");

        assert_eq!(
            i.eval_str(
                b"proc target args {return TARGET}
                      proc repair {ens args} {
                          namespace ensemble configure $ens -map {nope ::target}; return {}
                      }
                      namespace eval se7 {
                          namespace ensemble create -command ::se7 -unknown ::repair
                      }
                      ::se7 nope"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"TARGET");

        assert_eq!(
            i.eval_str(
                b"namespace eval k1 {namespace ensemble create -subcommands ghost}
                      ::k1 ghost"
            ),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"invalid command name \"ghost\"");
    });
}

#[test]
fn ensemble_unknown_retains_the_live_command_token() {
    // Same-name create is replacement, not mutation: the callback's old
    // active token is dead even though a new ensemble occupies `::E`.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc new args {return NEW}
                      proc rebuild {ens args} {
                          namespace eval ::N {
                              namespace ensemble create -command ::E -map {x ::new}
                          }
                          return {}
                      }
                      namespace eval ::N {
                          namespace ensemble create -command ::E -unknown ::rebuild
                      }
                      set c [catch {::E nope} m o]
                      list $c $m [dict get $o -errorcode] [::E x]"
            ),
            Code::Ok
        );
        assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} NEW"
            );
    });

    // Replacement fires and drops the old command's delete trace.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set seen {}
                      proc deleted args {lappend ::seen $args}
                      namespace ensemble create -command ::E
                      trace add command ::E delete deleted
                      set replacement [namespace ensemble create -command ::E]
                      list $seen $replacement"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"{{::E {} delete}} ::E");
    });

    // The delete trace may move the captured old token. Replacement must
    // retire that identity at its new location, then install the fresh
    // ensemble at the original binding.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc move_old {old new op} {rename $old ::OLD}
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete move_old
                      namespace eval N {namespace ensemble create -command ::E}
                      list [info commands ::E] [info commands ::OLD] \
                           [namespace ensemble exists ::E] \
                           [namespace ensemble exists ::OLD]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::E {} 1 0");
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc target args {return TARGET}
                      namespace eval ER {
                          proc repair {ens args} {
                              namespace ensemble configure $ens -map {nope ::target}
                              rename $ens ::ER2
                              return {}
                          }
                          namespace ensemble create -command ::ER -unknown ::ER::repair
                      }
                      list [::ER nope] [namespace ensemble configure ::ER2 -map] [::ER2 nope]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"TARGET {nope ::target} TARGET");
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set seen {}
                      proc target args {return TARGET}
                      proc repair {ens args} {
                          set ::seen $ens
                          namespace ensemble configure $ens -map {nope ::target}
                          return {}
                      }
                      namespace eval S {
                          namespace export E
                          namespace ensemble create -command E -unknown ::repair
                      }
                      namespace eval I {namespace import ::S::E}
                      list [::I::E nope] $seen [namespace origin ::I::E]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"TARGET ::S::E ::S::E");
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc target args {return TARGET}
                      proc replace {ens args} {
                          rename $ens {}
                          namespace ensemble create -command $ens -map {nope ::target}
                          return \\{
                      }
                      namespace eval D {
                          namespace ensemble create -command E -unknown ::replace
                      }
                      set c [catch {::D::E nope} m o]
                      list $c $m [dict get $o -errorcode] [::D::E nope]"
            ),
            Code::Ok
        );
        assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} TARGET"
            );
    });

    leak_free(|i| {
        let code = i.eval_str(
                b"proc target args {return TARGET}
                      set seen {}
                      proc hide_repair {ens args} {
                          namespace ensemble configure $ens -map {nope ::target} -unknown ::hidden_repair
                          interp hide {} $ens heldE
                          return {}
                      }
                      proc hidden_repair {ens args} {
                          set ::seen $ens
                          return [list ::target]
                      }
                      namespace ensemble create -command ::EH -unknown ::hide_repair
                      set first [::EH nope]
                      set seen {}
                      set second [interp invokehidden {} heldE other]
                      list $first $second $seen [info commands ::EH] [interp hidden {}]",
            );
        assert_eq!(
            code,
            Code::Ok,
            "{}",
            String::from_utf8_lossy(&i.result_bytes())
        );
        assert_eq!(i.result_bytes(), b"TARGET TARGET ::heldE {} heldE");
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc zap {ens args} {namespace delete ::ND; return {}}
                      namespace eval ND {
                          namespace ensemble create -command ::NDE -unknown ::zap
                      }
                      set c [catch {::NDE nope} m o]
                      list $c $m [dict get $o -errorcode] [info commands ::NDE]"
            ),
            Code::Ok
        );
        assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} {}"
            );
    });
}

/// Exact Tcl 9.0.4 command-token oracles: an ensemble import follows its
/// source token through hide/expose, not a replacement installed at the
/// vacated name; recreating an occupied ensemble retargets the import to a
/// new token; true deletion removes the import permanently.
#[test]
fn imported_ensemble_retains_source_token_identity() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc tgt_old args {return OLD}
                      proc tgt_new args {return NEW}
                      namespace eval S {
                          namespace export E
                          namespace ensemble create -command ::S::E -map {x ::tgt_old}
                      }
                      namespace eval I {namespace import ::S::E}
                      namespace eval S {
                          namespace ensemble create -command ::S::E -map {x ::tgt_new}
                      }
                      list [::I::E x] [namespace origin ::I::E] \
                           [namespace ensemble configure ::I::E -map]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"NEW ::S::E {x ::tgt_new}");

        assert_eq!(
            i.eval_str(
                b"rename ::S::E {}
                      set before [list [info commands ::I::E] \
                                           [namespace eval I {namespace import}]]
                      namespace eval S {
                          namespace ensemble create -command ::S::E -map {x ::tgt_new}
                      }
                      list {*}$before [info commands ::I::E] \
                           [namespace eval I {namespace import}]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"{} {} {} {}");
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc tgt_old args {return OLD}
                      namespace eval Source {
                          namespace ensemble create -command ::E -map {x ::tgt_old}
                      }
                      namespace export E
                      namespace eval I {namespace import ::E}
                      interp hide {} E held
                      proc E args {return REPLACEMENT}
                      list [::I::E x] [namespace origin ::I::E] \
                           [namespace ensemble configure ::I::E -map] \
                           [namespace ensemble exists ::I::E]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"OLD ::held {x ::tgt_old} 1");

        assert_eq!(
            i.eval_str(
                b"interp expose {} held E2
                      list [::I::E x] [namespace origin ::I::E] \
                           [namespace ensemble configure ::I::E -map] [::E2 x]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"OLD ::E2 {x ::tgt_old} OLD");
    });

    // Replacing an occupied source binding with a non-ensemble keeps the
    // import attached to the source command token. The retained ensemble
    // token is now dead, so dispatch and origin fall back to the by-name
    // source, including after that replacement is renamed.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc tgt_old args {return OLD}
                      namespace eval S {
                          namespace export E
                          namespace ensemble create -command E -map {x ::tgt_old}
                      }
                      namespace eval I {namespace import ::S::E}
                      proc ::S::E args {return PROC}
                      set first [list [::I::E x] [namespace origin ::I::E] \
                                          [namespace ensemble exists ::I::E] \
                                          [info commands ::I::E]]
                      rename ::S::E ::S::E2
                      list {*}$first [::I::E x] [namespace origin ::I::E]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"PROC ::S::E 0 ::I::E PROC ::S::E2");
    });

    // The by-name source shadow moves even while the retained token is
    // live. Replacing the renamed ensemble with a proc retires the token;
    // fallback and origin therefore use the renamed source, and keep
    // following it across a subsequent proc rename.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval S {
                          namespace export E
                          namespace ensemble create -command E
                      }
                      namespace eval I {namespace import ::S::E}
                      rename ::S::E ::S::Moved
                      proc ::S::Moved args {return PROC}
                      set first [list [::I::E] [namespace origin ::I::E] \
                                          [namespace ensemble exists ::I::E]]
                      rename ::S::Moved ::S::Final
                      list {*}$first [::I::E] [namespace origin ::I::E]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"PROC ::S::Moved 0 PROC ::S::Final");
    });
}

/// Tcl 9.0.4 deletes a hidden real ensemble when its configured namespace
/// dies, including its hidden-table binding and every import that retains
/// the token. The active unknown callback therefore observes a dead token.
#[test]
fn namespace_delete_retires_hidden_ensemble_token() {
    // Namespace deletion fires a visible namespace-owned ensemble's delete
    // trace before marking/detaching the namespace. Tcl's ensemble list is
    // retired first, so the callback still sees both the owning namespace
    // and the captured command token; teardown removes both afterwards.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set seen {}
                      proc observe {old new op} {
                          lappend ::seen [list [namespace exists ::N] $old \
                              [info commands $old] \
                              [namespace ensemble exists $old]]
                      }
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete observe
                      namespace delete ::N
                      list $seen [namespace exists ::N] [info commands ::E]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"{{1 ::E ::E 1}} 0 {}");
    });

    // Exposing a hidden victim from its delete callback only moves the old
    // identity; teardown follows it and removes both the command and its
    // moved trace sidecar.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set seen {}
                      proc expose_dying {old new op} {
                          lappend ::seen [list $old [info commands $old]]
                          interp expose {} held E2
                      }
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete expose_dying
                      interp hide {} E held
                      namespace delete ::N
                      list $seen [interp hidden {}] [info commands ::E2]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"{{::held {}}} {} {}");
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc zap {ens args} {
                          interp hide {} NDE held
                          namespace delete ::ND
                          return {}
                      }
                      namespace eval ND {
                          namespace ensemble create -command ::NDE -unknown ::zap
                      }
                      namespace export NDE
                      namespace eval I {namespace import ::NDE}
                      set c [catch {::NDE nope} m o]
                      list $c $m [dict get $o -errorcode] [interp hidden {}] \
                           [info commands ::NDE] [info commands ::I::NDE] \
                           [namespace eval I {namespace import}]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"1 {unknown subcommand handler deleted its ensemble} \
                  {TCL ENSEMBLE UNKNOWN_DELETED} {} {} {} {}"
        );
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc tgt args {return OK}
                      namespace eval HS {
                          namespace export E
                          namespace ensemble create -command E -map {x ::tgt}
                      }
                      namespace import ::HS::E
                      interp hide {} E heldImport
                      namespace delete ::HS
                      set c [catch {interp invokehidden {} heldImport x} m]
                      list [interp hidden {}] $c $m"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"{} 1 {invalid hidden command name \"heldImport\"}"
        );
    });

    // Hidden real ensembles and hidden imported commands both carry their
    // command trace sidecars to the hidden live name. Namespace-driven
    // retirement fires each once and drops it before an unrelated command
    // later occupies the old visible name.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set seen {}
                      proc cb {old new op} {lappend ::seen [list $old $new $op]}
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete cb
                      interp hide {} E held
                      namespace delete ::N
                      proc ::E {} {}
                      list $seen [trace info command ::E] [interp hidden {}]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"{{::held {} delete}} {} {}");
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set seen {}
                      proc cb {old new op} {lappend ::seen [list $old $new $op]}
                      namespace eval S {
                          namespace export p
                          proc p {} {return P}
                      }
                      namespace import ::S::p
                      trace add command ::p delete cb
                      interp hide {} p heldImport
                      namespace delete ::S
                      proc ::p {} {}
                      list $seen [trace info command ::p] [interp hidden {}]"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"{{::heldImport {} delete}} {} {}");
    });
}

/// A nonempty unknown result is spliced using the post-callback parameter
/// count. Tcl 9.0.4 treats `missing` as the newly-added second parameter and
/// removes `newsub` as the live subcommand word.
#[test]
fn ensemble_unknown_prefix_uses_live_parameter_layout() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc target args {return $args}
                      proc mutate {ens args} {
                          namespace ensemble configure $ens -parameters {p q}
                          return ::target
                      }
                      namespace eval M {
                          namespace ensemble create -command ::M \
                              -parameters p -unknown ::mutate
                      }
                      ::M P missing newsub tail"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"P missing tail");
    });
}

/// Exact Tcl 9.0.4 result-code and list-parser diagnostics for an ensemble
/// `-unknown` callback.
#[test]
fn ensemble_unknown_normalizes_bad_results() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc u {mode ens args} {return -code $mode RESULT}
                      namespace ensemble create -command ::E -unknown {::u break}
                      set out {}
                      foreach mode {break continue return 7} {
                          namespace ensemble configure ::E -unknown [list ::u $mode]
                          set c [catch {::E nope} m o]
                          lappend out $c $m [dict get $o -errorcode]
                      }
                      set out"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"1 {unknown subcommand handler returned bad code: break} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  1 {unknown subcommand handler returned bad code: continue} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  1 {unknown subcommand handler returned bad code: return} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  1 {unknown subcommand handler returned bad code: 7} \
                  {TCL ENSEMBLE UNKNOWN_RESULT}"
        );

        assert_eq!(
            i.eval_str(
                b"namespace ensemble configure ::E -unknown {::u break}
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] \
                           [join [lrange [split [dict get $o -errorinfo] \\n] 0 1] \\n]"
            ),
            Code::Ok
        );
        assert_eq!(
                i.result_bytes(),
                b"{unknown subcommand handler returned bad code: break} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  {unknown subcommand handler returned bad code: break\n    result of ensemble unknown subcommand handler: ::u break ::E nope}"
            );
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc malformed {ens args} {return \\{}
                      namespace ensemble create -command ::E -unknown ::malformed
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] \
                           [join [lrange [split [dict get $o -errorinfo] \\n] 0 1] \\n]"
            ),
            Code::Ok
        );
        assert_eq!(
                i.result_bytes(),
                b"{unmatched open brace in list} {TCL VALUE LIST BRACE} \
                  {unmatched open brace in list\n    while parsing result of ensemble unknown subcommand handler}"
            );
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc malformed {ens args} {return \"{a}junk\"}
                      namespace ensemble create -command ::E -unknown ::malformed
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] \
                           [dict get $o -errorinfo]"
            ),
            Code::Ok
        );
        assert_eq!(
                i.result_bytes(),
                b"{list element in braces followed by \"junk\" instead of space} \
                  {TCL VALUE LIST JUNK} \
                  {list element in braces followed by \"junk\" instead of space\n    while parsing result of ensemble unknown subcommand handler\n    invoked from within\n\"::E nope\"}"
            );
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc baderr args {
                          return -code error -errorcode {CUSTOM CODE} BOOM
                      }
                      namespace ensemble create -command ::E -unknown ::baderr
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] [dict get $o -errorinfo]"
            ),
            Code::Ok
        );
        assert_eq!(
                i.result_bytes(),
                b"BOOM {CUSTOM CODE} {BOOM\n    while executing\n\"::baderr ::E nope\"\n    (ensemble unknown subcommand handler)\n    invoked from within\n\"::E nope\"}"
            );
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"proc delete_unknown {ens args} {rename $ens {}; return {}}
                      namespace ensemble create -command ::E -unknown ::delete_unknown
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] [dict get $o -errorinfo]"
            ),
            Code::Ok
        );
        assert_eq!(
                i.result_bytes(),
                b"{unknown subcommand handler deleted its ensemble} \
                  {TCL ENSEMBLE UNKNOWN_DELETED} \
                  {unknown subcommand handler deleted its ensemble\n    (ensemble unknown subcommand handler)\n    invoked from within\n\"::E nope\"}"
            );
    });
}

#[test]
fn ensemble_default_miss_preserves_custom_unknown_options() {
    leak_free(|i| {
        let code = i.eval_str(
            b"proc ::unknown {cmd args} {
                          return -code error -errorcode {CUSTOM CODE} \
                              -errorinfo CUSTOMINFO \
                              -errorstack {INNER foo CALL bar} \
                              \"invalid command name \\\"$cmd\\\"\"
                      }
                      namespace eval N {
                          namespace ensemble create -command ::E -subcommands x
                      }
                      set c [catch {::E x} m o]
                      list $c $m [dict get $o -errorcode] \
                           [dict get $o -errorinfo] [dict get $o -errorstack]",
        );
        assert_eq!(
            code,
            Code::Ok,
            "{}",
            String::from_utf8_lossy(&i.result_bytes())
        );
        assert_eq!(
            i.result_bytes(),
            b"1 {invalid command name \"x\"} {CUSTOM CODE} \
                  {CUSTOMINFO\n    invoked from within\n\"::E x\"} \
                  {INNER foo CALL bar}"
        );
    });
}
