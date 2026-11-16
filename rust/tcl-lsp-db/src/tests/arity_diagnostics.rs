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

//! `project_diagnostics`: cross-file wrong-argument-count and unknown-command advice, its cutoffs and its incremental parity.

use super::*;

#[test]
fn original_project_diagnostics_select_opaque_call_slots_and_namespace_priority() {
    // Implementation contract: naming.database.original-call-arity-diagnostics
    // docs/design/analysis/name-resolution-proofs/database-original-call-arity-diagnostics.md
    let db = TclDatabase::default();
    let config = cfg(&db);
    let provider = SourceFile::new(
        &db,
        r"proc p\uD800 {a} {}; proc p\uD801 {a b} {};
namespace eval N {proc only_here {a} {}}"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let caller = SourceFile::new(
        &db,
        r"p\uD800 A B
p\uD801 A B"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![provider, caller]);
    let analysis = file_analysis_incremental(&db, caller, config);
    let calls = analysis
        .command_invocations
        .iter()
        .filter(|invocation| invocation.original_name_input.is_some())
        .collect::<Vec<_>>();
    let first = calls
        .iter()
        .find(|invocation| {
            invocation
                .original_name_input
                .as_ref()
                .is_some_and(|input| input.bytes() == b"p\xed\xa0\x80")
        })
        .expect("original first call");
    let second = calls
        .iter()
        .find(|invocation| {
            invocation
                .original_name_input
                .as_ref()
                .is_some_and(|input| input.bytes() == b"p\xed\xa0\x81")
        })
        .expect("original second call");
    let diagnostics = project_diagnostics(&db, caller, config, project);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::E003 && diagnostic.span == first.range),
        "first byte name accepts exactly one argument"
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.span == second.range),
        "distinct second byte name accepts two arguments"
    );
    // Namespace advice is an independent source point. The external
    // calls above do not close the world for a following command.
    let unrelated = SourceFile::new(&db, "only_here A".to_owned(), "tcl8.6".to_owned(), None);
    let unrelated_project = Project::new(&db, vec![provider, unrelated]);
    let unrelated_diagnostics = project_diagnostics(&db, unrelated, config, unrelated_project);
    let unresolved = unrelated_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagCode::W123)
        .collect::<Vec<_>>();
    assert_eq!(
        unresolved.len(),
        1,
        "unrelated namespace tails cannot suppress this call"
    );
    assert_eq!(
        unresolved[0]
            .unresolved_command()
            .unwrap()
            .name_input()
            .bytes(),
        b"only_here"
    );
}

/// Per-symbol cross-file precision: editing an *unrelated* proc's signature in
/// the defining file must **not** re-run a calling file's `project_diagnostics`.
///
/// `b` calls only `foo` (defined in `a`); `a` also defines `bar`, which `b`
/// never calls. Each demanded original source slot has its own arity query.
/// A signature edit to `bar` changes the project header table while the
/// projected `foo` header is unchanged, so salsa backdates that result →
/// `project_diagnostics(b)` does **not** re-execute (the per-symbol early-cutoff
/// the whole-table dependency could not give).  Editing `foo` itself, by
/// contrast, *must* re-run `b` and flow the new arity through.
#[test]
fn project_diagnostics_per_symbol_cutoff() {
    use salsa::Setter as _;
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    // Count `project_diagnostics` re-executions only (not the cheap accessor /
    // aggregate, which legitimately re-run on a signature edit).
    let runs = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("project_diagnostics"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    // `a` defines `foo` (1 param) and an unrelated `bar`.
    let a = SourceFile::new(
        &db,
        "proc foo {x} {}\nproc bar {y} {}\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // `b` calls `foo` with one arg — resolves cross-file, fits arity (1, 1),
    // so its diagnostics are empty.  It never references `bar`.
    let b = SourceFile::new(&db, "foo 1\n".to_owned(), "tcl8.6".to_owned(), None);
    let project = Project::new(&db, vec![a, b]);

    assert!(
        project_diagnostics(&db, b, cfg, project).is_empty(),
        "cold: foo resolves cross-file (1 arg fits arity 1) → no diagnostics"
    );
    let _ = runs(&log);

    // Signature edit to the UNRELATED `bar` (add a param).  The arity table and
    // The project table changes, but the `foo` slot backdates → `b`'s
    // cross-file diagnostics must NOT re-run.
    a.set_text(&mut db)
        .to("proc foo {x} {}\nproc bar {y z} {}\n".to_owned());
    let after_unrelated = project_diagnostics(&db, b, cfg, project);
    assert_eq!(
        runs(&log),
        0,
        "unrelated proc's signature edit must not re-run the caller's project_diagnostics"
    );
    assert!(
        after_unrelated.is_empty(),
        "b's diagnostics unchanged by an edit to a proc it never calls"
    );

    // Control: a signature edit to `foo` itself (now needs 2 args) MUST re-run
    // `b` and surface the cross-file arity error.
    a.set_text(&mut db)
        .to("proc foo {x w} {}\nproc bar {y z} {}\n".to_owned());
    let after_foo = project_diagnostics(&db, b, cfg, project);
    assert_eq!(
        runs(&log),
        1,
        "the called proc's signature edit must re-run the caller's project_diagnostics"
    );
    assert!(
        after_foo.iter().any(|d| d.code == DiagCode::E002),
        "foo now needs 2 args but b passes 1 → cross-file E002 (too few)"
    );
}

/// Project diagnostics for a diagnostic-vec comparison: `(code, start, end,
/// message)` per diagnostic, in order (the analyser emits deterministically).
#[cfg(test)]
fn diag_keys(
    diags: &[tcl_compiler::analyser::types::Diagnostic],
) -> Vec<(String, u32, u32, String)> {
    diags
        .iter()
        .map(|d| {
            (
                d.code.to_string(),
                d.span.start(),
                d.span.end(),
                d.message.clone(),
            )
        })
        .collect()
}

/// Cross-file W123: a command unresolved locally but
/// defined as a `proc` in another project file must have its W123 suppressed
/// when (and only when) that file is in the `Project`.
#[test]
fn project_diagnostics_suppresses_cross_file_w123() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let a = SourceFile::new(
        &db,
        "helper foo bar\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let b = SourceFile::new(
        &db,
        "proc helper {x y} { return $x }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let has_helper_w123 = |diags: &[tcl_compiler::analyser::types::Diagnostic]| {
        diags
            .iter()
            .any(|d| d.code == DiagCode::W123 && d.message.contains("helper"))
    };
    // A alone: `helper` is unresolved → W123 present.
    let proj_a = Project::new(&db, vec![a]);
    assert!(
        has_helper_w123(&project_diagnostics(&db, a, cfg, proj_a)),
        "helper must be unresolved (W123) when B is not in the project"
    );
    // A + B (B defines `proc helper`): cross-file resolved → W123 suppressed.
    let proj_ab = Project::new(&db, vec![a, b]);
    assert!(
        !has_helper_w123(&project_diagnostics(&db, a, cfg, proj_ab)),
        "helper must resolve cross-file (no W123) when B defines proc helper"
    );
}

/// The multi-file `incremental == fresh`
/// differential.  Drive a 2-file project through edits to the *defining* file
/// (`b`) — adding/removing the procs the calling file (`a`) invokes — and
/// assert the calling file's cross-file diagnostics always match a fresh
/// whole-project rebuild.  Catches any untracked read / non-deterministic
/// fold in `project_diagnostics` or its cross-file dependency edges.
/// Reduce a `u64` PRNG draw to an index into a small slice. The modulo
/// is done in `u64` and the result (always `< len`) converts back
/// losslessly, so there is no truncating cast.
fn pick_index(r: u64, len: usize) -> usize {
    usize::try_from(r % len as u64).unwrap()
}

#[test]
fn project_diagnostics_incremental_matches_fresh_under_edits() {
    use salsa::Setter as _;
    // `a` calls four commands; `set` is a builtin, the rest resolve only if
    // `b` defines them.
    let a_text = "alpha 1\nbeta 2\ngamma 3\nset x 4\ndelta 5\n";
    let b_variants = [
        "",
        "proc alpha {x} {}\n",
        "proc alpha {x} {}\nproc beta {y} {}\n",
        "proc beta {y} {}\nproc gamma {z} {}\n",
        "namespace eval ns { proc delta {q} {} }\n",
        "proc alpha {x} {}\nproc beta {y} {}\nproc gamma {z} {}\nproc delta {q} {}\n",
    ];
    let mk = |b_text: &str| -> Vec<(String, u32, u32, String)> {
        let db = TclDatabase::default();
        let cfg = AnalyserConfig::new(
            &db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            0,
            Vec::new(),
            Vec::new(),
        );
        let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned(), None);
        let b = SourceFile::new(&db, b_text.to_owned(), "tcl8.6".to_owned(), None);
        let project = Project::new(&db, vec![a, b]);
        diag_keys(&project_diagnostics(&db, a, cfg, project))
    };

    let mut db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned(), None);
    let b = SourceFile::new(&db, b_variants[0].to_owned(), "tcl8.6".to_owned(), None);
    let project = Project::new(&db, vec![a, b]);

    let mut rng = 0x1234_5678_9abc_def0_u64;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    for _ in 0..60 {
        let b_text = b_variants[pick_index(next(), b_variants.len())];
        b.set_text(&mut db).to(b_text.to_owned());
        let inc = diag_keys(&project_diagnostics(&db, a, cfg, project));
        let fresh = mk(b_text);
        assert_eq!(inc, fresh, "incremental != fresh after b = {b_text:?}");
    }
}

/// Cross-file differential, both files edited: the
/// stronger sibling of the B-only fuzzer above — drive a 2-file project through
/// independent edits to **both** the calling file (`a`) and the defining file
/// (`b`), asserting `a`'s cross-file diagnostics always match a fresh
/// whole-project rebuild.  Editing the caller changes the call sites (and their
/// arg counts) while editing the callee changes the resolution/arity domain;
/// catches any stale cross-file edge that a single-file fuzzer would miss.
#[test]
fn project_diagnostics_incremental_matches_fresh_both_files_edited() {
    use salsa::Setter as _;
    // Caller variants: vary which commands are called and with how many args
    // (so the cross-file arity error path is exercised, not just W123).
    let a_variants = [
        "alpha 1\nbeta 2\n",
        "alpha 1 2 3\nbeta\n", // wrong arg counts → arity error once resolved
        "alpha\nset x 1\ngamma 9\n", // gamma may be unresolved → W123
        "beta 1 2\nalpha 7\n",
        "",
    ];
    // Defining variants: vary which procs exist and their arities.
    let b_variants = [
        "",
        "proc alpha {x} {}\n",
        "proc alpha {x} {}\nproc beta {y z} {}\n",
        "proc alpha {a b c} {}\nproc beta {} {}\nproc gamma {q} {}\n",
        "proc alpha {args} {}\nproc beta {x {y 1}} {}\n",
    ];
    let mk = |a_text: &str, b_text: &str| -> Vec<(String, u32, u32, String)> {
        let db = TclDatabase::default();
        let cfg = AnalyserConfig::new(
            &db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            0,
            Vec::new(),
            Vec::new(),
        );
        let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned(), None);
        let b = SourceFile::new(&db, b_text.to_owned(), "tcl8.6".to_owned(), None);
        let project = Project::new(&db, vec![a, b]);
        diag_keys(&project_diagnostics(&db, a, cfg, project))
    };

    let mut db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let a = SourceFile::new(&db, a_variants[0].to_owned(), "tcl8.6".to_owned(), None);
    let b = SourceFile::new(&db, b_variants[0].to_owned(), "tcl8.6".to_owned(), None);
    let project = Project::new(&db, vec![a, b]);

    let mut rng = 0x0f0f_1234_dead_beef_u64;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    for _ in 0..80 {
        // Edit one file (sometimes both) per round.
        let pick = next() % 3;
        let a_text = a_variants[pick_index(next(), a_variants.len())];
        let b_text = b_variants[pick_index(next(), b_variants.len())];
        if pick != 1 {
            a.set_text(&mut db).to(a_text.to_owned());
        }
        if pick != 0 {
            b.set_text(&mut db).to(b_text.to_owned());
        }
        // Read back the *current* committed texts for the fresh comparison.
        let cur_a = a.text(&db).clone();
        let cur_b = b.text(&db).clone();
        let inc = diag_keys(&project_diagnostics(&db, a, cfg, project));
        let fresh = mk(&cur_a, &cur_b);
        assert_eq!(inc, fresh, "incremental != fresh: a={cur_a:?} b={cur_b:?}");
    }
}

/// Cross-file arity: a call to a workspace-defined
/// proc with the wrong argument count emits the analyser's arity error
/// (`E003` too many here) — *not* the unrelated `W124` IP-literal warning — and
/// the W123 it would have drawn is suppressed; a correct count emits neither.
#[test]
fn project_diagnostics_emits_cross_file_arity() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    // B defines `proc helper {x y}` — arity exactly 2.
    let b = SourceFile::new(
        &db,
        "proc helper {x y} { return $x }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
        diags
            .iter()
            .any(|d| d.code.as_str() == code && d.message.contains("helper"))
    };

    // 3 args to a 2-param proc → E003 (too many), and the W123 is suppressed.
    let a3 = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned(), None);
    let p3 = Project::new(&db, vec![a3, b]);
    let d3 = project_diagnostics(&db, a3, cfg, p3);
    assert!(
        has(&d3, "E003"),
        "3 args to a 2-param cross-file proc must emit E003"
    );
    assert!(
        !has(&d3, "W124"),
        "must not reuse W124 (the IP-literal warning)"
    );
    assert!(
        !has(&d3, "W123"),
        "W123 must be suppressed once resolved cross-file"
    );

    // Correct arity (2 args) → no arity error and no W123.
    let a2 = SourceFile::new(&db, "helper a b\n".to_owned(), "tcl8.6".to_owned(), None);
    let p2 = Project::new(&db, vec![a2, b]);
    let d2 = project_diagnostics(&db, a2, cfg, p2);
    assert!(
        !has(&d2, "E002") && !has(&d2, "E003"),
        "correct arity → no arity error"
    );
    assert!(!has(&d2, "W123"), "resolved cross-file → no W123");
}

/// Mixed proc / non-proc tail: when a class (or alias
/// / ensemble) and a proc share a tail name, a call may dispatch to the
/// arity-less class command, so no arity error may fire even when the arg count
/// fits no proc arity — while the call still resolves (no W123).
#[test]
fn cross_file_arity_suppressed_for_mixed_proc_nonproc_tail() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    // B: a class `Widget` AND a proc whose tail is also `Widget` (arity 1).
    let b = SourceFile::new(
        &db,
        "oo::class create Widget { method draw {} {} }\n\
             namespace eval ns { proc Widget {x} {} }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // The actual root class slot is occupied without a procedure arity;
    // a same-tailed declaration in another namespace cannot donate one.
    let proj_names = Project::new(&db, vec![b]);
    let headers = item_sigs(&db, b);
    let class = headers
        .iter()
        .find(|header| header.id.kind == tcl_compiler::analyser::ItemKind::Class)
        .unwrap();
    let name = class.original_declaration.as_ref().unwrap().name();
    let slot = OriginalCommandSlot::new(&db, name.policy(), name.slot().clone());
    assert_eq!(
        original_command_arity(&db, proj_names, slot).map(|arities| (*arities).clone()),
        Some(Vec::new()),
        "an occupied non-procedure slot must carry an empty arity list"
    );
    // A calls `Widget new extra` (3 args) — fits no proc arity, but resolves to
    // the class → neither an arity error nor W123.
    let a = SourceFile::new(
        &db,
        "Widget new extra\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let proj = Project::new(&db, vec![a, b]);
    let d = project_diagnostics(&db, a, cfg, proj);
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::E002 || x.code == DiagCode::E003),
        "a mixed proc/non-proc tail must never draw an arity error"
    );
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::W123 && x.message.contains("Widget")),
        "the call still resolves cross-file (no W123)"
    );
}

/// Object-instance method callbacks resolve cross-file — including when the
/// dispatch is **inside a proc body**, which the incremental per-item
/// firewall (`file_analysis_incremental`, used by `project_diagnostics`)
/// otherwise missed because it defers instance creation to the graft.
/// Registry object-factories now bind eagerly in an isolated body, so the
/// in-body `$g walk … -command cb` records the callback and its arity is
/// resolved against the other file.
#[test]
fn cross_file_in_proc_instance_method_callback_arity() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    // B defines onNode with 2 params; `graph walk -command` appends 3.
    let b = SourceFile::new(
        &db,
        "proc onNode {a b} { }\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let has_e003 = |src: &str| {
        let a = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
        let p = Project::new(&db, vec![a, b]);
        project_diagnostics(&db, a, cfg, p)
            .iter()
            .any(|d| d.code.as_str() == "E003" && d.message.contains("onNode"))
    };
    // `struct::graph` is a tcllib command: C knows it only after the
    // `package require` (tclsh 8.4 to 9.1: invalid command name
    // "struct::graph" without it).
    // Named factory + dispatch both inside a proc body.
    assert!(
        has_e003(
            "package require struct::graph\n\
                 proc build {} {\n struct::graph g\n g walk root -command onNode\n}\n"
        ),
        "in-proc `struct::graph g; g walk -command onNode` must resolve cross-file arity (E003)"
    );
    // Handle form (`set g [struct::graph]`) inside a proc body.
    assert!(
        has_e003(
            "package require struct::graph\n\
                 proc build {} {\n set g [struct::graph]\n $g walk root -command onNode\n}\n"
        ),
        "in-proc `set g [struct::graph]; $g walk -command onNode` must resolve cross-file arity"
    );
    // Correct arity (3 params) is silent.
    let b3 = SourceFile::new(
        &db,
        "proc onNode {a b c} { }\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let a_ok = SourceFile::new(
        &db,
        "package require struct::graph\n\
             proc build {} {\n struct::graph g\n g walk root -command onNode\n}\n"
            .to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let p_ok = Project::new(&db, vec![a_ok, b3]);
    assert!(
        !project_diagnostics(&db, a_ok, cfg, p_ok)
            .iter()
            .any(|d| (d.code.as_str() == "E003" || d.code.as_str() == "E002")
                && d.message.contains("onNode")),
        "a correct 3-param in-proc instance callback must be silent cross-file"
    );
}

/// Cross-file arity honours `disabled_diagnostics`:
/// the synthesised arity error is produced *after* the analyser's own code
/// filter (and the LSP lift doesn't re-filter), so it must replicate it —
/// disabling `E003` (while keeping W123) must drop the cross-file arity error,
/// yet the call still resolves (no W123).
#[test]
fn cross_file_arity_honors_disabled_code() {
    let db = TclDatabase::default();
    // B defines a 2-param proc; A calls it with 3 args (too many → E003).
    let b = SourceFile::new(
        &db,
        "proc helper {x y} { return $x }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let a = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned(), None);
    let proj = Project::new(&db, vec![a, b]);
    let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
        diags
            .iter()
            .any(|d| d.code.as_str() == code && d.message.contains("helper"))
    };

    // E003 enabled (default): wrong arity surfaces as E003, W123 suppressed.
    let cfg_on = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let d_on = project_diagnostics(&db, a, cfg_on, proj);
    assert!(has(&d_on, "E003"), "baseline: E003 present when enabled");
    assert!(!has(&d_on, "W123"), "baseline: W123 suppressed (resolved)");

    // E003 disabled (W123 left enabled): no E003, W123 still suppressed — the
    // call genuinely resolves cross-file regardless of the arity-code toggle.
    let cfg_off = AnalyserConfig::new(
        &db,
        vec!["E003".to_owned()],
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let d_off = project_diagnostics(&db, a, cfg_off, proj);
    assert!(
        !has(&d_off, "E003"),
        "disabling E003 must suppress the cross-file arity error"
    );
    assert!(
        !has(&d_off, "W123"),
        "W123 stays suppressed even with E003 disabled"
    );
}

/// Cross-file arity independent of the W123 toggle:
/// disabling W123 (unknown-command) must NOT also silence cross-file arity —
/// the analyser drops the W123 markers the arity pass keys off, so
/// `project_diagnostics` drives off a W123-forced analysis.  A wrong-arg
/// cross-file call must still report `E003` (matching local arity), with no
/// W123 leaking through.
#[test]
fn cross_file_arity_survives_w123_disabled() {
    let db = TclDatabase::default();
    let b = SourceFile::new(
        &db,
        "proc helper {x y} { return $x }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
        diags.iter().any(|d| d.code.as_str() == code)
    };
    // W123 disabled, E003 left enabled.
    let cfg = AnalyserConfig::new(
        &db,
        vec!["W123".to_owned()],
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );

    // Wrong arity (3 args to a 2-param proc) → E003 still fires; no W123.
    let bad = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned(), None);
    let pb = Project::new(&db, vec![bad, b]);
    let d_bad = project_diagnostics(&db, bad, cfg, pb);
    assert!(
        has(&d_bad, "E003"),
        "cross-file arity must survive W123 being disabled, got: {:?}",
        d_bad.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
    assert!(
        !has(&d_bad, "W123"),
        "W123 stays suppressed (it is disabled)"
    );

    // Correct arity → no arity error and no W123 (resolved, W123 disabled).
    let ok = SourceFile::new(&db, "helper a b\n".to_owned(), "tcl8.6".to_owned(), None);
    let pok = Project::new(&db, vec![ok, b]);
    let d_ok = project_diagnostics(&db, ok, cfg, pok);
    assert!(
        !has(&d_ok, "E002") && !has(&d_ok, "E003"),
        "correct arity → no arity error"
    );
    assert!(
        !has(&d_ok, "W123"),
        "no W123 (disabled, and resolved anyway)"
    );
}

/// `tclLsp.extraCommands` threaded through `AnalyserConfig` makes a named
/// command known, so calling it never draws a W123.
#[test]
fn extra_commands_config_suppresses_w123() {
    let db = TclDatabase::default();
    let src = "mylibsend foo\n";
    let has_w123 = |cfg: AnalyserConfig| {
        let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        file_analysis_incremental(&db, file, cfg)
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "W123")
    };
    // Baseline: unknown command → W123.
    let base = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    assert!(has_w123(base), "baseline W123 expected");
    // With the command declared extra → suppressed.
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        vec!["mylibsend".to_owned()],
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    assert!(!has_w123(cfg), "extraCommands should suppress W123");
}

/// Cross-file classes: a command resolving to a
/// class (the class command) defined in another project file is resolved —
/// W123 suppressed — and, being a non-proc, never draws an arity error.
#[test]
fn project_diagnostics_resolves_cross_file_class() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    // B defines a TclOO class `Widget`.
    let b = SourceFile::new(
        &db,
        "oo::class create Widget { method draw {} {} }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // A invokes the `Widget` class command (defined cross-file).
    let a = SourceFile::new(&db, "Widget new\n".to_owned(), "tcl8.6".to_owned(), None);
    let proj = Project::new(&db, vec![a, b]);
    let d = project_diagnostics(&db, a, cfg, proj);
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::W123 && x.message.contains("Widget")),
        "cross-file class command must resolve (no W123)"
    );
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::E002 || x.code == DiagCode::E003),
        "a class command has no proc arity → no arity error"
    );
}

/// Cross-file arity edge cases: exercise the
/// `proc_arity` `(min, max)` computation across required-only, optional
/// defaults, a trailing `args` (unbounded), and the no-parameter proc — the
/// subtle part where an off-by-one would mis-fire the arity error.  Checks the
/// *specific* code too: too-few → `E002`, too-many → `E003`.
#[test]
fn cross_file_arity_edge_cases() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let b = SourceFile::new(
        &db,
        "proc two {a b} {}\nproc opt {a {b 1}} {}\nproc variadic {a args} {}\nproc none {} {}\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // The cross-file arity code drawn by calling `src` (file A over {A, B}):
    // Some("E002") too few, Some("E003") too many, None if it fits.
    let arity_code = |src: &str| -> Option<String> {
        let a = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        let proj = Project::new(&db, vec![a, b]);
        project_diagnostics(&db, a, cfg, proj)
            .iter()
            .find(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
            .map(|d| d.code.to_string())
    };
    let e002 = || Some("E002".to_owned());
    let e003 = || Some("E003".to_owned());
    // two {a b} → arity (2, 2).
    assert_eq!(
        arity_code("two 1\n"),
        e002(),
        "1 arg to a 2-param proc → too few"
    );
    assert_eq!(arity_code("two 1 2\n"), None, "2 args → ok");
    assert_eq!(arity_code("two 1 2 3\n"), e003(), "3 args → too many");
    // opt {a {b 1}} → arity (1, 2).
    assert_eq!(arity_code("opt\n"), e002(), "0 args to (1,2) → too few");
    assert_eq!(arity_code("opt 1\n"), None, "1 arg to (1,2) → ok");
    assert_eq!(arity_code("opt 1 2\n"), None, "2 args to (1,2) → ok");
    assert_eq!(
        arity_code("opt 1 2 3\n"),
        e003(),
        "3 args to (1,2) → too many"
    );
    // variadic {a args} → arity (1, unbounded).
    assert_eq!(
        arity_code("variadic\n"),
        e002(),
        "0 args to (1,∞) → too few"
    );
    assert_eq!(arity_code("variadic 1\n"), None, "1 arg → ok");
    assert_eq!(
        arity_code("variadic 1 2 3 4 5\n"),
        None,
        "many → ok (trailing args)"
    );
    // none {} → arity (0, 0).
    assert_eq!(arity_code("none\n"), None, "0 args to a no-param proc → ok");
    assert_eq!(
        arity_code("none 1\n"),
        e003(),
        "1 arg to a 0-param proc → too many"
    );
}

/// A proc whose parameter-list word is **computed** has
/// unknown formals, so the *cross-file* arity check must abstain.
/// `ItemSig` must carry an "unknown, not none" flag distinct from a
/// genuinely empty parameter list — without it the cross-file table
/// would read the empty list as "takes no arguments" and every call
/// would draw a false `E003` — on code both tclsh 9.0.4 and 8.6.16 run
/// (`proc makeargs {} {return {a b}}`; `proc p [makeargs] {…}`;
/// `info args p` → `a b`; `p 1 2` → runs).
#[test]
fn cross_file_arity_abstains_for_computed_parameter_lists() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let b = SourceFile::new(
        &db,
        "proc makeargs {} { return {a b} }\n\
             proc computed [makeargs] { return 1 }\n\
             set params {x y}\n\
             proc dollar $params { return 1 }\n\
             proc literal {a b} { return 1 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let arity_code = |src: &str| -> Option<String> {
        let a = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        let proj = Project::new(&db, vec![a, b]);
        project_diagnostics(&db, a, cfg, proj)
            .iter()
            .find(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
            .map(|d| d.code.to_string())
    };
    // FP guards — every call count is acceptable for an unknown signature.
    for call in [
        "computed\n",
        "computed 1\n",
        "computed 1 2\n",
        "dollar 1 2\n",
    ] {
        assert_eq!(
            arity_code(call),
            None,
            "computed parameter list must abstain, not draw an arity error for {call:?}"
        );
    }
    // TP control — a literal list in the same file still checks arity.
    assert_eq!(arity_code("literal 1 2\n"), None, "2 args → ok");
    assert_eq!(
        arity_code("literal 1 2 3\n"),
        Some("E003".to_owned()),
        "a literal list is still checked"
    );
}

/// A required parameter
/// positioned *after* a defaulted one does not lower the minimum by
/// the defaulted parameters ahead of it — Tcl's argument binding is
/// strictly positional, so supplying a value for the later required
/// parameter requires also supplying one for every position before
/// it, including the "optional" one. Confirmed against real `tclsh`
/// 9.0.4: `proc opt {a {b 5} c} {}` accepts exactly 3 arguments,
/// never 2 — computing `min` as the count of non-default params (2)
/// would silently accept a 2-argument call that real Tcl rejects.
#[test]
fn cross_file_arity_required_after_default_forces_exact_count() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let b = SourceFile::new(
        &db,
        "proc opt {a {b 5} c} {}\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let arity_code = |src: &str| -> Option<String> {
        let a = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        let proj = Project::new(&db, vec![a, b]);
        project_diagnostics(&db, a, cfg, proj)
            .iter()
            .find(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
            .map(|d| d.code.to_string())
    };
    assert_eq!(
        arity_code("opt 1\n"),
        Some("E002".to_owned()),
        "1 arg → too few (min is 3, not 2)"
    );
    assert_eq!(
        arity_code("opt 1 2\n"),
        Some("E002".to_owned()),
        "2 args → still too few — real tclsh rejects this exact call"
    );
    assert_eq!(arity_code("opt 1 2 3\n"), None, "3 args → ok");
    assert_eq!(
        arity_code("opt 1 2 3 4\n"),
        Some("E003".to_owned()),
        "4 args → too many"
    );
}

/// Conservative arity: a `{*}`-expanded call has an
/// unknown runtime arg count (`argc == None`), so it must never draw an arity
/// error even though its literal word count looks wrong — while still resolving
/// (no W123).
#[test]
fn cross_file_arity_skips_expanded_call() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let b = SourceFile::new(
        &db,
        "proc two {a b} {}\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // `two {*}$lst` — one literal word, `{*}`-expanded → runtime arity unknown.
    let a = SourceFile::new(
        &db,
        "set lst {1 2 3}\ntwo {*}$lst\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let proj = Project::new(&db, vec![a, b]);
    let d = project_diagnostics(&db, a, cfg, proj);
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::E002 || x.code == DiagCode::E003),
        "a {{*}}-expanded call must not draw an arity error (argc unknown)"
    );
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::W123 && x.message.contains("two")),
        "the call still resolves cross-file (no W123)"
    );
}

/// Nested cross-file arity: a wrong-arg call to a
/// workspace proc *inside a command substitution* (`set x [helper a b c]`)
/// must still draw the cross-file arity error — the nested call's argument
/// count is statically known.
#[test]
fn cross_file_arity_in_command_substitution() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let b = SourceFile::new(
        &db,
        "proc helper {x y} { return $x }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // `helper` called with 3 args inside a `[…]` substitution → too many.
    let a = SourceFile::new(
        &db,
        "set x [helper a b c]\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let proj = Project::new(&db, vec![a, b]);
    let d = project_diagnostics(&db, a, cfg, proj);
    assert!(
        d.iter()
            .any(|x| x.code == DiagCode::E003 && x.message.contains("helper")),
        "nested cross-file call must draw E003, got: {:?}",
        d.iter().map(|x| (&x.code, &x.message)).collect::<Vec<_>>()
    );
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::W123 && x.message.contains("helper")),
        "the nested call still resolves cross-file (no W123)"
    );
}
