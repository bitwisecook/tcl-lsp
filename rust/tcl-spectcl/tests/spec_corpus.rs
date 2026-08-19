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

//! **The `SpecTcl` corpus-validation harness** — every `.tclspec` the repo
//! ships, loaded, installed, analysed, and run through the sandboxed hook
//! host, plus the hostile pack that proves containment.
//!
//! `docs/design/spec-packs.md` states the guarantees this file turns into a
//! gate. Per pack:
//!
//! 1. **Load** through the real loader ([`tcl_spectcl::pack::load`]) with
//!    **zero unexpected notices**. What counts as expected is
//!    `spec_corpus_baseline.txt` beside this file — one accepted notice per
//!    line, regenerated with `SPECTCL_CORPUS_BLESS=1`.
//! 2. **Install** into a real per-profile registry through the real install
//!    path, then run the **compiler/analyser and the optimiser** over corpus
//!    Tcl.
//! 3. **Exercise the hooks through the sandboxed host** — `tcl-spec-hooks`
//!    with `HostConfig::default()`, so the shipped budgets and the shipped
//!    quarantine policy are what run. No quarantine and no crash record may
//!    occur for a shipped pack.
//! 4. **Report** commands loaded, notices, hooks invoked, quarantines, and
//!    wall-clock load + analysis time, per pack.
//!
//! ## Why the packs are loaded one file at a time
//!
//! A pack is a logical unit, not a file, and nine of the eleven ports under
//! `docs/design/spec-dsl-examples/` declare `speclib tcl` (the other two are
//! `speclib tcllib` and `speclib f5-irules`, and `snit-type.tclspec` shares
//! `tcllib` with the external draft) — merging them would
//! produce one synthetic "tcl" pack that exists on nobody's disk, with
//! duplicate-definition notices that say nothing about any real pack. Each
//! file is therefore its own pack set here, which is also the unit the report
//! is keyed by.
//!
//! ## Why the ports are installed `-override`
//!
//! The shipped collision policy is *shipped wins unless the pack says
//! `-override`*, and no example port declares `-override` — they are ports of
//! shipped specs, so installing them naturally is a no-op and step 2 would be
//! vacuous for eleven of the twenty-two packs. The harness therefore reports
//! the natural outcome ([`tcl_spectcl::pack::collision_notices`], the
//! `collide` column) **and** installs with the override flag forced on, so the
//! analyser reads the *pack's* specs and the *pack's* hook bodies run. Both
//! numbers are in the report; neither is hidden.
//!
//! ## Corpus
//!
//! `samples/` is the in-repo corpus: 81 `.tcl` files and 15 `.irul` iRules. A
//! pack takes the corpus files that mention one of its commands — a
//! **word-boundary text match**, not resolution, so a vendor pack owning a
//! short generic name (`eda_mentor`'s `run`, `add`) picks up sample files that
//! merely contain the word. That over-selects rather than under-selects, which
//! is the safe direction for a harness whose job is to run the analyser over
//! more code, not less; the `corpus` column is therefore an upper bound on
//! genuinely relevant files, and the per-pack list below the table names every
//! one so the claim can be checked. For
//! a pack whose commands appear nowhere — every EDA vendor pack, and every
//! external draft — the harness **synthesises** an exercising call per command
//! from that command's own arity and argument roles, and the report says how
//! many calls were synthesised versus how many corpus files were real.
//!
//! ### The `tmp/` corpus is opt-in
//!
//! A box that has run the source-fetch hook has five full Tcl source trees
//! under `tmp/` — ~320 further `.tcl` files of real library code, roughly
//! fifteen times the committed corpus. Absorbing them silently turned this test
//! into a ten-minute sweep that then tripped its own watchdog, and it did so on
//! developer and agent machines only: CI checkouts have no `tmp/`, so CI never
//! saw it. The `tmp/` half is therefore behind an explicit opt-in:
//!
//! ```text
//! SPECTCL_CORPUS_TMP=1 cargo test -p tcl-spectcl --test spec_corpus
//! ```
//!
//! Without it the harness draws on `samples/` alone, which is exactly what CI
//! exercises and what `spec_corpus_baseline.txt` pins — the load, install,
//! hook-host and containment guarantees are all reached by the committed
//! corpus. The `tmp/` sweep only adds *more third-party Tcl to analyse*, which
//! is `xtask fp-sweep`'s job; [`MAX_CORPUS_FILES_PER_PACK`] says the same thing
//! ("the cap keeps the harness a test rather than a sweep"). The test stays
//! un-`#[ignore]`d in its fast, samples-only form; the environment variable is
//! the opt-in to the big corpus, and the report names which corpora were drawn
//! on either way.
//!
//! ## Containment
//!
//! [`a_hostile_pack_degrades_to_abstention_and_never_hangs`] is the negative
//! half: `tests/fixtures/hostile.tclspec` declares an unbounded loop, a
//! dispatch-heavy fold, and a body the engine panics on. Every test body here
//! runs on a watchdogged worker thread, so a hook that genuinely hung would
//! fail the suite rather than wedge it.

use std::cell::Cell;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use tcl_compiler::analyser::Analyser;
use tcl_compiler::optimiser::manager::optimise_raw;
use tcl_engine_api::{Budget, CompileUnit, Engine, EngineError, HostCommand, Value};
use tcl_engine_tclvm::TclVmEngine;
use tcl_registry::arg_role::ArgRole;
use tcl_registry::arity::Arity;
use tcl_registry::pack_hooks;
use tcl_registry::profile_queries::ProfileQueries;
use tcl_registry::spec::CommandSpec;
use tcl_spec_hooks::{CrashRecord, HookHost};
use tcl_spectcl::discovery::{Origin, PackFile, Tier};
use tcl_spectcl::hooks;
use tcl_spectcl::pack::PackSet;

// ---------------------------------------------------------------------------
// Where things are
// ---------------------------------------------------------------------------

/// The repository root — this crate is `rust/tcl-spectcl`.
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::canonicalize(manifest.join("../.."))
        .expect("the repository root is two levels above this crate")
}

/// The baseline of accepted load notices, beside this file.
fn baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/spec_corpus_baseline.txt")
}

fn relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

// ---------------------------------------------------------------------------
// The inventory: every `.tclspec` the repo ships
// ---------------------------------------------------------------------------

/// One pack file under test, with the tier it ships at and the dialect its
/// commands belong to.
struct PackUnderTest {
    path: PathBuf,
    tier: Tier,
    origin: Origin,
    dialect: &'static str,
}

/// The dialect a pack's commands are written for.
///
/// The five vendor packs each own a shell; `sdc_base` rides along with every
/// one of them (the shared SDC constraint library), so it is exercised under
/// Vivado. `irules-http-header` is an iRules port. Everything else — the core
/// Tcl ports and the external drafts — is plain Tcl, which is also the profile
/// `tcl-spec-studio/tests/spectcl_ports.rs` compares those ports under.
fn dialect_for(stem: &str) -> &'static str {
    match stem {
        // `sdc_base` shares Vivado's profile: the shared SDC constraint
        // library is installed under every vendor shell, and one of them has
        // to be picked to exercise it.
        "eda_xilinx" | "sdc_base" => "xilinx-eda-tcl",
        // `upf` is the same cross-vendor shape as `sdc_base`; it is exercised
        // under the profile `.upf` files detect as, which is also Synopsys's.
        "upf" | "eda_synopsys" => "synopsys-eda-tcl",
        "eda_microchip" => "microchip-libero-eda-tcl",
        "eda_cadence" => "cadence-eda-tcl",
        "eda_quartus" => "intel-quartus-eda-tcl",
        "eda_mentor" => "mentor-eda-tcl",
        "irules-http-header" => "f5-irules",
        _ => "tcl9.1",
    }
}

fn tclspec_files_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == tcl_spectcl::PACK_EXTENSION)
        })
        .collect();
    files.sort();
    files
}

/// Every `.tclspec` the repository ships: the bundled EDA loadables under
/// `specs/`, the eleven ports, and the five external drafts.
///
/// Discovered by scanning, never listed, so a pack added tomorrow is covered
/// by this harness the day it lands — and, if it loads with a notice, fails
/// until the notice is either fixed or blessed into the baseline.
fn inventory(root: &Path) -> Vec<PackUnderTest> {
    let mut packs: Vec<PackUnderTest> = Vec::new();
    let sources = [
        (root.join("specs"), Tier::Bundled, Origin::Bundled),
        (
            root.join("docs/design/spec-dsl-examples"),
            Tier::Workspace,
            Origin::DotDir,
        ),
        (
            root.join("docs/design/spec-dsl-examples/external"),
            Tier::Workspace,
            Origin::DotDir,
        ),
    ];
    for (dir, tier, origin) in sources {
        for path in tclspec_files_in(&dir) {
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_owned();
            packs.push(PackUnderTest {
                dialect: dialect_for(&stem),
                path,
                tier,
                origin,
            });
        }
    }
    packs
}

// ---------------------------------------------------------------------------
// The corpus
// ---------------------------------------------------------------------------

/// Which corpus a dialect draws from: iRules read `.irul`, everything else
/// reads `.tcl`.
fn corpus_family(dialect: &str) -> &'static str {
    if dialect == "f5-irules" {
        "irul"
    } else {
        "tcl"
    }
}

/// A corpus file: its path, its text, and the extension family it belongs to.
struct CorpusFile {
    path: PathBuf,
    text: String,
    family: &'static str,
    /// Every maximal identifier run in `text`, built once when the file is
    /// read.
    ///
    /// [`mentions`] asks whether a command name occurs with no identifier
    /// character on either side — i.e. whether it *is* one of these runs — so
    /// the set answers it with a hash lookup.  Scanning `text` afresh per
    /// candidate name instead made the pack loop quadratic in the corpus: every
    /// one of the ~20 packs re-scanned all ~400 files once per command it
    /// declares, which is gigabytes of `match_indices` for a question that does
    /// not depend on the pack at all.
    tokens: HashSet<String>,
}

/// Every maximal run of identifier characters in `text` — the token alphabet
/// [`mentions`] defines its word boundaries against.
fn identifier_tokens(text: &str) -> HashSet<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
        .filter(|tok| !tok.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Directories the corpus walk never descends into.
const SKIP_DIRECTORY_NAMES: &[&str] = &[".git", "node_modules", "target", "build", "dist"];

/// The largest corpus file the harness will read. A fetched Tcl source tree in
/// `tmp/` can hold generated files far larger than anything a spec pack
/// meaningfully exercises.
const MAX_CORPUS_BYTES: u64 = 256 * 1024;

/// How many corpus files one pack analyses. The cap keeps the harness a test
/// rather than a sweep: `xtask fp-sweep` is the unbounded corpus tool.
const MAX_CORPUS_FILES_PER_PACK: usize = 24;

fn walk_corpus(dir: &Path, into: &mut Vec<CorpusFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            let skip = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| SKIP_DIRECTORY_NAMES.contains(&name));
            if !skip {
                walk_corpus(&path, into);
            }
            continue;
        }
        let family = match path.extension().and_then(|ext| ext.to_str()) {
            Some("tcl") => "tcl",
            Some("irul") => "irul",
            _ => continue,
        };
        if std::fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_CORPUS_BYTES) {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            let tokens = identifier_tokens(&text);
            into.push(CorpusFile {
                path,
                text,
                family,
                tokens,
            });
        }
    }
}

/// The environment variable that opts the fetched `tmp/` source trees into the
/// corpus. See this file's header for why they are not in it by default.
const TMP_CORPUS_ENV: &str = "SPECTCL_CORPUS_TMP";

/// What the `tmp/` half of the corpus contributed to this run.
#[derive(Clone, Copy)]
enum TmpCorpus {
    /// The opt-in was set and a fetched source tree was there to walk.
    Included,
    /// A fetched source tree is present, but without the opt-in it is left
    /// alone — the default run is the committed corpus CI exercises.
    SkippedNotOptedIn,
    /// No fetched source tree on this box; the opt-in would find nothing.
    Absent,
}

impl TmpCorpus {
    /// The report's one-line account of the `tmp/` half.
    fn describe(self) -> &'static str {
        match self {
            Self::Included => "present and included",
            Self::SkippedNotOptedIn => {
                "present but not walked — set SPECTCL_CORPUS_TMP=1 for the full sweep"
            }
            Self::Absent => "absent — no fetched source trees, so nothing was drawn from it",
        }
    }
}

/// Has the caller asked for the fetched `tmp/` source trees?
///
/// Any non-empty value other than `0` counts, so `SPECTCL_CORPUS_TMP=1` reads
/// the way the header documents it without `SPECTCL_CORPUS_TMP=0` silently
/// meaning the opposite of what it says.
fn opted_into_tmp_corpus() -> bool {
    std::env::var_os(TMP_CORPUS_ENV).is_some_and(|value| !value.is_empty() && value != "0")
}

/// The in-repo corpus, plus `tmp/` when a fetched source tree is present *and*
/// the caller opted into it.
fn corpus(root: &Path) -> (Vec<CorpusFile>, TmpCorpus) {
    let mut files = Vec::new();
    walk_corpus(&root.join("samples"), &mut files);
    let tmp = root.join("tmp");
    let status = if tmp.is_dir() {
        if opted_into_tmp_corpus() {
            walk_corpus(&tmp, &mut files);
            TmpCorpus::Included
        } else {
            TmpCorpus::SkippedNotOptedIn
        }
    } else {
        TmpCorpus::Absent
    };
    (files, status)
}

/// Does `file` call `name` as a command word — i.e. does the name occur with
/// no identifier character on either side?
///
/// A plain `contains` would match `if` inside `notify`, which would hand every
/// pack the entire corpus and make the "relevant corpus" column meaningless.
fn mentions(file: &CorpusFile, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    // A name bounded by non-identifier characters on both sides *is* a maximal
    // identifier run, so membership in the file's precomputed token set is the
    // same question — `::` included, so `struct::tree` still does not match
    // inside `struct::treeql`.
    file.tokens.contains(name)
}

// ---------------------------------------------------------------------------
// Synthesising an exercising script for a command with no corpus
// ---------------------------------------------------------------------------

/// A valid argument count for `arity`, biased towards two so a resolver has
/// something to resolve.
fn exercising_argc(arity: Arity) -> u16 {
    let mut count = arity.min.max(2).min(arity.max).max(arity.min);
    if arity.step > 1 {
        let on_step = |count: u16| (count - arity.min).is_multiple_of(arity.step);
        while count < arity.max && !on_step(count) {
            count += 1;
        }
        if !on_step(count) {
            count = arity.min;
        }
    }
    count
}

/// A literal word that reads as `role` — the fold families' precondition is
/// that every word is literal, so nothing here is a substitution.
fn word_for(role: Option<ArgRole>, index: u16) -> String {
    match role {
        Some(ArgRole::Body) => "{ set inner 1 }".to_owned(),
        Some(ArgRole::Expr) => "{1 + 1}".to_owned(),
        Some(ArgRole::ParamList) => "{a b}".to_owned(),
        Some(ArgRole::LoopVarList) => "{item}".to_owned(),
        Some(ArgRole::VarWrite | ArgRole::VarRead) => format!("v{index}"),
        Some(ArgRole::CommandPrefix) => "puts".to_owned(),
        Some(ArgRole::Pattern) => "p*".to_owned(),
        Some(ArgRole::Channel) => "stdout".to_owned(),
        Some(ArgRole::Index) => "0".to_owned(),
        Some(ArgRole::OptionTerminator) => "--".to_owned(),
        _ => format!("w{index}"),
    }
}

fn role_at(roles: &[(u8, ArgRole)], index: u16) -> Option<ArgRole> {
    u8::try_from(index)
        .ok()
        .and_then(|index| roles.iter().find(|(at, _)| *at == index).map(|(_, r)| *r))
}

fn arguments(roles: &[(u8, ArgRole)], count: u16) -> String {
    (0..count)
        .map(|index| word_for(role_at(roles, index), index))
        .collect::<Vec<_>>()
        .join(" ")
}

/// One exercising call per declared form of `spec`: the bare command, and one
/// call per subcommand it declares.
///
/// Built from the command's *own* arity and argument roles — the synopsis in
/// other words — which is exactly what a vendor pack with no corpus can be
/// held to.
fn synthesise(spec: &CommandSpec) -> Vec<String> {
    let mut calls = Vec::new();
    if spec.subcommands.is_empty() {
        let count = exercising_argc(spec.arity);
        calls.push(format!(
            "{} {}",
            spec.name,
            arguments(spec.arg_roles, count)
        ));
    } else {
        for sub in spec.subcommands {
            let count = exercising_argc(sub.arity);
            calls.push(format!(
                "{} {} {}",
                spec.name,
                sub.name,
                arguments(sub.arg_roles, count)
            ));
        }
    }
    calls.into_iter().map(|c| c.trim_end().to_owned()).collect()
}

/// How many synthesised calls go into one analysed script. Batching keeps a
/// 68-command vendor pack to three analyses rather than sixty-eight, without
/// making any single script large enough to hide a failure.
const SYNTHESISED_CALLS_PER_SCRIPT: usize = 25;

// ---------------------------------------------------------------------------
// The engine wrapper: counts invocations, and injects the one panic no Tcl
// body can be trusted to produce
// ---------------------------------------------------------------------------

/// A body carrying this marker panics at the engine boundary.
///
/// The containment guarantee is about the *host*, not about the absence of a
/// VM bug, so the panic arm cannot be written in Tcl against a correct engine.
/// Injecting it here drives the identical `catch_unwind` path a real panic
/// would, under the real loader, the real slot plan, and the real host.
const PANIC_MARKER: &str = "HARNESS-PANIC-INJECT";

#[derive(Default)]
struct EngineStats {
    invocations: Cell<u64>,
}

/// `tcl-vm`, wrapped so the harness can report how many hook bodies actually
/// ran. Every other obligation is delegated untouched — this is the shipped
/// engine, with a counter.
struct CountingEngine {
    inner: TclVmEngine,
    stats: Rc<EngineStats>,
}

impl Engine for CountingEngine {
    type Handle = (bool, <TclVmEngine as Engine>::Handle);

    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        self.inner.define_command(name, command)
    }

    fn restrict_commands(&mut self, allowed: &[&str]) -> Result<(), EngineError> {
        self.inner.restrict_commands(allowed)
    }

    fn compile(&mut self, unit: CompileUnit<'_>) -> Result<Self::Handle, EngineError> {
        let injects_panic = unit.body.contains(PANIC_MARKER);
        Ok((injects_panic, self.inner.compile(unit)?))
    }

    fn invoke(&mut self, handle: &Self::Handle, arguments: &[Value]) -> Result<Value, EngineError> {
        self.stats.invocations.set(self.stats.invocations.get() + 1);
        assert!(!handle.0, "hook body requested an engine panic");
        self.inner.invoke(&handle.1, arguments)
    }

    fn set_budget(&mut self, budget: Budget) -> Result<(), EngineError> {
        self.inner.set_budget(budget)
    }

    fn commands_spent(&self) -> Option<u64> {
        self.inner.commands_spent()
    }
}

/// A host with the **shipped** configuration — default budgets, default
/// quarantine policy — running the shipped engine behind the counter.
fn counting_host(stats: &Rc<EngineStats>) -> HookHost<CountingEngine> {
    let stats = Rc::clone(stats);
    HookHost::new(move || CountingEngine {
        inner: TclVmEngine::new(),
        stats: Rc::clone(&stats),
    })
}

// ---------------------------------------------------------------------------
// The per-pack result
// ---------------------------------------------------------------------------

struct PackReport {
    file: String,
    pack: String,
    dialect: &'static str,
    declared: usize,
    installed: usize,
    gated_out: usize,
    collisions: usize,
    notices: Vec<String>,
    hook_bodies: usize,
    hook_slots: usize,
    hooks_invoked: u64,
    quarantined: usize,
    crashes: Vec<CrashRecord>,
    corpus_files: usize,
    corpus_paths: Vec<String>,
    synthesised_calls: usize,
    diagnostics: usize,
    optimisations: usize,
    load: Duration,
    analysis: Duration,
    unresolved: Vec<String>,
}

impl PackReport {
    fn row(&self) -> String {
        format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {}/{} | {} | {} | {} | {} | {} | {:.1} | {:.1} |",
            self.file,
            self.pack,
            self.dialect,
            self.declared,
            self.installed,
            self.gated_out,
            self.collisions,
            self.notices.len(),
            self.hook_slots,
            self.hook_bodies,
            self.hooks_invoked,
            self.quarantined,
            self.crashes.len(),
            self.corpus_files,
            self.synthesised_calls,
            self.load.as_secs_f64() * 1000.0,
            self.analysis.as_secs_f64() * 1000.0,
        )
    }
}

const HEADER: &str = "\
| pack file | speclib | dialect | cmds | inst | gated | collide | notices | hooks bound/declared | invoked | quar | crash | corpus | synth | load ms | analyse ms |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|";

fn render(reports: &[PackReport], tmp: TmpCorpus) -> String {
    let mut out = String::new();
    out.push_str("\nSpecTcl corpus validation — per pack\n\n");
    out.push_str(HEADER);
    out.push('\n');
    for report in reports {
        out.push_str(&report.row());
        out.push('\n');
    }
    out.push_str("\nCorpus drawn on, per pack\n\n");
    for report in reports {
        let _ = writeln!(
            out,
            "- {}: {}{}",
            report.file,
            if report.corpus_paths.is_empty() {
                "no corpus file mentions any of its commands".to_owned()
            } else {
                report.corpus_paths.join(", ")
            },
            if report.synthesised_calls > 0 {
                format!(
                    " (+{} synthesised call(s) from the pack's own arity/roles)",
                    report.synthesised_calls
                )
            } else {
                String::new()
            }
        );
    }

    let commands: usize = reports.iter().map(|r| r.declared).sum();
    let invoked: u64 = reports.iter().map(|r| r.hooks_invoked).sum();
    let bodies: usize = reports.iter().map(|r| r.hook_bodies).sum();
    let load: f64 = reports.iter().map(|r| r.load.as_secs_f64()).sum();
    let analysis: f64 = reports.iter().map(|r| r.analysis.as_secs_f64()).sum();
    let diagnostics: usize = reports.iter().map(|r| r.diagnostics).sum();
    let optimisations: usize = reports.iter().map(|r| r.optimisations).sum();
    let _ = write!(
        out,
        "\n{} packs, {commands} commands declared, {bodies} hook bodies, \
         {invoked} hook invocations, {diagnostics} diagnostics, \
         {optimisations} optimisations.\n\
         load {:.1} ms, analysis {:.1} ms, total {:.1} ms.\n\
         corpus: samples/ (in-repo); tmp/ {}.\n",
        reports.len(),
        load * 1000.0,
        analysis * 1000.0,
        (load + analysis) * 1000.0,
        tmp.describe(),
    );
    out
}

// ---------------------------------------------------------------------------
// The harness proper
// ---------------------------------------------------------------------------

fn load_one(pack: &PackUnderTest) -> (PackSet, Duration) {
    let file = PackFile {
        tier: pack.tier,
        path: pack.path.clone(),
        origin: pack.origin,
    };
    let started = Instant::now();
    let set = tcl_spectcl::pack::load(std::slice::from_ref(&file));
    (set, started.elapsed())
}

/// Analyse `source` under `dialect` with `packs` overlaid, and run the
/// optimiser over it too — the const-fold families answer there, not in the
/// analyser.
fn analyse(source: &str, dialect: &str, overlay: u64) -> (usize, usize) {
    let mut analyser = Analyser::new().with_pack_overlay(overlay);
    let result = analyser.analyse(source, dialect);
    let registry = tcl_registry::cache::registry_for_profile_if_built(
        tcl_dialect::DialectProfile::by_name(dialect),
        overlay,
    )
    .unwrap_or_else(|| tcl_registry::registry_handle_for_dialect(dialect));
    let optimisations = optimise_raw(source, &registry, Some(dialect));
    (result.diagnostics.len(), optimisations.len())
}

/// Every load notice, in the baseline's line format.
fn notice_lines(set: &PackSet, root: &Path) -> Vec<String> {
    set.notices
        .iter()
        .map(|notice| {
            format!(
                "{}\t{:?}\t{}\t{}",
                relative(&notice.path, root),
                notice.severity,
                notice.context,
                notice.message
            )
        })
        .collect()
}

/// Hook bodies the pack declares, and how many of them the plan could bind to
/// a process-wide slot. A difference is slot exhaustion, which the harness
/// fails on rather than reporting quietly as "fewer hooks".
fn hook_counts(set: &PackSet, plan: &hooks::HookPlan) -> (usize, usize) {
    let declared = set
        .packs
        .iter()
        .map(|p| {
            hooks::programs_of(&p.name, &p.dsl_version, "", &p.commands)
                .programs
                .len()
        })
        .sum();
    let bound = plan
        .packs()
        .iter()
        .map(|programs| {
            programs
                .programs
                .iter()
                .filter(|p| p.slot.is_some())
                .count()
        })
        .sum();
    (declared, bound)
}

/// Build the sandboxed host for `plan`, install it on this thread, and return
/// it with the slots it claimed.
///
/// [`counting_host`] is `HookHost::new`, so this is the shipped configuration:
/// the shipped budgets, `catch_unwind` on every invocation, quarantine on the
/// first crash.
fn start_host(
    plan: &hooks::HookPlan,
    stats: &Rc<EngineStats>,
) -> (Rc<HookHost<CountingEngine>>, Vec<pack_hooks::HookSlot>) {
    let host = Rc::new(counting_host(stats));
    let mut slots = Vec::new();
    for programs in plan.packs() {
        for installation in host.load_pack(programs.clone()) {
            if let Some(slot) = installation.slot {
                slots.push(slot);
            }
        }
    }
    pack_hooks::install_host(Rc::<HookHost<CountingEngine>>::clone(&host));
    pack_hooks::clear_cache();
    (host, slots)
}

/// How the pack's commands fared in `registry`: installed, held back by the
/// vendor package gate, or — the failure — installed yet unresolvable.
fn installation_of(
    set: &PackSet,
    profile: &'static tcl_dialect::DialectProfile,
    registry: &tcl_registry::registry::CommandRegistry,
) -> (usize, usize, Vec<String>) {
    let (mut installed, mut gated_out) = (0usize, 0usize);
    let mut unresolved: Vec<String> = Vec::new();
    for merged in &set.packs {
        for command in &merged.commands {
            if profile.package_available(command.spec.required_package) {
                installed += 1;
                if registry.get(command.spec.name).is_none() {
                    unresolved.push(command.spec.name.to_owned());
                }
            } else {
                gated_out += 1;
            }
        }
    }
    (installed, gated_out, unresolved)
}

/// The corpus files that call one of the pack's commands, and a synthesised
/// call for every command none of them reached.
fn corpus_and_synthesis<'c>(
    set: &PackSet,
    corpus: &'c [CorpusFile],
    family: &str,
) -> (Vec<&'c CorpusFile>, Vec<String>) {
    let names: Vec<&str> = set
        .packs
        .iter()
        .flat_map(|p| p.commands.iter().map(|c| c.spec.name))
        .collect();
    let mut selected: Vec<&CorpusFile> = Vec::new();
    let mut covered: Vec<&str> = Vec::new();
    for file in corpus.iter().filter(|f| f.family == family) {
        let hits: Vec<&str> = names
            .iter()
            .copied()
            .filter(|name| mentions(file, name))
            .collect();
        if hits.is_empty() {
            continue;
        }
        for hit in hits {
            if !covered.contains(&hit) {
                covered.push(hit);
            }
        }
        if selected.len() < MAX_CORPUS_FILES_PER_PACK {
            selected.push(file);
        }
    }

    let mut synthesised: Vec<String> = Vec::new();
    for merged in &set.packs {
        for command in &merged.commands {
            if !covered.contains(&command.spec.name) {
                synthesised.extend(synthesise(command.spec));
            }
        }
    }
    (selected, synthesised)
}

/// Analyse and optimise every selected corpus file and every synthesised
/// script, returning the diagnostic and optimisation counts and the wall clock
/// it took.
fn analyse_all(
    selected: &[&CorpusFile],
    synthesised: &[String],
    dialect: &str,
    overlay: u64,
) -> (usize, usize, Duration) {
    let (mut diagnostics, mut optimisations) = (0usize, 0usize);
    let started = Instant::now();
    for file in selected {
        let (diags, opts) = analyse(&file.text, dialect, overlay);
        diagnostics += diags;
        optimisations += opts;
    }
    for chunk in synthesised.chunks(SYNTHESISED_CALLS_PER_SCRIPT) {
        let script = format!("{}\n", chunk.join("\n"));
        let (diags, opts) = analyse(&script, dialect, overlay);
        diagnostics += diags;
        optimisations += opts;
    }
    (diagnostics, optimisations, started.elapsed())
}

fn run_pack(pack: &PackUnderTest, root: &Path, corpus: &[CorpusFile]) -> PackReport {
    let (mut set, load) = load_one(pack);
    let profile = tcl_dialect::DialectProfile::by_name(pack.dialect);
    let notices = notice_lines(&set, root);

    // The natural collision outcome, recorded before the override flag is
    // forced — this is what a user installing the pack unchanged would get.
    let plain_registry = tcl_registry::registry_for_dialect(pack.dialect);
    let collisions = tcl_spectcl::pack::collision_notices(&set, plain_registry).len();

    let declared: usize = set.packs.iter().map(|p| p.commands.len()).sum();
    let name = set
        .packs
        .first()
        .map_or_else(|| "<none>".to_owned(), |p| p.name.clone());

    let plan = hooks::plan_for(&set);
    let (hook_bodies, hook_slots) = hook_counts(&set, &plan);
    let stats = Rc::new(EngineStats::default());
    let (host, slots) = start_host(&plan, &stats);

    // Force `-override` so the pack's own specs are what the analyser reads.
    // See this file's header: the natural outcome is the `collide` column.
    for merged in &mut set.packs {
        for command in &mut merged.commands {
            command.overrides_shipped = true;
        }
    }
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs(pack.dialect, &set);
    let (installed, gated_out, unresolved) = installation_of(&set, profile, &registry);

    let (selected, synthesised) = corpus_and_synthesis(&set, corpus, corpus_family(pack.dialect));
    let (diagnostics, optimisations, analysis) =
        analyse_all(&selected, &synthesised, pack.dialect, set.key);

    let quarantined = slots
        .iter()
        .filter(|slot| host.is_quarantined(**slot))
        .count();
    let crashes = host.crash_records();

    pack_hooks::clear_host();
    pack_hooks::clear_cache();

    PackReport {
        file: relative(&pack.path, root),
        pack: name,
        dialect: pack.dialect,
        declared,
        installed,
        gated_out,
        collisions,
        notices,
        hook_bodies,
        hook_slots,
        hooks_invoked: stats.invocations.get(),
        quarantined,
        crashes,
        corpus_files: selected.len(),
        corpus_paths: selected
            .iter()
            .map(|file| relative(&file.path, root))
            .collect(),
        synthesised_calls: synthesised.len(),
        diagnostics,
        optimisations,
        load,
        analysis,
        unresolved,
    }
}

// ---------------------------------------------------------------------------
// The notice baseline
// ---------------------------------------------------------------------------

const BASELINE_HEADER: &str = "\
# SpecTcl corpus-validation harness — accepted load notices.
#
# Regenerate with:  SPECTCL_CORPUS_BLESS=1 cargo test -p tcl-spectcl --test spec_corpus
#
# Every line is one notice the loader emitted for a `.tclspec` the repository
# ships, in the form
#
#     <pack file>\\t<severity>\\t<context>\\t<message>
#
# tab-separated, sorted, one line per occurrence — 49 of them today, in four
# groups. `tests/spec_corpus.rs`
# compares the notices of every pack against this file as a multiset and fails
# on any difference in either direction, so a notice that gets fixed must also
# be deleted from here — the baseline cannot silently rot into a wildcard.
#
# THE LOAD-BEARING FACT, and the reason this file is not simply a mute-button:
# `specs/` — the eight bundled EDA loadables, the packs an installed tcl-lsp
# actually loads — appears NOWHERE below. All 776 of their commands load
# without a single notice. Every entry here belongs to a design artefact under
# `docs/design/spec-dsl-examples/`, and each is a known, named gap rather than
# a defect:
#
#   `unknown property `object_class` dropped` (13 lines, all four external
#       drafts) — a REAL LOADER GAP, and the most useful thing this baseline
#       records. `object_class NAME ?-superclass {…}? ?-allow-unknown?
#       { method … }` is ratified vocabulary: it is in the frozen syntax
#       (`docs/design/spec-dsl-examples/README.md`, the field table and the
#       statement index) and it is a compiled-in `SpecTcl` self-spec statement
#       (`tcl-registry/src/commands/spectcl/blocks.rs`). `tcl-spectcl`'s
#       loader implements none of it — `object_class` does not appear in
#       `src/loader.rs` at all — so every handle-returning factory in the four
#       external drafts loses its method table on the way in. Nothing under
#       `specs/` uses the statement, which is why the gap survived the EDA
#       migration unnoticed.
#
#   `unknown flag `-readonly` on `option` dropped` (3, tcllib draft) — not a
#       gap: the draft says so itself at `tcllib.tclspec:288`, \"no field in the
#       DSL to lower `-readonly` into — written on all three options anyway\".
#       A marked invention, dropped exactly as the compatibility policy
#       promises.
#
#   `names a lowering hook` / `names a codegen hook` (6, the `foreach`, `if`,
#       `return`, `switch` and `upvar` ports) — deliberate. A pack naming a
#       *native* lowering or codegen hook changes how the compiler translates a
#       command, so the loader warns and installs it anyway. These five ports
#       are ports of shipped specs that legitimately carry those hooks; a
#       private pack getting the same warning is being told something true.
#
#   ``state_transitions` row … is not yet loadable` /
#   ``world_effects` row … is not yet loadable` (27, the `oo-class` and `upvar`
#       ports) — the loader's own words. Those two descriptor families are the
#       named, still-unimplemented tail of the schema; the ports transcribe
#       them because the ports were written against the *design*, and the
#       notices are the honest record of the distance left.
#
# Adding a line here is therefore an admission with a reason attached. Adding
# one for a file under `specs/` would be a regression in the only tier that
# ships.
";

fn read_baseline() -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(baseline_path()) else {
        return Vec::new();
    };
    let mut lines: Vec<String> = text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect();
    lines.sort();
    lines
}

fn write_baseline(notices: &[String]) {
    let mut out = String::from(BASELINE_HEADER);
    for notice in notices {
        out.push_str(notice);
        out.push('\n');
    }
    std::fs::write(baseline_path(), out).expect("write the notice baseline");
}

// ---------------------------------------------------------------------------
// The watchdog
// ---------------------------------------------------------------------------

/// Run `body` on a worker thread and fail — rather than wedge — if it does not
/// finish inside `budget`.
///
/// A hook host is thread-confined (`Rc`, one VM per pack), so the harness has
/// to own a thread anyway. Making that thread watchdogged is what turns "a
/// runaway hook must not hang the process" from a claim into an assertion:
/// containment failing means this test fails in bounded time.
///
/// `overrun` is what an expiry *means* for this particular caller, and callers
/// differ. For the hostile pack, three 250 ms budgets against a two-minute
/// watchdog leave no other reading than a hang. For the corpus sweep the
/// watchdog is a budget over an amount of work that varies with the machine and
/// with how much corpus was opted into, so an expiry there says "too much work
/// for this box", not "containment failed" — and reporting the second when the
/// first happened is how a slow laptop got told it had a hung hook.
fn with_watchdog<T: Send + 'static>(
    label: &'static str,
    budget: Duration,
    overrun: &'static str,
    body: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let worker = std::thread::Builder::new()
        .name(label.to_owned())
        // Analysis of a real corpus file recurses; the default 2 MiB is the
        // one thing about this thread that must not differ from a test thread.
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let out = body();
            let _ = tx.send(());
            out
        })
        .expect("spawn the harness thread");
    match rx.recv_timeout(budget) {
        // Disconnected means the worker panicked; join propagates it with the
        // original message, which is what a caller wants to read.
        Ok(()) | Err(RecvTimeoutError::Disconnected) => match worker.join() {
            Ok(out) => out,
            Err(payload) => std::panic::resume_unwind(payload),
        },
        Err(RecvTimeoutError::Timeout) => {
            panic!("{label} did not finish within {budget:?} — {overrun}")
        }
    }
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

/// Every shipped `.tclspec`: loaded, installed, analysed against corpus, with
/// its hooks run through the sandboxed host — and the per-pack report.
///
/// Runs against the committed `samples/` corpus by default, which is what CI
/// exercises; `SPECTCL_CORPUS_TMP=1` adds the fetched trees under `tmp/` for
/// the full sweep. Run with `-- --nocapture` to see the table on a pass; it is
/// included in the failure message either way.
#[test]
fn every_shipped_tclspec_loads_installs_and_analyses_against_corpus() {
    // The watchdog bounds the work asked for, so it scales with the corpus
    // asked for: the samples-only default is seconds, the opt-in sweep is
    // hundreds of real-world files per pack.
    let budget = if opted_into_tmp_corpus() {
        Duration::from_mins(30)
    } else {
        Duration::from_mins(10)
    };
    let (report, failures) = with_watchdog(
        "spectcl-corpus",
        budget,
        "the corpus analysis budget was exceeded. This is a *budget* verdict, \
         not a containment one: no hook is known to have hung. Either this \
         machine is heavily loaded, or SPECTCL_CORPUS_TMP=1 opted the fetched \
         tmp/ source trees in and there is more corpus here than the budget \
         allows",
        || {
            let root = repo_root();
            let (corpus_files, tmp) = corpus(&root);
            let packs = inventory(&root);
            assert!(
                packs.len() >= 24,
                "the repository ships 24 .tclspec files (8 bundled + 11 ports + 5 external); \
             found {}",
                packs.len()
            );

            let reports: Vec<PackReport> = packs
                .iter()
                .map(|pack| run_pack(pack, &root, &corpus_files))
                .collect();

            let mut failures: Vec<String> = Vec::new();

            let mut seen: Vec<String> = reports
                .iter()
                .flat_map(|report| report.notices.iter().cloned())
                .collect();
            seen.sort();
            if std::env::var_os("SPECTCL_CORPUS_BLESS").is_some() {
                write_baseline(&seen);
            } else {
                let baseline = read_baseline();
                let mut counts: BTreeMap<&str, i32> = BTreeMap::new();
                for line in &seen {
                    *counts.entry(line.as_str()).or_default() += 1;
                }
                for line in &baseline {
                    *counts.entry(line.as_str()).or_default() -= 1;
                }
                for (line, delta) in counts {
                    if delta > 0 {
                        failures.push(format!("unexpected load notice (x{delta}): {line}"));
                    } else if delta < 0 {
                        failures.push(format!(
                            "baseline notice no longer produced (x{}) — delete it from \
                         tests/spec_corpus_baseline.txt: {line}",
                            -delta
                        ));
                    }
                }
            }

            for report in &reports {
                if report.quarantined > 0 {
                    failures.push(format!(
                        "{}: {} hook(s) quarantined — a shipped pack must never crash \
                     or outspend its budget",
                        report.file, report.quarantined
                    ));
                }
                for crash in &report.crashes {
                    failures.push(format!(
                        "{}: crash record: {}",
                        report.file,
                        crash.headline()
                    ));
                }
                if report.hook_slots != report.hook_bodies {
                    failures.push(format!(
                        "{}: {} declared hook bodies but only {} got a slot — the \
                     per-family slot budget is exhausted",
                        report.file, report.hook_bodies, report.hook_slots
                    ));
                }
                if !report.unresolved.is_empty() {
                    failures.push(format!(
                        "{}: installed but unresolvable in the registry: {}",
                        report.file,
                        report.unresolved.join(", ")
                    ));
                }
            }

            (render(&reports, tmp), failures)
        },
    );

    println!("{report}");
    assert!(
        failures.is_empty(),
        "{report}\nSpecTcl corpus validation failed:\n  {}",
        failures.join("\n  ")
    );
}

/// What the hostile pack did: per hook, the folds it emitted (must be none)
/// and how long the call site took; plus the crash records, the quarantined
/// hooks, and how many bodies actually ran.
struct Containment {
    folds: Vec<(String, usize, Duration)>,
    crashes: Vec<CrashRecord>,
    quarantined: Vec<String>,
    invocations: u64,
}

/// Load the hostile fixture, host it, and drive each hook once through the
/// optimiser.
fn drive_hostile_pack() -> Containment {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hostile.tclspec");
    let set = tcl_spectcl::pack::load(&[PackFile {
        tier: Tier::Workspace,
        path: fixture,
        origin: Origin::DotDir,
    }]);
    assert!(
        set.notices.is_empty(),
        "the hostile pack must be hostile at *run* time, not malformed: {:?}",
        set.notices
    );
    assert_eq!(set.packs.len(), 1);
    assert_eq!(set.packs[0].commands.len(), 3);

    let plan = hooks::plan_for(&set);
    let stats = Rc::new(EngineStats::default());
    let host = Rc::new(counting_host(&stats));
    let mut slots = BTreeMap::new();
    for programs in plan.packs() {
        for installation in host.load_pack(programs.clone()) {
            assert!(
                installation.declined.is_none(),
                "{} declined: {:?}",
                installation.command,
                installation.declined
            );
            if let Some(slot) = installation.slot {
                slots.insert(installation.command.clone(), slot);
            }
        }
    }
    pack_hooks::install_host(Rc::<HookHost<CountingEngine>>::clone(&host));
    pack_hooks::clear_cache();

    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.1", &set);

    // Silence the panic hook for the injected panic — the record, not the
    // stderr noise, is the evidence.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    // Order matters: a *panic* poisons the whole pack (the documented blast
    // radius), so the two budget hooks are exercised first.
    let mut folds: Vec<(String, usize, Duration)> = Vec::new();
    for command in ["hostile::spin", "hostile::burn", "hostile::boom"] {
        let script = format!("proc ::probe {{}} {{\n    set y [{command} abcde]\n}}\n");
        let started = Instant::now();
        let optimisations = optimise_raw(&script, &registry, Some("tcl9.1"))
            .into_iter()
            .filter(|opt| opt.code.as_str() == "O129")
            .count();
        folds.push((command.to_owned(), optimisations, started.elapsed()));
    }
    std::panic::set_hook(previous);

    let outcome = Containment {
        folds,
        crashes: host.crash_records(),
        quarantined: slots
            .iter()
            .filter(|(_, slot)| host.is_quarantined(**slot))
            .map(|(command, _)| command.clone())
            .collect(),
        invocations: stats.invocations.get(),
    };
    pack_hooks::clear_host();
    pack_hooks::clear_cache();
    outcome
}

fn render_containment(outcome: &Containment) -> String {
    let mut report = String::from("\nSpecTcl containment — hostile pack\n\n");
    let _ = writeln!(report, "| hook | folds emitted | wall clock |");
    let _ = writeln!(report, "|---|---|---|");
    for (command, count, elapsed) in &outcome.folds {
        let _ = writeln!(
            report,
            "| {command} | {count} | {:.1} ms |",
            elapsed.as_secs_f64() * 1000.0
        );
    }
    let _ = writeln!(report, "\nhook invocations: {}", outcome.invocations);
    let _ = writeln!(report, "quarantined: {}", outcome.quarantined.join(", "));
    for crash in &outcome.crashes {
        let _ = writeln!(report, "crash: {:?} — {}", crash.kind, crash.headline());
    }
    report
}

/// Containment, against the real loader and the real host: a hostile pack
/// degrades to abstention with a crash record, and the harness neither hangs
/// nor dies.
#[test]
fn a_hostile_pack_degrades_to_abstention_and_never_hangs() {
    // Generous next to the three default 250 ms wall-clock budgets, and far
    // below anything that reads as a hang: if this expires, containment failed.
    let outcome = with_watchdog(
        "spectcl-hostile",
        Duration::from_mins(2),
        "a hook hung, which is exactly the containment failure this harness \
         exists to catch — three 250 ms budgets cannot overrun a two-minute \
         watchdog for any other reason",
        drive_hostile_pack,
    );
    let Containment {
        folds,
        crashes,
        quarantined,
        invocations,
    } = &outcome;
    let report = render_containment(&outcome);
    println!("{report}");

    for (command, count, _) in folds {
        assert_eq!(
            *count, 0,
            "{report}\n{command} must abstain, not fold — a contained hook \
             produces no answer at all"
        );
    }
    assert!(
        *invocations >= 3,
        "{report}\nall three hostile bodies must actually have been invoked"
    );
    assert_eq!(
        crashes.len(),
        3,
        "{report}\none crash record per hostile hook"
    );
    assert_eq!(
        quarantined.len(),
        3,
        "{report}\nevery hostile hook must end quarantined"
    );

    let kinds: Vec<tcl_spec_hooks::CrashKind> = crashes.iter().map(|c| c.kind).collect();
    assert!(
        kinds.contains(&tcl_spec_hooks::CrashKind::Panic),
        "{report}\nthe panicking body must be recorded as a panic"
    );
    assert!(
        kinds
            .iter()
            .filter(|kind| matches!(
                kind,
                tcl_spec_hooks::CrashKind::CommandBudget
                    | tcl_spec_hooks::CrashKind::WallClockBudget
            ))
            .count()
            >= 2,
        "{report}\nboth the unbounded loop and the dispatch-heavy fold must be \
         recorded as budget blowouts"
    );
    for crash in crashes {
        assert_eq!(crash.pack, "hostile");
        assert!(
            !crash.word_shapes.is_empty(),
            "{report}\na crash record carries the input word *shapes*"
        );
    }
}
