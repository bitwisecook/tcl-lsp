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

//! Byte-identity guard for the `function_lattice` memo path.
//!
//! The memoised [`compiler_check_diagnostics`] (offset-0 per-procedure lattice +
//! rebase, via `build_for_memoized`) must be **byte-identical** to a fresh,
//! whole-module [`compiler_check_diagnostics_uncached`] build
//! (`build_for_with_config`) — they are the two halves of the salsa-native
//! lattice graph.
//!
//! The risk is *nondeterminism*, not an offset-0-vs-whole-module analysis
//! difference: several diagnostic producers fold over `HashMap`s, so their
//! iteration order must be pinned deterministic
//! (phi-span resolution, `run_all_checks` output order, the optimiser's group-id
//! allocation, W313's "first offending path variable", and the order-sensitive
//! `type_join` fold over a phi's predecessors), or
//! the memo and whole-module builds would disagree byte-for-byte.  See
//! `docs/design/rust/incremental-analysis.md` ("Memo byte-identity").

use std::path::{Path, PathBuf};

use salsa::Setter as _;
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, TclDb, compiler_check_diagnostics,
    compiler_check_diagnostics_if_available, compiler_check_diagnostics_uncached,
};
use tcl_test_support::fixed_corpus::{FixedCorpusSource, TCLLIB_2_0_HTTPD, TCLLIB_2_0_IMAP4};

mod common;
use common::Progress;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A default `AnalyserConfig` (no overrides) for the memo-vs-uncached
/// differential — both sides then use the built-in generic-name patterns.
fn default_config(db: &TclDatabase) -> AnalyserConfig {
    AnalyserConfig::new(
        db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn clay_source_inventory_retains_exact_httpd_server_method_sites_1956() {
    use tcl_compiler::analyser::Analyser;
    use tcl_lexer::{SourceImage, Span};
    use tcl_registry::definer::DefinitionMemberNamePurpose;

    let corpus = std::env::var_os("TCLLIB_2_0_DIR")
        .map_or_else(|| repo_root().join("tmp/tcllib-2.0"), PathBuf::from);
    let held = FixedCorpusSource::read(&corpus, &TCLLIB_2_0_HTTPD)
        .expect("#1956 requires the pinned Tcllib 2.0 HTTPD input");
    let source = std::str::from_utf8(held.bytes()).expect("HTTPD source is UTF-8");
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let declaration = analysis
        .original_class_declarations()
        .find(|declaration| declaration.metadata().qualified_name == "::httpd::server")
        .expect("actual original HTTPD server declaration");
    assert_eq!(declaration.declaration_site().offset, 20_371);
    let class = declaration.metadata();
    assert_eq!(class.name_span, Span::new(20_386, 20_401));
    let method = class
        .original_members
        .declarations()
        .find(|method| method.original_name_input().bytes() == b"connect")
        .expect("actual original connect method words");
    assert_eq!(method.metadata().name_span, Span::new(21_872, 21_879));
    assert_eq!(
        method.name_purpose(),
        DefinitionMemberNamePurpose::SourceValue
    );
    let parameters = method.parameters_word().expect("original method formals");
    let body = method.body_word().expect("original unchanged method body");
    let image = SourceImage::document(source);
    assert_eq!(parameters.image(), &image);
    assert_eq!(body.image(), &image);
    assert_eq!(parameters.span().start(), 21_880);
    assert_eq!(body.span().start(), 21_895);
    assert_eq!(
        method
            .metadata()
            .params
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>(),
        ["sock", "ip", "port"]
    );
    assert!(class.inheritance_unknown && class.member_set_incomplete);
    assert!(class.factory.is_none());
    assert!(
        tcl_compiler::registry_invocation::source_structure::source_class_declaration_at(
            source, &analysis, 20_371
        )
        .is_none()
    );
}

fn gather(dir: &Path, out: &mut Vec<PathBuf>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            gather(&p, out, cap);
        } else if p.extension().is_some_and(|x| x == "tcl") {
            out.push(p);
            if out.len() >= cap {
                return;
            }
        }
    }
}

/// Focused regression for the `proc_summary_cascade` memo: `graphops.tcl`'s
/// `::struct::graph::op::distance` tripped the debug fixpoint guard when the
/// memoised `proc_summary_cascade` reconstructed only the *reachable* summaries
/// — a resolved callee that was absent (rather than present-and-clean) made
/// `propagate_taints` fall through to its conservative bare-argument join and
/// over-taint.  Fast (one file), runs in debug so the guard is live.
#[test]
fn compiler_check_memo_matches_uncached_graphops() {
    let dialect = "tcl8.6";
    let path = repo_root().join("tmp/tcllib-2.0/modules/struct/graphops.tcl");
    let Ok(src) = std::fs::read_to_string(&path) else {
        eprintln!("skip: {} not present", path.display());
        return;
    };
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, default_config(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
    assert_eq!(
        got.checks, want.checks,
        "graphops checks diverge (memo vs uncached)"
    );
    assert_eq!(
        got.optimisations, want.optimisations,
        "graphops optimisations diverge (memo vs uncached)"
    );
}

/// Compare returned reports for the exact #2243 input. Agreement alone does
/// not establish source-read coverage; the separate audit records that limit.
#[test]
fn compiler_check_memo_matches_uncached_imap4_argc() {
    use tcl_compiler::compiler_checks::DiagCode;

    let dialect = "tcl8.6";
    let corpus = std::env::var_os("TCLLIB_2_0_DIR")
        .map_or_else(|| repo_root().join("tmp/tcllib-2.0"), PathBuf::from);
    let held = FixedCorpusSource::read(&corpus, &TCLLIB_2_0_IMAP4)
        .expect("#2243 requires the pinned Tcllib 2.0 IMAP4 input");
    let src = String::from_utf8(held.bytes().to_vec()).expect("Tcllib IMAP4 source is UTF-8");
    assert_eq!(src.as_bytes().get(43_921..43_926), Some(&b"$argc"[..]));
    assert_eq!(src.as_bytes().get(43_960..43_965), Some(&b"$argc"[..]));
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics_if_available(&db, file, default_config(&db))
        .expect("the pinned IMAP4 document must complete the checked provider");
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
    let got_t100: Vec<_> = got
        .checks
        .iter()
        .filter(|check| check.code == DiagCode::T100)
        .collect();
    let want_t100: Vec<_> = want
        .checks
        .iter()
        .filter(|check| check.code == DiagCode::T100)
        .collect();
    assert_eq!(got_t100, want_t100, "imap4 T100s diverge");
    assert_eq!(got.checks, want.checks, "imap4 checks diverge");
    assert_eq!(
        got.optimisations, want.optimisations,
        "imap4 optimisations diverge"
    );
    assert_eq!(
        got.taint_replays, want.taint_replays,
        "imap4 replays diverge"
    );
}

#[test]
fn compiler_check_memo_matches_uncached_argc_t100_controls() {
    use tcl_compiler::compiler_checks::DiagCode;

    let dialect = "tcl8.6";
    let profile = tcl_lsp_core::stated_profile_for_dialect(dialect);
    let surface = tcl_registry::special_vars::surface_query_for_profile(profile);
    assert!(tcl_registry::special_vars::is_initially_bound(
        "argc",
        Some(surface)
    ));
    assert!(
        tcl_registry::special_vars::special_var("argc")
            .is_some_and(|spec| spec.read_taint.is_none())
    );

    let quiet = "if {$argc > 3} {\n for {set i 3} {$i<$argc} {incr i} { puts $i }\n}\n";
    let positive = "proc p {} {\n set x [gets stdin]\n if {$x + 1 > 5} {puts big}\n}\n";
    let rebound = "set argc [gets stdin]\nif {$argc > 3} {puts big}\n";
    let unset_rebound = "unset argc\nset argc [gets stdin]\nif {$argc > 3} {puts big}\n";
    for (src, expected_subject) in [
        (quiet, None),
        (positive, Some("x")),
        (rebound, Some("argc")),
        (unset_rebound, Some("argc")),
    ] {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
        let got = memo_checked(&db, file, dialect, src);
        match expected_subject {
            Some(name) => assert!(
                got.checks.iter().any(|check| {
                    check.code == DiagCode::T100
                        && check
                            .taint_subject
                            .as_ref()
                            .is_some_and(|subject| subject.variable() == name)
                }),
                "missing T100 for {name}: {src}"
            ),
            None => assert!(
                got.checks.iter().all(|check| check.code != DiagCode::T100),
                "unexpected T100: {src}"
            ),
        }
    }
}

/// Focused regression for the per-function check memo:
/// `init.tcl` has `foreach` loops whose constant-true condition emits an O100
/// with a **`None`** span → the `(0, 0)` "unknown" sentinel.  The per-proc memo
/// returns offset-0 spans rebased by `body_offset`, but that sentinel must be
/// left alone (the whole-module build rebases the `Option<Span>` before lowering,
/// so `None` stays `(0, 0)`).  A blanket `+ body_offset` regressed this; fast
/// (one file) so it pins the fix without the corpus sweep.
#[test]
fn compiler_check_memo_matches_uncached_init() {
    let dialect = "tcl8.6";
    let path = repo_root().join("tmp/tcl8.6.18/library/init.tcl");
    let Ok(src) = std::fs::read_to_string(&path) else {
        eprintln!("skip: {} not present", path.display());
        return;
    };
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, default_config(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
    assert_eq!(
        got.checks, want.checks,
        "init.tcl checks diverge (memo vs uncached)"
    );
}

/// `generalClasses.tcl` is ~140 KB of `TclOO`: almost all of its code
/// lives in *method* bodies, which are not procedures and so never reach
/// `proc_taint_solve`'s `analysable_functions` loop.  That query's top-up over
/// `analysable_methods_and_body_units` must run the whole
/// `function_nontaint_checks` family, not only `shimmer_family_checks`:
/// running only the shimmer half would report 20 fewer
/// `O100`/`O105`/`O106` hints than the direct build.  The
/// procedural `tmp/tcl*/library` + `tcllib` sweep below cannot see this: those
/// trees define practically no `TclOO` methods.
///
/// Corpus-gated (`experiments/corpus/fetch_corpus.sh`), so it skips when the
/// checkout is absent, exactly like the `tmp/` fixtures above.
#[test]
fn compiler_check_memo_matches_uncached_tcloo_corpus() {
    let dialect = "tcl8.6";
    let path = repo_root().join("experiments/corpus/georgtree/SpiceGenTcl/src/generalClasses.tcl");
    let Ok(src) = std::fs::read_to_string(&path) else {
        eprintln!("skip: {} not present", path.display());
        return;
    };
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, default_config(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
    assert_eq!(
        got.checks.len(),
        want.checks.len(),
        "generalClasses.tcl check count diverges (memo {} vs uncached {}) — #1117",
        got.checks.len(),
        want.checks.len()
    );
    assert_eq!(
        got.checks, want.checks,
        "generalClasses.tcl checks diverge (memo vs uncached) — #1117"
    );
    assert_eq!(
        got.optimisations, want.optimisations,
        "generalClasses.tcl optimisations diverge (memo vs uncached)"
    );
}

#[test]
#[ignore = "slow corpus sweep (~100s over tmp/); run with --ignored"]
fn compiler_check_memo_matches_uncached_over_corpus() {
    let dialect = "tcl8.6";
    let mut files = Vec::new();
    for v in [
        "tcl8.4.20/library",
        "tcl8.5.19/library",
        "tcl8.6.18/library",
        "tcl9.0.4/library",
        "tcllib-2.0/modules",
    ] {
        gather(&repo_root().join("tmp").join(v), &mut files, 1500);
    }

    let (files, start0, total) = Progress::slice(&files);
    let mut prog = Progress::new("compiler_check_gate");
    let db = TclDatabase::default();
    let mut checked = 0usize;
    let mut bad: Vec<String> = Vec::new();
    for (idx, path) in files.iter().enumerate() {
        let name = path
            .strip_prefix(repo_root())
            .unwrap_or(path)
            .display()
            .to_string();
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if src.is_empty() || src.len() > 400_000 {
            continue;
        }
        let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
        let got = compiler_check_diagnostics(&db, file, default_config(&db));
        let registry = db.registry(dialect);
        let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
        checked += 1;
        if got.checks != want.checks || got.optimisations != want.optimisations {
            let detail = format!(
                "{name}: checks {}->{} opts {}->{}",
                want.checks.len(),
                got.checks.len(),
                want.optimisations.len(),
                got.optimisations.len()
            );
            prog.finding(&detail);
            if bad.len() < 40 {
                bad.push(detail);
            }
        }
        if (idx + 1) % 25 == 0 || idx + 1 == files.len() {
            prog.tick(start0 + idx + 1, total, &format!("last={name}"));
        }
    }
    prog.finish(&format!("{checked} files, {} mismatches", bad.len()));

    assert!(
        bad.is_empty(),
        "compiler_check_diagnostics (memo) != uncached in {}+ / {checked} files:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// Corpus-scale **random-edit** differential for the memoised checks path
/// over real source. The sweep above compares memo vs
/// uncached on a *fresh* db per file; this drives each file through a fuzzed
/// **edit sequence** on one **warm** db — prepending blank lines (an offset
/// shift the per-proc memo must reuse), appending a fuzzed synthetic proc (a
/// re-analysed body), and toggling that proc's shape — asserting the warm
/// memoised `compiler_check_diagnostics` stays byte-identical to a from-scratch
/// `compiler_check_diagnostics_uncached` after every edit.  A stale per-proc
/// lattice / summary cache surviving an edit would diverge here where the
/// fresh-db sweep cannot see it.
#[test]
#[ignore = "slow corpus random-edit sweep (~minutes over tmp/); run with --ignored"]
#[allow(clippy::cast_possible_truncation)] // index modulo tiny arrays
fn compiler_check_memo_matches_uncached_under_corpus_edits() {
    let dialect = "tcl8.6";
    let mut files = Vec::new();
    for v in [
        "tcl8.6.18/library",
        "tcllib-2.0/modules",
        "tcl9.0.4/library",
    ] {
        gather(&repo_root().join("tmp").join(v), &mut files, 120);
    }
    files.sort();

    // Fuzzed trailing-proc shapes (exercise SCCP const-branch, loop optimisations,
    // a taint passthrough+sink, and a plain body).
    let tail_procs = [
        "",
        "\nproc __fuzz_tail {n} { if {1} { set y 1 }\n return $y }\n",
        "\nproc __fuzz_tail {n} { set acc 0\n for {set i 0} {$i < $n} {incr i} { set acc [expr {$acc + $i}] }\n return $acc }\n",
        "\nproc __fuzz_tail {} { set u [gets stdin]; exec $u }\n",
    ];

    let mut rng = 0x5eed_1234_abcd_0001_u64;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };

    let (files, start0, total) = Progress::slice(&files);
    let mut prog = Progress::new("compiler_check_edit_fuzz");
    let mut bad: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for (idx, path) in files.iter().enumerate() {
        let name = path
            .strip_prefix(repo_root())
            .unwrap_or(path)
            .display()
            .to_string();
        let Ok(base) = std::fs::read_to_string(path) else {
            continue;
        };
        if base.is_empty() || base.len() > 200_000 {
            continue;
        }
        let mut db = TclDatabase::default();
        let registry = db.registry(dialect);
        let file = SourceFile::new(&db, base.clone(), dialect.to_owned(), None);

        for _ in 0..8 {
            let blanks = (next() as usize) % 4; // offset shift above the real procs
            let tail = tail_procs[(next() as usize) % tail_procs.len()];
            let src = format!("{}{base}{tail}", "\n".repeat(blanks));
            file.set_text(&mut db).to(src.clone());

            let got = compiler_check_diagnostics(&db, file, default_config(&db));
            let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
            checked += 1;
            if got.checks != want.checks || got.optimisations != want.optimisations {
                let detail = format!(
                    "{name} (blanks={blanks} tail_len={}): checks {}->{} opts {}->{}",
                    tail.len(),
                    want.checks.len(),
                    got.checks.len(),
                    want.optimisations.len(),
                    got.optimisations.len()
                );
                prog.finding(&detail);
                if bad.len() < 40 {
                    bad.push(detail);
                }
                break; // one report per file is enough
            }
        }
        prog.tick(
            start0 + idx + 1,
            total,
            &format!("last={name} steps={checked}"),
        );
    }
    prog.finish(&format!("{checked} steps, {} findings", bad.len()));

    assert!(
        bad.is_empty(),
        "memo != uncached under edits in {}+ files ({checked} edit-steps checked):\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// The memoised and the whole-module build agree on `src`, byte for byte, and
/// return what the memoised one drew.
fn memo_checked(
    db: &TclDatabase,
    file: SourceFile,
    dialect: &str,
    src: &str,
) -> std::sync::Arc<tcl_lsp_db::CompilerDiagnostics> {
    let got = compiler_check_diagnostics(db, file, default_config(db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(src, registry, dialect, None, None);
    assert_eq!(
        got.checks, want.checks,
        "checks diverge (memo vs uncached): {src}"
    );
    assert_eq!(
        got.optimisations, want.optimisations,
        "optimisations diverge (memo vs uncached): {src}"
    );
    got
}

/// A write no statement of a procedure shows — an arm of a `switch` the flow
/// graph keeps as one statement, directly or through a callee that sets the
/// caller's variable, a variable trace's callback run in the procedure's
/// frame — leaves the procedure's stale value unfolded on the memoised path
/// exactly as on the whole-module one, and an edit that adds such a callback
/// re-keys the lattice, so a warm memo never serves the fold it made wrong.
/// An `after` script, with or without a `list`-built prefix, runs at level
/// `#0`, as does a trace callback that names `::go`: neither reaches the
/// procedure's local `go`, so the fold stands (tclsh 8.4 to 9.1 print `a`
/// and `1` from `p` before and after the event fires).
#[test]
fn compiler_check_memo_matches_uncached_for_hidden_writes() {
    use tcl_compiler::compiler_checks::DiagCode;
    let dialect = "tcl8.6";
    let folds = |opts: &[tcl_compiler::optimiser::Optimisation]| {
        opts.iter().any(|o| {
            o.code == DiagCode::O112
                || (matches!(o.code, DiagCode::O100 | DiagCode::O102) && o.replacement == "1")
        })
    };
    let control = "proc p {} {\n set go 1\n if {$go} { puts a } else { puts b }\n puts $go\n}\n";
    let local_trace = "proc p {} {\n set go 1\n trace add variable x write { set go 0 ;# }\n set x 1\n if {$go} { puts a } else { puts b }\n puts $go\n}\n";
    for src in [
        "proc p {s} {\n set go 1\n switch -glob -- $s { q* { set go 0 } }\n if {$go} { puts a } else { puts b }\n puts $go\n}\n",
        local_trace,
        "proc zero {v} {upvar 1 $v x; set x 0}\nproc p {s} {\n set go 1\n switch -glob -- $s { q* { zero go } }\n if {$go} { puts a } else { puts b }\n puts $go\n}\n",
    ] {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
        let got = memo_checked(&db, file, dialect, src);
        assert!(!folds(&got.optimisations), "{src}: {:?}", got.optimisations);
    }
    for src in [
        "after 100 { set go 0 }\nproc p {} {\n set go 1\n if {$go} { puts a } else { puts b }\n puts $go\n}\n",
        "proc p {} {\n set go 1\n trace add variable x write { set ::go 0 ;# }\n set x 1\n if {$go} { puts a } else { puts b }\n puts $go\n}\n",
        "after 100 [list set go 0]\nproc p {} {\n set go 1\n if {$go} { puts a } else { puts b }\n puts $go\n}\n",
    ] {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
        let got = memo_checked(&db, file, dialect, src);
        assert!(folds(&got.optimisations), "{src}: {:?}", got.optimisations);
    }
    // The control folds; an `after` callback added elsewhere writes `::go`
    // and leaves the fold, and the edit that adds a frame-local trace
    // callback stops it.
    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, control.to_owned(), dialect.to_owned(), None);
    let got = memo_checked(&db, file, dialect, control);
    assert!(folds(&got.optimisations), "{:?}", got.optimisations);
    let edited = format!("after 100 {{ set go 0 }}\n{control}");
    file.set_text(&mut db).to(edited.clone());
    let got = memo_checked(&db, file, dialect, &edited);
    assert!(folds(&got.optimisations), "{:?}", got.optimisations);
    file.set_text(&mut db).to(local_trace.to_owned());
    let got = memo_checked(&db, file, dialect, local_trace);
    assert!(!folds(&got.optimisations), "{:?}", got.optimisations);
}

/// The iRules flow checks read the same lattice on the memoised path: a flag
/// a `switch` arm sets leaves the guarded `HTTP::respond` and `drop`
/// reachable, so IRULE1201 and IRULE5002 are drawn, and an arm's write of
/// `svc` is a second write that keeps IRULE4004 off `set svc foo`.
#[test]
fn compiler_check_memo_matches_uncached_for_irules_flow_checks() {
    use tcl_compiler::compiler_checks::DiagCode;
    let dialect = "f5-irules";
    let drawn = |src: &str, code: DiagCode| {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
        memo_checked(&db, file, dialect, src)
            .checks
            .iter()
            .any(|check| check.code == code)
    };
    assert!(drawn(
        "when HTTP_REQUEST { set is_api 0; switch -glob [HTTP::uri] { \"/api*\" { set is_api 1 } }; if {$is_api} { HTTP::respond 403 }; HTTP::header insert X-Seen 1 }\n",
        DiagCode::Irule1201
    ));
    assert!(drawn(
        "when CLIENT_ACCEPTED { set bad 0; switch -glob [IP::client_addr] { 10.* { set bad 1 } }; if {$bad} { drop } }\n",
        DiagCode::Irule5002
    ));
    assert!(!drawn(
        "when HTTP_REQUEST { set svc foo; switch -glob [HTTP::uri] { /a* { set svc bar } }; pool $svc }\n",
        DiagCode::Irule4004
    ));
}
