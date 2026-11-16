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

use super::*;

#[test]
fn counted_class_inventory_alone_requires_original_owner() {
    let source = "oo::class create C\\uD800 {}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let counted = analysis
        .original_counted_class_metadata
        .first()
        .unwrap()
        .clone();
    let mut shell = Analyser::new();
    let commands = shell.per_item_setup("proc f {} {}", "tcl8.6");
    shell.defer_proc_bodies = true;
    shell.walk_commands_top_level(&commands, false);
    let body = shell.deferred_bodies.pop().unwrap();
    let mut fragment = analyse_proc_body_isolated(
        &body,
        "tcl8.6",
        &std::collections::HashSet::<String>::new(),
        super::super::state::NonAsciiMode::Off,
        None,
        None,
        None,
    );
    assert!(!fragment_needs_original_source_owner(&fragment));
    fragment
        .result
        .original_counted_class_metadata
        .push(counted);
    assert!(fragment_needs_original_source_owner(&fragment));
}

#[test]
fn counted_class_metadata_requires_original_owner_in_per_item_body() {
    let source = "proc f {} {oo::class create C\\uD800 {}}";
    let full = Analyser::new().analyse(source, "tcl8.6");
    let partial = Analyser::new().analyse_per_item(source, "tcl8.6");
    assert_eq!(full.original_counted_class_metadata.len(), 1);
    crate::analyser::types::assert_analysis_results_eq(&partial, &full, "counted class owner");
    for record in &partial.original_counted_class_metadata {
        assert_eq!(
            record.name_input().source_image().bytes(),
            source.as_bytes()
        );
    }
}

/// Proven-word findings inside a procedure body reach the per-item path
/// as they reach the whole-file one (#2294).
#[test]
fn per_item_body_keeps_proven_word_findings() {
    for (src, code) in [
        (
            "proc p {} {\n set l {a b c}\n lindex $l 9\n}\n",
            tcl_core_types::DiagCode::W230,
        ),
        (
            "proc p {input} {set re {(a+)+$}; regexp $re $input}\n",
            tcl_core_types::DiagCode::W303,
        ),
    ] {
        let codes = |result: &AnalysisResult| {
            result
                .diagnostics
                .iter()
                .filter(|d| d.code == code)
                .map(|d| d.span)
                .collect::<Vec<_>>()
        };
        let want = codes(&Analyser::new().analyse(src, "tcl8.6"));
        assert_eq!(want.len(), 1, "{code}: {src}");
        assert_eq!(
            codes(&Analyser::new().analyse_per_item(src, "tcl8.6")),
            want,
            "{code}: {src}"
        );
    }
}

fn eq(src: &str) {
    let want = Analyser::new().analyse(src, "tcl8.6");
    let got = Analyser::new().analyse_per_item(src, "tcl8.6");
    crate::analyser::types::assert_analysis_results_eq(
        &got,
        &want,
        &format!("per_item != analyse for:\n{src}"),
    );
}

#[test]
fn original_incremental_callbacks_retain_full_source_targets_and_barriers() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    // Incremental and full consumers share the genuine registration-horizon
    // issuer. Empty bodies keep this control on the incremental fast path;
    // no isolated fragment can donate a target or suppress a known barrier.
    use crate::command_binding::OriginalSourceCallbackProcedureRefusal::KnownSourceBarrier;
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for (source, local, barrier) in [
            ("lsort -command {cb BAKED} {3 1 2}", false, false),
            (
                "interp alias {} cb {} external FIXED; lsort -command {cb BAKED} {3 1 2}",
                false,
                false,
            ),
            (
                "proc cb {left right} {}; lsort -command cb {3 1 2}",
                true,
                false,
            ),
            (
                "proc cb {left right} {}; rename cb {}; lsort -command cb {3 1 2}",
                false,
                true,
            ),
            (
                "interp alias {} cb {} list; lsort -command cb {3 1 2}",
                false,
                true,
            ),
        ] {
            let (gate, incremental) = gate_and_result(source, dialect);
            assert_eq!(
                gate, None,
                "{dialect}: fast source-header control: {source}"
            );
            let lookup = incremental
                .command_invocations
                .iter()
                .filter_map(|row| row.original_callback_signature_lookup.as_ref())
                .find(|lookup| lookup.prefix().name_input().bytes() == b"cb")
                .unwrap_or_else(|| panic!("{dialect}: missing original callback lookup: {source}"));
            assert_eq!(lookup.declaration().is_some(), local, "{dialect}: {source}");
            assert_eq!(
                lookup.original().refusal() == Some(KnownSourceBarrier),
                barrier,
                "{dialect}: {source}"
            );
            assert_ne!(lookup.original().obligations(), []);
            if let Some(target) = lookup.original().target() {
                assert_eq!(
                    target.captured_arguments().len(),
                    usize::from(source.contains("external FIXED"))
                );
            }
            assert_eq!(
                lookup.prefix().baked_argument_count(),
                usize::from(source.contains("BAKED"))
            );
            let complete = Analyser::new().analyse(source, dialect);
            let complete_lookup = complete
                .command_invocations
                .iter()
                .filter_map(|row| row.original_callback_signature_lookup.as_ref())
                .find(|lookup| lookup.prefix().name_input().bytes() == b"cb")
                .expect("full source owner supplies the same target/refusal");
            assert_eq!(lookup, complete_lookup, "{dialect}: {source}");
        }
    }
}

/// One `analyse_per_item` run, reported as *both* the gate it tripped and
/// the result it produced.
///
/// The gate and the result come from the same pass deliberately.  Asking
/// for them separately — one `analyse_per_item` for the gate, a second for
/// the result — analysed every fixture twice for no extra coverage: the two
/// runs are the same pure function of `(src, dialect)`, so the second can
/// only ever agree with the first.  On the oversized-body fixtures below
/// that redundancy was minutes per test.
fn gate_and_result(src: &str, dialect: &str) -> (Option<PerItemFallback>, AnalysisResult) {
    let mut a = Analyser::new();
    let got = a.analyse_per_item(src, dialect);
    (a.per_item_fallback, got)
}

/// Assert that `src` reaches the incremental fast path — i.e. no gate
/// fired.  Also asserts byte-identity with the whole-file walk, so a
/// widened gate can never buy speed with a wrong answer.
fn fast_path(src: &str) {
    let (gate, got) = gate_and_result(src, "tcl8.6");
    assert_eq!(gate, None, "expected fast path for:\n{src}");
    let want = Analyser::new().analyse(src, "tcl8.6");
    crate::analyser::types::assert_analysis_results_eq(
        &got,
        &want,
        &format!("per_item != analyse for:\n{src}"),
    );
}

/// Assert that `src` hands off to a full walk with `reason`, and that the
/// handed-off result still matches the whole-file walk exactly.
fn falls_back(src: &str, dialect: &str, reason: PerItemFallback) {
    let (gate, got) = gate_and_result(src, dialect);
    assert_eq!(gate, Some(reason), "gate for:\n{src}");
    let want = Analyser::new().analyse(src, dialect);
    crate::analyser::types::assert_analysis_results_eq(
        &got,
        &want,
        &format!("per_item != analyse for:\n{src}"),
    );
}

// Body-defined global cells.

/// The Tk `SetFocusGrab` / `RestoreFocusGrab` shape: one proc names a
/// fixed global array cell through `upvar`'s `otherVar` word, a sibling
/// proc reaches the same cell by its own qualified spelling.
///
/// The cell is defined *inside a deferred body*, so the isolated pass
/// wrote it into that body's throwaway root scope and the graft — which
/// merges the fragment's **proc** scope only — dropped it.  The
/// whole-file walk left it in the document's global scope, which is what
/// every navigation provider and `attach_qualified_var_references` read,
/// so hover / go-to-definition answered in the compiler library and
/// nothing at all in the live server.  `fast_path` pins both halves:
/// the document still takes the incremental path, and the result is
/// byte-identical to `analyse`.
#[test]
fn a_body_defined_global_cell_survives_the_graft() {
    fast_path(
        "proc SetFocusGrab {grab focus} {\n    set index \"$grab,$focus\"\n    \
             upvar ::tk::FocusGrab($index) data\n    lappend data one\n}\n\
             proc RestoreFocusGrab {grab focus} {\n    set index \"$grab,$focus\"\n    \
             if {[info exists ::tk::FocusGrab($index)]} {\n        \
             set data2 $::tk::FocusGrab($index)\n        \
             unset ::tk::FocusGrab($index)\n    }\n}\n",
    );
}

/// The `#0`-level spelling of the same fact: `upvar #0 counter c` names
/// the global `::counter` from any depth, so the cell must reach the
/// shell's global scope too.
#[test]
fn a_body_defined_hash_zero_global_survives_the_graft() {
    fast_path("proc bump {} { upvar #0 counter c; incr c }\nproc show {} { puts $::counter }\n");
}

/// TN — a body-local `upvar` whose target is frame-relative names no
/// fixed cell, so nothing is captured and the graft is unchanged.
#[test]
fn a_caller_frame_upvar_defines_no_global_cell() {
    fast_path("proc setdef {d} { upvar 1 $d dst; set dst 1 }\nproc build {} { setdef opts }\n");
}

// Tk activation gate.  The gate decides whether a document
// abandons per-body memoisation for a whole-file walk on *every keystroke*,
// so both directions matter: a false positive is a large, permanent
// latency cost, and a false negative would silently drop TK diagnostics.

#[test]
fn tk_gate_tp_package_require_tk_hands_off() {
    // True positive: the plain form.
    falls_back(
        "package require Tk\nframe .top\n",
        "tcl8.6",
        PerItemFallback::TkActive,
    );
}

#[test]
fn tk_gate_tp_exact_versioned_and_continued_forms() {
    // Still true positives: the `-exact` flag, a version constraint, and a
    // backslash line continuation splitting the words. None of these is
    // special-cased here — they all fall out of the ordinary
    // registry-driven `package require` walk, which is the point of
    // resolving activation from command facts rather than from a byte
    // pattern. (A byte pattern would have to enumerate them.)
    for src in [
        "package require -exact Tk 8.6\nframe .top\n",
        "package require Tk 8.6\nframe .top\n",
        "package \\\n  require \\\n  Tk\nframe .top\n",
    ] {
        falls_back(src, "tcl8.6", PerItemFallback::TkActive);
    }
}

#[test]
fn tk_gate_rooted_package_head_activates() {
    // `::package require Tk` is the same command as `package require Tk`
    // in Tcl. The shared registry's rooted fallback therefore records the
    // package requirement and activates the same whole-file Tk handoff as
    // the bare spelling.
    falls_back(
        "::package require Tk\nframe .top.x\n",
        "tcl8.6",
        PerItemFallback::TkActive,
    );
}

#[test]
fn tk_gate_tp_tk_dialect_short_circuits_at_entry() {
    // The `tk` dialect is decidable before any work, so it must hand off
    // without paying a per-item pass first.
    falls_back("frame .top\n", "tk", PerItemFallback::TkActive);
}

#[test]
fn tk_gate_tp_require_inside_a_body_is_still_caught() {
    // A `package require Tk` the *shell* pass cannot see: only the merged
    // body fragments reveal it, which is why the gate is checked again
    // after the body pass.
    falls_back(
        "proc setup {} { package require Tk }\nsetup\nframe .top\n",
        "tcl8.6",
        PerItemFallback::TkActive,
    );
    // …and one inside a `namespace eval`, which the shell pass *does* walk.
    falls_back(
        "namespace eval ::ui { package require Tk }\nframe .top\n",
        "tcl8.6",
        PerItemFallback::TkActive,
    );
}

#[test]
fn tk_gate_fp_three_words_scattered_across_unrelated_commands() {
    // The exact shape that made this gate ~78% false positives: the words
    // `package`, `require`, and `Tk` all occur, none of them as a
    // `package require Tk` invocation.
    fast_path(
        "package require Tcl 8.6\n\
             set doc \"Tcl/Tk script text executable\"\n\
             proc render {x} { return $x }\n",
    );
}

#[test]
fn tk_gate_fp_comments_strings_and_regexes_mentioning_tk() {
    fast_path("# package require Tk\nproc p {} { set x 1 }\n");
    fast_path("set s \"package require Tk\"\nproc p {} { set x 1 }\n");
    fast_path("regexp {package require Tk} $line\nproc p {} { set x 1 }\n");
}

#[test]
fn tk_gate_fp_package_provide_tk_and_require_tcl() {
    // `package provide Tk` declares this file *is* (part of) Tk; it does
    // not load Tk into this interpreter, so it must not activate. Nor may
    // `package require Tcl`, whatever else the file mentions.
    fast_path("package provide Tk 8.6\nproc p {} { set x 1 }\n");
    fast_path("package require Tcl 8.6\n# Tk not required here\n");
    // `package present Tk` is a query, not a load.
    fast_path("if {[catch {package present Tk}]} { set headless 1 }\n");
}

#[test]
fn tk_gate_tn_dynamic_package_name_does_not_activate() {
    // The documented conservative limit: a computed package name is
    // recorded verbatim, so it cannot match `Tk` and the checks stay off —
    // exactly as on the whole-file path, which `eq` pins.
    fast_path("set p Tk\npackage require $p\nproc q {} { set x 1 }\n");
    fast_path("package require [pkgName]\n# Tk\n");
}

#[test]
fn tk_gate_tn_renamed_package_command_does_not_activate() {
    // `package` renamed away, then a call through the new name: not a
    // resolvable `package require`, so no activation — and, again, the
    // per-item result still matches the whole-file walk exactly.
    fast_path("rename package pkg\npkg require Tk\nproc q {} { set x 1 }\n");
}

#[test]
fn tk_gate_fn_conditional_require_still_activates() {
    // A conditional load is still a load as far as the checks are
    // concerned (the whole-file walk records it too), so the gate must not
    // miss it.
    falls_back(
        "if {$gui} { package require Tk }\nframe .top\n",
        "tcl8.6",
        PerItemFallback::TkActive,
    );
}

// Oversized-body guard.  Analysing a body in isolation costs far more
// than analysing the same content in place, so a generated single-body
// document must not be handed to the decomposition just because it is
// Tk-clean.

#[test]
fn oversized_body_hands_off_to_the_whole_file_walk() {
    // The guard measures body *bytes* (`db.body_text.len()`), not command
    // count, so the fixture only has to be large — it does not also have to
    // be 26,000 separate commands.  Two real commands keep the body a body,
    // and one long comment carries it past the threshold: same gate, same
    // fallback, but the whole-file walk the fallback pays for then costs
    // seconds instead of minutes at `opt-level = 0`.
    let filler = "x".repeat(OVERSIZED_BODY_BYTES);
    let src = format!("proc generated {{}} {{\n    set x 1\n    set y 2\n    # {filler}\n}}\n");
    assert!(
        src.len() > OVERSIZED_BODY_BYTES,
        "fixture must be oversized"
    );
    falls_back(&src, "tcl8.6", PerItemFallback::OversizedBody);
}

#[test]
fn ordinary_large_body_stays_on_the_fast_path() {
    // Well under the threshold: a big hand-written proc keeps its
    // memoisation. The guard exists for generated files, not large ones.
    let body = "    set x 1\n".repeat(1000);
    let src = format!("proc big {{}} {{\n{body}}}\nproc other {{}} {{ set y 2 }}\n");
    assert!(src.len() < OVERSIZED_BODY_BYTES, "fixture must be ordinary");
    fast_path(&src);
}

#[test]
fn many_small_bodies_totalling_over_the_threshold_stay_fast() {
    // The guard is per body, not per document: a document made of many
    // ordinary procs is exactly the case per-body memoisation pays off
    // for, however large the file gets.
    //
    // As above, the threshold is byte-based, so each body reaches its size
    // with a comment rather than with 80 more commands — what this test
    // pins is that 400 under-threshold bodies summing to over the threshold
    // keep the fast path, not how many commands each one holds.
    // Each body reaches its size with a comment rather than with 80 more
    // commands, and the document clears the threshold with 100 bodies
    // rather than 400.  Isolated-body analysis carries a large *per body*
    // constant — that cliff is the whole reason the guard above exists — so
    // body count, not body content, is what costs time here.  What the test
    // pins: every body is under the per-body guard, their sum is over it,
    // and the fast path still holds.
    use std::fmt::Write as _;
    let pad = "y".repeat(3000);
    let body = format!("    set x 1\n    set y 2\n    # {pad}\n");
    assert!(
        body.len() < OVERSIZED_BODY_BYTES,
        "each body must stay under the per-body guard"
    );
    let mut src = String::new();
    for i in 0..100 {
        let _ = write!(src, "proc p{i} {{}} {{\n{body}}}\n");
    }
    assert!(src.len() > OVERSIZED_BODY_BYTES, "fixture must be large");
    fast_path(&src);
}

#[test]
fn private_source_advice_requires_whole_document_owner() {
    let source = "proc caller {} {::tcl::clock::GetSystemTimeZone}";
    let (gate, result) = gate_and_result(source, "tcl8.6");
    assert_eq!(gate, Some(PerItemFallback::OriginalSourceOwnership));
    let whole = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(result.diagnostics, whole.diagnostics);
}

#[test]
fn nested_opaque_declarations_use_full_original_source_owner() {
    let source = r"proc outer {} {proc p\uD800 {} {}; proc p\uD801 {} {}}";
    let (gate, result) = gate_and_result(source, "tcl8.6");
    assert_eq!(gate, Some(PerItemFallback::OriginalSourceOwnership));
    let records = result.original_procedure_declarations().collect::<Vec<_>>();
    assert_eq!(records.len(), 3);
    for record in &records {
        assert_eq!(
            record.name_input().source_image().bytes(),
            source.as_bytes()
        );
    }
    let names = records
        .iter()
        .map(|record| record.name().slot().simple.as_bytes().to_vec())
        .collect::<std::collections::HashSet<_>>();
    assert!(names.contains(b"p\xed\xa0\x80".as_slice()));
    assert!(names.contains(b"p\xed\xa0\x81".as_slice()));
    let tree = super::super::item_tree::ItemTree::from_analysis(
        &result,
        &std::collections::HashSet::new(),
    );
    assert_eq!(tree.file_decls().original_declarations.len(), 3);
}

#[test]
fn original_lexical_member_body_keeps_the_whole_source_owner() {
    // naming.tcloo.original-lexical-member-context
    // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
    let source = "oo::class create C {method a::b {} {return body}}";
    let (gate, result) = gate_and_result(source, "tcl8.6");
    assert_eq!(gate, Some(PerItemFallback::OriginalSourceOwnership));
    let context = result
        .global_scope
        .children
        .iter()
        .find_map(|scope| scope.original_member_context.as_deref())
        .unwrap();
    assert_eq!(
        context
            .ordinary_method()
            .unwrap()
            .original_name_input()
            .bytes(),
        b"a::b"
    );
    assert_eq!(
        context.body_word().image(),
        &tcl_lexer::SourceImage::document(source)
    );
    assert!(context.source_class(&result).is_some());
}

#[test]
fn original_next_advice_reconstructs_whole_source_after_preceding_edits() {
    // naming.tcloo.original-lexical-member-context
    // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
    for prefix in ["", "# preceding line\nset unrelated value\n"] {
        let source = format!(
            "{prefix}oo::class create Base {{method f {{a b}} {{}}}}\noo::class create Child {{superclass Base; method f {{}} {{next 1 2 3}}}}"
        );
        let (gate, result) = gate_and_result(&source, "tcl9.0");
        assert_eq!(gate, Some(PerItemFallback::OriginalSourceOwnership));
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| {
                matches!(
                    diagnostic.subject(),
                    Some(super::super::DiagnosticSubject::ObjectSourceArity(_))
                )
            })
            .expect("original next signature advice");
        assert_eq!(&source[diagnostic.span.as_range()], "next 1 2 3");
        let Some(super::super::DiagnosticSubject::ObjectSourceArity(subject)) =
            diagnostic.subject()
        else {
            unreachable!()
        };
        let super::super::ObjectSourceAritySubject::LexicalNext(original) = subject.as_ref() else {
            panic!("next source purpose")
        };
        assert!(
            original
                .original_words()
                .iter()
                .all(|word| word.image() == &tcl_lexer::SourceImage::document(&source))
        );
        assert!(original.member_context().matches_source(
            &tcl_lexer::SourceImage::document(&source),
            result.body_lexer_config.unwrap()
        ));
    }
}

#[test]
fn computed_body_head_keeps_genuine_whole_document_value_owner() {
    let source = "proc p {} {}; proc outer {} {set cmd p; $cmd}";
    let (gate, result) = gate_and_result(source, "tcl8.6");
    assert_eq!(gate, Some(PerItemFallback::OriginalSourceOwnership));
    let offset = u32::try_from(source.rfind("$cmd").unwrap()).unwrap();
    let invocation = result
        .command_invocations
        .iter()
        .find(|invocation| invocation.range.start() == offset && invocation.indirect)
        .unwrap();
    let input = invocation.original_name_input.as_ref().unwrap();
    assert_eq!(input.bytes(), b"p");
    assert!(input.original_word_key().is_none());
    let lookup = invocation.original_lookup.as_ref().unwrap();
    assert_eq!(
        lookup.site().source.source_image().bytes(),
        source.as_bytes()
    );
    assert_eq!(lookup.site().offset, offset);
}

#[test]
fn two_top_level_procs() {
    eq("proc a {} { set x 1 }\nproc b {} { puts hi }\n");
}

#[test]
fn quoted_var_substitution_does_not_false_fire_e202() {
    eq("puts \"$sum\"\n");
}

#[test]
fn same_file_arity_e003_matches_across_proc_body_call() {
    // A proc-body call to a same-file proc with the wrong argument
    // count — exercises `pending_user_call_arity`'s isolated-body
    // capture + span-rebase-on-graft path, not just the whole-file
    // walk.
    eq("proc need2 {a b} {}\nproc caller {} { need2 1 2 3 }\n");
}

#[test]
fn same_file_arity_via_alias_and_rename_matches_across_proc_body_call() {
    eq("proc target {a b c} {}\n\
            interp alias {} shortcut {} target 100\n\
            rename target target_orig\n\
            proc caller {} { shortcut 1; target_orig 1 2 }\n");
}

#[test]
fn namespace_import_inside_body_falls_back() {
    // A `namespace import` buried in a proc body leaks into the whole-file
    // walk's global import set (reached by a later sibling body's bare
    // imported command); the per-item decomposition can't reproduce that
    // cross-body effect from the isolated body alone, so it falls back to a
    // full rebuild — still byte-identical. See `body_needs_enclosing_context`.
    eq("proc setup {} { namespace import -force ::tcltest::* }\n\
            proc p {cmd} { test t desc -body { {*}$cmd } -result {} }\n");
}

#[test]
fn proc_calling_proc() {
    eq("proc a {x} { return $x }\nproc b {} { a 1 }\n");
}

#[test]
fn renamed_away_builtin_w123_matches() {
    // A top-level rename-away of a builtin plus a later call — the W123
    // deleted-builtin gate reads shell-pass state (`deleted_commands`),
    // which both paths populate identically for top-level statements.
    eq("rename puts {}\nputs x\n");
    eq("puts early\nrename puts myputs\nmyputs late\n");
    // An in-body rename is *conditional* (runs at call time): the gate
    // ignores it on both paths — the whole-file walk records it into
    // `deleted_commands` but filters it as inside a proc body, while an
    // isolated body's analyser state never reaches the shell at all.
    eq("proc hook {} { rename puts _p }\nputs hi\n");
}

#[test]
fn registry_defined_command_names_match() {
    // `coroutine NAME cmd` / `interp create NAME` bind NAME via the
    // registry's `defines_command_at` — recorded eagerly in the isolated
    // body too, and merged at graft, so both paths agree.
    eq("proc gen {} { while 1 { yield 1 } }\ncoroutine nextNum gen\nnextNum\n");
    eq("proc mk {} { coroutine nextNum ::gen }\ninterp create child\nchild eval { puts hi }\n");
}

#[test]
fn literal_namespace_import_w123_matches() {
    // Top-level literal imports feed the W123 tail-glob suppression from
    // `result.namespace_imports`, recorded by the shell pass on both
    // paths (a body-local import falls back — see
    // `namespace_import_inside_body_falls_back`).
    eq("namespace import ::acme::widgets::render_*\nrender_box 10\nfrobnicate 1\n");
}

#[test]
fn wildcard_namespace_import_gated_by_export_matches() {
    // The headline shape exercised through the
    // incremental/whole-file equivalence harness: `namespace export`
    // gates which commands a wildcard import can resolve
    // (`result.namespace_exports`), recorded identically by the shell
    // pass on both paths.
    eq("namespace eval Foo {\n    proc bar {} { return 1 }\n    \
             proc other {} { return 2 }\n    namespace export bar\n}\n\
             namespace import ::Foo::*\nbar\nother\n");
}

#[test]
fn namespace_export_clear_tombstone_ordering_matches() {
    // `-clear` is recorded as an ordered tombstone rather than applied by
    // deleting the namespace's earlier entries, so the export log keeps
    // a row per directive and their relative order is load-bearing. Both
    // walks must produce the identical log — a rebased offset or a
    // dropped tombstone on either path would change what a
    // per-import-site snapshot answers.
    eq("namespace eval src {\n    proc p {} { return P }\n    \
             namespace export p\n}\nnamespace eval dst {\n    \
             namespace import ::src::*\n}\nnamespace eval src {\n    \
             namespace export -clear q\n}\n");
}

#[test]
fn namespace_export_inside_body_falls_back() {
    // A `namespace export` buried in a proc body leaks into the
    // whole-file walk's global export set (reached by a later
    // sibling's wildcard import); the per-item decomposition can't
    // reproduce that cross-body effect from the isolated body alone,
    // so it falls back to a full rebuild — still byte-identical. See
    // `body_needs_enclosing_context`.
    eq("namespace eval Foo {\n    proc bar {} { return 1 }\n    \
             proc setup {} { namespace export bar }\n}\n\
             namespace import ::Foo::*\nbar\n");
}

#[test]
fn namespace_unknown_handler_suppression_matches() {
    // `namespace unknown HANDLER` flips `has_dynamic_providers` (merged
    // with `|=` at graft), suppressing W123 identically on both paths.
    eq("proc h {args} { puts $args }\nnamespace unknown h\nmystery_cmd 1\n");
    eq("proc inst {} { namespace unknown ::h }\nmystery_cmd 1\n");
}

#[test]
fn namespace_and_top_level_code() {
    eq("namespace eval ns { proc foo {} {} }\nset g 1\nputs $g\n");
}

#[test]
fn isolated_global_reads_retain_selected_literal_name_and_element_data() {
    let source = "set {::$literal} VALUE; set {::arr($literal)} ARRAY; set {::arr()} EMPTY_INDEX; set {::} EMPTY; proc read_names {} {puts ${::$literal}; puts ${::arr($literal)}; puts ${::arr()}; puts ${::}}";
    let mut shell = Analyser::new();
    let commands = shell.per_item_setup(source, "tcl8.6");
    shell.defer_proc_bodies = true;
    shell.walk_commands_top_level(&commands, false);
    shell.defer_proc_bodies = false;
    assert_eq!(shell.deferred_bodies.len(), 1);
    let body = shell.deferred_bodies.pop().unwrap();
    let fragment = analyse_proc_body_isolated(
        &body,
        "tcl8.6",
        &std::collections::HashSet::<String>::new(),
        super::super::state::NonAsciiMode::Off,
        None,
        None,
        None,
    );
    let scalar = fragment
        .global_reads
        .iter()
        .find(|read| read.base_name == "::$literal")
        .expect("original scalar source read");
    assert!(scalar.element.is_none());
    let element = fragment
        .global_reads
        .iter()
        .find(|read| read.base_name == "::arr")
        .expect("original array source read");
    assert_eq!(element.element.as_deref(), Some("$literal"));
    assert!(
        fragment
            .global_reads
            .iter()
            .all(|read| read.base_name != "::literal")
    );
    let empty = fragment
        .global_reads
        .iter()
        .find(|read| read.base_name == "::")
        .expect("original absolute empty-name source read");
    assert!(empty.element.is_none());
    let empty_element = fragment
        .global_reads
        .iter()
        .find(|read| read.base_name == "::arr" && read.element.as_deref() == Some(""))
        .expect("original empty-index array source read");
    assert_eq!(empty_element.element.as_deref(), Some(""));
    eq(source);
}

#[test]
fn top_level_global_read_in_body() {
    eq("set g 1\nproc r {} { puts $::g }\nputs $g\n");
}

#[test]
fn isolated_method_seed_names_keep_literal_sigils_from_the_original_class() {
    let source = "oo::class create C {variable {$literal} {${braced}}; method m {} {}}";
    let mut shell = Analyser::new();
    let commands = shell.per_item_setup(source, "tcl8.6");
    shell.defer_proc_bodies = true;
    shell.walk_commands_top_level(&commands, false);
    shell.defer_proc_bodies = false;
    assert_eq!(shell.deferred_bodies.len(), 1);
    let body = shell.deferred_bodies.pop().unwrap();
    assert!(body.is_method);
    assert!(body.seeded_variables.contains(&"$literal".to_owned()));
    assert!(body.seeded_variables.contains(&"${braced}".to_owned()));
    let fragment = analyse_proc_body_isolated(
        &body,
        "tcl8.6",
        &std::collections::HashSet::<String>::new(),
        super::super::state::NonAsciiMode::Off,
        None,
        None,
        None,
    );
    assert!(fragment.proc_scope.variables.contains_key("$literal"));
    assert!(fragment.proc_scope.variables.contains_key("${braced}"));
    assert!(
        !fragment.proc_scope.variables.contains_key("literal")
            && !fragment.proc_scope.variables.contains_key("braced")
    );
}

#[test]
fn oo_class_with_method() {
    eq("oo::class create K {\n  variable n\n  method m {a} { set n $a; return $n }\n}\n");
}

#[test]
fn method_body_defines_duplicate_proc() {
    // A method that defines `::p`, plus a later top-level `proc p`: a
    // whole-file walk ends with the top-level definition (source order),
    // so the per-item path must fall back rather than let the pass-2
    // method body win the shared `all_procs["::p"]` key.
    eq("oo::class create C { method m {} { proc p {} { return 1 } } }\nproc p {} { return 2 }\n");
    // Same, reversed order (top-level first, then the method).
    eq("proc p {} { return 2 }\noo::class create C { method m {} { proc p {} { return 1 } } }\n");
    // A method that defines a non-colliding proc stays byte-identical too.
    eq("oo::class create C { method m {} { proc helper {} { return 1 } } }\n");
}

#[test]
fn nested_proc_in_body() {
    eq("proc outer {} { proc ::inner {} { set z 1 } ; inner }\n");
}

#[test]
fn qualified_read_marks_enclosing_global_used() {
    // A body's `$::g` read must land as a reference on the enclosing `::g`
    // (captured + replayed at graft), so the "unused" state matches a
    // whole-file walk even though the body is analysed in isolation.
    eq("set ::g 1\nproc r {} { return [expr {$::g + 1}] }\n");
    eq("set ::g 1\nproc r {} { return 0 }\n"); // unused global stays unused
}

#[test]
fn duplicate_top_level_proc_byte_identical() {
    // Platform-conditional redefinition: last-def-wins in a whole-file walk.
    // The per-item path reproduces it on the fast path (the graft unions each
    // definition's `all_variables` with last-definition-wins on shared keys).
    eq("if {1} {\n  proc p {} { set file a; puts $file }\n} else {\n  proc p {} { return 2 }\n}\n");
}

#[test]
fn duplicate_proc_same_local_last_wins() {
    // Same-named local across two definitions: `all_variables` keeps the last
    // definition's span + references (not the first, not the union of refs).
    eq("proc p {} { set a 1 }\nproc p {} { set a 2 }\n");
    eq("proc p {} { set a 1; incr a }\nproc p {} { set a 2; incr a }\n");
}

#[test]
fn duplicate_proc_unique_locals_unioned() {
    // A local defined only in an earlier definition survives (union), while a
    // shared local is last-wins — the whole-file `define_var` semantics.
    eq("proc p {} { set early 1 }\nproc p {} { set late 2 }\n");
}

#[test]
fn duplicate_proc_with_params() {
    // Param references on a duplicate proc are last-definition-wins (the
    // earlier body's reads are dropped), and the shell's real param span is
    // kept (not the isolated body's placeholder).
    eq("proc p {key} { return $key }\nproc p {key} { puts $key }\n");
}

#[test]
fn multi_method_class_byte_identical() {
    // A class with several methods must NOT be treated as a set of mutual
    // "duplicates" (the empty-scope-name bug) — it takes the fast path.
    eq(
        "oo::class create K {\n  method a {} { set x 1 }\n  method b {} { set y 2 }\n  method c {x} { return $x }\n}\n",
    );
}

#[test]
fn instance_creation_in_proc_body() {
    // An isolated proc body can't resolve the sibling class; the creation is
    // captured and replayed at graft so `instance_classes` matches.
    eq("oo::class create Dog {}\nproc make {} { set d [Dog new] }\n");
    eq("oo::class create Dog {}\nproc make {} { Dog create rex }\n");
}

#[test]
fn class_extension_in_proc_falls_back() {
    // A proc body that *extends* an existing class (oo::define adds a method)
    // accumulates across the definition site; the per-item graft can't
    // reproduce that, so it falls back — and stays byte-identical.
    eq("oo::class create K { method a {} {} }\nproc ext {} { oo::define K method b {} {} }\next\n");
}

#[test]
fn self_redefining_proc_falls_back() {
    // Lazy-init: the inner `proc p` redefines the outer; nested-redefinition
    // detection forces a fallback so the result matches a whole-file walk.
    eq("proc p {a} { proc p {a} { return $a }\n p $a }\n");
}

// Offset-carrying synthetic identities must be rebase-stable: the
// isolated body pass mints them from body-relative offsets (keeping the
// memo offset-invariant) and the graft rebases them to the absolute
// offsets the whole-file walk mints.  Each shape below is one that
// diverges without that rebase (`@dynns@`/`@dynclass@`/`@autoname@` keys
// and every string embedding them).

#[test]
fn dynamic_namespace_eval_in_body_rebases_dynns_identity() {
    // `namespace eval $n` inside a proc body: the synthetic scope name,
    // its `all_variables` keys, and the procs defined under it must all
    // carry the whole-file offset.
    eq(
        "proc mk {n} {\n    namespace eval $n {\n        variable state\n        proc go {} { return 1 }\n    }\n}\n",
    );
    // Two occurrences writing the same variable name stay distinct.
    eq(
        "proc a {n} { namespace eval $n { variable x } }\nproc b {n} { namespace eval $n { variable x } }\n",
    );
}

#[test]
fn dynamic_oo_define_in_body_rebases_dynclass_identity() {
    eq("proc ext {cls} {\n    oo::define $cls method poke {} { return 1 }\n}\n");
}

#[test]
fn pathless_interp_create_in_body_rebases_autoname_identity() {
    // `set VAR [interp create -safe]` with no path mints
    // `@autoname@<offset>`, which then appears inside
    // `@interp@@autoname@…`-qualified keys (procs defined via the
    // handle) and `resolution_candidates` strings.
    eq(
        "proc spawn {} {\n    set i [interp create -safe]\n    interp eval $i { proc inner {} { return 1 } }\n}\n",
    );
}

#[test]
fn literal_name_spelled_like_marker_is_not_rebased() {
    // A source-literal proc named like a minted marker must survive
    // untouched (the minted-set gate) — and stay byte-identical.
    eq("proc @dynns@5 {} { return 1 }\nproc user {} { @dynns@5 }\n");
}

// Instance-creation visibility must follow the whole-file DFS: a creation
// site resolves against exactly the classes whose definition precedes it
// in source order, whether the site or the
// class definition sits at top level or inside a body.

#[test]
fn instance_creation_before_class_definition_stays_unresolved() {
    // The class is defined *after* the proc: the whole-file walk has no
    // `Dog` when it walks the body, so no instance may be recorded
    // (tcllib `halfpipe.tcl` / `fifo2.tcl` shape).
    eq("proc make {} { set d [Dog new] }\noo::class create Dog {}\n");
}

#[test]
fn top_level_creation_sees_class_defined_in_earlier_body() {
    // The class is defined inside an earlier proc body; the top-level
    // creation site resolves it in a whole-file walk (the body was
    // walked at the proc's definition point) — the shell pass alone
    // cannot see it, so the per-item path replays post-graft
    // (tcllib `oodialect.tcl` shape).
    eq("proc def {} { oo::class create ::K {} }\n::K create inst\ninst poke\n");
}

// Whole-file-fact diagnostics reached from inside a body.

#[test]
fn w103_dynamic_open_in_body_matches() {
    // Body-local values refine only through the complete original CU.
    eq("proc p {} {\n    set c {|nm -g}\n    set f [open $c r]\n    close $f\n}\n");
    // An outer spelling cannot donate a local value. Both paths retain
    // the same original source selection and actual value owner.
    eq("set c {|nm -g}\nproc p {} { set f [open $c r]; close $f }\n");
    // TN parity: a benign literal filename stays silent on both paths.
    eq("set c ./data.txt\nproc p {} { set f [open $c r]; close $f }\n");
}

#[test]
fn w300_dynamic_source_in_body_matches() {
    eq("proc p {} { set s lib.tcl; source $s }\n");
    eq("set s lib.tcl\nproc p {} { source $s }\n");
}

#[test]
fn widget_dispatch_sites_inside_bodies_match() {
    // Registry object-factory instance + dispatch, both inside a body,
    // and an instance created at top level dispatched inside a body —
    // the W001 unknown-subcommand check needs the buffered sites from
    // the isolated pass (tcllib `bench_wtext.tcl` / `graphops.tcl`
    // shape).
    eq("proc go {} {\n    struct::graph g\n    g bogus_subcommand 1\n}\n");
    eq("struct::graph g\nproc go {} { g bogus_subcommand 1 }\n");
}

#[test]
fn ensemble_subcommand_call_inside_body_matches() {
    // The ensemble map is declared at top level; a body's
    // `<ensemble> <sub> …` call must record the same existence-probed
    // subcommand invocation the whole-file walk does (tcl `tm.tcl`
    // shape).
    eq(
        "namespace eval ::tm {\n    proc add {p} { return $p }\n}\nnamespace ensemble create -command ::tm::path -map {add ::tm::add}\nproc use {} { ::tm::path add x }\n",
    );
}

#[test]
fn prefixless_ensemble_setting_reaches_and_returns_from_bodies() {
    // The `-prefixes 0` opt-out has to travel **both** ways across the
    // isolated-body seam, because `resolve_ensemble_subcommand` reads it
    // together with the subcommand map.
    //
    // Seeding: the ensemble is configured at top level and abbreviated
    // inside a body. Real Tcl (8.6.16 / 9.0.4) answers `unknown
    // subcommand "fo"`, so neither walk may record a dispatch — seeding
    // only the map would make the isolated pass record one.
    eq(
        "namespace eval ::e {\n    proc Foo {} { return FOO }\n}\nnamespace ensemble create -command ::e::g -map {foo ::e::Foo} -prefixes 0\nproc use {} { ::e::g fo }\n",
    );
    // The positive control: without `-prefixes 0` the same abbreviation
    // *does* dispatch (`FOO`), so the dispatch must still be recorded.
    eq(
        "namespace eval ::e {\n    proc Foo {} { return FOO }\n}\nnamespace ensemble create -command ::e::h -map {foo ::e::Foo}\nproc use {} { ::e::h fo }\n",
    );
    // Merging back: a body may *create* the ensemble, and the opt-out has
    // to reach the shell result the way its subcommand map already does.
    eq(
        "namespace eval ::e {\n    proc Foo {} { return FOO }\n}\nproc setup {} {\n    namespace ensemble create -command ::e::g -map {foo ::e::Foo} -prefixes 0\n}\nproc use {} { ::e::g fo }\n",
    );
}

#[test]
fn in_body_interp_alias_feeds_w123_of_sibling_body() {
    // The whole-file walk records an `interp alias` buried in one proc
    // body, and that suppresses W123 (or feeds its "did you mean…?")
    // for a call in a sibling body (tcllib `list.test.tcl` shape).
    eq(
        "proc setup {} { interp alias {} shorten {} ::string range }\nproc use {} { shorten abcdef 0 2 }\n",
    );
}

#[test]
fn const_dispatch_sites_inside_bodies_match() {
    // `$fn` dispatch whose contributors are branch constants — settled
    // in the CFG/SSA phase on the whole-file unit, so the isolated
    // body's pending sites must be re-queued (tcllib `otp.tcl` /
    // `cfront.tcl` shape).
    eq(
        "proc f {} { return 1 }\nproc dispatch {k} {\n    switch -exact -- $k {\n        a { set fn ::f }\n        b { set fn ::f }\n    }\n    $fn\n}\n",
    );
}

#[test]
fn qualified_param_name_records_one_occurrence_not_two() {
    // A parameter literally named with a `::` qualifier (degenerate but
    // legal): the shell walk records its qualified occurrence once; the
    // isolated pass's structural rebind must not add a second, phantom
    // zero-width one (caught by the edit fuzzer on `util_flow.tcl`).
    eq("proc f {a ns::v} { set a 1 }\n");
    eq("oo::class create K { method m {x ns::v} { set x 1 } }\n");
}

#[test]
fn param_list_with_backslash_continuation_no_spurious_w215() {
    // A param list split across lines by `\<newline>` list-parses as
    // separate parameters (`parse_param_list` treats the continuation as an
    // element separator), so no param name ends in a raw
    // backslash and neither the full `analyse` walk nor the isolated
    // per-item rebind emits a spurious W215 unreachable-name warning.
    eq("proc foo {a b staticsok\\\n    c d} {\n  set x 1\n}\n");
    // Method form (instance vars + params rebind under the same suppression).
    eq("oo::class create K {\n  method m {a b\\\n    c} { return $a }\n}\n");
}
