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

//! Arity of calls to same-file procedures, `after` callbacks, `interp alias` targets and renamed commands, across deletions and re-establishment.

use super::*;

// Same-file proc / TclOO forward / `interp alias` / `rename` arity
// (generalises E002/E003 beyond the builtin registry).

fn arity_codes(src: &str, dialect: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, dialect)
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn reached_user_call_arity_uses_nested_and_replacement_formals() {
    let nested = include_str!(
        "../../../../../../../docs/work/name-handling/handover/oo-dispatch-probe/dynamic/e17-proc-inside-proc.tcl"
    );
    let fixed = include_str!(
        "../../../../../../../docs/work/name-handling/handover/oo-dispatch-probe/dynamic/e07-shadow-builtin.tcl"
    );
    let variadic = include_str!(
        "../../../../../../../docs/work/name-handling/handover/oo-dispatch-probe/dynamic/e30-rename-builtin-proc-wrap.tcl"
    );
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert_eq!(arity_codes(nested, dialect), ["E002"], "{dialect}");
        assert_eq!(arity_codes(fixed, dialect), ["E003"], "{dialect}");
        assert!(arity_codes(variadic, dialect).is_empty(), "{dialect}");
    }
}

#[test]
fn irules_arity_accepts_documented_pool_log_and_class_iteration_forms() {
    // BIG-IP 21.0.0.1 TMSH rule manuals for pool, log and class.
    for src in [
        "pool /Common/web member 192.0.2.10 443\n",
        "log -noname 192.0.2.20:514 local0.info message\n",
        "class nextelement -index -name -value -element -list -- /Common/users search_id\n",
        "class anymore /Common/users search_id\n",
        "class donesearch /Common/users search_id\n",
    ] {
        assert_eq!(
            arity_codes(src, "f5-irules"),
            Vec::<String>::new(),
            "{src:?}"
        );
    }

    // Keep the upper bounds useful after widening the three valid forms.
    for src in [
        "pool /Common/web member 192.0.2.10 443 extra\n",
        "log 192.0.2.20 local0.info message extra\n",
        "class nextelement /Common/users search_id extra\n",
    ] {
        assert_eq!(
            arity_codes(src, "f5-irules"),
            vec!["E003".to_owned()],
            "{src:?}"
        );
    }
}

#[test]
fn e003_fires_for_same_file_proc_call_too_many_args() {
    // The reported bug: a same-file call to a 7-parameter proc with 8
    // arguments produced no diagnostic at all. `arg8` inside the body
    // correctly fires W210 (undefined variable), but the call site
    // itself must fire E003 too.
    let src = "\
proc demonstrate {arg1 arg2 arg3 arg4 arg5 arg6 arg7} {
    return \"$arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7\"
}
demonstrate one two three four five six seven eight
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let e003: Vec<&Diagnostic> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    assert_eq!(
        e003.len(),
        1,
        "expected exactly one E003 for the 8-arg call to a 7-param proc, got {:?}",
        r.diagnostics
    );
    assert!(e003[0].message.contains("demonstrate"));
    assert!(e003[0].message.contains("expected at most 7"));
    assert!(e003[0].message.contains("got 8"));
}

#[test]
fn same_file_proc_call_with_correct_arity_is_silent() {
    let src = "\
proc demonstrate {arg1 arg2 arg3 arg4 arg5 arg6 arg7} {
    return \"$arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7\"
}
demonstrate one two three four five six seven
";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "a correctly-arg-counted same-file proc call must not fire E002/E003"
    );
}

#[test]
fn same_file_proc_call_too_few_args_fires_e002() {
    let src = "proc need3 {a b c} {}\nneed3 1 2\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
}

#[test]
fn same_file_proc_required_after_default_forces_exact_count() {
    // Same regression as `cross_file_arity_required_after_default_forces_exact_count`
    // (tcl-lsp-db), but exercised same-file: `proc opt {a {b 5} c}`
    // accepts exactly 3 arguments per real tclsh 9.0.4, not 2.
    let src = "proc opt {a {b 5} c} {}\nopt 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "2 args must still be too few — min is 3, not 2"
    );
    assert_eq!(
        arity_codes("proc opt {a {b 5} c} {}\nopt 1 2 3\n", "tcl8.6"),
        Vec::<String>::new()
    );
    assert_eq!(
        arity_codes("proc opt {a {b 5} c} {}\nopt 1 2 3 4\n", "tcl8.6"),
        vec!["E003".to_owned()]
    );
}

#[test]
fn same_file_proc_with_trailing_args_is_unbounded() {
    let src = "proc variadic {a args} {}\nvariadic 1 2 3 4 5\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "trailing `args` must accept any number of extra arguments"
    );
    assert_eq!(
        arity_codes("proc variadic {a args} {}\nvariadic\n", "tcl8.6"),
        vec!["E002".to_owned()],
        "the one required parameter before `args` must still be enforced"
    );
}

#[test]
fn after_default_form_braced_callback_is_arity_checked() {
    // `after ms script` appends zero args when it fires the script (unlike a
    // `-command` callback), so the callback is checked like an ordinary
    // in-body call, not the command-prefix mechanism — `cb` needs 2 args and
    // gets none. Body recursion only descends a *braced* word (`analyse_body`'s
    // `TokenType::Str` guard, the same convention `uplevel`'s resolver
    // documents), so the script must be braced here.
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter 1000 {cb}\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "after's script word must recurse as a real call, not an opaque value"
    );
}

#[test]
fn after_idle_braced_callback_is_arity_checked() {
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter idle {cb}\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "after idle's script word must recurse exactly like after ms's"
    );
}

#[test]
fn after_multi_word_script_concatenation_abstains() {
    // `after ms script script script ...?`
    // concatenates every trailing word into ONE script before evaluating
    // it — confirmed against tclsh 9.0.4 (`after info` shows the
    // registered script as `cb 1 2`, space-joined). Marking only the first
    // word (`{cb}`) as Body would recurse into a truncated fragment and
    // wrongly flag a 2-arg `cb` as under-supplied; must abstain instead.
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter 1000 {cb} 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "a multi-word after script must abstain, not mis-recurse the first word alone"
    );
    let src_idle = "proc cb {a b} { return [expr {$a+$b}] }\nafter idle {cb} 1 2\n";
    assert_eq!(
        arity_codes(src_idle, "tcl8.6"),
        Vec::<String>::new(),
        "after idle has the identical concatenation shape"
    );
}

#[test]
fn after_default_form_bareword_callback_is_now_arity_checked() {
    // TP — a *bareword* callback (no braces) is valid Tcl — equally callable, and
    // equally arity-checkable, as a braced one: `dispatch_body_arguments`
    // dispatches a genuinely-static bareword body (`Esc`-kind, single word,
    // no `$`/`[`)
    // through the ordinary `process_command` path, so it gets full call
    // treatment, arity checking included, exactly like a braced one.
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter 1000 cb\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_string()]);
}

#[test]
fn after_default_form_bareword_callback_correct_arity_is_silent() {
    // TN sibling — a bareword callback with the RIGHT arity (no extra
    // args needed) must not falsely fire E002.
    let src = "proc cb {} { return 1 }\nafter 1000 cb\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn after_correct_arity_callback_is_silent() {
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter 1000 {cb 1 2}\nafter idle {cb 3 4}\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn after_cancel_argument_is_not_arity_checked_as_a_callback() {
    // `after cancel` takes an id (or a script used only for *matching*, never
    // executed) — its argument must not be treated as a Body/call.
    let src = "proc cb {a b} { return [expr {$a+$b}] }\nafter cancel cb\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "after cancel's argument identifies a pending callback; it is never invoked"
    );
}

#[test]
fn same_file_args_not_last_is_an_ordinary_required_name() {
    // `args` only collects extra arguments when it's the *last*
    // parameter; here it's an ordinary required name, so the proc has
    // exact arity 3.
    let src = "proc f {args a b} {}\nf 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "args-not-last must not be treated as variadic"
    );
    assert_eq!(
        arity_codes("proc f {args a b} {}\nf 1 2 3\n", "tcl8.6"),
        Vec::<String>::new()
    );
}

#[test]
fn same_file_interp_alias_arity_is_shifted_by_prepended_args() {
    // tclsh 9.0.4: `interp alias {} shortcut {} target 100` requires
    // exactly 2 more arguments at the `shortcut` call site (target's
    // arity of 3, minus the 1 prepended argument).
    let src = "proc target {a b c} {}\ninterp alias {} shortcut {} target 100\nshortcut 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "shortcut needs 2 more args, only 1 given"
    );
    assert_eq!(
        arity_codes(
            "proc target {a b c} {}\ninterp alias {} shortcut {} target 100\nshortcut 2 3\n",
            "tcl8.6"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn same_file_chained_alias_accumulates_prepended_shift() {
    // tclsh 9.0.4: `step2` (alias for `step1` with one prepended arg,
    // itself an alias for the 2-param `real`) needs exactly 1 more
    // argument.
    let src = "\
proc real {a b} {}
interp alias {} step1 {} real
interp alias {} step2 {} step1 9
step2
";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
    assert_eq!(
        arity_codes(
            "proc real {a b} {}\ninterp alias {} step1 {} real\ninterp alias {} step2 {} step1 9\nstep2 1\n",
            "tcl8.6"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn same_file_static_rename_inherits_original_arity() {
    // `rename` is a pure name move — `target_orig` must be checked
    // against the exact same arity as `target` always had.
    let src = "proc target {a b c} {}\nrename target target_orig\ntarget_orig 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "target_orig must still require exactly 3 arguments"
    );
    assert_eq!(
        arity_codes(
            "proc target {a b c} {}\nrename target target_orig\ntarget_orig 1 2 3\n",
            "tcl8.6"
        ),
        Vec::<String>::new()
    );
}

#[test]
pub(in super::super) fn same_file_rename_reestablished_after_deletion_checks_new_arity() {
    // `rename target {}` deletes `target` outright, but a fresh `proc
    // target` afterwards re-establishes the name with its own (here,
    // different) arity — confirmed against tclsh 9.0.4: `proc target {a
    // b} {}; rename target {}; proc target {a b c d} {}; target 1 2 3 4`
    // succeeds, and `target 1 2` fails "wrong # args" against the *new*
    // 4-arg signature, not the deleted 2-arg one. Before the timestamp
    // compare (`fact_superseded_by_deletion`), the single stored
    // `deleted_commands["::target"]` offset made every later call look
    // permanently dead, silently dropping this diagnostic (FN).
    let src = "proc target {a b} {}\nrename target {}\nproc target {a b c d} {}\ntarget 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "target was re-established with 4 params after its deletion"
    );
    assert_eq!(
        arity_codes(
            "proc target {a b} {}\nrename target {}\nproc target {a b c d} {}\ntarget 1 2 3 4\n",
            "tcl8.6"
        ),
        Vec::<String>::new(),
        "4 args satisfy the re-established signature"
    );
    // Still correctly dead when there is no re-establishment at all.
    assert_eq!(
        arity_codes(
            "proc target {a b} {}\nrename target {}\ntarget 1 2\n",
            "tcl8.6"
        ),
        Vec::<String>::new(),
        "no re-establishment — target stays permanently deleted (TN)"
    );
}

// A `rename` / `interp alias` deletion recorded *inside* a
// proc body that's never called is conditional: it may never execute, so
// it must not supersede a fact established outside that body.
// `fact_superseded_by_deletion` cannot decide that from `fact_in_effect`
// (call-site order-gating) alone, which only asks whether the call is
// order-gated against its own top-level/body status — never whether the
// deletion's own offset sits inside a different, possibly-never-invoked body.
// It applies the same `offset_is_inside_any_definition_body` guard the W123
// pass's `fact_live_for_call` uses for the identical question. All cases
// confirmed against tclsh 8.6.14.

#[test]
fn e003_fp_issue_1007_conditional_deletion_never_triggered_proc_stays_live() {
    let src = "proc target {a b} {}\nproc maybeDelete {} { rename target {} }\ntarget 1 2\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn e003_tp_issue_1007_conditional_deletion_never_triggered_still_checks_arity() {
    // `maybeDelete` is never called, so `target` keeps its original 2-arg
    // signature — an over-applied call must still fire E003, not
    // silently abstain as if the name were permanently gone.
    let src = "proc target {a b} {}\nproc maybeDelete {} { rename target {} }\ntarget 1 2 3\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E003".to_owned()]);
}

#[test]
fn e003_tn_issue_1007_unconditional_deletion_before_call_stays_dead() {
    // An *unconditional* (top-level) deletion permanently kills the name —
    // only a deletion recorded inside a body is exempt.
    let src = "proc target {a b} {}\nrename target {}\ntarget 1 2\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn e003_tp_issue_1007_call_before_later_unconditional_deletion_still_checked() {
    // Call-site order gating still applies to a top-level, unconditional
    // deletion.
    let src = "proc target {a b} {}\ntarget 1 2 3\nrename target {}\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E003".to_owned()]);
}

#[test]
fn e003_tp_issue_1007_alias_conditional_deletion_never_triggered_still_checks_arity() {
    // Same fix, `interp alias` deletion form.
    let src = "proc target {a b} {}\ninterp alias {} short {} target\nproc maybeDelete {} { interp alias {} short {} }\nshort 1 2 3\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E003".to_owned()]);
}

#[test]
fn same_file_rename_target_reestablished_after_further_rename_checks_new_arity() {
    // `rename target target_orig` moves `target`'s identity onward (like
    // `same_file_static_rename_inherits_original_arity`), but a *fresh*
    // `proc target` written afterwards re-establishes `target` itself as
    // a brand-new, independent command — it must be checked against its
    // own arity, not treated as still shadowed by the earlier rename.
    let src = "proc target {a b} {}\nrename target target_orig\nproc target {x} {}\ntarget 1 2\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E003".to_owned()],
        "the re-established target only takes 1 argument"
    );
    assert_eq!(
        arity_codes(
            "proc target {a b} {}\nrename target target_orig\nproc target {x} {}\ntarget 1\n",
            "tcl8.6"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn same_file_alias_reestablished_after_deletion_checks_new_target_arity() {
    // `interp alias {} short {} target` then `interp alias {} short {}
    // {}` deletes the alias outright (tclsh 9.0.4: `short` then fails
    // "invalid command name"); a fresh `interp alias {} short {}
    // target2` afterwards re-establishes `short` against a
    // differently-aritied target — must be checked against `target2`,
    // not silently dropped as still-deleted.
    let src = "\
proc target {a b} {}
proc target2 {a b c} {}
interp alias {} short {} target
interp alias {} short {} {}
interp alias {} short {} target2
short 1 2
";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "short now aliases target2, which needs 3 arguments"
    );
    let ok = src.replace("short 1 2\n", "short 1 2 3\n");
    assert_eq!(arity_codes(&ok, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_dynamic_but_resolvable_rename_target_checks_arity() {
    // `$newname` is dynamic-*looking* but a known constant (`set newname
    // target_orig`) — constant folding resolves it, so `target_orig`
    // correctly inherits `target`'s arity
    // (a rename is a pure name move, never an arity change) and a call
    // with too few arguments is caught exactly like the fully-literal
    // `same_file_static_rename_inherits_original_arity` case. Before
    // that fix this whole rename was untracked (dynamic per a naive
    // `$`-in-the-text check), so `target_orig 1` was silently missed.
    let src =
        "proc target {a b c} {}\nset newname target_orig\nrename target $newname\ntarget_orig 1\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E002".to_owned()],
        "newname's value is a known constant, so this rename must resolve and check arity"
    );
    let ok = src.replace("target_orig 1\n", "target_orig 1 2 3\n");
    assert_eq!(arity_codes(&ok, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_genuinely_dynamic_rename_target_does_not_false_positive() {
    // Unlike the sibling test above, `newname`'s value here is never a
    // tracked compile-time constant (piped through `gets`) — the rename
    // stays genuinely unresolvable and must never invent a diagnostic.
    let src =
        "proc target {a b c} {}\nset newname [gets stdin]\nrename target $newname\ntarget_orig 1\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "a genuinely dynamic rename target must not be resolved to target's arity"
    );
}

// Rename / deletion arity cases, all verified against tclsh 9.0.4.

#[test]
fn same_file_call_to_renamed_away_name_does_not_false_positive() {
    // `rename target target_orig` removes `target` as a command entirely
    // (tclsh 9.0.4: calling it afterwards fails "invalid command name",
    // not a "wrong # args" against its original 2-arg signature) — a
    // call to the old name must abstain, not be checked against the proc that
    // name denoted before the rename.
    let src = "proc target {a b} {}\nrename target target_orig\ntarget 1\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "target is gone after the rename; must not fire on its old arity"
    );
    // A call to `target` *before* the rename executes is unaffected.
    let before = "proc target {a b} {}\ntarget 1\nrename target target_orig\n";
    assert_eq!(
        arity_codes(before, "tcl8.6"),
        vec!["E002".to_owned()],
        "target is still callable before the rename runs"
    );
}

#[test]
fn same_file_call_to_deleted_command_does_not_false_positive() {
    // `rename target {}` deletes `target` outright (tclsh 9.0.4: also
    // "invalid command name" afterwards) — same abstention as a rename
    // to a new name.
    let src = "proc target {a b} {}\nrename target {}\ntarget 1\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_chained_rename_still_resolves_original_arity() {
    // Chasing `c -> b -> a` through two renames still reaches `a`'s own
    // 2-arg signature, even though both `a` and `b` are themselves
    // recorded as "deleted" by the renames that moved them onward —
    // confirmed against tclsh 9.0.4.
    let src = "proc a {x y} {}\nrename a b\nrename b c\nc 1\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
    assert_eq!(
        arity_codes("proc a {x y} {}\nrename a b\nrename b c\nc 1 2\n", "tcl8.6"),
        Vec::<String>::new()
    );
}

#[test]
fn same_file_top_level_call_before_alias_established_does_not_false_positive() {
    // `shortcut` doesn't exist yet at the point it's called — the
    // `interp alias` statement establishing it hasn't executed (tclsh
    // 9.0.4: fails "invalid command name", not an arity error against
    // the eventual alias target). Order matters at top level, unlike
    // inside a proc body.
    let src = "proc target {a b} {}\nshortcut 1\ninterp alias {} shortcut {} target\n";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        Vec::<String>::new(),
        "shortcut doesn't exist yet when called"
    );
    // Calling it after the alias is established is checked normally.
    let after = "proc target {a b} {}\ninterp alias {} shortcut {} target\nshortcut 1\n";
    assert_eq!(arity_codes(after, "tcl8.6"), vec!["E002".to_owned()]);
}

#[test]
fn same_file_proc_body_call_to_later_alias_is_not_order_gated() {
    // Inside a proc body, the alias is established by the time the body
    // ever runs (the whole file loads first) regardless of where the
    // `interp alias` statement sits textually — order-gating would be
    // unsound here, unlike at top level.
    let src =
        "proc target {a b} {}\nproc caller {} { shortcut 1 }\ninterp alias {} shortcut {} target\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
}

#[test]
fn same_file_alias_target_renamed_away_does_not_false_positive() {
    // An `interp alias` target is re-resolved *by name* on every call —
    // once that name is gone (renamed or deleted), the alias breaks too
    // (tclsh 9.0.4: `interp alias {} bar {} foo` then `rename foo baz` —
    // or `rename foo {}` — makes `bar` fail "invalid command name foo").
    // Unlike a rename-chase hop, an alias hop must still respect the
    // target's own deletion.
    let renamed_away = "proc foo {a} {}\ninterp alias {} bar {} foo\nrename foo baz\nbar 1\n";
    assert_eq!(arity_codes(renamed_away, "tcl8.6"), Vec::<String>::new());
    let deleted = "proc foo {a} {}\ninterp alias {} bar {} foo\nrename foo {}\nbar 1\n";
    assert_eq!(arity_codes(deleted, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_alias_to_renamed_target_still_resolves() {
    // By contrast, a rename that merely *moves* the target (not deletes
    // it) leaves any alias created against the new name working
    // normally — confirmed against tclsh 9.0.4.
    let src = "proc foo {a} {}\nrename foo bar\ninterp alias {} baz {} bar\nbaz 1 2\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E003".to_owned()]);
}

#[test]
fn same_file_over_applied_alias_always_flags_regardless_of_argcount() {
    // `target` takes exactly 1 argument, but the alias already bakes in
    // 2 prepended words — every call fails at run time no matter how
    // many further arguments are supplied (confirmed against tclsh
    // 9.0.4: `bad`, `bad x`, and `bad x y` all fail "wrong # args").
    // Must never silently accept a zero-argument call as valid.
    let src = |call: &str| {
        format!("proc target {{a}} {{}}\ninterp alias {{}} bad {{}} target fixed extra\n{call}\n")
    };
    for call in ["bad", "bad x", "bad x y"] {
        let codes = arity_codes(&src(call), "tcl8.6");
        assert!(
            !codes.is_empty(),
            "over-applied alias must always flag '{call}', got {codes:?}"
        );
    }
}

#[test]
fn same_file_deleted_alias_call_does_not_false_positive() {
    // `interp alias {} bar {}` (target path present, target command
    // absent) deletes a previously-created alias — confirmed against
    // tclsh 9.0.4: a later `bar` call fails "invalid command name", not
    // a "wrong # args" against the alias's stale target arity. Must
    // abstain, not misdiagnose the failure as an arity mismatch.
    let src = "proc foo {a b} {}\ninterp alias {} bar {} foo\ninterp alias {} bar {}\nbar 1\n";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_deleted_alias_call_inside_proc_body_does_not_false_positive() {
    // Deletion is not order-gated inside a proc body, same convention as
    // every other fact here — the deletion is unconditionally in effect
    // by the time any proc body runs.
    let src = "\
proc foo {a b} {}
interp alias {} bar {} foo
interp alias {} bar {}
proc use {} { bar 1 }
";
    assert_eq!(arity_codes(src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_deleted_alias_call_is_unknown_command() {
    // Regression: the arity resolver already abstains for a call through a
    // deleted alias (`same_file_deleted_alias_call_does_not_false_positive`
    // above), but `command_aliases` itself was never pruned on deletion, so
    // W123 ("unknown command") still treated the deleted name as known —
    // the call went through completely unchecked, neither an arity
    // diagnostic nor an unknown-command one. Confirmed against tclsh 9.0.4:
    // a call through a deleted alias fails "invalid command name".
    let mut a = Analyser::new();
    let src = "interp alias {} bar {} puts\ninterp alias {} bar {}\nbar 1\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W128),
        "a call through a deleted alias must be flagged; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn same_file_redeclared_alias_after_deletion_is_still_known() {
    // The inverse of the regression above: a name deleted and then
    // re-declared later in the file must stay known — the re-declaration
    // wins, exactly as `command_aliases`'s last-write-wins map already
    // implies for arity resolution.
    let mut a = Analyser::new();
    let src = "interp alias {} bar {} puts\ninterp alias {} bar {}\ninterp alias {} bar {} format\nbar 1\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "a re-declared alias must not be flagged unknown; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn same_file_alias_query_form_is_not_a_deletion() {
    // `interp alias srcPath srcCmd` (no target path at all — 2 args
    // after `alias`) is a *query*, not a deletion; the alias must keep
    // resolving normally afterwards.
    let src = "proc foo {a b} {}\ninterp alias {} bar {} foo\ninterp alias {} bar\nbar 1\n";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
}

#[test]
fn same_file_relative_qualified_call_resolves_against_current_namespace() {
    // `inner::p` is qualified but not absolute — Tcl resolves it against
    // the *current* namespace first, not global (confirmed against
    // tclsh 9.0.4: called from inside `namespace eval ::ns { … }`, it
    // reaches `::ns::inner::p`, not `::inner::p`).
    let src = "\
namespace eval ::ns {
    namespace eval inner {
        proc p {a b} {}
    }
    inner::p 1
}
";
    assert_eq!(arity_codes(src, "tcl8.6"), vec!["E002".to_owned()]);
    let ok_src = "\
namespace eval ::ns {
    namespace eval inner {
        proc p {a b} {}
    }
    inner::p 1 2
}
";
    assert_eq!(arity_codes(ok_src, "tcl8.6"), Vec::<String>::new());
}

#[test]
fn same_file_relative_qualified_call_prefers_current_namespace_over_global() {
    // When both `::inner::p` (global) and `::ns::inner::p` (nested)
    // exist, a relative `inner::p` call from inside `::ns` resolves to
    // the nested one — confirmed against tclsh 9.0.4.
    // `::inner` is created first: tclsh 8.6.18 / 9.0.4 refuse `proc
    // ::inner::p` in an unknown namespace and nothing after it runs.
    let src = "\
namespace eval ::inner {}
proc ::inner::p {} {}
namespace eval ::ns {
    namespace eval inner {
        proc p {a b} {}
    }
    inner::p 1 2 3
}
";
    assert_eq!(
        arity_codes(src, "tcl8.6"),
        vec!["E003".to_owned()],
        "must resolve against ::ns::inner::p (2 args), not ::inner::p (0 args)"
    );
}
