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

//! Guardrail for the **interned garbage collector** invariant (issue #1299).
//!
//! Six interned structs in this crate key on content that changes as a
//! procedure body is edited — `ItemBodyKey`, `FnLatticeKey`, `ProcBodyKey`,
//! `OptDepsKey`, `TaintSummaryKey`, and `SummaryDepsKey`. A typing session
//! therefore mints brand-new interned ids for them continuously, each holding
//! a body's worth of `Arc` payload in its memo table — the exact shape of an
//! unbounded interning leak.
//!
//! It is bounded only because salsa 0.27 **garbage-collects interned slots**: a
//! cold intern walks the shard's LRU tail and reuses any slot untouched for
//! `DEFAULT_REVISIONS` (3) revisions, replacing the fields and clearing the
//! slot's memo table — which is what releases the retained payload. A slot is
//! eligible only when its durability is exactly `Durability::LOW`, and a slot
//! interned with no active tracked query is stamped `Durability::MAX` and is
//! never eligible at all. See the "The interned garbage collector is
//! load-bearing" section of the crate documentation and
//! `docs/design/rust/salsa-interned-gc.md`.
//!
//! # What is measured, and why it is not a proxy
//!
//! Salsa fires `EventKind::DidReuseInternedValue` exactly when it recycles a
//! stale slot, so counting those events per ingredient reads the collector
//! directly. `TclDatabase::with_interned_reuse_logger` exposes them.
//!
//! Live slot *counts* would be the more obvious measure, but their steady
//! state depends on salsa's shard count, which is
//! `(available_parallelism() * 4).next_power_of_two()` — so any absolute budget
//! would be tuned to the machine it was written on and would fail on a bigger
//! one.
//!
//! Reuse counts remove the dependence from the *assertion*, but not from the
//! number of revisions needed to observe one, and assuming a fixed edit count
//! sufficed was a real flake: a slot is only reclaimable once its **own**
//! shard's LRU tail holds a stale entry, so the sparser ingredients need more
//! interns before any shard accumulates enough. Measured on one machine by
//! varying the visible CPU count, all else equal:
//!
//! | cores | shards | `TaintSummaryKey` reuses | `SummaryDepsKey` reuses |
//! |------:|-------:|-------------------------:|------------------------:|
//! |     1 |      4 |                       32 |                     119 |
//! |     2 |      8 |                       28 |                      39 |
//! |     4 |     16 |                       22 |                      34 |
//! |    CI |    32+ |                    **0** |                   **1** |
//!
//! So the session drives revisions **until the collector is observed for every
//! ingredient**, bounded by [`edit_cap`], instead of a fixed count: it stops as
//! soon as the invariant is demonstrated (24 edits on a small machine) and
//! keeps going where sharding makes that take longer. The bound is what turns
//! "we did not look long enough" into a real failure.
//!
//! # The two behavioural tests
//!
//! [`interned_slots_are_reclaimed_across_an_edit_session`] drives a typing
//! session through the production queries and asserts every one of the six
//! ingredients is being reclaimed.
//!
//! [`raising_input_durability_disables_the_collector`] drives the *same*
//! session with `SourceFile` and `AnalyserConfig` marked `Durability::HIGH` and
//! asserts the collector goes completely silent while slot counts grow with the
//! edit count. It is the break-the-invariant experiment, kept in the suite
//! rather than performed once and discarded: it is what proves the first test
//! is not vacuous, and it pins the durability sensitivity that makes the rule
//! necessary. If it ever fails because reuse *did* happen at `HIGH`, salsa's
//! collection policy changed and the design doc needs revisiting — a deliberate
//! alarm, not a flake.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use salsa::Setter as _;

use tcl_compiler::analyser::NonAsciiMode;
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, compiler_check_diagnostics, file_analysis_incremental,
};

/// The interned structs whose key contains per-revision content — the ones the
/// garbage collector has to reclaim for the edit path to stay bounded.
///
/// Names are salsa's `IngredientInfo::debug_name`, i.e. the struct name; a
/// rename that is not mirrored here fails both tests rather than silently
/// dropping coverage.
const PER_REVISION_INTERNED: [&str; 6] = [
    "ItemBodyKey",
    "FnLatticeKey",
    "ProcBodyKey",
    "OptDepsKey",
    "TaintSummaryKey",
    "SummaryDepsKey",
];

/// The four of [`PER_REVISION_INTERNED`] keyed on body *text* or body IR, so
/// the corpus below re-keys all of them on every single edit. The other two are
/// keyed on the interprocedural *projections*, which move less often by design
/// (that firewall is the point of them), so they accumulate more slowly and
/// carry no per-edit lower bound.
const BODY_KEYED_INTERNED: [&str; 4] = ["ItemBodyKey", "FnLatticeKey", "ProcBodyKey", "OptDepsKey"];

/// Worker procedures in the corpus. Each edit rewrites every worker body, so
/// one edit produces `WORKERS` cold interns of each body-keyed ingredient.
const WORKERS: u32 = 4;

/// Edits every session drives before it may stop. Well above salsa's
/// `DEFAULT_REVISIONS` staleness window (3), and `WORKERS * EDITS` cold interns
/// is enough for the collector to reach steady state on a small machine while
/// keeping the run around three seconds in a debug build.
///
/// A floor, not a budget: [`drive_until_collected`] keeps going past it while
/// any ingredient has yet to show a reuse. The control session, which must
/// observe *no* reuse, always runs exactly this many.
const EDITS: u32 = 24;

/// Salsa's interned-table shard count — `(available_parallelism() * 4)`
/// rounded up to a power of two (`salsa::table::memo` sharding).  Read here
/// only to scale [`edit_cap`]; nothing asserts on it.
fn interned_shards() -> u32 {
    let cpus = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    u32::try_from(cpus.saturating_mul(4).next_power_of_two()).unwrap_or(u32::MAX)
}

/// The most edits [`drive_until_collected`] will drive before giving up and
/// letting the assertion fail.
///
/// Scaled by the shard count because that is what sets how many interns are
/// needed before a shard accumulates a reclaimable slot (see the module docs'
/// table).  Generous — roughly 40 interns per shard for the sparsest
/// ingredient — so reaching it means the collector genuinely is not running,
/// not that the machine is merely wide.
fn edit_cap() -> u32 {
    (interned_shards() * 40).max(EDITS)
}

/// The corpus at revision `edit` — a small Tcl module shaped so every one of the
/// six ingredients is actually re-keyed as the session runs:
///
/// * each **worker** body embeds `edit`, so its text, IR, and optimiser
///   dependencies are new every revision (`ItemBodyKey`, `ProcBodyKey`,
///   `FnLatticeKey`, `OptDepsKey`);
/// * each worker's `return` alternates between a parameter and a local, which
///   flips its `return_passthrough_param`, and each **caller** calls a rotating
///   worker, which moves its `calls` set — between them that shifts the
///   interprocedural projections the summary keys are built from
///   (`TaintSummaryKey`, `SummaryDepsKey`);
/// * every procedure calls `puts`, so none is `pure`: a pure-but-not-constant-
///   return procedure sends `solve_optimisations` down its whole-module
///   fallback and `OptDepsKey` would never be interned at all;
/// * no `TclOO` methods and a plain `tcl8.6` dialect, the remaining
///   whole-module fallback conditions.
///
/// [`raising_input_durability_disables_the_collector`] keeps this shaping
/// honest: it asserts the uncollected slot count really does track the edit
/// count, which only holds while the corpus re-keys as described.
fn corpus(edit: u32) -> String {
    let mut src = String::from("namespace eval ::bench {\n    variable counter 0\n}\n\n");
    for worker in 0..WORKERS {
        let returned = if (worker + edit).is_multiple_of(2) {
            "$a"
        } else {
            "$total"
        };
        write!(
            src,
            "proc ::bench::w{worker} {{a b}} {{\n\
             \x20   set probe_v {edit:08}\n\
             \x20   set total [expr {{$a * {worker} + $b}}]\n\
             \x20   if {{$total > 10}} {{\n\
             \x20       set total 10\n\
             \x20   }}\n\
             \x20   puts \"w{worker} $total $probe_v\"\n\
             \x20   return {returned}\n\
             }}\n\n"
        )
        .expect("write to String");
    }
    for caller in 0..WORKERS {
        let callee = (caller + edit) % WORKERS;
        write!(
            src,
            "proc ::bench::c{caller} {{a}} {{\n\
             \x20   set r [::bench::w{callee} $a {caller}]\n\
             \x20   puts \"c{caller} $r\"\n\
             \x20   return $r\n\
             }}\n\n"
        )
        .expect("write to String");
    }
    src
}

/// Whether the session marks its salsa inputs more durable than the default.
#[derive(Clone, Copy, PartialEq, Eq)]
enum InputDurability {
    /// Salsa's default, `Durability::LOW` — what production does, and the only
    /// setting under which interned slots are collectable.
    Default,
    /// `Durability::HIGH` on every field of both inputs — the hazard, driven
    /// deliberately so the guardrail can be shown to detect it.
    RaisedToHigh,
}

/// What one edit session produced: live interned slots per ingredient, and how
/// many times the collector recycled a slot of each ingredient.
struct Session {
    slots: BTreeMap<String, usize>,
    reuses: BTreeMap<String, usize>,
    /// Revisions actually driven — variable, since
    /// [`drive_until_collected`] stops as soon as the invariant is shown.
    edits: u32,
}

impl Session {
    fn slots(&self, ingredient: &str) -> usize {
        self.slots.get(ingredient).copied().unwrap_or(0)
    }

    fn reuses(&self, ingredient: &str) -> usize {
        self.reuses.get(ingredient).copied().unwrap_or(0)
    }

    /// The measured numbers for the six tracked ingredients, for failure
    /// messages.
    fn summary(&self) -> String {
        PER_REVISION_INTERNED
            .iter()
            .map(|name| {
                format!(
                    "{name}: {} slots, {} reuses",
                    self.slots(name),
                    self.reuses(name)
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// [`Self::summary`] with the session length, which a failure needs in
    /// order to be actionable now that the length is not a constant.
    fn report(&self) -> String {
        format!(
            "{} (over {} edits, cap {}, {} interned shards)",
            self.summary(),
            self.edits,
            edit_cap(),
            interned_shards(),
        )
    }
}

/// Build the session's two salsa inputs at the requested durability.
fn make_inputs(db: &TclDatabase, durability: InputDurability) -> (SourceFile, AnalyserConfig) {
    let dialect = "tcl8.6".to_owned();
    match durability {
        InputDurability::Default => (
            SourceFile::new(db, corpus(0), dialect, None),
            AnalyserConfig::new(
                db,
                Vec::new(),
                NonAsciiMode::Default,
                Vec::new(),
                None,
                None,
            ),
        ),
        InputDurability::RaisedToHigh => (
            SourceFile::builder(corpus(0), dialect, None, None, None)
                .durability(salsa::Durability::HIGH)
                .new(db),
            AnalyserConfig::builder(Vec::new(), NonAsciiMode::Default, Vec::new(), None, None)
                .durability(salsa::Durability::HIGH)
                .new(db),
        ),
    }
}

/// Drive revisions of [`corpus`] until every tracked ingredient has shown the
/// collector at work, or [`edit_cap`] revisions have run.
///
/// Always drives at least [`EDITS`], so a session that stops early has still
/// cleared salsa's staleness window several times over.  Returns whatever it
/// measured; the caller asserts, so the failure message carries the real
/// numbers and the edit count that produced them.
fn drive_until_collected() -> Session {
    drive_edit_session(InputDurability::Default, edit_cap(), true)
}

/// Drive exactly `max_edits` revisions of [`corpus`] through the two production
/// diagnostic queries, recording interned slot reuse as it goes.
///
/// With `stop_when_collected`, returns as soon as every ingredient in
/// [`PER_REVISION_INTERNED`] has been reused at least once **and** at least
/// [`EDITS`] revisions have run.  The control session passes `false`: it
/// expects no reuse at all, so stopping on one would be nonsense.
fn drive_edit_session(
    durability: InputDurability,
    max_edits: u32,
    stop_when_collected: bool,
) -> Session {
    let reuse_log: Arc<Mutex<Vec<String>>> = Arc::default();
    let sink = Arc::clone(&reuse_log);
    let mut db = TclDatabase::with_interned_reuse_logger(move |key| {
        sink.lock().expect("reuse log").push(key);
    });

    let (file, config) = make_inputs(&db, durability);

    // Cold build first: the collector cannot reclaim anything until slots have
    // had time to go stale, so the measurement starts from a populated graph.
    let _ = file_analysis_incremental(&db, file, config);
    let _ = compiler_check_diagnostics(&db, file, config);

    let mut edits_driven = 0_u32;
    for edit in 1..=max_edits {
        let setter = file.set_text(&mut db);
        match durability {
            InputDurability::Default => setter.to(corpus(edit)),
            InputDurability::RaisedToHigh => setter
                .with_durability(salsa::Durability::HIGH)
                .to(corpus(edit)),
        };
        let _ = file_analysis_incremental(&db, file, config);
        let _ = compiler_check_diagnostics(&db, file, config);
        edits_driven = edit;
        if stop_when_collected && edit >= EDITS && all_collected(&reuse_log) {
            break;
        }
    }

    let slots = <dyn salsa::Database>::memory_usage(&db)
        .structs
        .iter()
        .map(|info| (info.debug_name().to_owned(), info.count()))
        .collect();
    // The logged key is `"<IngredientName>(Id(..))"`; the name is all this
    // needs.
    let mut reuses: BTreeMap<String, usize> = BTreeMap::new();
    for key in reuse_log.lock().expect("reuse log").iter() {
        let name = key.split('(').next().unwrap_or(key);
        *reuses.entry(name.to_owned()).or_default() += 1;
    }
    Session {
        slots,
        reuses,
        edits: edits_driven,
    }
}

/// Whether every tracked ingredient has appeared in the reuse log at least
/// once — the stopping condition for [`drive_until_collected`].
fn all_collected(reuse_log: &Arc<Mutex<Vec<String>>>) -> bool {
    let log = reuse_log.lock().expect("reuse log");
    PER_REVISION_INTERNED.iter().all(|name| {
        log.iter()
            .any(|key| key.split('(').next().unwrap_or(key) == *name)
    })
}

#[test]
fn interned_slots_are_reclaimed_across_an_edit_session() {
    let session = drive_until_collected();

    for name in PER_REVISION_INTERNED {
        assert!(
            session.slots(name) > 0,
            "`{name}` was never interned during the session, so this test proves nothing about \
             it — either the ingredient was renamed, or the corpus no longer reaches its query \
             path (see the `corpus` doc comment). Measured: {}",
            session.report()
        );
        assert!(
            session.reuses(name) > 0,
            "salsa's interned garbage collector never reclaimed a `{name}` slot, even after \
             driving revisions up to the shard-scaled cap, so every keystroke's key is retained \
             for the session — a KB-per-keystroke leak of the issue #1035 class. The two ways to \
             cause this are documented in the crate docs' \"The interned garbage collector is \
             load-bearing\" section: an input raised above `Durability::LOW` (see \
             `raising_input_durability_disables_the_collector`, which reproduces exactly this \
             reading), or a slot interned outside a tracked query. If the collector *is* running \
             and this machine simply needs longer, `edit_cap` is the bound to revisit — but \
             check the two causes first. Measured: {}",
            session.report()
        );
    }
}

#[test]
fn raising_input_durability_disables_the_collector() {
    // Fixed length, and no early stop: this session must observe *no* reuse,
    // so there is nothing to stop on.
    let session = drive_edit_session(InputDurability::RaisedToHigh, EDITS, false);

    for name in BODY_KEYED_INTERNED {
        let slots = session.slots(name);
        assert!(
            slots >= EDITS as usize,
            "the control session retained only {slots} `{name}` slots across {EDITS} edits, so \
             the corpus is no longer re-keying it once per edit and \
             `interned_slots_are_reclaimed_across_an_edit_session` is measuring nothing. Fix the \
             corpus (see its doc comment), not this bound. Measured: {}",
            session.report()
        );
    }

    for name in PER_REVISION_INTERNED {
        assert_eq!(
            session.reuses(name),
            0,
            "salsa reclaimed `{name}` slots even though both inputs were set to \
             `Durability::HIGH`. Interned slot reuse is supposed to be restricted to \
             `Durability::LOW`, so this means salsa's collection policy changed: the reasoning in \
             docs/design/rust/salsa-interned-gc.md and the crate docs' \"The interned garbage \
             collector is load-bearing\" section no longer describes the library, and the \
             durability rule may no longer be load-bearing. Measured: {}",
            session.report()
        );
    }
}

/// Crate roots that can name this crate's salsa inputs, and so are the only
/// places a durability bump on them could be written.
///
/// `tcl-lsp-server` is the sole dependant of `tcl-lsp-db` in the workspace
/// (`grep -l tcl-lsp-db rust/*/Cargo.toml`); if that stops being true, the new
/// dependant belongs in this list. Paths are relative to this crate's manifest
/// directory.
const DURABILITY_SCAN_ROOTS: [&str; 2] = [".", "../tcl-lsp-server"];

/// The ways salsa lets a caller move an input off `Durability::LOW`:
/// `Setter::with_durability`, the builder's whole-struct `durability`, and the
/// builder's generated per-field `<field>_durability`.
fn durability_call(code: &str) -> bool {
    code.contains(".durability(") || code.contains("_durability(")
}

/// Every `.rs` file under `dir`, skipping build output.
fn rust_sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|err| {
        panic!(
            "cannot read {} while scanning for durability call sites: {err}",
            dir.display()
        )
    });
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The half of the invariant the behavioural tests cannot reach: durability is
/// chosen at the *setter call site*, so a `Durability::HIGH` written by the
/// server would never appear in a database this crate's own tests build.
///
/// The workspace currently contains **zero** such call sites, so the gate is a
/// "still zero?" over the two crates that can name `SourceFile`,
/// `AnalyserConfig`, and `Project`. A future input that genuinely wants a
/// durability bump — one no interned key is created under — has to update this
/// test deliberately, with the reasoning written down, rather than slip through
/// review as a free optimisation.
#[test]
fn no_input_durability_is_raised() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // This file sets durability on purpose, in the control session above.
    let this_file = manifest.join("tests").join("interned_gc.rs");

    let mut files = Vec::new();
    for root in DURABILITY_SCAN_ROOTS {
        let path = manifest.join(root);
        assert!(
            path.is_dir(),
            "durability scan root {} no longer exists — the crate layout moved and this \
             guardrail is scanning nothing",
            path.display()
        );
        rust_sources(&path, &mut files);
    }
    assert!(
        files.len() > 1,
        "the durability scan found only {} Rust source(s), which cannot be right",
        files.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for file in files {
        if file == this_file {
            continue;
        }
        let text = std::fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("read {}: {err}", file.display()));
        for (number, line) in text.lines().enumerate() {
            // Comments and doc comments discuss this rule at length; only real
            // code counts.
            let code = line.split("//").next().unwrap_or("");
            if durability_call(code) {
                offenders.push(format!(
                    "{}:{}: {}",
                    file.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "salsa input durability is set at {} call site(s):\n  {}\n\nEvery per-body interned key \
         in tcl-lsp-db is minted by a query that has read `SourceFile` / `AnalyserConfig` / \
         `Project`, and inherits the minimum durability of those reads. Salsa's interned garbage \
         collector only ever reclaims `Durability::LOW` slots, so raising any of these turns the \
         edit path back into a KB-per-keystroke leak of the issue #1035 class — while reading in \
         review like a revalidation win. See the \"The interned garbage collector is \
         load-bearing\" section of tcl-lsp-db's crate documentation and \
         docs/design/rust/salsa-interned-gc.md.",
        offenders.len(),
        offenders.join("\n  ")
    );
}
