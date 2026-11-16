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

//! Missing subcommands and methods (E001), disabled commands (W002) and unknown subcommands (W001), with the shadowing that silences them.

use super::*;

// E001 ("missing subcommand" / TclOO "missing method") highlights only the
// command head. Bare `history` defaults to `history info` per history(n), so
// it must not be flagged; TclOO object dispatch (`$obj` with no method word
// at all) is the same diagnostic generalised to objects.

#[test]
fn bare_string_is_e001_with_tight_command_head_span() {
    // The diagnostic must highlight only the command word itself — there is
    // no subcommand to include, so the span should not creep onto the
    // trailing newline or beyond the four characters of `string`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "string\n");
    let e001: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E001"))
        .collect();
    assert_eq!(e001.len(), 1, "expected exactly one E001: {diags:?}");
    let range = &e001[0]["range"];
    assert_eq!(range["start"]["line"], 0);
    assert_eq!(range["start"]["character"], 0);
    assert_eq!(range["end"]["line"], 0);
    assert_eq!(
        range["end"]["character"], 6,
        "span must cover only 'string'"
    );
    assert_eq!(
        e001[0].get("severity").and_then(Value::as_i64),
        Some(1) // Error
    );
}

#[test]
fn bare_history_has_no_e001() {
    // FP regression (history(n)): `history` alone is a well-defined call
    // (equivalent to `history info`), not a missing-subcommand error, even
    // though `history` is a `WithSubcommands` registry command like `string`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "history\n");
    assert!(
        !has_code(&diags, "E001"),
        "bare `history` must not be E001: {diags:?}"
    );
}

#[test]
fn history_with_subcommand_is_still_clean() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "history clear\n");
    assert!(!has_code(&diags, "E001"));
    assert!(!has_code(&diags, "W001"));
}

#[test]
fn history_unknown_subcommand_is_still_w001_not_e001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "history bogus\n");
    assert!(has_code(&diags, "W001"));
    assert!(!has_code(&diags, "E001"));
}

#[test]
fn bare_tcloo_command_substitution_head_is_e001() {
    // `[Dog new]` runs the constructor, then invokes the object command
    // with no method word — tclsh 9.0 fails `wrong # args: should be
    // "::oo::Obj… method ?arg ...?"` (`-errorcode {TCL WRONGARGS}`). The
    // span anchors on the substitution head.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Dog {\n    method bark {} {return woof}\n}\n[Dog new]\n",
    );
    assert_eq!(
        on_line(&diags, "E001").into_iter().collect::<Vec<_>>(),
        vec![3],
        "bare `[Dog new]` must be E001 on its own line: {diags:?}"
    );
    // A well-formed dispatch through the same head stays clean.
    let uri2 = unique_uri("tcl");
    let diags2 = lsp.open_ready(
        &uri2,
        "oo::class create Dog {\n    method bark {} {return woof}\n}\n[Dog new] bark\n",
    );
    assert!(
        !has_code(&diags2, "E001") && !has_code(&diags2, "W307"),
        "`[Dog new] bark` is well-formed: {diags2:?}"
    );
}

#[test]
fn method_return_captured_handle_has_no_w307_and_bare_dispatch_is_e001() {
    // `set b [$a make]` types `b` through the lattice's method-return edge,
    // so `$b greet` is silent (no W307), and a bare `$b` is the zero-word
    // E001. The producing
    // method roots its command heads so TclOO's runtime receiver namespace
    // cannot select a different `return` or constructor implementation.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create A { method make {} { ::return [::B new] } }\n\
         oo::class create B { method greet {} { ::return \"hi\" } }\n\
         set a [A new]\n\
         set b [$a make]\n\
         $b greet\n",
    );
    assert!(
        !has_code(&diags, "W307") && !has_code(&diags, "E001"),
        "the method-return-captured dispatch must be silent: {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags2 = lsp.open_ready(
        &uri2,
        "oo::class create A { method make {} { ::return [::B new] } }\n\
         oo::class create B { method greet {} { ::return \"hi\" } }\n\
         set a [A new]\n\
         set b [$a make]\n\
         $b\n",
    );
    assert_eq!(
        on_line(&diags2, "E001").into_iter().collect::<Vec<_>>(),
        vec![4],
        "the bare `$b` dispatch must be E001: {diags2:?}"
    );
}

/// The `(character, character)` span of the first diagnostic carrying `code`
/// on `line`, or `None` if there isn't one.
fn range_on_line(diags: &[Value], code: &str, line: i64) -> Option<(i64, i64)> {
    diags.iter().find_map(|d| {
        if code_str(d).as_deref() != Some(code) {
            return None;
        }
        let range = d.get("range")?;
        let start = range.get("start")?;
        if start.get("line")?.as_i64()? != line {
            return None;
        }
        let end = range.get("end")?;
        Some((
            start.get("character")?.as_i64()?,
            end.get("character")?.as_i64()?,
        ))
    })
}

#[test]
fn disabled_command_is_w002_with_a_tight_span() {
    // `dict` is Tcl 8.5+; under `tcl8.4` the call is disabled, and the
    // squiggle must cover exactly the 4-character `dict` token — not the
    // whole line — so the editor highlights only the offending name.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.4\ndict create a 1\n");
    let w002: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W002"))
        .collect();
    assert_eq!(w002.len(), 1, "expected exactly one W002: {diags:?}");
    assert_eq!(
        message(w002[0]),
        "'dict' is disabled in the active dialect profile \
         (available in: Tcl 8.5, Tcl 8.6, Tcl 9.0, Tcl 9.1)"
    );
    assert_eq!(
        range_on_line(&diags, "W002", 1),
        Some((0, 4)),
        "the span must cover exactly 'dict' on line 1: {diags:?}"
    );
}

#[test]
fn disabled_command_enabled_in_active_dialect_is_clean() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl9.0\ndict create a 1\n");
    assert!(
        !has_code(&diags, "W002"),
        "dict is native in 9.0: {diags:?}"
    );
}

#[test]
fn disabled_subcommand_is_w002_with_a_command_plus_subcommand_span() {
    // `package files` is Tcl 9.0+; the squiggle covers `package files`
    // (command head through the subcommand word), not the whole call.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "package files mypackage\n");
    assert!(
        !has_code(&diags, "W001"),
        "must not report as unknown: {diags:?}"
    );
    assert_eq!(
        range_on_line(&diags, "W002", 0),
        Some((0, 13)),
        "the span must cover 'package files' (13 chars): {diags:?}"
    );
}

#[test]
fn disabled_command_shadowed_by_namespace_scoped_proc_is_clean() {
    // A namespace-scoped `proc dict` shadows the disabled builtin for
    // unqualified calls resolved inside that namespace — Tcl's real
    // current-namespace-then-global resolution rule, not merely "was `dict`
    // defined anywhere in the file".
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.4\nnamespace eval ::ns {\n    proc dict {args} { return $args }\n    dict foo bar\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W002"),
        "a namespace-scoped shadowing proc must suppress W002: {diags:?}"
    );
}

#[test]
fn disabled_command_shadowed_by_forward_declared_proc_body_is_clean() {
    // The shadowing proc is declared *after* the call site textually, but
    // the call only runs (inside another proc's body) once the whole file
    // has loaded — so the later definition is already in effect.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.4\nproc use_dict {} {\n    dict create a 1\n}\nproc dict {args} { return $args }\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W002"),
        "a forward-declared proc-body shadow must suppress W002: {diags:?}"
    );
}

#[test]
fn disabled_command_top_level_call_before_shadowing_proc_still_fires() {
    // The mirror image: a *top-level* call before its shadowing proc's
    // definition still reaches the disabled builtin at load time.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.4\ndict create a 1\nproc dict {args} { return $args }\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "W002"),
        "a top-level call before its shadowing proc must still fire W002: {diags:?}"
    );
}

#[test]
fn disabled_command_shadowed_by_interp_alias_is_clean() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.4\ninterp alias {} dict {} list\ndict create a 1\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W002"),
        "an interp alias establishing the name must suppress W002: {diags:?}"
    );
}

#[test]
fn disabled_command_shadowed_by_rename_is_clean() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.4\nproc myimpl {args} { return $args }\nrename myimpl dict\ndict create a 1\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W002"),
        "a rename establishing the name must suppress W002: {diags:?}"
    );
}

#[test]
fn disabled_subcommand_form_shadowed_by_ensemble_head_proc_is_clean() {
    // A user `proc package` overrides the *whole* ensemble command — the
    // call never reaches the registry `package files` subcommand check.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc package {args} { return $args }\npackage files mypackage\n",
    );
    assert!(
        !has_code(&diags, "W002"),
        "a shadowed ensemble head must suppress the subcommand-form W002: {diags:?}"
    );
}

#[test]
fn w001_span_covers_only_the_subcommand_word() {
    // The squiggle must sit tightly on the offending word alone — not the
    // command name too. `string bogus $x`: "string " is 7 characters, so
    // "bogus" spans [7, 12).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "string bogus $x\n");
    let w001: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W001"))
        .collect();
    assert_eq!(w001.len(), 1, "expected exactly one W001: {diags:?}");
    let range = &w001[0]["range"];
    assert_eq!(range["start"]["line"], 0);
    assert_eq!(
        range["start"]["character"], 7,
        "span must start at 'bogus', not 'string'"
    );
    assert_eq!(range["end"]["line"], 0);
    assert_eq!(
        range["end"]["character"], 12,
        "span must cover only 'bogus'"
    );
}

#[test]
fn proc_shadowing_ensemble_command_suppresses_w001() {
    // FP regression (FP-STY-17): a same-file `proc string {...}` replaces
    // the builtin `string` ensemble at the call site, end to end.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc string {op args} { return $op }\nstring reverse hello\n",
    );
    assert!(
        !has_code(&diags, "W001"),
        "proc-shadowed ensemble call must not fire W001: {diags:?}"
    );
}

#[test]
fn unshadowed_ensemble_command_still_fires_w001_alongside_a_shadowed_one() {
    // TP control paired with the FP above: shadowing `string` must not
    // blind the server to a genuine unknown subcommand on a different
    // ensemble in the same file.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc string {op args} { return $op }\ninfo bogus\n");
    assert!(
        has_code(&diags, "W001"),
        "unshadowed ensemble must still fire W001: {diags:?}"
    );
}

#[test]
fn namespace_ensemble_configure_splice_onto_tk_suppresses_w001_end_to_end() {
    // The real `tk/library/systray.tcl`
    // idiom splices `systray`/`sysnotify` onto the pre-existing,
    // registry-builtin `tk` ensemble via `namespace ensemble configure tk
    // -map [dict merge [namespace ensemble configure tk -map] {systray
    // ::tk::systray sysnotify ::tk::sysnotify::sysnotify}]` — a
    // `CONFIGURE`, not `CREATE`. tclsh9.0/8.6 both confirm `tk systray
    // create`/`tk sysnotify ...` are correct, documented calls.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc ::tk::systray {args} {}\n\
         proc ::tk::sysnotify::sysnotify {a b} {}\n\
         namespace ensemble configure tk -map \
         [dict merge [namespace ensemble configure tk -map] \
         {systray ::tk::systray sysnotify ::tk::sysnotify::sysnotify}]\n\
         tk systray create -image book\n\
         tk sysnotify Alert message\n",
    );
    assert!(
        !has_code(&diags, "W001"),
        "a statically-spliced ensemble subcommand must not fire W001: {diags:?}"
    );
}

#[test]
fn namespace_ensemble_configure_genuinely_unknown_tk_subcommand_still_fires_w001_end_to_end() {
    // TP control paired with the FP above: splicing `systray` onto `tk`
    // must not blind the server to an unrelated, genuinely unknown `tk`
    // subcommand in the same file.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "namespace ensemble configure tk -map \
         [dict merge [namespace ensemble configure tk -map] \
         {systray ::tk::systray}]\n\
         tk zzznotreal\n",
    );
    assert!(
        has_code(&diags, "W001"),
        "a genuinely unknown tk subcommand must still fire W001: {diags:?}"
    );
}

#[test]
fn bare_tcloo_object_dispatch_is_e001() {
    // `set o [Dog new]; $o` — TclOO's per-object dispatcher requires a
    // method word before it attempts any method lookup.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Dog { method bark {} { return woof } }\nset o [Dog new]\n$o\n";
    let diags = lsp.open_ready(&uri, src);
    let e001: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E001"))
        .collect();
    assert_eq!(e001.len(), 1, "expected exactly one E001: {diags:?}");
    assert_eq!(message(e001[0]), "'o' requires a method");
    assert_eq!(on_line(&diags, "E001"), BTreeSet::from([2]));
}

#[test]
fn tcloo_object_dispatch_with_method_is_not_e001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Dog { method bark {} { return woof } }\nset o [Dog new]\n$o bark\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "E001"));
}

#[test]
fn bare_snit_instance_dispatch_is_not_e001() {
    // snit's generated dispatcher proc is a different mechanism the
    // analyser does not model precisely enough to assume it shares
    // TclOO's unconditional "wrong # args" behaviour on a bare call.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "snit::type Dog { method bark {} { return woof } }\nDog t\n$t\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "E001"));
}
