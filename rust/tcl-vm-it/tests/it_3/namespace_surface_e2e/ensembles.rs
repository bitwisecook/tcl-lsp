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

//! `namespace ensemble`: create and configure options, the subcommand map, dispatch messages and the `-unknown` handler.

use super::*;

// Ensemble option tables and subcommand resolution
// (`TclNamespaceEnsembleCmd` + `NsEnsembleImplementationCmd`, tclEnsemble.c)

#[test]
fn ensemble_create_options_abbreviate() {
    // `Tcl_GetIndexFromObj` flags 0 (tclEnsemble.c:211). tclsh: ::ab5.
    assert_eq!(
        run(
            "namespace eval e5 {namespace export *; proc go {} {return G}}\n\
             namespace eval e5 {namespace ensemble create -comm ::ab5 -sub go}"
        ),
        "::ab5"
    );
    assert_eq!(
        run(
            "namespace eval e5 {namespace export *; proc go {} {return G}\n\
             namespace ensemble create -comm ::ab5 -sub go}\n\
             ::ab5 go"
        ),
        "G"
    );
    // The `namespace ensemble` subcommand table abbreviates too.
    assert_eq!(
        run("namespace eval e5 {proc go {} {}}\n\
             namespace eval e5 {namespace ensemble cr -command ::ab7 -subcommands go}"),
        "::ab7"
    );
    assert_eq!(
        run("namespace eval e5 {proc go {} {}\n\
             namespace ensemble create -command ::ab7 -subcommands go}\n\
             namespace ensemble ex ::ab7"),
        "1"
    );
    assert_eq!(
        run("catch {namespace ensemble frobnicate} m; set m"),
        "bad subcommand \"frobnicate\": must be configure, create, or exists"
    );
}

#[test]
fn ensemble_create_has_no_namespace_option() {
    // `ensembleCreateOptions` (tclEnsemble.c:57-60) omits -namespace.
    assert_eq!(
        run("catch {namespace ensemble create -namespace ::x} m; set m"),
        "bad option \"-namespace\": must be -command, -map, -parameters, \
         -prefixes, -subcommands, or -unknown"
    );
    assert_eq!(
        run("catch {namespace ensemble create -bogus v} m; set m"),
        "bad option \"-bogus\": must be -command, -map, -parameters, \
         -prefixes, -subcommands, or -unknown"
    );
    // `-p` prefixes both -parameters and -prefixes.
    assert_eq!(
        run("catch {namespace ensemble create -p 1} m; set m"),
        "ambiguous option \"-p\": must be -command, -map, -parameters, \
         -prefixes, -subcommands, or -unknown"
    );
}

#[test]
fn ensemble_create_accepts_parameters_and_threads_them() {
    assert_eq!(
        run("namespace eval e6 {namespace ensemble create -command ::ab6 -parameters {q}}"),
        "::ab6"
    );
    // `ens p sub args…` → `target p args…` (C's numParameters threading).
    assert_eq!(
        run("namespace eval e6 {namespace export *\n\
             proc go {p a} {return \"$p/$a\"}\n\
             namespace ensemble create -command ::ab6 -parameters {q}}\n\
             ::ab6 VAL go ARG"),
        "VAL/ARG"
    );
}

#[test]
fn ensemble_wrong_args_list_quotes_the_command_and_parameters() {
    // The usage prefix is a Tcl list of command/parameter words, not a string
    // joined with spaces. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("namespace eval {E space} {
                 namespace ensemble create -command {::E space} \
                     -parameters {{a b}}
             }
             catch {{::E space}} m o
             list $m [dict get $o -errorcode]"),
        "{wrong # args: should be \"{::E space} {a b} subcommand ?arg ...?\"} \
         {TCL WRONGARGS}"
    );
}

#[test]
fn ensemble_create_checks_pair_arity_before_option_words() {
    // C: `if (objc & 1)` fires before any `Tcl_GetIndexFromObj`
    // (tclEnsemble.c:192), so an odd tail is `wrong # args`, never `bad option`.
    let usage = "wrong # args: should be \"namespace ensemble create ?option value ...?\"";
    assert_eq!(
        run("catch {namespace ensemble create -command} m; set m"),
        usage
    );
    assert_eq!(
        run("catch {namespace ensemble create -command ::zz -sub} m; set m"),
        usage
    );
    assert_eq!(
        run("catch {namespace ensemble create -bogus} m; set m"),
        usage
    );
}

/// `-map` is a dict, so the read-back preserves the order the pairs were given
/// (never sorted) and a repeated key keeps its first position while taking the
/// last value. All three expectations are tclsh 9.0.4-pinned.
#[test]
fn ensemble_map_preserves_dict_order_and_collapses_repeats() {
    let setup = "namespace eval M {namespace export *\n\
                 proc zeta {} {return Z}\n\
                 proc alpha {} {return A}\n\
                 proc mid {} {return M}\n\
                 namespace ensemble create -command ::E -map {zz zeta aa alpha}}\n";
    // Insertion order, not sorted — `aa` would sort first.
    assert_eq!(
        run(&format!("{setup}namespace ensemble configure ::E -map")),
        "zz ::M::zeta aa ::M::alpha"
    );
    // A later `configure` replaces the map wholesale, in its own order.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval M {{namespace ensemble configure ::E \
             -map {{mm mid zz zeta aa alpha}}}}\n\
             namespace ensemble configure ::E -map"
        )),
        "mm ::M::mid zz ::M::zeta aa ::M::alpha"
    );
    // A repeated key: last value wins, first position kept.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval M {{namespace ensemble configure ::E \
             -map {{zz zeta aa alpha zz mid}}}}\n\
             namespace ensemble configure ::E -map"
        )),
        "zz ::M::mid aa ::M::alpha"
    );
    // Dispatch follows the collapsed entry, not the stale first one.
    assert_eq!(
        run(&format!(
            "{setup}namespace eval M {{namespace ensemble configure ::E \
             -map {{zz zeta aa alpha zz mid}}}}\n\
             ::E zz"
        )),
        "M"
    );
}

#[test]
fn ensemble_configure_reads_and_writes_the_config_table() {
    let setup = "namespace eval e5 {namespace export *; proc go {} {return G}\n\
                 namespace ensemble create -command ::ab5 -subcommands go}\n";
    // The query dict, in C's option order.
    assert_eq!(
        run(&format!("{setup}namespace ensemble configure ::ab5")),
        "-map {} -namespace ::e5 -parameters {} -prefixes 1 -subcommands go -unknown {}"
    );
    // A single (abbreviated) option reads its value.
    assert_eq!(
        run(&format!("{setup}namespace ensemble configure ::ab5 -sub")),
        "go"
    );
    // `-namespace` is readable but never writable.
    assert_eq!(
        run(&format!(
            "{setup}catch {{namespace ensemble configure ::ab5 -namespace ::e5}} m o
             list $m [dict get $o -errorcode]"
        )),
        "{option -namespace is read-only} {TCL ENSEMBLE READ_ONLY}"
    );
    // `-command` is create-only, so it is not in the configure table.
    assert_eq!(
        run(&format!(
            "{setup}catch {{namespace ensemble configure ::ab5 -command ::zz}} m; set m"
        )),
        "bad option \"-command\": must be -map, -namespace, -parameters, \
         -prefixes, -subcommands, or -unknown"
    );
    // An odd tail longer than one word is `wrong # args`.
    assert_eq!(
        run(&format!(
            "{setup}catch {{namespace ensemble configure ::ab5 -subcommands go -prefixes}} m; set m"
        )),
        "wrong # args: should be \"namespace ensemble configure cmdname \
         ?-option value ...? ?arg ...?\""
    );
    // A write takes effect.
    assert_eq!(
        run(&format!(
            "{setup}namespace ensemble configure ::ab5 -prefixes 0\n\
             namespace ensemble configure ::ab5 -prefixes"
        )),
        "0"
    );
}

#[test]
fn ensemble_dispatch_messages_match_c() {
    // `NsEnsembleImplementationCmd`'s three miss shapes. Note the ensemble
    // enumeration keeps its comma before `or` for two entries.
    let two = "namespace eval e7 {namespace export *; proc bar {} {}; proc baz {} {}\n\
               namespace ensemble create -command ::ab8}\n";
    assert_eq!(
        run(&format!("{two}catch {{::ab8 ba}} m; set m")),
        "unknown or ambiguous subcommand \"ba\": must be bar, or baz"
    );
    assert_eq!(
        run(&format!("{two}catch {{::ab8 zz}} m; set m")),
        "unknown or ambiguous subcommand \"zz\": must be bar, or baz"
    );
    assert_eq!(
        run(&format!(
            "{two}catch {{::ab8 zz}} m o; list $m [dict get $o -errorcode]"
        )),
        "{unknown or ambiguous subcommand \"zz\": must be bar, or baz} \
         {TCL LOOKUP SUBCOMMAND zz}"
    );
    // `-prefixes 0` drops the "or ambiguous" half.
    let exact = "namespace eval e9 {namespace export *; proc bar {} {}; proc baz {} {}\n\
                 namespace ensemble create -command ::ab10 -prefixes 0}\n";
    assert_eq!(
        run(&format!("{exact}catch {{::ab10 ba}} m; set m")),
        "unknown subcommand \"ba\": must be bar, or baz"
    );
    // An ensemble over a namespace that exports nothing has its own message.
    assert_eq!(
        run(
            "namespace eval e8 {namespace ensemble create -command ::ab9}\n\
             catch {::ab9 zz} m; set m"
        ),
        "unknown subcommand \"zz\": namespace ::e8 does not export any commands"
    );
}

#[test]
fn ensemble_subcommand_scan_matches_c_on_the_empty_word() {
    // C's prefix scan is a `strncmp` over the word's length, so an empty
    // subcommand prefixes every entry: a one-entry table resolves outright
    // (unlike `Tcl_GetIndexFromObj`, which forces the error path for an empty
    // key), a two-entry one is ambiguous. tclsh 8.6.16 / 9.0.4: G.
    assert_eq!(
        run(
            "namespace eval e5 {namespace export *; proc go {} {return G}}\n\
             namespace eval e5 {namespace ensemble create -command ::ab5}\n\
             ::ab5 {}"
        ),
        "G"
    );
    assert_eq!(
        run(
            "namespace eval e7 {namespace export *; proc bar {} {}; proc baz {} {}\n\
             namespace ensemble create -command ::ab8}\n\
             catch {::ab8 {}} m; set m"
        ),
        "unknown or ambiguous subcommand \"\": must be bar, or baz"
    );
    // `-prefixes 0` turns the empty word into a plain miss.
    assert_eq!(
        run(
            "namespace eval e5 {namespace export *; proc go {} {return G}}\n\
             namespace eval e5 {namespace ensemble create -command ::ab5 -prefixes 0}\n\
             catch {::ab5 {}} m; set m"
        ),
        "unknown subcommand \"\": must be go"
    );
}

#[test]
fn ensemble_map_validation_matches_tcl_dict_and_target_errors() {
    assert_eq!(
        run("catch {namespace ensemble create -map {go}} m; set m"),
        "missing value to go with key"
    );
    assert_eq!(
        run("catch {namespace ensemble create -map {go {}}} m; set m"),
        "ensemble subcommand implementations must be non-empty lists"
    );
    assert_eq!(
        run("set badmap \"go \\{\"
             catch {namespace ensemble create -map $badmap} m; set m"),
        "unmatched open brace in dict"
    );
}

#[test]
fn ensemble_unknown_result_is_redispatched_as_a_command_prefix() {
    assert_eq!(
        run("proc uh {ens args} {return [list list REPLACED $ens]}
             namespace eval se5 {
                 namespace ensemble create -command ::se5 -subcommands {} -unknown ::uh
             }
             ::se5 nope 1 2"),
        "REPLACED ::se5 1 2"
    );
    // An empty successful result requests one reparse, so a command exported
    // by the callback becomes visible to the same ensemble invocation.
    assert_eq!(
        run("proc define {ens args} {
                 namespace eval se6 {proc nope args {return DEFINED}; namespace export nope}
                 return {}
             }
             namespace eval se6 {namespace ensemble create -command ::se6 -unknown ::define}
             ::se6 nope"),
        "DEFINED"
    );
    // Reparse reads the live ensemble configuration, not the dispatch-time
    // snapshot. Tcl 9.0.4 makes a map installed by the callback visible now.
    assert_eq!(
        run("proc target args {return TARGET}
             proc repair {ens args} {
                 namespace ensemble configure $ens -map {nope ::target}; return {}
             }
             namespace eval se7 {
                 namespace ensemble create -command ::se7 -unknown ::repair
             }
             ::se7 nope"),
        "TARGET"
    );
}

#[test]
fn ensemble_unknown_nonempty_prefix_uses_live_parameter_count() {
    // A non-empty replacement prefix uses the post-callback `-parameters`
    // layout too. `missing` becomes the live parameter and `ARG` the live
    // subcommand, so Tcl splices `missing` rather than the old trailing ARG.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc reroute {ens args} {
                 namespace ensemble configure $ens -parameters {p}
                 return [list list PREFIX]
             }
             namespace eval LP {
                 namespace ensemble create -command ::LP -subcommands {} \
                     -unknown ::reroute
             }
             ::LP missing ARG"),
        "PREFIX missing"
    );
}

#[test]
fn ensemble_unknown_exceptional_codes_and_malformed_results_match_c() {
    // TCL_ERROR propagates unchanged. The other exceptional callback codes
    // are invalid ensemble results and are converted to UNKNOWN_RESULT using
    // Tcl's completion-code names (or the integer for a custom code). Exact
    // Tcl 9.0.4 oracle result.
    assert_eq!(
        run("set bad_code error
             proc exceptional {ens args} {return -code $::bad_code VALUE}
             namespace eval UC {
                 namespace ensemble create -command ::UC -subcommands {} \
                     -unknown ::exceptional
             }
             set results {}
             foreach code {error break continue return 7} {
                 set bad_code $code
                 set c [catch {::UC nope ARG} m o]
                 lappend results [list $c $m [dict get $o -errorcode]]
             }
             set results"),
        "{1 VALUE NONE} \
         {1 {unknown subcommand handler returned bad code: break} \
            {TCL ENSEMBLE UNKNOWN_RESULT}} \
         {1 {unknown subcommand handler returned bad code: continue} \
            {TCL ENSEMBLE UNKNOWN_RESULT}} \
         {1 {unknown subcommand handler returned bad code: return} \
            {TCL ENSEMBLE UNKNOWN_RESULT}} \
         {1 {unknown subcommand handler returned bad code: 7} \
            {TCL ENSEMBLE UNKNOWN_RESULT}}"
    );

    // A successful callback result is parsed as a Tcl list. Preserve the list
    // parser's VALUE code and the ensemble-specific errorInfo frame. Exact
    // Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc malformed {ens args} {return [string index {{}} 0]}
             namespace eval UML {
                 namespace ensemble create -command ::UML -subcommands {} \
                     -unknown ::malformed
             }
             catch {::UML nope} m o
             list $m [dict get $o -errorcode] \
                  [expr {[string first \
                      {while parsing result of ensemble unknown subcommand handler} \
                      [dict get $o -errorinfo]] >= 0}]"),
        "{unmatched open brace in list} {TCL VALUE LIST BRACE} 1"
    );
}

#[test]
fn ensemble_unknown_retains_the_live_command_token() {
    assert_eq!(
        run("proc target args {return TARGET}
             namespace eval ER {
                 proc repair {ens args} {
                     namespace ensemble configure $ens -map {nope ::target}
                     rename $ens ::ER2
                     return {}
                 }
                 namespace ensemble create -command ::ER -unknown ::ER::repair
             }
             list [::ER nope] [namespace ensemble configure ::ER2 -map] [::ER2 nope]"),
        "TARGET {nope ::target} TARGET"
    );

    assert_eq!(
        run("set seen {}
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
             list [::I::E nope] $seen [namespace origin ::I::E]"),
        "TARGET ::S::E ::S::E"
    );

    assert_eq!(
        run("proc target args {return TARGET}
             proc replace {ens args} {
                 rename $ens {}
                 namespace ensemble create -command $ens -map {nope ::target}
                 return \\{
             }
             namespace eval D {
                 namespace ensemble create -command E -unknown ::replace
             }
             set c [catch {::D::E nope} m o]
             list $c $m [dict get $o -errorcode] [::D::E nope]"),
        "1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} TARGET"
    );

    assert_eq!(
        run("proc target args {return TARGET}
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
             list $first $second $seen [info commands ::EH] [interp hidden {}]"),
        "TARGET TARGET ::heldE {} heldE"
    );

    assert_eq!(
        run("proc zap {ens args} {namespace delete ::ND; return {}}
             namespace eval ND {
                 namespace ensemble create -command ::NDE -unknown ::zap
             }
             set c [catch {::NDE nope} m o]
             list $c $m [dict get $o -errorcode] [info commands ::NDE]"),
        "1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} {}"
    );

    // A hidden real ensemble still belongs to its implementation namespace.
    // Deleting that subtree retires the hidden token and every import of it;
    // the active unknown callback observes the token's deleted state. Exact
    // Tcl 9.0.4 oracle result.
    assert_eq!(
        run(
            "proc hidden_zap {ens args} {namespace delete ::HD; return {}}
             namespace eval HD {
                 namespace ensemble create -command ::HDE -unknown ::hidden_zap
             }
             namespace export HDE
             namespace eval HI {namespace import ::HDE}
             interp hide {} ::HDE heldHDE
             set c [catch {interp invokehidden {} heldHDE nope} m o]
             list $c $m [dict get $o -errorcode] [interp hidden {}] \
                  [info commands ::HI::HDE] \
                  [namespace ensemble exists ::HI::HDE]"
        ),
        "1 {unknown subcommand handler deleted its ensemble} \
         {TCL ENSEMBLE UNKNOWN_DELETED} {} {} 0"
    );
}

#[test]
fn ensemble_unknown_deleted_seeds_the_handler_errorinfo_frame() {
    // Tcl seeds this special context before the ordinary command-invocation
    // machinery appends its enclosing frame. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc target args {return TARGET}
             proc replace {ens args} {
                 rename $ens {}
                 namespace ensemble create -command $ens -map {nope ::target}
                 return \\{
             }
             namespace eval D {
                 namespace ensemble create -command E -unknown ::replace
             }
             catch {::D::E nope} message options
             split [dict get $options -errorinfo] [format %c 10]"),
        "{unknown subcommand handler deleted its ensemble} \
         {    (ensemble unknown subcommand handler)} \
         {    invoked from within} {\"::D::E nope\"}"
    );
}

#[test]
fn ensemble_default_target_miss_names_the_rewritten_subcommand() {
    assert_eq!(
        run(
            "namespace eval k1 {namespace ensemble create -subcommands ghost}
             catch {::k1 ghost} m; set m"
        ),
        "invalid command name \"ghost\""
    );

    // The ensemble rewrites only the default target's result spelling. A
    // custom `unknown` command owns all completion options, even if it chooses
    // Tcl's ordinary invalid-command message. Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc ::unknown {cmd args} {
                 return -code error -errorcode {CUSTOM USER} \
                     \"invalid command name \\\"$cmd\\\"\"
             }
             namespace eval E {
                 namespace ensemble create -command ::E -subcommands x
             }
             catch {::E x} m o
             list $m [dict get $o -errorcode]"),
        "{invalid command name \"x\"} {CUSTOM USER}"
    );
}
