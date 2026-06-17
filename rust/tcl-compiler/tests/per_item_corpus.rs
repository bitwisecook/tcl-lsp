//! Slice-2b corpus gate: `analyse_per_item` must equal `analyse` byte-for-byte
//! over the real-world `tmp/` corpus (the per-item walk decomposition must
//! converge to exactly a full rebuild). Corpus-gated (`--ignored`), mirroring
//! `differential_incremental`.

use std::path::{Path, PathBuf};

use rayon::prelude::*;
use tcl_compiler::analyser::Analyser;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
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

#[test]
#[ignore = "corpus gate; run explicitly with --ignored (needs tmp/ trees)"]
fn per_item_matches_analyse_over_corpus() {
    let dialect = "tcl8.6";
    let mut files = Vec::new();
    for v in [
        "tcl8.4.20/library",
        "tcl8.5.19/library",
        "tcl8.6.16/library",
        "tcl9.0.3/library",
        "tcllib-2.0/modules",
    ] {
        gather(&repo_root().join("tmp").join(v), &mut files, 1500);
    }

    // Each file is analysed by independent `Analyser::new()` instances, so the
    // sweep parallelises cleanly across files (the bottleneck is the ~2 deep
    // analyses per file in a debug build). Collect a per-file outcome and
    // aggregate afterwards so the gate is order-independent.
    let outcomes: Vec<(bool, Option<String>)> = files
        .par_iter()
        .filter_map(|path| {
            let src = std::fs::read_to_string(path).ok()?;
            if src.len() > 400_000 {
                return None;
            }
            let want = Analyser::new().analyse(&src, dialect);
            let got = Analyser::new().analyse_per_item(&src, dialect);
            let fellback = !tcl_lexer::script_is_complete(&src) || src.contains("tcl-lsp: stub");
            let mismatch = (got != want).then(|| {
                let name = path.file_name().unwrap().to_string_lossy();
                let field = if got.all_procs != want.all_procs {
                    "all_procs"
                } else if got.all_classes != want.all_classes {
                    "all_classes"
                } else if got.global_scope != want.global_scope {
                    "global_scope"
                } else if got.diagnostics != want.diagnostics {
                    "diagnostics"
                } else if got.command_invocations != want.command_invocations {
                    "command_invocations"
                } else if got.all_variables != want.all_variables {
                    "all_variables"
                } else {
                    "other"
                };
                format!("{name}: field={field}")
            });
            Some((fellback, mismatch))
        })
        .collect();

    let checked = outcomes.len();
    let fellback = outcomes.iter().filter(|(f, _)| *f).count();
    let mismatches: Vec<String> = outcomes
        .into_iter()
        .filter_map(|(_, m)| m)
        .take(25)
        .collect();

    assert!(
        mismatches.is_empty(),
        "analyse_per_item != analyse in {} / {checked} files (fellback~{fellback}):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    eprintln!(
        "slice-2b gate: {checked} files, analyse_per_item == analyse ({fellback} trivially-fellback)"
    );
}

// Tiny deterministic xorshift PRNG so failures reproduce from the seed.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn upto(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            #[allow(clippy::cast_possible_truncation)]
            let bounded = (self.next() % n as u64) as usize;
            bounded
        }
    }
}

/// Apply one random edit, keeping `text` valid UTF-8 (edit on char boundaries).
fn random_edit(text: &str, rng: &mut Rng) -> String {
    let bounds: Vec<usize> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect();
    if bounds.len() < 2 {
        return format!("{text}\nset x 1\n");
    }
    let pick = |rng: &mut Rng| bounds[rng.upto(bounds.len())];
    match rng.upto(3) {
        0 => {
            let at = pick(rng);
            let snips = [
                "\nset y 2\n",
                " ",
                "# c\n",
                "puts $z",
                "}\nproc q {} {",
                "[",
                "$a",
                "\nvariable ns::v\n",
                "\nset ::g 1\n",
                "\nglobal g\n",
            ];
            let s = snips[rng.upto(snips.len())];
            format!("{}{}{}", &text[..at], s, &text[at..])
        }
        1 => {
            let a = pick(rng);
            let b = pick(rng);
            let (lo, hi) = (a.min(b), a.max(b));
            let hi = (lo + (hi - lo).min(12)).min(text.len());
            let hi = bounds
                .iter()
                .copied()
                .find(|&x| x >= hi)
                .unwrap_or(text.len());
            format!("{}{}", &text[..lo], &text[hi..])
        }
        _ => {
            let a = pick(rng);
            let b = pick(rng);
            let (lo, hi) = (a.min(b), a.max(b));
            format!("{}{}{}", &text[..lo], "X", &text[hi..])
        }
    }
}

/// Edit-based per-item gate: `analyse_per_item` must equal `analyse`
/// byte-for-byte after *every* random edit (not just on the pristine corpus).
/// This is the `incremental == fresh` contract for the per-item walk — the
/// permanent backstop for the widened enclosing-context fast path, which the
/// existing `differential_incremental` fuzzer does not cover (it exercises the
/// older `analyse_commands`-based `analyse_incremental`).  `analyse_per_item`
/// falls back to a full rebuild whenever it can't prove equivalence, so the
/// equality must hold unconditionally; a fast-path divergence is a real bug.
#[test]
#[ignore = "corpus fuzz; run explicitly with --ignored (needs tmp/ trees)"]
fn per_item_matches_analyse_under_edits() {
    let dialect = "tcl8.6";
    let mut files = Vec::new();
    for v in [
        "tcl8.4.20/library",
        "tcl8.5.19/library",
        "tcl8.6.16/library",
        "tcl9.0.3/library",
        "tcllib-2.0/modules",
    ] {
        gather(&repo_root().join("tmp").join(v), &mut files, 1500);
    }

    // Random edits can produce malformed transient inputs that make the
    // *reference* `analyse` itself panic (e.g. a non-char-boundary slice deep in
    // the expr lexer) — a pre-existing analyser-robustness concern orthogonal to
    // the incremental contract.  Silence the default panic hook so those
    // expected, caught panics don't spam the test log.
    std::panic::set_hook(Box::new(|_| {}));
    let run = |src: &str| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Analyser::new().analyse(src, dialect)
        }))
        .ok()
    };
    let run_pi = |src: &str| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Analyser::new().analyse_per_item(src, dialect)
        }))
        .ok()
    };

    let mut checked = 0usize;
    let mut steps = 0usize;
    let mut mismatches: Vec<String> = Vec::new();
    'files: for path in &files {
        let Ok(orig) = std::fs::read_to_string(path) else {
            continue;
        };
        if orig.is_empty() || orig.len() > 200_000 {
            continue;
        }
        checked += 1;
        // A few independent edit sequences per file (different seeds).
        for seed in [0x9E37_79B9_7F4A_7C15u64, 0xD1B5_4A32_D192_ED03] {
            let mut rng = Rng(seed ^ (orig.len() as u64).wrapping_mul(0x1000_0001B3));
            let mut text = orig.clone();
            for _ in 0..16 {
                text = random_edit(&text, &mut rng);
                if text.len() > 400_000 {
                    break;
                }
                let want = run(&text);
                let got = run_pi(&text);
                steps += 1;
                // If the reference panics on this malformed transient, there is
                // nothing to compare — but `analyse_per_item` must not diverge
                // (it either matches, or also panics by falling back to the same
                // `analyse`).  A one-sided panic, or differing results, is a bug.
                let bad = match (&want, &got) {
                    (Some(w), Some(g)) => w != g,
                    (None, None) => false,
                    _ => true,
                };
                if bad {
                    let name = path.file_name().unwrap().to_string_lossy();
                    let kind = match (&want, &got) {
                        (Some(_), None) => "per_item panicked, analyse ok",
                        (None, Some(_)) => "analyse panicked, per_item ok",
                        _ => "result mismatch",
                    };
                    mismatches.push(format!("{name} (seed {seed:#x}): {kind}"));
                    if mismatches.len() >= 25 {
                        break 'files;
                    }
                    break; // next seed/file
                }
            }
        }
    }
    let _ = std::panic::take_hook();

    assert!(
        mismatches.is_empty(),
        "analyse_per_item != analyse after edit in {} cases ({checked} files, {steps} steps):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    eprintln!("per-item edit gate: {checked} files, {steps} edited states, all byte-identical");
}
