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

//! `cargo xtask tcltest-sweep` — run the upstream C `tcltest` `.test` files
//! through the bytecode VM and reference `tclsh`, per file, and (re)generate the
//! VM-vs-C parity scoreboard `docs/design/runtime/rust-vm-tier-parity.md`.
//!
//! This is the Rust-native replacement for the retired `scripts/tcltest_sweep/`
//! Python sweep. Each backend runs a `.test` file under a `timeout` (so a hang
//! or native-stack overflow can't stall the sweep), and `tcltest` itself prints
//! the `Total N Passed X Skipped Y Failed Z` summary the sweeper parses.
//!
//! - VM backend = the `run_test` example (`rust/tcl-vm/examples/run_test.rs`),
//!   built `--release` once and invoked per stem.
//! - C backend  = the locally-built reference `tclsh9.0`; its results are the
//!   stable baseline, cached in `tests/baselines/tcl9-tcltest/c-tclsh.ndjson`.
//!
//! Stems are grouped by the capability ladder in
//! `docs/design/runtime/tcl-test-tiers.md`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tcl_dialect::TclVersion;
use tcl_test_support::{TclSourceTree, locate_source_tree, locate_tclsh};

use crate::util::{license_banner, repo_root};

/// The capability-ladder tiers and the upstream `.test` stems in each, from
/// `docs/design/runtime/tcl-test-tiers.md`. Only stems with a file under
/// `tmp/tcl9.0.4/tests/` are swept; a missing file is reported, not fatal.
const TIERS: &[(u8, &str, &[&str])] = &[
    (
        1,
        "Parsing",
        &["parse", "parseOld", "parseExpr", "word", "subst"],
    ),
    (
        2,
        "Interpretation",
        &[
            "basic",
            "compile",
            "execute",
            "eval",
            "obj",
            "nre",
            "appendComp",
            "lsetComp",
            "regexpComp",
            "compExpr",
            "compExpr-old",
            "misc",
        ],
    ),
    (
        3,
        "Fundamentals",
        &[
            "var",
            "set",
            "set-old",
            "append",
            "incr",
            "incr-old",
            "upvar",
            "uplevel",
            "get",
            "namespace",
            "namespace-old",
            "resolver",
            "trace",
            "rename",
            "info",
            "cmdInfo",
            "indexObj",
            "chan",
            "io",
        ],
    ),
    (
        4,
        "Data types",
        &[
            "abstractlist",
            "assocd",
            "concat",
            "dict",
            "dstring",
            "format",
            "join",
            "lindex",
            "linsert",
            "list",
            "listObj",
            "listRep",
            "llength",
            "lmap",
            "lpop",
            "lrange",
            "lrepeat",
            "lreplace",
            "lsearch",
            "lseq",
            "lset",
            "range",
            "reg",
            "regexp",
            "scan",
            "split",
            "stack",
            "string",
            "stringObj",
            "cmdIL",
            "cmdMZ",
        ],
    ),
    (
        5,
        "Control flow & procs",
        &[
            "apply",
            "error",
            "expr",
            "expr-old",
            "for",
            "for-old",
            "foreach",
            "if",
            "if-old",
            "mathop",
            "proc",
            "proc-old",
            "result",
            "switch",
            "unknown",
            "while",
            "while-old",
        ],
    ),
    (6, "TclOO", &["oo", "ooNext2", "ooProp", "ooUtil"]),
    (7, "Packages & loading", &["source"]),
    (
        8,
        "Interpreters",
        &["interp", "safe", "safe-stock", "safe-stock86"],
    ),
    (9, "Advanced I/O & events", &["chanio", "ioCmd"]),
    (10, "Concurrency", &["coroutine", "tailcall"]),
    (
        11,
        "Platform & library",
        &["aaa_exit", "brodnik", "cmdAH", "opt"],
    ),
];

const SCOREBOARD: &str = "docs/design/runtime/rust-vm-tier-parity.md";
const BASELINE: &str = "tests/baselines/tcl9-tcltest/c-tclsh.ndjson";
/// Per-stem working directory root (under `target/`, so it's git-ignored). File-
/// creating tests (`io`, `source`, `fCmd`, …) write here, not into the repo.
const WORK_DIR: &str = "target/tcltest-sweep-work";
const DEFAULT_TIMEOUT_S: u64 = 120;

/// Which backend(s) to run.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Run both the VM and reference `tclsh`, refresh the C baseline + scoreboard.
    Both,
    /// Run only the VM; take the C column from the committed baseline.
    Vm,
    /// Run only reference `tclsh`; refresh the C baseline (no scoreboard).
    Tclsh,
}

impl std::str::FromStr for Backend {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "both" => Ok(Self::Both),
            "vm" => Ok(Self::Vm),
            "tclsh" => Ok(Self::Tclsh),
            other => bail!("unknown backend {other:?}; expected vm|tclsh|both"),
        }
    }
}

/// One backend's outcome for one stem.
#[derive(Clone, Serialize, Deserialize)]
struct Record {
    stem: String,
    tier: u8,
    /// `ran` (summary parsed), `crash` (no summary / nonzero exit), `timeout`,
    /// or `missing` (no `.test` file).
    outcome: String,
    passed: u32,
    skipped: u32,
    failed: u32,
    /// A short crash detail (last stderr/stdout line) for `crash`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    detail: String,
    duration_s: f64,
}

impl Record {
    /// The `P/S/F` scoreboard cell, or `ERROR`/`TIMEOUT`/`(no file)`.
    fn cell(&self) -> String {
        match self.outcome.as_str() {
            "ran" => format!("{}/{}/{}", self.passed, self.skipped, self.failed),
            "timeout" => "TIMEOUT".to_owned(),
            "missing" => "(no file)".to_owned(),
            _ => "ERROR".to_owned(),
        }
    }
}

/// The `tier` for a stem, or `0` if it is not in the ladder table.
fn tier_of(stem: &str) -> u8 {
    for (n, _, stems) in TIERS {
        if stems.contains(&stem) {
            return *n;
        }
    }
    0
}

/// All swept stems, in ladder order.
fn all_stems() -> Vec<&'static str> {
    TIERS
        .iter()
        .flat_map(|(_, _, s)| s.iter().copied())
        .collect()
}

/// Parse the *last* `Total N Passed X Skipped Y Failed Z` line tcltest prints.
fn parse_summary(out: &str, re: &Regex) -> Option<(u32, u32, u32)> {
    re.captures_iter(out).last().map(|c| {
        let g = |i: usize| c[i].parse::<u32>().unwrap_or(0);
        (g(2), g(3), g(4))
    })
}

fn last_nonempty_line(s: &str) -> String {
    s.lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .chars()
        .take(200)
        .collect()
}

/// Run one `.test` file through `argv…` under a `timeout`, returning its record.
fn run_one(
    root: &Path,
    source_tree: &TclSourceTree,
    argv: &[&Path],
    stem: &str,
    match_filter: Option<&str>,
    timeout_s: u64,
    re: &Regex,
) -> Record {
    let tier = tier_of(stem);
    let testfile = source_tree.tests_dir().join(format!("{stem}.test"));
    if !testfile.is_file() {
        return Record {
            stem: stem.to_owned(),
            tier,
            outcome: "missing".to_owned(),
            passed: 0,
            skipped: 0,
            failed: 0,
            detail: String::new(),
            duration_s: 0.0,
        };
    }
    // Run in a fresh per-stem working dir so file-creating tests don't pollute the
    // repo (the test file is passed as an absolute path, so `info script` still
    // resolves the testsDirectory). Recreated each run for isolation.
    let work = root.join(WORK_DIR).join(stem);
    let _ = std::fs::remove_dir_all(&work);
    let _ = std::fs::create_dir_all(&work);

    let start = Instant::now();
    let mut cmd = Command::new("timeout");
    // Default SIGTERM at the budget so coreutils `timeout` exits a clean 124;
    // `--kill-after` is a SIGKILL backstop for a process that ignores TERM.
    cmd.arg("--kill-after=15").arg(timeout_s.to_string());
    for a in argv {
        cmd.arg(a);
    }
    cmd.arg(&testfile);
    if let Some(pattern) = match_filter {
        cmd.arg("-match").arg(pattern);
    }
    cmd.env("TCL_LIBRARY", source_tree.library_dir())
        .current_dir(&work);
    let out = cmd.output();
    let duration_s = start.elapsed().as_secs_f64();
    let _ = std::fs::remove_dir_all(&work);

    let (outcome, passed, skipped, failed, detail) = match out {
        Err(e) => ("crash".to_owned(), 0, 0, 0, format!("spawn failed: {e}")),
        Ok(o) => {
            // `timeout` exits 124 on TERM, 137 on a KILL backstop; a KILL that
            // also fells `timeout` shows up as a signal (no exit code). A genuine
            // VM abort (SIGABRT/SIGSEGV) is a crash, not a timeout.
            let timed_out = matches!(o.status.code(), Some(124 | 137))
                || matches!(o.status.signal(), Some(9 | 15));
            if timed_out {
                ("timeout".to_owned(), 0, 0, 0, String::new())
            } else {
                let stdout = String::from_utf8_lossy(&o.stdout);
                if let Some((p, s, f)) = parse_summary(&stdout, re) {
                    ("ran".to_owned(), p, s, f, String::new())
                } else {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    let mut d = last_nonempty_line(&stderr);
                    if d.is_empty() {
                        d = last_nonempty_line(&stdout);
                    }
                    ("crash".to_owned(), 0, 0, 0, d)
                }
            }
        }
    };
    Record {
        stem: stem.to_owned(),
        tier,
        outcome,
        passed,
        skipped,
        failed,
        detail,
        duration_s,
    }
}

/// Build the VM `run_test` example once (`--release`) and return its binary path.
fn build_vm_runner(root: &Path) -> Result<PathBuf> {
    eprintln!("==> building tcl-vm run_test example (release)…");
    let status = Command::new(std::env::var("CARGO").as_deref().unwrap_or("cargo"))
        .args([
            "build",
            "-p",
            "tcl-vm",
            "--example",
            "run_test",
            "--release",
        ])
        .current_dir(root)
        .status()
        .context("spawning cargo build for run_test")?;
    if !status.success() {
        bail!("cargo build -p tcl-vm --example run_test --release failed");
    }
    let bin = root.join("target/release/examples/run_test");
    if !bin.is_file() {
        bail!("run_test binary not found at {}", bin.display());
    }
    Ok(bin)
}

/// Sweep `stems` through one backend command, logging progress.
fn sweep(
    root: &Path,
    source_tree: &TclSourceTree,
    label: &str,
    argv: &[&Path],
    stems: &[&str],
    match_filter: Option<&str>,
    timeout_s: u64,
) -> Vec<Record> {
    let re = Regex::new(r"Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)")
        .expect("valid summary regex");
    let total = stems.len();
    let mut records = Vec::with_capacity(total);
    for (i, stem) in stems.iter().enumerate() {
        eprint!("[{label} {}/{total}] {stem} … ", i + 1);
        let r = run_one(root, source_tree, argv, stem, match_filter, timeout_s, &re);
        eprintln!("{} ({:.1}s)", r.cell(), r.duration_s);
        records.push(r);
    }
    records
}

fn write_baseline(root: &Path, records: &[Record]) -> Result<()> {
    let path = root.join(BASELINE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    }
    let mut out = String::new();
    for r in records {
        out.push_str(&serde_json::to_string(r).expect("serialize record"));
        out.push('\n');
    }
    std::fs::write(&path, out).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn read_baseline(root: &Path) -> Result<BTreeMap<String, Record>> {
    let path = root.join(BASELINE);
    let text = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "reading C baseline {} — run `cargo xtask tcltest-sweep --backend both` first",
            path.display()
        )
    })?;
    let mut map = BTreeMap::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let r: Record = serde_json::from_str(line).context("parsing baseline record")?;
        map.insert(r.stem.clone(), r);
    }
    Ok(map)
}

/// Classify a stem's VM result vs its C reference.
fn status(c: &Record, vm: &Record) -> &'static str {
    match vm.outcome.as_str() {
        "timeout" => "TIMEOUT",
        "crash" | "missing" => "CRASH",
        _ => {
            if c.outcome == "ran"
                && (vm.passed, vm.skipped, vm.failed) == (c.passed, c.skipped, c.failed)
            {
                "MATCH"
            } else {
                "gap"
            }
        }
    }
}

/// Render the scoreboard markdown from the C + VM record maps.
fn render_scoreboard(c: &BTreeMap<String, Record>, vm: &BTreeMap<String, Record>) -> String {
    let mut matched = 0u32;
    let mut gap = 0u32;
    let mut crash = 0u32;
    let mut timeout = 0u32;
    let mut total = 0u32;
    let mut body = String::new();

    for (tier, name, stems) in TIERS {
        let _ = writeln!(body, "\n## Tier {tier} — {name}\n");
        body.push_str("| stem | C P/S/F | VM P/S/F | status |\n|---|---|---|---|\n");
        for stem in *stems {
            let (Some(cr), Some(vr)) = (c.get(*stem), vm.get(*stem)) else {
                continue;
            };
            let st = status(cr, vr);
            match st {
                "MATCH" => matched += 1,
                "gap" => gap += 1,
                "TIMEOUT" => timeout += 1,
                _ => crash += 1,
            }
            total += 1;
            let _ = writeln!(body, "| {stem} | {} | {} | {} |", cr.cell(), vr.cell(), st);
        }
    }

    let mut out = license_banner("#");
    out.push('\n');
    let _ = write!(
        out,
        "# Rust bytecode VM — tcltest parity scoreboard\n\n\
         _Regenerate with `cargo xtask tcltest-sweep --backend both` (Tcl 9.0.4)._\n\n\
         Goal: the VM's (passed/skipped/failed) per stem matches C Tcl 9 exactly.\n\
         `CRASH` = an uncaught error / timeout aborts the file (highest leverage — one\n\
         fix unlocks it). `gap` = ran but the counts differ. Columns: **C P/S/F** vs\n\
         **VM P/S/F**. Grouped by the capability ladder\n\
         ([`tcl-test-tiers.md`](tcl-test-tiers.md)).\n\n\
         **Tally: {matched} MATCH · {gap} gap · {crash} crash · {timeout} timeout** \
         of {total} stems.\n"
    );
    out.push_str(&body);
    out
}

fn selected_stems(stem_filters: &[String]) -> Vec<&str> {
    if stem_filters.is_empty() {
        return all_stems();
    }
    stem_filters
        .iter()
        .map(String::as_str)
        .inspect(|stem| {
            let stem = *stem;
            if tier_of(stem) == 0 {
                eprintln!("note: stem {stem:?} is not in the ladder table (tier 0)");
            }
        })
        .collect()
}

fn source_tree_for_sweep(
    root: &Path,
    tcl_root: Option<&Path>,
    focused: bool,
) -> Result<TclSourceTree> {
    let source_tree = locate_source_tree(root, TclVersion::V9_0, tcl_root)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no Tcl 9.0 source tree found; pass --tcl-root, set \
             TCL_LSP_TCL_ROOT90, or fetch the pinned tmp/tcl9.0.4 tree"
        )
    })?;
    if !focused && source_tree.patchlevel != TclVersion::V9_0.patchlevel() {
        bail!(
            "a full scoreboard run requires the pinned Tcl {}, but {} contains \
             Tcl {}; use this tree only with focused --stem runs",
            TclVersion::V9_0.patchlevel(),
            source_tree.root.display(),
            source_tree.patchlevel
        );
    }
    Ok(source_tree)
}

fn sweep_reference(
    root: &Path,
    source_tree: &TclSourceTree,
    stems: &[&str],
    match_filter: Option<&str>,
    timeout_s: u64,
) -> Result<Vec<Record>> {
    let tclsh = locate_tclsh(TclVersion::V9_0)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no reference tclsh9.0 found; set TCL_LSP_TCLSH90 or run \
             `make ensure-test-deps`"
        )
    })?;
    if tclsh.patchlevel != source_tree.patchlevel {
        bail!(
            "reference {} reports Tcl {}, but source tree {} is Tcl {}; \
             the oracle binary and tests must match",
            tclsh.path.display(),
            tclsh.patchlevel,
            source_tree.root.display(),
            source_tree.patchlevel
        );
    }
    Ok(sweep(
        root,
        source_tree,
        "C",
        &[tclsh.path.as_path()],
        stems,
        match_filter,
        timeout_s,
    ))
}

/// `cargo xtask tcltest-sweep`.
pub fn run(
    backend: Backend,
    stem_filters: &[String],
    match_filter: Option<&str>,
    tcl_root: Option<&Path>,
    timeout_s: Option<u64>,
    check: bool,
) -> Result<ExitCode> {
    let root = repo_root();
    let timeout_s = timeout_s.unwrap_or(DEFAULT_TIMEOUT_S);
    if match_filter.is_some() && stem_filters.is_empty() {
        bail!("--match requires at least one --stem");
    }
    let focused = !stem_filters.is_empty() || match_filter.is_some();
    // Explicit stems form a focused run; no stems means the whole ladder.
    let stems = selected_stems(stem_filters);
    let source_tree = source_tree_for_sweep(&root, tcl_root, focused)?;

    // Run the reference tclsh (unless VM-only).
    let c_records: Vec<Record> = if backend == Backend::Vm {
        Vec::new()
    } else {
        sweep_reference(&root, &source_tree, &stems, match_filter, timeout_s)?
    };

    // Run the VM (unless tclsh-only).
    let vm_records: Vec<Record> = if backend == Backend::Tclsh {
        Vec::new()
    } else {
        let bin = build_vm_runner(&root)?;
        sweep(
            &root,
            &source_tree,
            "VM",
            &[bin.as_path()],
            &stems,
            match_filter,
            timeout_s,
        )
    };

    // Persist the C baseline whenever tclsh ran a full sweep.
    if backend != Backend::Vm && !focused {
        write_baseline(&root, &c_records)?;
        eprintln!("wrote C baseline → {BASELINE}");
    }

    // A tclsh-only run just refreshes the baseline (no scoreboard).
    if backend == Backend::Tclsh {
        return Ok(ExitCode::SUCCESS);
    }

    // Assemble C + VM maps (C from this run, or the cached baseline).
    let c_map: BTreeMap<String, Record> = if backend == Backend::Vm {
        read_baseline(&root)?
    } else {
        c_records.into_iter().map(|r| (r.stem.clone(), r)).collect()
    };
    let vm_map: BTreeMap<String, Record> = vm_records
        .into_iter()
        .map(|r| (r.stem.clone(), r))
        .collect();

    // Focused runs just print; they never rewrite the committed scoreboard.
    if focused {
        for s in &stems {
            let c = c_map.get(*s);
            let vm = vm_map.get(*s);
            if let (Some(c), Some(vm)) = (c, vm) {
                println!("{s}: C {} | VM {} | {}", c.cell(), vm.cell(), status(c, vm));
            } else {
                println!(
                    "{s}: incomplete (C {:?}, VM {:?})",
                    c.is_some(),
                    vm.is_some()
                );
            }
        }
        return Ok(ExitCode::SUCCESS);
    }

    let rendered = render_scoreboard(&c_map, &vm_map);
    let path = root.join(SCOREBOARD);
    if check {
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if current == rendered {
            eprintln!("scoreboard up to date");
            Ok(ExitCode::SUCCESS)
        } else {
            eprintln!("scoreboard drift — run `cargo xtask tcltest-sweep --backend both`");
            Ok(ExitCode::FAILURE)
        }
    } else {
        std::fs::write(&path, &rendered).with_context(|| format!("writing {}", path.display()))?;
        eprintln!("wrote scoreboard → {SCOREBOARD}");
        Ok(ExitCode::SUCCESS)
    }
}
