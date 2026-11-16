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

//! Command names carried in variables and dispatch tables, [incr Tcl] class procedures and namespace-scoped class dispatch.

use super::*;

// Command names carried in variables / dispatch tables.

#[test]
fn references_include_a_const_cmd_dispatch_site_m7() {
    // tclsh: `proc target {} {..}; set cmd target; $cmd` runs `target` — the
    // `$cmd` head is a genuine (indirect) invocation of it, and the defining
    // literal in `set cmd target` is the writable reference feeding it.
    let src = "proc target {} { return hi }\nset cmd target\n$cmd\ntarget\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1, 2, 3],
        "decl + defining literal + $cmd dispatch + direct call expected; got {refs:?}",
    );
}

#[test]
fn rename_rewrites_the_defining_literal_and_never_the_dispatch_span_945() {
    // The non-corrupting rename rewrites the *defining constant's literal*
    // (`set cmd target` → `set cmd renamed`), never the `$cmd` head.
    // Leaving the literal stale produces Tcl that fails with
    // `invalid command name "target"` under tclsh 9.0.4.
    let src = "proc target {} { return hi }\nset cmd target\n$cmd\ntarget\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "renamed",
        &analysis,
        None,
    );
    assert_eq!(
        edit_lines(&edits),
        vec![0, 1, 3],
        "decl + defining literal + direct call; the $cmd span stays: {edits:?}",
    );
    let renamed_src = apply_edits(src, &edits);
    assert!(
        renamed_src.contains("set cmd renamed") && renamed_src.contains("$cmd"),
        "the literal follows the rename and the head is untouched:\n{renamed_src}",
    );
    // Execute the transformed output under a real Tcl — the renamed script
    // must still dispatch (this is exactly what the old edit set broke).
    if let Some(tclsh) = find_tclsh() {
        let script = format!("{renamed_src}puts [$cmd]\n");
        let outcome = tcl_test_support::run_script(&tclsh.path, script.as_bytes())
            .unwrap_or_else(|error| panic!("running Tcl oracle: {error}"));
        assert!(
            outcome.success(),
            "renamed output must execute cleanly under {}:\n{}\nstderr: {}",
            tclsh.path.display(),
            script,
            String::from_utf8_lossy(&outcome.stderr),
        );
        assert_eq!(
            String::from_utf8_lossy(&outcome.stdout).trim(),
            "hi",
            "the dispatch must still reach the renamed proc",
        );
    } else {
        eprintln!("skipping tclsh execution leg: no pinned Tcl 9.0/8.6 oracle is available");
    }
}

#[test]
fn rename_from_the_defining_literal_cursor_follows_the_dispatch_945() {
    // The cursor on the *literal* (`set cmd target`, on `target`) names the
    // same command — rename from there produces the identical edit set.
    let src = "proc target {} { return hi }\nset cmd target\n$cmd\ntarget\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        8,
        "renamed",
        &analysis,
        None,
    );
    assert_eq!(
        edit_lines(&edits),
        vec![0, 1, 3],
        "identical edit set from the literal cursor: {edits:?}",
    );
}

#[test]
fn rename_abstains_when_a_contributing_constant_has_no_writable_span_945() {
    // `foreach cmd {target other} { $cmd }` proves the value set finitely
    // (SCCP folds the loop variable to the element CONSTSET) but the list
    // elements carry no per-element writable span — an edit set could not
    // keep the dispatch alive, so the rename must abstain wholly rather
    // than corrupt.
    let src = "proc target {} { return hi }\nforeach cmd {target other} { $cmd }\ntarget\n";
    let analysis = analyse(src);
    let has_unsafe_indirect = analysis
        .command_invocations
        .iter()
        .any(|i| i.indirect && !i.rename_safe);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "renamed",
        &analysis,
        None,
    );
    if has_unsafe_indirect {
        assert!(
            edits.is_empty(),
            "an unwritable contributor must abstain the whole rename: {edits:?}",
        );
    } else {
        // The value oracle abstained entirely (no indirect reference at
        // all) — the rename then legitimately proceeds over the direct
        // sites only, and no stale-literal claim was ever made.
        assert_eq!(
            edit_lines(&edits),
            vec![0, 2],
            "direct sites only: {edits:?}"
        );
    }
}

#[test]
fn rename_rewrites_a_consumed_dispatch_table_literal_m7() {
    // The table value literal IS the written command name, so renaming the
    // proc rewrites the table entry too — keeping the dispatch alive.
    let src = "proc do_add {a b} { }\narray set ops {add do_add}\nset k add\n$ops($k) 1 2\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1],
        "decl + table literal expected; got {refs:?}",
    );
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "sum",
        &analysis,
        None,
    );
    assert_eq!(
        edit_lines(&edits),
        vec![0, 1],
        "decl + table literal rewritten: {edits:?}",
    );
    // Applying the line-1 edit yields a consistent table.
    let table_edit = edits
        .iter()
        .find(|e| e.range.start_line == 1)
        .expect("table edit");
    assert_eq!(table_edit.new_text, "sum");
}

// rename — [incr Tcl] `Factory::make` class-proc dispatch

/// TP: renaming an itcl class-scoped `proc` rewrites its declaration, its
/// bare sibling call, and the tail of every `::`-qualified dispatch — the
/// as-written qualifier is preserved (`Factory::make` → `Factory::produce`),
/// exactly as an ordinary qualified proc rename already does.
#[test]
fn rename_itcl_class_proc_rewrites_every_dispatch_spelling() {
    let src = concat!(
        "itcl::class Factory {\n",
        "    proc make {} { return 1 }\n",
        "    proc other {} { return [make] }\n",
        "}\n",
        "Factory::make\n",
        "::Factory::make\n",
    );
    let analysis = analyse(src);
    // Cursor on the declaration name.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        10,
        "produce",
        &analysis,
        None,
    );
    assert_eq!(edit_lines(&edits), vec![1, 2, 4, 5], "{edits:?}");
    assert_eq!(
        apply_edits(src, &edits),
        concat!(
            "itcl::class Factory {\n",
            "    proc produce {} { return 1 }\n",
            "    proc other {} { return [produce] }\n",
            "}\n",
            "Factory::produce\n",
            "::Factory::produce\n",
        ),
        "the qualifier must survive; only the member name changes"
    );
}

/// TP: the same rename triggered *from* a `::`-qualified call site reaches
/// the same set.
#[test]
fn rename_itcl_class_proc_from_the_call_site_matches_the_declaration() {
    let src = concat!(
        "itcl::class Factory {\n",
        "    proc make {} { return 1 }\n",
        "}\n",
        "Factory::make\n",
    );
    let analysis = analyse(src);
    let from_site = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        3,
        11,
        "produce",
        &analysis,
        None,
    );
    let from_decl = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        10,
        "produce",
        &analysis,
        None,
    );
    assert_eq!(edit_lines(&from_site), vec![1, 3], "{from_site:?}");
    assert_eq!(edit_lines(&from_site), edit_lines(&from_decl));
}

/// TN: an ordinary namespace-qualified proc of the same spelling is renamed
/// by the plain proc path and is unaffected by the class-proc resolver —
/// `Factory` here is a namespace, not a class.
#[test]
fn rename_plain_namespace_qualified_proc_is_unaffected() {
    let src = "namespace eval ::Factory {\n    proc make {} { return 1 }\n}\nFactory::make\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        3,
        11,
        "produce",
        &analysis,
        None,
    );
    assert_eq!(edit_lines(&edits), vec![1, 3], "{edits:?}");
    assert_eq!(
        apply_edits(src, &edits),
        "namespace eval ::Factory {\n    proc produce {} { return 1 }\n}\nFactory::produce\n"
    );
}

// rename — namespace-scoped bare class dispatch

/// Two classes sharing a simple name in different namespaces, one bare
/// `Factory make` dispatch each.  Oracle (tclsh 8.6.14 and 9.0.4): the
/// dispatch inside `::b` reaches `::b::Factory`, so renaming `::a`'s
/// classmethod must not rewrite it — before the fix it did, silently
/// corrupting an unrelated class's call site.
const TWO_NAMESPACE_FACTORIES: &str = concat!(
    "namespace eval ::a {\n",
    "    oo::class create Factory {\n",
    "        classmethod make {} { return 1 }\n",
    "    }\n",
    "    Factory make\n",
    "}\n",
    "namespace eval ::b {\n",
    "    oo::class create Factory {\n",
    "        classmethod make {} { return 2 }\n",
    "    }\n",
    "    Factory make\n",
    "}\n",
);

/// TN: renaming `::a`'s classmethod rewrites only `::a`'s own sites.
#[test]
fn rename_classmethod_does_not_rewrite_a_sibling_namespaces_dispatch() {
    let src = TWO_NAMESPACE_FACTORIES;
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        2,
        20,
        "produce",
        &analysis,
        None,
    );
    assert_eq!(edit_lines(&edits), vec![2, 4], "{edits:?}");
    assert_eq!(
        apply_edits(src, &edits),
        concat!(
            "namespace eval ::a {\n",
            "    oo::class create Factory {\n",
            "        classmethod produce {} { return 1 }\n",
            "    }\n",
            "    Factory produce\n",
            "}\n",
            "namespace eval ::b {\n",
            "    oo::class create Factory {\n",
            "        classmethod make {} { return 2 }\n",
            "    }\n",
            "    Factory make\n",
            "}\n",
        ),
        "`::b`'s class and its dispatch must be untouched"
    );
}

/// TN, the mirror direction: renaming `::b`'s classmethod leaves `::a` alone.
#[test]
fn rename_classmethod_is_scoped_in_the_other_direction_too() {
    let src = TWO_NAMESPACE_FACTORIES;
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        8,
        20,
        "produce",
        &analysis,
        None,
    );
    assert_eq!(edit_lines(&edits), vec![8, 10], "{edits:?}");
}
