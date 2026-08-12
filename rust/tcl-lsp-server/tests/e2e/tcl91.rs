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

//! Tcl 9.1 dialect support, end-to-end over LSP. Oracle: C Tcl 9.1b0 source. The
//! dialect is pinned with a `# tcl-dialect: tcl9.1` directive (server-side source
//! detection), and behaviour is observed through completion + diagnostics.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

use std::collections::BTreeSet;

/// The set of completion labels from a completion result.
fn labels(result: &Value) -> BTreeSet<String> {
    completion_labels(result).into_iter().collect()
}

/// The set of `code` strings carried by `diags`.
fn codes(diags: &[Value]) -> BTreeSet<String> {
    diags
        .iter()
        .map(|d| match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "None".to_owned(),
        })
        .collect()
}

/// Open a buffer pinned to `dialect` and complete a command-position word.
fn complete_cmd(lsp: &mut Lsp, dialect: &str, partial: &str) -> BTreeSet<String> {
    let uri = unique_uri("tcl");
    let src = format!("# tcl-dialect: {dialect}\n{partial}\n");
    lsp.open_ready(&uri, &src);
    labels(&lsp.completion(&uri, 1, u32::try_from(partial.len()).unwrap()))
}

// -- TestTcl91Completion -------------------------------------------------

#[test]
fn unicode_and_timer_offered_in_91() {
    // doc/unicode.n, doc/timer.n — both are new commands in 9.1.
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "unic").contains("unicode"));
    assert!(complete_cmd(&mut lsp, "tcl9.1", "time").contains("timer"));
}

#[test]
fn unicode_and_timer_absent_in_90() {
    let mut lsp = Lsp::tcl();
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "unic").contains("unicode"));
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "time").contains("timer"));
}

#[test]
fn math_and_lfilter_offered_in_91() {
    // doc/divmod.n, doc/lfilter.n — new commands in 9.1 (C tclBasic.c).
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "divm").contains("divmod"));
    assert!(complete_cmd(&mut lsp, "tcl9.1", "lfil").contains("lfilter"));
}

#[test]
fn math_absent_in_90() {
    let mut lsp = Lsp::tcl();
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "divm").contains("divmod"));
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "lfil").contains("lfilter"));
}

#[test]
fn commands_90_still_offered_in_91() {
    // A `.1` release is additive: `lseq` (9.0) persists in 9.1.
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "lseq").contains("lseq"));
}

// `oo::Helpers::link` version-gating (issue #923, Codex review on PR
// #1020): a genuine core TclOO builtin only since 9.0 (confirmed against
// tclsh 9.0.4 — no package needed); under 8.6/8.7 it exists only via the
// Tcllib `ooutil` package (confirmed against tclsh 8.6.14 — bare `link`
// with no `package require` is `invalid command name "link"`).

#[test]
fn link_is_unconditionally_available_in_90() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl9.0\noo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n        return [foo 1]\n    }\n}\n",
    );
    assert!(
        !codes(&diags).contains("W120"),
        "core Tcl 9.0 link needs no package require ooutil: {:?}",
        codes(&diags)
    );
}

#[test]
fn link_requires_ooutil_package_require_in_86() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n        return [foo 1]\n    }\n}\n",
    );
    assert!(
        codes(&diags).contains("W120"),
        "8.6/8.7 link is Tcllib ooutil-only, not core, with no package require anywhere: {:?}",
        codes(&diags)
    );
}

#[test]
fn link_stays_silent_in_86_once_ooutil_is_required() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "package require ooutil\noo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n        return [foo 1]\n    }\n}\n",
    );
    assert!(
        !codes(&diags).contains("W120"),
        "a real `package require ooutil` makes 8.6/8.7 link known: {:?}",
        codes(&diags)
    );
}

// The whole `oo::Helpers` family is method-context-scoped (issue #1026).
//
// tclsh 9.0.4 at the top level answers `invalid command name` for every one
// of `link` / `my` / `next` / `nextto` / `self` / `classvariable`, and
// `info commands ::link` is empty — while inside a method body
// `namespace which -command link` answers `::oo::Helpers::link` (and
// `… my` answers `::oo::ObjN::my`, an object-namespace command rather than
// a helper). tclsh 8.6.14 agrees for the four members it ships.

/// The `# tcl-dialect: tcl9.0` document the repro in issue #1026 uses.
fn scoped_family_doc(body: &str) -> String {
    format!("# tcl-dialect: tcl9.0\n{body}")
}

#[test]
fn top_level_link_draws_w123() {
    // Issue #1026's own repro: `link foo` at the top level under tcl9.0.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, &scoped_family_doc("link foo\n"));
    assert!(
        codes(&diags).contains("W123"),
        "top-level `link foo` is `invalid command name \"link\"` in real Tcl 9.0: {:?}",
        codes(&diags)
    );
}

#[test]
fn method_body_link_stays_clean() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        &scoped_family_doc(
            "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n        return [foo 1]\n    }\n}\n",
        ),
    );
    assert!(
        !codes(&diags).contains("W123"),
        "`link` resolves inside a method body: {:?}",
        codes(&diags)
    );
}

#[test]
fn top_level_family_members_all_draw_w123() {
    let mut lsp = Lsp::tcl();
    for word in [
        "my",
        "next",
        "nextto",
        "self",
        "classvariable",
        "callback",
        "mymethod",
    ] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, &scoped_family_doc(&format!("{word} foo\n")));
        assert!(
            codes(&diags).contains("W123"),
            "top-level `{word}` is `invalid command name`: {:?}",
            codes(&diags)
        );
    }
}

#[test]
fn link_is_not_offered_in_top_level_completion() {
    // Completion follows the same scoping: offering `link` at the top level
    // would insert a call real Tcl rejects.
    let mut lsp = Lsp::tcl();
    assert!(
        !complete_cmd(&mut lsp, "tcl9.0", "lin").contains("link"),
        "no top-level completion of a method-context-only command",
    );
}

/// Byte-for-byte the VS Code fixture `editors/vscode/testFixture/
/// ooMethodContext.tcl`, so the extension test's cursor positions are
/// pinned here against the same server.
const OO_METHOD_CONTEXT_FIXTURE: &str = "# tcl-dialect: tcl9.0\nlin\noo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        lin\n    }\n}\n";

#[test]
fn link_is_offered_inside_a_method_body() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, OO_METHOD_CONTEXT_FIXTURE);
    // Line 1 is the top-level `lin`; line 5 is the `lin` inside `method bar`.
    let top = labels(&lsp.completion(&uri, 1, 3));
    assert!(!top.contains("link"), "top level offered `link`: {top:?}");
    let inner = labels(&lsp.completion(&uri, 5, 11));
    assert!(inner.contains("link"), "expected `link` among {inner:?}");
}

#[test]
fn top_level_link_has_no_hover_but_a_method_body_one_does() {
    let mut lsp = Lsp::tcl();
    let top = unique_uri("tcl");
    lsp.open_ready(&top, &scoped_family_doc("link foo\n"));
    assert!(
        hover_text(&lsp.hover(&top, 1, 1)).is_empty(),
        "an unresolvable top-level `link` must not hover as a builtin",
    );
    let inner = unique_uri("tcl");
    lsp.open_ready(
        &inner,
        &scoped_family_doc(
            "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n    }\n}\n",
        ),
    );
    assert!(
        hover_text(&lsp.hover(&inner, 4, 9)).contains("link"),
        "the same word hovers inside a method body",
    );
}

#[test]
fn the_qualified_oo_helpers_spelling_resolves_at_the_top_level() {
    // `info commands ::oo::Helpers::link` answers `::oo::Helpers::link`
    // under tclsh 9.0.4 — calling it outside a method is a *runtime* error
    // ("may only be called from inside a method"), not an unknown command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, &scoped_family_doc("::oo::Helpers::link foo\n"));
    assert!(
        !codes(&diags).contains("W123"),
        "the qualified spelling is a real global command: {:?}",
        codes(&diags)
    );
}

// `callback` / `mymethod` version gating (issue #923 audit, `ticklecharts`
// idx 51): both are genuine core `::oo::Helpers` members from 9.0 onward and
// exist nowhere in 8.6 core, so a bare call inside a method body must be
// silent on 9.0 and reported on 8.6.
//
// tclsh 9.0.4: `info commands ::oo::Helpers::*` -> `… callback … mymethod …`,
// and inside a method `callback Foo 1 2` -> `::oo::Obj22::my Foo 1 2`.
// tclsh 8.6.14: the same list holds only `next nextto self`, and the same
// method body answers `invalid command name "callback"`.

/// The class layout `ticklecharts`'s `esnap.tcl` uses: a capitalised (hence
/// auto-private) handler method reached through a `callback`-built prefix.
fn callback_doc(dialect_directive: &str, word: &str) -> String {
    format!(
        "{dialect_directive}oo::class create Snap {{\n    method Start {{}} {{\n        return [{word} ReadBrowser]\n    }}\n    method ReadBrowser {{}} {{ return 1 }}\n}}\n"
    )
}

#[test]
fn callback_and_mymethod_resolve_inside_a_method_body_in_90() {
    // FP fix: before the registry entries the bare word drew
    // `Unknown command 'callback'` on every dialect.
    let mut lsp = Lsp::tcl();
    for word in ["callback", "mymethod"] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, &callback_doc("# tcl-dialect: tcl9.0\n", word));
        assert!(
            !codes(&diags).contains("W123"),
            "`{word}` is core Tcl 9.0 and resolves in a method body: {:?}",
            codes(&diags)
        );
    }
}

#[test]
fn callback_is_unknown_in_86_core() {
    // TN, the version dimension: 8.6 core has no `callback` at all, and no
    // package supplies it either — `ticklecharts` hand-rolls a
    // `proc ::oo::Helpers::callback` behind a version guard precisely
    // because of that, and the workspace index resolves *that* proc on its
    // own. With no such proc in the document the word is genuinely unknown.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, &callback_doc("", "callback"));
    assert!(
        codes(&diags).contains("W123"),
        "8.6 core has no `callback`: {:?}",
        codes(&diags)
    );
}

#[test]
fn mymethod_in_86_asks_for_ooutil_rather_than_being_unknown() {
    // `mymethod` is the one of the pair Tcllib backfills: `ooutil` defines
    // `proc ::oo::Helpers::mymethod`. So under 8.6 it is a
    // needs-a-package-require report, not an unknown command — and a real
    // `package require ooutil` silences it.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, &callback_doc("", "mymethod"));
    let seen = codes(&diags);
    assert!(
        seen.contains("W120") && !seen.contains("W123"),
        "8.6 `mymethod` is ooutil-provided, not unknown: {seen:?}",
    );

    let required = unique_uri("tcl");
    let diags = lsp.open_ready(
        &required,
        &callback_doc("package require ooutil\n", "mymethod"),
    );
    let seen = codes(&diags);
    assert!(
        !seen.contains("W120") && !seen.contains("W123"),
        "a real `package require ooutil` makes 8.6 `mymethod` known: {seen:?}",
    );
}

#[test]
fn the_qualified_callback_spelling_resolves_at_the_top_level_in_90() {
    // Same rule the `link` twin pins: `info commands ::oo::Helpers::callback`
    // answers under tclsh 9.0.4, and calling it outside a method is the
    // *runtime* error "may only be called from inside a method".
    let mut lsp = Lsp::tcl();
    for word in ["callback", "mymethod"] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(
            &uri,
            &scoped_family_doc(&format!("::oo::Helpers::{word} Foo\n")),
        );
        assert!(
            !codes(&diags).contains("W123"),
            "`::oo::Helpers::{word}` is a real global command: {:?}",
            codes(&diags)
        );
    }
}

/// A constructor body that links a class-shared variable — mirrors
/// `callback_doc`'s shape for the same family of gaps.
fn classvariable_doc(prefix: &str) -> String {
    format!(
        "{prefix}oo::class create Counted {{\n    variable number\n    constructor {{}} {{\n        classvariable count\n        set number 1\n    }}\n}}\n"
    )
}

#[test]
fn classvariable_resolves_inside_a_method_body_in_90() {
    // FP fix: `classvariable` is a genuine core Tcl 9.0 `::oo::Helpers`
    // member and resolves with no `package require` at all.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, &classvariable_doc("# tcl-dialect: tcl9.0\n"));
    assert!(
        !codes(&diags).contains("W123"),
        "`classvariable` is core Tcl 9.0 and resolves in a method body: {:?}",
        codes(&diags)
    );
}

#[test]
fn classvariable_in_86_asks_for_ooutil_rather_than_being_unknown() {
    // `classvariable` gets the same 8.6/8.7 `ooutil` route as `link` and
    // `mymethod`: Tcllib's `ooutil` defines `proc ::oo::Helpers::classvariable`
    // (tcllib-2-0 `modules/ooutil/ooutil.tcl:27`), so under 8.6 it is a
    // needs-a-package-require report, not an unknown command — and a real
    // `package require ooutil` silences it.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, &classvariable_doc(""));
    let seen = codes(&diags);
    assert!(
        seen.contains("W120") && !seen.contains("W123"),
        "8.6 `classvariable` is ooutil-provided, not unknown: {seen:?}",
    );

    let required = unique_uri("tcl");
    let diags = lsp.open_ready(&required, &classvariable_doc("package require ooutil\n"));
    let seen = codes(&diags);
    assert!(
        !seen.contains("W120") && !seen.contains("W123"),
        "a real `package require ooutil` makes 8.6 `classvariable` known: {seen:?}",
    );
}

// -- TestTcl91Operators --------------------------------------------------
// doc/expr.n — the `lt`/`le`/`gt`/`ge` string operators (TIP 461) are 9.0+.

#[test]
fn lt_operator_no_w003_in_91() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl9.1\nexpr {$a lt $b}\n");
    assert!(!codes(&diags).contains("W003"));
}

#[test]
fn lt_operator_flags_w003_in_86() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.6\nexpr {$a lt $b}\n");
    assert!(codes(&diags).contains("W003"));
}

#[test]
fn ge_operator_no_w003_in_91() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl9.1\nexpr {$a ge $b}\n");
    assert!(!codes(&diags).contains("W003"));
}

#[test]
fn ge_operator_flags_w003_in_86() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.6\nexpr {$a ge $b}\n");
    assert!(codes(&diags).contains("W003"));
}

// -- Codex review of PR #1084 ------------------------------------------------

/// Whether any completion item labelled `label` is a **built-in** — i.e. came
/// from the registry rather than from a user `proc` of the same name.
///
/// The label alone cannot answer that. A workspace may legitimately define
/// `proc link {arr} {…}` (the VS Code fixture folder does, in
/// `nameVsValuePositions.tcl`), and such a proc is a real global command that
/// completion must offer everywhere — so a label-only assertion about `link`
/// says nothing about the `oo::Helpers` scoping rule. A registry item carries
/// the spec's hover summary as `documentation`; a user proc carries none and
/// its `detail` is the parameter list.
fn has_builtin(result: &Value, label: &str) -> bool {
    completion_items(result).iter().any(|item| {
        item.get("label").and_then(Value::as_str) == Some(label)
            && item
                .get("documentation")
                .and_then(Value::as_str)
                .is_some_and(|d| !d.is_empty())
    })
}

/// Whether any completion item labelled `label` is a **user proc**.
fn has_user_proc(result: &Value, label: &str) -> bool {
    completion_items(result).iter().any(|item| {
        item.get("label").and_then(Value::as_str) == Some(label)
            && item.get("documentation").is_none()
    })
}

/// A workspace `proc link` must not mask the scoping rule — the regression
/// CI's `test-ext` caught, where the extension harness opens the whole
/// `testFixture` folder (which defines `proc link {arr}`) and my label-only
/// assertion read that proc as the builtin.
///
/// Both facts hold at once and neither is about the other: the user proc is a
/// real global command offered in **both** positions, while the built-in
/// `link` is offered only in the method body. Real Tcl agrees on both halves
/// — `proc link {arr} {…}` then `link foo` runs at the top level, and bare
/// `link` there is otherwise `invalid command name "link"`.
#[test]
fn a_user_proc_named_link_does_not_mask_the_builtin_scoping() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl9.0\nproc link {arr} { return $arr }\noo::class create W {\n    method m {} {\n        lin\n    }\n}\nlin\n",
    );
    let top = lsp.completion(&uri, 7, 3);
    let inner = lsp.completion(&uri, 4, 11);
    assert!(
        has_user_proc(&top, "link"),
        "a real global `proc link` is callable at the top level and must be offered",
    );
    assert!(
        !has_builtin(&top, "link"),
        "the *built-in* `link` must stay gated at the top level",
    );
    assert!(
        has_user_proc(&inner, "link"),
        "the user proc is in scope inside a method body too",
    );
    assert!(
        has_builtin(&inner, "link"),
        "the built-in `link` is offered inside a method body",
    );
}

/// A Tcl 9 class-level `initialise` body **resolves** the family but cannot
/// **call** it, and the two facts drive different features.
///
/// tclsh 9.0.4, inside `oo::class create ::P { initialize { … } }`:
///
/// ```text
/// ns=::oo::Obj20  path=::oo::Helpers ::oo
/// link:  which='::oo::Helpers::link'  call -> link may only be called from inside a method
/// self:  which='::oo::Helpers::self'  call -> self may only be called from inside a method
/// my:    which='::oo::Obj20::my'      call -> OK  (`my new` returns ::oo::Obj22)
/// ```
///
/// So W123 stays silent (the command is not unknown), completion offers only
/// `my`, and hover describes only `my`.
#[test]
fn class_init_body_resolves_the_family_but_only_my_is_callable() {
    let mut lsp = Lsp::tcl();
    // A *complete* call, to observe W123: the command is not unknown here.
    let resolved = unique_uri("tcl");
    let diags = lsp.open_ready(
        &resolved,
        "# tcl-dialect: tcl9.0\noo::class create C {\n    initialize { link m }\n    method m {} { return 1 }\n}\n",
    );
    assert!(
        !codes(&diags).contains("W123"),
        "the family resolves in an `initialize` body: {:?}",
        codes(&diags)
    );
    // A *partial* word, to observe completion. Line 2 is the init-body
    // probe, line 4 the method-body one.
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl9.0\noo::class create C {\n    initialize { lin }\n    method m {} {\n        lin\n    }\n}\n",
    );
    let init = lsp.completion(&uri, 2, 20);
    assert!(
        !has_builtin(&init, "link"),
        "`link` raises `may only be called from inside a method` in an init body",
    );
    let method = lsp.completion(&uri, 4, 11);
    assert!(
        has_builtin(&method, "link"),
        "a real method body still offers it",
    );
}

#[test]
fn class_init_body_still_offers_and_hovers_my() {
    // `my` is not an `::oo::Helpers` member — it is `::oo::ObjN::my`, the
    // object's own dispatch command, and a class *is* an object, so `my new`
    // in an `initialize` body genuinely makes an instance (tclsh 9.0.4).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl9.0\noo::class create C {\n    initialize { m }\n    method m {} { return 1 }\n}\n";
    lsp.open_ready(&uri, src);
    let init = lsp.completion(&uri, 2, 18);
    assert!(
        has_builtin(&init, "my"),
        "`my` is callable in a class-level init body",
    );
    let hovered = unique_uri("tcl");
    lsp.open_ready(
        &hovered,
        "# tcl-dialect: tcl9.0\noo::class create C {\n    initialize { my new }\n    method m {} { return 1 }\n}\n",
    );
    assert!(
        !hover_text(&lsp.hover(&hovered, 2, 18)).is_empty(),
        "`my` hovers in a class-level init body",
    );
}

#[test]
fn class_init_body_does_not_hover_a_method_only_helper() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl9.0\noo::class create C {\n    initialize { self }\n    method m {} { return 1 }\n}\n",
    );
    assert!(
        hover_text(&lsp.hover(&uri, 2, 18)).is_empty(),
        "`self` raises `may only be called from inside a method` in an init body",
    );
}

/// Tcllib's `ooutil` installs a real `::oo::Helpers::link` under 8.6/8.7, so
/// the **qualified** spelling needs the same package gating its bare twin has
/// (Codex review of PR #1084). Without the second spec the fully qualified
/// call was unknown on exactly the dialect where a user must reach for it.
#[test]
fn qualified_link_resolves_in_86_once_ooutil_is_required() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.6\npackage require ooutil\noo::class create W {\n    method foo {} { return 1 }\n    method m {} { ::oo::Helpers::link foo }\n}\n",
    );
    let seen = codes(&diags);
    assert!(
        !seen.contains("W123") && !seen.contains("W120"),
        "8.6 + ooutil provides ::oo::Helpers::link: {seen:?}",
    );
    assert!(
        !hover_text(&lsp.hover(&uri, 4, 22)).is_empty(),
        "the qualified spelling hovers once the package is required",
    );
}

#[test]
fn qualified_link_needs_the_ooutil_require_in_86() {
    // Same convention as the bare twin (`link_requires_ooutil_package_require_in_86`):
    // 8.6 core has no `::oo::Helpers::link` at all.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.6\noo::class create W {\n    method foo {} { return 1 }\n    method m {} { ::oo::Helpers::link foo }\n}\n",
    );
    assert!(
        codes(&diags).contains("W120"),
        "8.6 without `package require ooutil` has no ::oo::Helpers::link: {:?}",
        codes(&diags)
    );
}

/// A Tcl 9 buffer must describe the **core** `link`, not the Tcllib one.
///
/// `link` has two specs under one name; the completion item used to take its
/// `detail` / `documentation` from whichever the by-name lookup returned
/// first, so a `tcl9.0` document showed `tcllib (ooutil)` for a core command.
#[test]
fn a_tcl90_buffer_describes_the_core_link_not_the_tcllib_one() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl9.0\noo::class create W {\n    method m {} {\n        lin\n    }\n}\n",
    );
    let detail = completion_items(&lsp.completion(&uri, 3, 11))
        .iter()
        .find(|item| {
            item.get("label").and_then(Value::as_str) == Some("link")
                && item.get("documentation").is_some()
        })
        .and_then(|item| item.get("detail").and_then(Value::as_str))
        .map(str::to_owned)
        .expect("the built-in `link` is offered in a method body");
    assert!(
        !detail.contains("ooutil"),
        "a tcl9.0 buffer must not describe core `link` as tcllib: {detail:?}",
    );
}
