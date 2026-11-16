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

//! Command-position candidates: procedures, built-ins, `call` targets, workspace procedures and the fuzzy fallback.

use super::*;

#[test]
fn proc_completion_lists_user_defined_procs() {
    let src = "proc greet {} {}\nproc shout {} {}\ng\n";
    let analysis = analyse(src);
    // Cursor right after `g` on third line.
    let items = completions(
        src,
        2,
        1,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0].kind, CompletionKind::Function);
    assert_eq!(items[0].label, "greet");
}

// usage-bucket sort-text

#[test]
fn usage_bucket_thresholds() {
    // Exhaustive bucket-table check across every threshold.
    assert_eq!(usage_bucket(0), 5);
    assert_eq!(usage_bucket(1), 4);
    assert_eq!(usage_bucket(2), 4);
    assert_eq!(usage_bucket(3), 3);
    assert_eq!(usage_bucket(7), 3);
    assert_eq!(usage_bucket(8), 2);
    assert_eq!(usage_bucket(19), 2);
    assert_eq!(usage_bucket(20), 1);
    assert_eq!(usage_bucket(49), 1);
    assert_eq!(usage_bucket(50), 0);
    assert_eq!(usage_bucket(1000), 0);
}

#[test]
fn proc_sort_text_has_lower_bucket_for_used_procs() {
    // Three calls to `greet` and zero calls to `shout`.
    // The completion-list partial is empty (cursor on
    // line 6 col 0), so both procs surface — `greet` in
    // bucket 3 (≥3 calls) and `shout` in bucket 5 (no
    // calls).
    let src = "proc greet {} {}\nproc shout {} {}\ngreet\ngreet\ngreet\n\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        5,
        0,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let greet = items
        .iter()
        .find(|i| i.label == "greet")
        .unwrap_or_else(|| panic!("greet missing from {items:?}"));
    let greet_sort = greet.sort_text.as_deref().unwrap_or("");
    assert!(
        greet_sort.starts_with("A003_"),
        "greet sort_text {greet_sort:?} should land in bucket 3",
    );
    let shout = items
        .iter()
        .find(|i| i.label == "shout")
        .unwrap_or_else(|| panic!("shout missing from {items:?}"));
    let shout_sort = shout.sort_text.as_deref().unwrap_or("");
    assert!(
        shout_sort.starts_with("A005_"),
        "shout sort_text {shout_sort:?} should land in bucket 5",
    );
    // Lower bucket sorts first.
    assert!(greet_sort < shout_sort);
}

#[test]
fn empty_partial_lists_all_procs() {
    let src = "proc alpha {} {}\nproc beta {} {}\n\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        2,
        0,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"alpha"));
    assert!(labels.contains(&"beta"));
}

// built-in command completion
//
// Tests pin the contract that a non-`None` registry parameter
// extends proc-name completion with every command the
// registry knows about, filtered by the partial typed at the
// cursor.

#[test]
fn builtin_completion_lists_registry_commands_at_command_position() {
    // No user-defined procs, partial `pu` → `puts` (and any
    // other registered command beginning with `pu`) should
    // surface.
    let src = "pu\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        2,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"puts"),
        "expected `puts` from registry; got {labels:?}",
    );
}

#[test]
fn builtin_completion_detail_shows_provenance() {
    // A core built-in shows `built-in`; a stdlib command shows its
    // `stdlib (PKG)` provenance from the registry `required_package`.
    let src = "pu\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        2,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let puts = items.iter().find(|i| i.label == "puts").expect("puts");
    assert_eq!(puts.detail.as_deref(), Some("built-in"), "{puts:?}");

    // `http::geturl` is a stdlib command requiring the `http` package.
    let src = "http::ge\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        0,
        8,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    if let Some(geturl) = items.iter().find(|i| i.label == "http::geturl") {
        assert_eq!(
            geturl.detail.as_deref(),
            Some("stdlib (http)"),
            "{geturl:?}"
        );
    }
}

#[test]
fn command_detail_formats_each_provenance() {
    let reg = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    // built-in: no package.
    assert_eq!(command_detail(reg.get("puts").unwrap(), reg), "built-in");
    // stdlib: required_package set.
    if let Some(spec) = reg.get("http::geturl") {
        assert_eq!(command_detail(spec, reg), "stdlib (http)");
    }
    // An ambient vendor surface reads as part of the runtime, never as
    // a require-gated stdlib package (§7.1 axis C). The registry is what
    // is asked, so a pack's `ambient_package` row would read the same way.
    let ireg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let http2 = ireg.get("HTTP2::header").expect("HTTP2::header spec");
    assert_eq!(command_detail(http2, ireg), "built-in");
}

#[test]
fn builtin_completion_filters_by_partial() {
    // Partial `whi` should yield `while` but not unrelated
    // commands like `puts`.
    let src = "whi\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"while"),
        "expected `while`; got {labels:?}"
    );
    assert!(
        !labels.contains(&"puts"),
        "should NOT contain `puts` for partial `whi`; got {labels:?}",
    );
}

#[test]
fn builtin_completion_skips_math_operators() {
    // The registry registers `+` / `-` / `*` / etc. as
    // commands (Tcl 9 `tcl::mathop`), but they don't make
    // sense as completion items at a command position.
    let src = "+\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        1,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    for op in &["+", "-", "*", "/", ">", ">=", "<", "<=", "==", "!="] {
        assert!(
            !labels.contains(op),
            "math operator `{op}` should be filtered out; got {labels:?}",
        );
    }
}

/// `SKIP_BUILTIN_NAMES` must cover the word-form mathop commands
/// (`eq`/`ne`/`in`/`ni`, and the 9.0+ `lt`/`le`/`gt`/`ge`) as well as the
/// 10 symbolic operators; otherwise they leak through as a nonsensical
/// "did you mean `eq`?" at a command position.
#[test]
fn builtin_completion_skips_word_form_operators() {
    let src = "\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        0,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    for op in &["eq", "ne", "in", "ni", "lt", "le", "gt", "ge"] {
        assert!(
            !labels.contains(op),
            "word-form operator `{op}` should be filtered out; got {labels:?}",
        );
    }
}

#[test]
fn builtin_completion_none_registry_keeps_minimal_behaviour() {
    // Passing `None` for the registry must not regress the
    // minimal port's behaviour — proc-name completion alone.
    let src = "proc helper {} {}\nhe\n";
    let analysis = analyse(src);
    let items_no_registry = completions(
        src,
        1,
        2,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels_no_registry: Vec<&str> =
        items_no_registry.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels_no_registry.contains(&"helper"),
        "expected `helper` proc; got {labels_no_registry:?}",
    );
    // No registry commands surface without the registry.
    assert_eq!(items_no_registry.len(), 1, "{items_no_registry:?}");
}

#[test]
fn builtin_completion_merges_procs_and_registry() {
    // Both a user-defined proc and built-in commands should
    // surface when both apply.
    let src = "proc parade {} {}\npar\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"parade"),
        "expected user proc `parade`; got {labels:?}",
    );
    // `parray` is a stdlib command starting with `par`.
    assert!(
        labels.contains(&"parray"),
        "expected built-in `parray`; got {labels:?}",
    );
}

#[test]
fn builtin_completion_skipped_inside_variable_trigger() {
    // Variable trigger (`$par`) must take precedence and
    // suppress built-in command completions even when a
    // registry is supplied, via the `variable_completions`
    // short-circuit at the top of `completions`.
    let src = "set apple 1\nset $par\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
        8,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    // Only variable completions allowed here.
    for it in &items {
        assert_eq!(
            it.kind,
            CompletionKind::Variable,
            "variable trigger should suppress built-ins; got {it:?}",
        );
    }
}

#[test]
fn command_completion_gates_commands_by_package_version() {
    // `ttk::button` needs Tk 8.5. On a tcl8.4 host the shipped Tk is
    // 8.4 (the §7.1 TracksBase pin) and a require cannot raise it —
    // not offered. On a tcl8.6 host the shipped Tk is 8.6 even when
    // the file writes `package require Tk 8.4` (a minimum, not a
    // downgrade) — offered; the old require-only floor wrongly hid it.
    let registry = CommandRegistry::build_default();
    let older = "package require Tk 8.4\nttk::b\n";
    let a1 = analyse_for(older, "tcl8.4");
    let l1: Vec<String> = completions(
        older,
        1,
        6,
        &a1,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        !l1.iter().any(|l| l == "ttk::button"),
        "a tcl8.4 host (Tk 8.4) must not offer ttk::button: {l1:?}",
    );

    let a1_86 = analyse_for(older, "tcl8.6");
    let l1_86: Vec<String> = completions(
        older,
        1,
        6,
        &a1_86,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        l1_86.iter().any(|l| l == "ttk::button"),
        "a tcl8.6 host ships Tk 8.6; `require Tk 8.4` does not downgrade it: {l1_86:?}",
    );

    let newer = "package require Tk 8.5\nttk::b\n";
    let a2 = analyse_for(newer, "tcl8.6");
    let l2: Vec<String> = completions(
        newer,
        1,
        6,
        &a2,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        l2.iter().any(|l| l == "ttk::button"),
        "Tk 8.5 must offer ttk::button: {l2:?}",
    );
}

// iRules `call PROC_NAME`
//
// When the cursor sits at word-index 1 of a command carrying
// `Traits::INVOKES_USER_PROC` (today only the iRules `call`
// command), the completion surface lists user-defined proc
// names and excludes built-in commands.

#[test]
fn call_completion_surfaces_user_procs_at_word_index_1() {
    // Two user procs starting with `he`, plus one that
    // doesn't.  Built-in `puts` starts with `p` so won't
    // surface against `he` either way — but we use it as
    // additional cover.
    let src = "proc helper {} {}\nproc help_inner {} {}\nproc unrelated {} {}\ncall he\n";
    let analysis = analyse_for(src, "f5-irules");
    let registry = irules_registry();
    let items = completions(
        src,
        3,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"helper") && labels.contains(&"help_inner"),
        "expected user procs `helper` and `help_inner`; got {labels:?}",
    );
    assert!(
        !labels.contains(&"unrelated"),
        "should filter on partial `he`; got {labels:?}",
    );
    // Every result must be a user proc — built-in commands
    // are excluded from the `call` context.
    for it in &items {
        assert_eq!(
            it.kind,
            CompletionKind::Function,
            "every call-completion item should be a Function; got {it:?}",
        );
    }
}

#[test]
fn call_completion_does_not_fire_in_plain_tcl_dialect() {
    // Plain Tcl registry has no `call` spec, so `help` is an argument of
    // an unknown command: data, where no command name is offered.
    let src = "proc helper {} {}\ncall help\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
        9,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"helper"),
        "the iRules `call` completion must not fire; got {labels:?}",
    );
}

#[test]
fn call_completion_excludes_builtin_commands() {
    // Built-in `puts` starts with `p`.  In a `call p`
    // context, the completion list should contain user
    // procs starting with `p` but never `puts`.
    let src = "proc parade {} {}\ncall p\n";
    let analysis = analyse_for(src, "f5-irules");
    let registry = irules_registry();
    let items = completions(
        src,
        1,
        6,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"parade"), "{labels:?}");
    assert!(
        !labels.contains(&"puts"),
        "`puts` is a built-in; should not surface in `call` context: {labels:?}",
    );
}

#[test]
fn tk_commands_only_offered_when_tk_is_loaded() {
    let registry = CommandRegistry::build_default();

    // Plain `.tcl` (tcl8.6) with no `package require Tk`: Tk widget
    // commands must NOT be offered, even though the registry knows them.
    let src = "butt\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        0,
        4,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"button"),
        "Tk `button` must not surface without `package require Tk`: {labels:?}",
    );

    // Once `package require Tk` is declared, `button` becomes available.
    let src = "package require Tk\nbutt\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        1,
        4,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"button"),
        "Tk `button` should surface after `package require Tk`: {labels:?}",
    );
}

#[test]
fn tk_commands_offered_in_wish_tk_dialect() {
    // A `wish`-labelled document (`tk` dialect) is implicitly Tk-loaded.
    let registry = CommandRegistry::build_default();
    let src = "butt\n";
    let analysis = analyse_for(src, "tk");
    let items = completions(
        src,
        0,
        4,
        &analysis,
        Some(&registry),
        None,
        crate::profile_for_dialect("tk"),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"button"),
        "Tk `button` should surface in the `tk` dialect: {labels:?}",
    );
}

/// The completion gate reads two placement queries: Tk names are offered where
/// Tk is in the world (ambient, or required here) **and** the
/// environment declares a Tk placement at all. The `wish` alias must
/// answer identically to `tk` — it is the same environment row — and a
/// vendor shell must refuse even with an explicit require.
#[test]
fn tk_completion_follows_the_placement_not_the_environment_name() {
    let registry = CommandRegistry::build_default();
    let offers = |src: &str, line: u32, character: u32, dialect: &str| {
        let analysis = analyse_for(src, dialect);
        completions(
            src,
            line,
            character,
            &analysis,
            Some(&registry),
            None,
            crate::profile_for_dialect(dialect),
        )
        .iter()
        .any(|item| item.label == "button")
    };
    // Ambient: `wish` under either spelling of its identity.
    assert!(offers("butt\n", 0, 4, "tk"), "tk");
    assert!(offers("butt\n", 0, 4, "wish"), "wish");
    // Hosted: offered only once this document requires it.
    assert!(!offers("butt\n", 0, 4, "tcl8.6"), "tcl8.6 bare");
    assert!(!offers("butt\n", 0, 4, "tcl"), "lenient sink bare");
    assert!(
        offers("package require Tk\nbutt\n", 1, 4, "tcl8.6"),
        "tcl8.6 with a require"
    );
    // Unhostable: a vendor shell declares no Tk placement, so a stray
    // require changes nothing.
    for shell in ["f5-iapps", "xilinx-eda-tcl", "bpf"] {
        assert!(
            !offers("package require Tk\nbutt\n", 1, 4, shell),
            "{shell}"
        );
    }
}

#[test]
fn tk_commands_never_offered_in_irules() {
    // Even a stray `package require Tk` cannot make Tk valid in iRules —
    // the command is dialect-gated out.
    let registry = CommandRegistry::build_default();
    let src = "package require Tk\nbutt\n";
    let analysis = analyse_for(src, "f5-irules");
    let items = completions(
        src,
        1,
        4,
        &analysis,
        Some(&registry),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"button"),
        "Tk `button` must never surface in iRules: {labels:?}",
    );
}

#[test]
fn call_completion_skipped_at_word_index_other_than_1() {
    // `call helper extra` — word-index 2 is an argument of the called
    // procedure: data, where neither procedures nor built-ins are offered.
    let src = "proc helper {} {}\ncall helper e\n";
    let analysis = analyse_for(src, "f5-irules");
    let registry = irules_registry();
    let items = completions(
        src,
        1,
        14,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.is_empty(),
        "no command name belongs at word-index 2; got {labels:?}",
    );
}

// workspace-index: cross-document proc completion

#[test]
fn workspace_procs_surface_in_completion() {
    use crate::workspace_index::WorkspaceIndex;
    // Current doc defines `local_proc`; a sibling doc
    // defines `shared_helper`.  Completing `s` should
    // surface the cross-document proc.
    let cur_src = "proc local_proc {} {}\ns\n";
    let cur = analyse(cur_src);
    let other = analyse("proc shared_helper {} {}\n");
    let index =
        WorkspaceIndex::from_documents([("file:///cur.tcl", &cur), ("file:///other.tcl", &other)]);
    let items = completions(
        cur_src,
        1,
        1,
        &cur,
        None,
        Some(&index),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"shared_helper"),
        "expected cross-doc proc; got {labels:?}",
    );
    // It's tagged as a workspace proc in the detail.
    let helper = items.iter().find(|i| i.label == "shared_helper").unwrap();
    assert!(
        helper.detail.as_deref().unwrap_or("").contains("workspace"),
        "{helper:?}",
    );
    assert!(helper.sort_text.as_deref().unwrap_or("").starts_with("C0_"));
}

#[test]
fn workspace_procs_do_not_duplicate_local_procs() {
    use crate::workspace_index::WorkspaceIndex;
    // Both the current doc and the index contain `greet`.
    // The result must list `greet` once (the local entry).
    let cur_src = "proc greet {} {}\ngr\n";
    let cur = analyse(cur_src);
    let index = WorkspaceIndex::from_documents([("file:///cur.tcl", &cur)]);
    let items = completions(
        cur_src,
        1,
        2,
        &cur,
        None,
        Some(&index),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let count = items.iter().filter(|i| i.label == "greet").count();
    assert_eq!(count, 1, "{items:?}");
}

#[test]
fn workspace_none_keeps_single_doc_behaviour() {
    let cur_src = "proc greet {} {}\ngr\n";
    let cur = analyse(cur_src);
    let items = completions(
        cur_src,
        1,
        2,
        &cur,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["greet"], "{labels:?}");
}

// Pinned prefix lists — the byte-identical contract for fragments
// that match something.

#[test]
fn prefix_proc_list_pinned() {
    let src = "proc alpha {} {}\nproc beta {} {}\nal\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        2,
        2,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["alpha"]);
    assert_eq!(items[0].insert_text, "::alpha");
    assert_eq!(
        items[0].filter_text, None,
        "prefix matches stay undecorated"
    );
}

// Fuzzy fallback behaviour through the public entry point.

#[test]
fn fuzzy_fallback_offers_lsearch_for_lsaerch() {
    let src = "lsaerch\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let lsearch = items
        .iter()
        .find(|i| i.label == "lsearch")
        .unwrap_or_else(|| panic!("expected fuzzy lsearch, got {items:?}"));
    // Decorated so editors neither hide nor re-order the item.
    assert_eq!(lsearch.filter_text.as_deref(), Some("lsaerch"));
    assert_eq!(lsearch.sort_text.as_deref(), Some("F00_lsearch"));
}

#[test]
fn fuzzy_fallback_skips_one_char_fragment() {
    // `q` prefix-matches no command and is below the fragment
    // minimum — the response stays empty rather than fuzzing.
    let src = "q\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        1,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(items.is_empty(), "got {items:?}");
}

#[test]
fn proc_completion_quotes_retained_callable_source() {
    let source = "proc {odd name} {} {}";
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, engine);
        let items = proc_completions(&analysis, "odd", &FxHashMap::default());
        assert_eq!(items.len(), 1, "{engine}: {items:?}");
        assert_eq!(items[0].insert_text, "\"::odd name\"", "{engine}");
    }
}

#[test]
fn proc_completion_keeps_independent_original_inventory_when_ui_geometry_is_removed() {
    let mut analysis = analyse("proc greet {} {}");
    analysis.all_procs.get_mut("::greet").unwrap().source_name = None;
    assert_eq!(
        proc_completions(&analysis, "gre", &FxHashMap::default()).len(),
        1
    );
    analysis.original_procedure_metadata.clear();
    assert_eq!(
        proc_completions(&analysis, "gre", &FxHashMap::default()),
        [] as [crate::completion::CompletionItem; 0]
    );
}

#[test]
fn proc_completion_withdraws_unaddressable_colon_publication() {
    let source = "namespace eval N {proc :f {} {return LOCAL}}";
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, engine);
        assert!(analysis.all_procs.contains_key("::N:::f"), "{engine}");
        assert!(
            proc_completions(&analysis, ":f", &FxHashMap::default()).is_empty(),
            "{engine}"
        );
    }
}

#[test]
fn invoked_proc_completions_fuzzy_falls_back() {
    let src = "proc greet {} {}\nproc shout {} {}\n";
    let analysis = analyse(src);
    // Prefix path unchanged…
    let items = invoked_proc_completions(&analysis, "gre");
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["greet"]);
    assert!(items[0].filter_text.is_none());
    // …and the typo'd fragment falls back to the closest proc.
    let items = invoked_proc_completions(&analysis, "gret");
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["greet"]);
    assert_eq!(items[0].filter_text.as_deref(), Some("gret"));
}

#[test]
fn command_completion_uses_profile_availability() {
    let src = "dic\n";
    let analysis = analyse_for(src, "f5-iapps");
    // iApps ride the `f5-tcl` trunk — a fork of Tcl at 8.4.6
    // (measured, bigip-irule-parser-measurements.md §4a: `dict` fails
    // in `IAppImplementation`) — so `dict` is NOT offered there…
    let registry = tcl_registry::model::ingress::static_context_for("f5-iapps").commands();
    let items = completions(
        src,
        0,
        3,
        &analysis,
        Some(registry),
        None,
        tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
    );
    assert!(
        !items.iter().any(|i| i.label == "dict"),
        "dict must not be offered under f5-iapps (8.4.6 trunk, measured)"
    );
    // …while the banned iRules commands never surface there.
    let irules_reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let exec_src = "exe\n";
    let exec_analysis = analyse_for(exec_src, "f5-irules");
    let irules_items = completions(
        exec_src,
        0,
        3,
        &exec_analysis,
        Some(irules_reg),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    assert!(
        !irules_items.iter().any(|i| i.label == "exec"),
        "exec is banned in iRules and must not be completed"
    );
}
