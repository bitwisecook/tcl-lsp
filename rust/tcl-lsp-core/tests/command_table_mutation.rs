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

//! Navigation across a **mutated command table**: `rename`, `interp alias`,
//! and `proc` self-redefinition, plus the `[namespace code [list X]]`
//! callback wrapper.
//!
//! Tcl's command table is not fixed at parse time, so a call site's spelling
//! does not by itself name the definition it reaches. Every claim below is
//! pinned to real C Tcl (tclsh 9.0.4 and tclsh 8.6.16 agree on all of them):
//!
//! * `proc greet {} {return hi}` / `rename greet hello` — `hello` then
//!   returns `hi`, `greet` raises `invalid command name "greet"`, and a
//!   `hello` written *before* the rename raises `invalid command name
//!   "hello"`. Go-to-definition therefore resolves `hello` after the rename
//!   and abstains before it.
//! * `interp alias {} sayHi {} greet` + two `[sayHi]` calls — `greet`'s body
//!   runs twice, so both `[sayHi]` sites are real references to `greet`.
//! * `proc ::ttk::spinbox …` / `proc ::tk::spinbox …` / `interp alias {}
//!   ::ttk::spinbox {} ::tk::spinbox` — a later `::ttk::spinbox` call prints
//!   `classic tk::spinbox:`, i.e. the alias has replaced the original proc,
//!   which is unreachable under that name from the alias onward.
//! * `interp alias {} withextra {} target pre` — `withextra x` fails `wrong #
//!   args: should be "withextra"`, so an argument-prepending alias is not the
//!   same call and navigation declines it.
//! * `proc p {} {return first}` / `p` / `proc p {a} {…}` / `p x` — the first
//!   call returns `first` and the second dispatches the one-parameter
//!   definition, so the two call sites resolve to two different headers.
//! * `trace add variable S(size) write [namespace code [list Tracer]]` —
//!   `trace info variable ::demo::S(size)` reports `{write {::namespace
//!   inscope ::demo Tracer}}` and writing the variable dispatches
//!   `::demo::Tracer`, so the wrapped `Tracer` word is a real call site.
//!
//! Three further claims were pinned on tclsh 9.0.4 and 8.6.14 for the PR
//! #1075 review round:
//!
//! * `proc a {} {return A}` / `proc b {} {return B}` / `rename a x` /
//!   `interp alias {} x {} b` — `x` returns `B`: the *later* of the two
//!   bindings on one name is the one the slot holds. (The mirror order is not
//!   a reachable program: `rename a x` onto a live `x` aborts with `can't
//!   rename to "x": command already exists`.)
//! * `proc p {} {return first}` / `rename p oldp` / `proc p {} {return
//!   second}` — `oldp` returns `first` and `p` returns `second`, and `oldp`'s
//!   arity is the *first* signature (`wrong # args: should be "oldp a"` for
//!   `proc p {a}`). A rename hands over the command object, so the two names
//!   are two different commands.
//! * `proc outer {} { proc p {} {return one}; p; proc p {a} {…} }` — the bare
//!   `p` returns `one`: a declaration that is a statement of the body now
//!   running is in genuine execution order, not covered by the
//!   load-before-body shortcut. That shortcut still holds for a declaration
//!   *outside* the body: `proc outer {} { later }` / `proc later {…}` prints
//!   `LATER`.
//!
//! Issue #1064, issue #1062's deferred B1/B2, and issue #923 differential
//! -audit findings idx 21 / 45 / 89 / 92.

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::definition::{LspRange, definition};
use tcl_lsp_core::references::references;

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl9.0")
}

fn start_lines(ranges: &[LspRange]) -> Vec<u32> {
    ranges.iter().map(|r| r.start_line).collect()
}

/// The reference count the code lens *displays* for `qname`.
///
/// Distinct from [`refs`] on purpose: `references()` dedupes by range, so a
/// call site recorded twice is invisible there, while `code_lenses` counts
/// `proc_reference_spans().len()` raw and would show it doubled (PR #1075
/// review, P2).
fn lens_count(source: &str, qname: &str) -> usize {
    let analysis = analyse(source);
    let title = tcl_lsp_core::code_lens::code_lenses(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        Some(&analysis),
        None,
        "",
    )
    .into_iter()
    .find(|lens| lens.qname == qname)
    .map_or_else(|| panic!("no lens for {qname}"), |lens| lens.command_title);
    // `reference_count_title` renders "N references" / "1 reference".
    title
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("unparsable lens title {title:?}"))
}

fn refs(source: &str) -> impl Fn(u32, u32) -> Vec<u32> + '_ {
    let analysis = analyse(source);
    move |line, character| {
        let mut lines = start_lines(&references(
            source,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            line,
            character,
            &analysis,
            true,
        ));
        lines.sort_unstable();
        lines
    }
}

// rename — go-to-definition (issue #1064)

#[test]
fn tp_definition_follows_rename_to_the_original_proc() {
    // TP: `hello` after `rename greet hello` runs `greet`'s body.
    let src = "proc greet {} { return hi }\nrename greet hello\nhello\n";
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 2, 1, &analysis)),
        vec![0],
        "the renamed-to name resolves to the original declaration"
    );
}

#[test]
fn tn_definition_declines_a_rename_written_after_the_call() {
    // TN: order gate. tclsh raises `invalid command name "hello"` for a call
    // that precedes the rename, so there is nothing to navigate to.
    let src = "proc greet {} { return hi }\nhello\nrename greet hello\n";
    let analysis = analyse(src);
    assert!(
        definition(src, 1, 1, &analysis).is_empty(),
        "a rename that has not run yet must not resolve the call"
    );
}

#[test]
fn tp_definition_follows_rename_inside_a_body_regardless_of_order() {
    // TP: the whole file loads before any body runs, so a rename written
    // after a proc that uses the new name is still in effect when that body
    // executes (tclsh 9.0.4/8.6.16: `caller` prints `hi`).
    let src = "proc greet {} { return hi }\nproc caller {} { hello }\nrename greet hello\ncaller\n";
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 1, 18, &analysis)),
        vec![0],
        "a body-internal call sees the file's renames"
    );
}

#[test]
fn tp_definition_follows_rename_to_a_class() {
    // TP: `rename Dog Cat` makes `Cat` the class command; the `ClassDef` is
    // unchanged (tclsh: `[Cat new]` builds a Dog and answers `bark`).
    let src = "oo::class create Dog {\n    method bark {} { return woof }\n}\nrename Dog Cat\nset d [Cat new]\n";
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 4, 8, &analysis)),
        vec![0],
        "the renamed class command resolves to the class declaration"
    );
}

// interp alias — go-to-definition (issue #923 idx 89)

#[test]
fn tp_definition_prefers_an_alias_over_the_proc_it_replaced() {
    // TP: the alias silently replaces the same-named proc, so the call
    // reaches `::tk::spinbox`, not the now-dead `::ttk::spinbox` body.
    let src = concat!(
        "namespace eval ::ttk {}\n",
        "namespace eval ::tk {}\n",
        "proc ::ttk::spinbox {w args} { }\n",
        "proc ::tk::spinbox {w args} { }\n",
        "interp alias {} ::ttk::spinbox {} ::tk::spinbox\n",
        "::ttk::spinbox .sb\n",
    );
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 5, 2, &analysis)),
        vec![3],
        "the call after the alias resolves to the alias target"
    );
}

#[test]
fn tn_definition_keeps_the_original_proc_before_the_alias_runs() {
    // TN: the same call written *before* the alias still reaches the
    // original proc — the mutation has not happened yet.
    let src = concat!(
        "namespace eval ::ttk {}\n",
        "namespace eval ::tk {}\n",
        "proc ::ttk::spinbox {w args} { }\n",
        "proc ::tk::spinbox {w args} { }\n",
        "::ttk::spinbox .early\n",
        "interp alias {} ::ttk::spinbox {} ::tk::spinbox\n",
    );
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 4, 2, &analysis)),
        vec![2],
        "the pre-alias call still reaches the original proc"
    );
}

#[test]
fn tn_definition_declines_an_argument_prepending_alias() {
    // TN: `interp alias {} withextra {} target pre` binds a leading argument,
    // so `withextra` is not the call `target` would be; navigation abstains
    // rather than pointing at a signature that does not describe it.
    let src = "proc target {a} { return $a }\ninterp alias {} withextra {} target pre\nwithextra\n";
    let analysis = analyse(src);
    assert!(
        definition(src, 2, 2, &analysis).is_empty(),
        "an alias with prepended arguments is not a navigable hop"
    );
}

// find-references across the mutation (issue #923 idx 21 / 89)

#[test]
fn tp_references_include_call_sites_spelled_through_an_alias() {
    // TP: both `[sayHi]` calls really execute `greet`, so a reference query
    // from `greet`'s declaration must list them.
    let src =
        "proc greet {} { return hi }\ninterp alias {} sayHi {} greet\nputs [sayHi]\nputs [sayHi]\n";
    assert_eq!(
        refs(src)(0, 6),
        vec![0, 1, 2, 3],
        "declaration, the alias target word, and both alias call sites"
    );
}

#[test]
fn tp_references_from_the_alias_name_offer_the_targets_sites() {
    // TP: the reverse direction — a query on the alias's own call site
    // resolves through to the target and answers the same unified set, the
    // convention rename/references already use for a renamed classmethod.
    let src =
        "proc greet {} { return hi }\ninterp alias {} sayHi {} greet\nputs [sayHi]\nputs [sayHi]\n";
    assert_eq!(
        refs(src)(2, 6),
        vec![0, 1, 2, 3],
        "the alias name resolves to the target's reference set"
    );
}

#[test]
fn tp_references_include_call_sites_spelled_through_a_rename() {
    let src = "proc greet {} { return hi }\nrename greet hello\nhello\n";
    assert_eq!(
        refs(src)(0, 6),
        vec![0, 1, 2],
        "declaration, the rename's own source word, and the renamed call"
    );
}

#[test]
fn tn_references_exclude_a_call_written_before_the_alias() {
    // TN: `sayHi` before the alias is `invalid command name` in tclsh, so it
    // is not a reference to `greet`.
    let src = "proc greet {} { return hi }\nputs [sayHi]\ninterp alias {} sayHi {} greet\n";
    assert_eq!(
        refs(src)(0, 6),
        vec![0, 2],
        "only the declaration and the alias target word"
    );
}

#[test]
fn tp_references_on_an_alias_target_reach_the_shadowing_call_site() {
    // TP: idx 89's other half — `::tk::spinbox`'s reference set must include
    // the `::ttk::spinbox` call the alias redirects onto it.
    let src = concat!(
        "namespace eval ::ttk {}\n",
        "namespace eval ::tk {}\n",
        "proc ::ttk::spinbox {w args} { }\n",
        "proc ::tk::spinbox {w args} { }\n",
        "interp alias {} ::ttk::spinbox {} ::tk::spinbox\n",
        "::ttk::spinbox .sb\n",
    );
    assert_eq!(
        refs(src)(3, 12),
        vec![3, 4, 5],
        "declaration, the alias target word, and the redirected call site"
    );
}

/// The other query direction of the same document — the one that was still
/// wrong after the go-to-definition half of idx 89 landed.
///
/// Oracle (tclsh 9.0.4 and 8.6.16, byte-identical): the script prints
/// `classic tk::spinbox: .sb -from 0 -to 100`, so `::ttk::spinbox`'s own
/// body never runs and the call on the last line is not one of its
/// references.
#[test]
fn tn_references_from_the_shadowed_original_exclude_the_redirected_call() {
    let src = ALIAS_SHADOW_SRC;
    assert_eq!(
        refs(src)(2, 12),
        vec![2],
        "only the shadowed declaration itself — the alias took the name over"
    );
}

/// The code lens on the shadowed proc must agree with Find All References:
/// zero call sites, not one.  A separate assertion because `references()`
/// dedupes by range while the lens counts `proc_reference_spans` raw.
#[test]
fn tn_lens_count_on_the_shadowed_original_is_zero() {
    assert_eq!(lens_count(ALIAS_SHADOW_SRC, "::ttk::spinbox"), 0);
    assert_eq!(
        lens_count(ALIAS_SHADOW_SRC, "::tk::spinbox"),
        2,
        "the alias target word and the redirected call site"
    );
}

/// Renaming the shadowed original must rewrite **only** its declaration.
///
/// Oracle: rewriting the call site too (what the edit set did before the
/// gate) turns `classic tk::spinbox: .sb` into `themed ttk::spinbox: .sb`
/// under both interpreters — the dead proc becomes live again under the new
/// name and the alias stops being consulted. Renaming just the header keeps
/// the original output.
#[test]
fn tn_rename_of_the_shadowed_original_does_not_touch_the_call_site() {
    let src = ALIAS_SHADOW_SRC;
    let analysis = analyse(src);
    let edits = tcl_lsp_core::rename::rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        2,
        12,
        "newname",
        &analysis,
        None,
    );
    assert_eq!(
        edits.iter().map(|e| e.range.start_line).collect::<Vec<_>>(),
        vec![2],
        "only the shadowed declaration is rewritten: {edits:?}"
    );
}

/// The symmetric direction stays whole: renaming the alias *target* rewrites
/// its declaration and the alias's own target word, and leaves the call
/// site's spelling (the alias's name) alone.
#[test]
fn tp_rename_of_the_alias_target_rewrites_the_declaration_and_alias_word() {
    let src = ALIAS_SHADOW_SRC;
    let analysis = analyse(src);
    let edits = tcl_lsp_core::rename::rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        3,
        12,
        "classicbox",
        &analysis,
        None,
    );
    assert_eq!(
        edits.iter().map(|e| e.range.start_line).collect::<Vec<_>>(),
        vec![3, 4],
        "declaration plus the alias's target word: {edits:?}"
    );
}

/// A `proc` written *after* an `interp alias` on the same name replaces the
/// alias, so the gate must not fire there.
///
/// Oracle (9.0.4 / 8.6.16): `proc target …; interp alias {} greet {} target;
/// proc greet …; greet` prints `greet body`.
#[test]
fn tn_a_proc_declared_after_the_alias_keeps_its_own_call_sites() {
    let src = concat!(
        "proc target {} { puts \"target body\" }\n",
        "interp alias {} greet {} target\n",
        "proc greet {} { puts \"greet body\" }\n",
        "greet\n",
    );
    assert_eq!(
        refs(src)(2, 6),
        vec![2, 3],
        "the later proc owns the name again, so the call is its reference"
    );
}

/// A mutation sitting in *another* proc's body may or may not ever run, so
/// the call site is not silently dropped from the reference (and rename)
/// set — abstaining toward keeping the edit whole.
#[test]
fn tn_an_alias_installed_in_another_body_does_not_drop_the_call_site() {
    let src = concat!(
        "proc target {} { }\n",
        "proc greet {} { }\n",
        "proc install {} { interp alias {} greet {} target }\n",
        "greet\n",
    );
    assert_eq!(
        refs(src)(1, 6),
        vec![1, 3],
        "a conditional alias does not prove the call misses `greet`"
    );
}

/// The idx 89 document, shared by the tests above so they all speak about
/// exactly the same program (`f8/alias_shadow.tcl`, run under both
/// interpreters).
const ALIAS_SHADOW_SRC: &str = concat!(
    "namespace eval ::ttk {}\n",
    "namespace eval ::tk {}\n",
    "proc ::ttk::spinbox {w args} { puts \"themed ttk::spinbox: $w $args\" }\n",
    "proc ::tk::spinbox {w args} { puts \"classic tk::spinbox: $w $args\" }\n",
    "interp alias {} ::ttk::spinbox {} ::tk::spinbox\n",
    "::ttk::spinbox .sb -from 0 -to 100\n",
);

// proc self-redefinition (issue #923 idx 45)

#[test]
fn tp_definition_between_two_declarations_reaches_the_first() {
    // TP: the in-between call runs the first definition (tclsh: `first`).
    let src = "proc p {} { return 1 }\np\nproc p {a} { return $a }\np x\n";
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 1, 0, &analysis)),
        vec![0],
        "the call between the two declarations reaches the first"
    );
    assert_eq!(
        start_lines(&definition(src, 3, 0, &analysis)),
        vec![2],
        "the call after both reaches the second"
    );
}

#[test]
fn tp_definition_on_a_superseded_header_stays_on_that_header() {
    // TP: a cursor on the displaced declaration's own name token resolves to
    // itself, not to its successor five lines down.
    let src = "proc p {} { return 1 }\nproc p {a} { return $a }\n";
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 0, 5, &analysis)),
        vec![0],
        "the superseded declaration is still a declaration"
    );
}

#[test]
fn tp_references_agree_from_either_declaration_of_a_redefined_proc() {
    // TP: both headers name one command, so both answer the same set.
    let src = "proc p {} { return 1 }\np\nproc p {a} { return $a }\np x\n";
    let query = refs(src);
    assert_eq!(query(0, 5), query(2, 5), "both headers agree");
    assert_eq!(
        query(0, 5),
        vec![1, 2, 3],
        "declaration plus both call sites"
    );
}

// Latest binding wins when a name carries both a rename and an alias
// (PR #1075 review, P2)

/// `proc a`, `proc b`, `rename a x`, `interp alias {} x {} b` — oracle
/// (tclsh 9.0.4 and 8.6.14): `x` returns `B`.  The alias ran last, so it is
/// what the slot holds; the earlier rename is dead.
const RENAME_THEN_ALIAS: &str = concat!(
    "proc a {} { return A }\n",
    "proc b {} { return B }\n",
    "rename a x\n",
    "interp alias {} x {} b\n",
    "x\n",
);

#[test]
fn tp_definition_prefers_the_later_alias_over_an_earlier_rename() {
    let analysis = analyse(RENAME_THEN_ALIAS);
    assert_eq!(
        start_lines(&definition(RENAME_THEN_ALIAS, 4, 0, &analysis)),
        vec![1],
        "`x` reaches `b`, the binding written last — not `a`'s rename"
    );
}

#[test]
fn tp_definition_uses_the_rename_while_it_is_still_the_latest_binding() {
    // The same document read *between* the two mutations: only the rename has
    // run there, so `x` is still `a` (tclsh: a call placed there prints `A`).
    let src = concat!(
        "proc a {} { return A }\n",
        "proc b {} { return B }\n",
        "rename a x\n",
        "x\n",
        "interp alias {} x {} b\n",
    );
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 3, 0, &analysis)),
        vec![0],
        "before the alias replaces it, the rename is the live binding"
    );
}

#[test]
fn tp_references_follow_the_latest_binding_not_the_first_map_read() {
    let query = refs(RENAME_THEN_ALIAS);
    assert_eq!(
        query(1, 5),
        vec![1, 3, 4],
        "`b` owns the post-alias `x` call site"
    );
    assert_eq!(
        query(0, 5),
        vec![0, 2],
        "`a` keeps only its declaration and the rename's own source word"
    );
}

#[test]
fn tp_alias_onto_a_live_command_still_resolves_when_written_first() {
    // The mirror order is not a reachable program — `rename a x` onto a live
    // `x` aborts with `can't rename to "x": command already exists` (tclsh
    // 9.0.4 and 8.6.14) — but the walk must still terminate and answer with
    // the latest binding rather than looping or preferring by map order.
    let src = concat!(
        "proc a {} { return A }\n",
        "proc b {} { return B }\n",
        "interp alias {} x {} b\n",
        "rename a x\n",
        "x\n",
    );
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 4, 0, &analysis)),
        vec![0],
        "the rename is the later binding of the two"
    );
}

// A rename moves the command object (PR #1075 review, P2)

/// Oracle (tclsh 9.0.4 and 8.6.14): `oldp` → `first`, `p` → `second`.  The
/// rename handed `oldp` the object `p` held *then*; the later `proc p` builds
/// a new, unrelated command under the vacated name.
const RENAME_THEN_REDEFINE: &str = concat!(
    "proc p {} { return first }\n",
    "rename p oldp\n",
    "proc p {} { return second }\n",
    "oldp\n",
    "p\n",
);

#[test]
fn tp_definition_through_a_rename_reaches_the_declaration_it_captured() {
    let analysis = analyse(RENAME_THEN_REDEFINE);
    assert_eq!(
        start_lines(&definition(RENAME_THEN_REDEFINE, 3, 0, &analysis)),
        vec![0],
        "`oldp` runs the definition the rename froze, not `p`'s successor"
    );
    assert_eq!(
        start_lines(&definition(RENAME_THEN_REDEFINE, 4, 0, &analysis)),
        vec![2],
        "`p` itself reaches the redefinition"
    );
}

#[test]
fn tn_references_do_not_merge_two_commands_split_by_a_rename() {
    // `oldp` and `p` are two different commands after the redefinition, so
    // the `oldp` call site is not a reference to the surviving `p`.
    let query = refs(RENAME_THEN_REDEFINE);
    assert!(
        !query(2, 5).contains(&3),
        "the `oldp` call belongs to the captured definition, not to `p`"
    );
    assert_eq!(
        query(2, 5),
        vec![1, 2, 4],
        "the rename's own source word, this header, and `p`'s own call"
    );
}

#[test]
fn tp_references_through_a_rename_without_a_redefinition_are_unaffected() {
    // TN guard for the identity check: with a single declaration there is
    // nothing to disambiguate, so idx 21's plain case still unifies.
    let src = "proc greet {} { return hi }\nrename greet hello\nhello\n";
    assert_eq!(refs(src)(0, 6), vec![0, 1, 2]);
}

// Same-body redefinition order (PR #1075 review, P2)

#[test]
fn tp_definition_inside_a_body_respects_that_bodys_own_redefinitions() {
    // Oracle (tclsh 9.0.4 and 8.6.14) for
    // `proc outer {} { proc p {} {return one}; p; proc p {a} {…} }`: the bare
    // `p` prints `one`.  The redefinitions are statements of the body that is
    // running, so they are in genuine execution order — the load-before-body
    // shortcut does not apply to them.
    let src = concat!(
        "proc outer {} {\n",
        "    proc p {} { return one }\n",
        "    p\n",
        "    proc p {a} { return two }\n",
        "}\n",
    );
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 2, 4, &analysis)),
        vec![1],
        "the in-between call reaches the declaration already executed"
    );
}

#[test]
fn tp_definition_inside_a_body_still_sees_a_later_top_level_declaration() {
    // TN for the same rule: a proc declared *outside* the body, after it, has
    // run by the time the body executes (tclsh: `outer` prints `LATER`).
    // This is what the load-before-body shortcut exists for.
    let src = concat!(
        "proc outer {} {\n",
        "    later\n",
        "}\n",
        "proc later {} { return LATER }\n",
        "outer\n",
    );
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 1, 4, &analysis)),
        vec![3],
        "a declaration outside the running body is already in effect"
    );
}

#[test]
fn tn_definition_declines_a_same_body_rename_written_after_the_call() {
    // The identical rule for the mutation timeline: a `rename` that is a
    // statement of the running body has not run yet at an earlier call in
    // that same body (tclsh: `invalid command name "x"`).
    let src = concat!(
        "proc a {} { return A }\n",
        "proc outer {} {\n",
        "    x\n",
        "    rename a x\n",
        "}\n",
    );
    let analysis = analyse(src);
    assert!(
        definition(src, 2, 4, &analysis).is_empty(),
        "a same-body rename below the call has not run yet"
    );
}

#[test]
fn tp_definition_follows_a_rename_from_another_bodys_statement() {
    // TN guard for the narrowing above: a rename inside a *different* body is
    // still leniently in effect — whether that body ever runs is not
    // statically decidable, and this is the pre-existing behaviour.
    let src = concat!(
        "proc a {} { return A }\n",
        "proc installer {} {\n",
        "    rename a x\n",
        "}\n",
        "proc caller {} {\n",
        "    x\n",
        "}\n",
    );
    let analysis = analyse(src);
    assert_eq!(
        start_lines(&definition(src, 5, 4, &analysis)),
        vec![0],
        "a rename in another body still resolves the call"
    );
}

// `namespace code [list X]` callbacks (issue #923 idx 92)

const TRACER: &str = concat!(
    "namespace eval ::demo {\n",
    "    variable S\n",
    "    proc Tracer {a b op} { }\n",
    "    proc Setup {} {\n",
    "        trace add variable S(size) write [namespace code [list Tracer]]\n",
    "    }\n",
    "}\n",
);

#[test]
fn tp_references_reach_a_namespace_code_list_wrapped_callback() {
    // TP: the trace really dispatches `::demo::Tracer`, so the wrapped word
    // is a call site of it.
    assert_eq!(
        refs(TRACER)(2, 10),
        vec![2, 4],
        "declaration plus the wrapped callback site"
    );
}

#[test]
fn tp_references_reach_a_namespace_code_bareword_callback() {
    // TP: the bare form is the same fact with no `[list]` around it.
    let src = concat!(
        "namespace eval ::demo {\n",
        "    variable S\n",
        "    proc Tracer {a b op} { }\n",
        "    proc Setup {} {\n",
        "        trace add variable S(size) write [namespace code Tracer]\n",
        "    }\n",
        "}\n",
    );
    assert_eq!(refs(src)(2, 10), vec![2, 4]);
}

#[test]
fn tn_namespace_code_does_not_double_count_a_braced_callback() {
    // TN/FP guard: the braced form is already walked as an ordinary script
    // body, so the command-prefix unwrap must not record it a second time.
    let src = concat!(
        "namespace eval ::demo {\n",
        "    variable S\n",
        "    proc Tracer {a b op} { }\n",
        "    proc Setup {} {\n",
        "        trace add variable S(size) write [namespace code {Tracer}]\n",
        "    }\n",
        "}\n",
    );
    assert_eq!(
        refs(src)(2, 10),
        vec![2, 4],
        "exactly one call site, not two"
    );
}

/// The bare, `[list]`-wrapped, and braced forms of the same callback, which
/// must all read as exactly one reference apiece.
fn tracer_source(callback: &str) -> String {
    format!(
        concat!(
            "namespace eval ::demo {{\n",
            "    variable S\n",
            "    proc Tracer {{a b op}} {{ }}\n",
            "    proc Setup {{}} {{\n",
            "        trace add variable S(size) write {callback}\n",
            "    }}\n",
            "}}\n",
        ),
        callback = callback
    )
}

#[test]
fn tp_lens_counts_each_namespace_code_callback_shape_once() {
    // The lens count is the raw span count, so a call site recorded by both
    // the body recursion and the command-prefix unwrap shows up as two
    // references for one callback (PR #1075 review, P2). One recorder per
    // shape: `[namespace code X]` and `[namespace code {X}]` are recorded by
    // the analyser's `ArgRole::Body` walk, `[namespace code [list X]]` — which
    // that walk's `has_substitution` guard stops at — by the unwrap.
    for callback in [
        "[namespace code Tracer]",
        "[namespace code {Tracer}]",
        "[namespace code [list Tracer]]",
    ] {
        assert_eq!(
            lens_count(&tracer_source(callback), "::demo::Tracer"),
            1,
            "one callback site, one reference, for {callback}"
        );
    }
}

#[test]
fn tn_namespace_code_with_a_dynamic_head_stays_unrecorded() {
    // TN: `[namespace code [list $cb]]` names no statically-known command;
    // recording it would invent a reference (and a W123) out of a runtime
    // value.
    let src = concat!(
        "namespace eval ::demo {\n",
        "    variable S\n",
        "    proc Tracer {a b op} { }\n",
        "    proc Setup {cb} {\n",
        "        trace add variable S(size) write [namespace code [list $cb]]\n",
        "    }\n",
        "}\n",
    );
    assert_eq!(
        refs(src)(2, 10),
        vec![2],
        "the declaration only — a dynamic head is not a reference"
    );
}
