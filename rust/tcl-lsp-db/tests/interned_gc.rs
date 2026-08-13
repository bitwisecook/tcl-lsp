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
//! Reuse counts looked like the fix, and were not. They remove the shard
//! dependence from the *assertion* but not from whether an event ever fires:
//! a slot is only reclaimable once its **own** shard's LRU tail holds a stale
//! entry, so a small working set spread across a wide machine's shards can be
//! perfectly healthy and still never produce a single reuse. Measured by
//! varying the visible CPU count, all else equal:
//!
//! | cores | shards | `TaintSummaryKey` reuses | `SummaryDepsKey` reuses |
//! |------:|-------:|-------------------------:|------------------------:|
//! |     1 |      4 |                       32 |                     119 |
//! |     2 |      8 |                       28 |                      39 |
//! |     4 |     16 |                       22 |                      34 |
//! |    CI |    32+ |                        0 |                       1 |
//!
//! Asserting on those zeroes was a false alarm, not a leak: the two summary
//! ingredients hold a *bounded* set of slots whether or not the collector
//! runs (24 and 32 respectively, unchanged from 24 edits to 96, under both
//! `LOW` and `HIGH` durability), so they can never demonstrate the leak the
//! test exists to catch.
//!
//! What the leak actually looks like is **slots tracking the edit count**, so
//! that is what is asserted: drive a short session and a session four times
//! longer, and require the live slot count not to scale with it
//! ([`SLOT_GROWTH_BOUND`]). That is machine-independent — it compares two runs
//! on the same machine — and strictly closer to the invariant. Reuse counts
//! stay in the failure report as a diagnostic.
//!
//! # The two behavioural tests
//!
//! [`interned_slots_stay_bounded_across_an_edit_session`] drives a short typing
//! session and one four times longer through the production queries, and
//! asserts the live slot count of every one of the six ingredients does not
//! scale with the session.
//!
//! [`raising_input_durability_disables_the_collector`] drives the *same* two
//! sessions with `SourceFile` and `AnalyserConfig` marked `Durability::HIGH`
//! and asserts the collector goes completely silent, and that the slot counts
//! then *do* break the bound the first test applies. It is the break-the-invariant experiment, kept in the suite
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

/// The short session. Well above salsa's `DEFAULT_REVISIONS` staleness window
/// (3), so the collector has had every chance to run by the end of it.
const SHORT_SESSION: u32 = 24;

/// The long session — four times the short one. The invariant is measured as
/// the ratio between the two.
const LONG_SESSION: u32 = SHORT_SESSION * 4;

/// How much the live slot count may grow when the session grows fourfold.
///
/// A healthy working set is bounded by the *program*, not by how long the user
/// has been typing, so it barely moves; a leak grows with the edit count.
/// Measured on this corpus, slots after 24 -> 96 edits:
///
/// | ingredient | healthy (`LOW`) | leaking (`HIGH`) |
/// |---|---|---|
/// | `ItemBodyKey`     | 54 -> 64 (1.19x) | 116 -> 404 (3.48x) |
/// | `FnLatticeKey`    | 50 -> 63 (1.26x) | 116 -> 404 (3.48x) |
/// | `TaintSummaryKey` | 23 -> 23 (1.00x) |  24 ->  24 (1.00x) |
/// | `SummaryDepsKey`  | 31 -> 31 (1.00x) |  32 ->  32 (1.00x) |
///
/// `2` sits with a wide margin either side. The two summary ingredients are
/// bounded under *both* conditions — their keys are the interprocedural
/// projections, whose set the corpus fixes — so they carry the invariant
/// trivially and the four body-keyed ones are what discriminates.
/// `raising_input_durability_disables_the_collector` asserts exactly that.
const SLOT_GROWTH_BOUND: usize = 2;

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
    /// Recorded so a failure report says how long the session was.
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
        format!("{} (over {} edits)", self.summary(), self.edits)
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
                0,
            ),
        ),
        InputDurability::RaisedToHigh => (
            SourceFile::builder(corpus(0), dialect, None, None, None, None, 0)
                .durability(salsa::Durability::HIGH)
                .new(db),
            AnalyserConfig::builder(Vec::new(), NonAsciiMode::Default, Vec::new(), None, None, 0)
                .durability(salsa::Durability::HIGH)
                .new(db),
        ),
    }
}

/// Drive exactly `max_edits` revisions of [`corpus`] through the two production
/// diagnostic queries, recording interned slot reuse as it goes.
///
fn drive_edit_session(durability: InputDurability, max_edits: u32) -> Session {
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

#[test]
fn interned_slots_stay_bounded_across_an_edit_session() {
    let short = drive_edit_session(InputDurability::Default, SHORT_SESSION);
    let long = drive_edit_session(InputDurability::Default, LONG_SESSION);

    for name in PER_REVISION_INTERNED {
        assert!(
            short.slots(name) > 0,
            "`{name}` was never interned during the session, so this test proves nothing about \
             it — either the ingredient was renamed, or the corpus no longer reaches its query \
             path (see the `corpus` doc comment). Measured: short {}",
            short.report()
        );
        let bound = short.slots(name) * SLOT_GROWTH_BOUND;
        assert!(
            long.slots(name) <= bound,
            "`{name}` retained {} slots after {LONG_SESSION} edits against {} after \
             {SHORT_SESSION} — {}x the session holding more than {SLOT_GROWTH_BOUND}x the \
             slots, so the working set is tracking how long the user has been typing rather \
             than the program. That is the issue #1035 leak shape. The two causes are \
             documented in the crate docs' \"The interned garbage collector is load-bearing\" \
             section: an input raised above `Durability::LOW` (see \
             `raising_input_durability_disables_the_collector`, which reproduces exactly this \
             reading), or a slot interned outside a tracked query. Measured: short {} | long {}",
            long.slots(name),
            short.slots(name),
            LONG_SESSION / SHORT_SESSION,
            short.report(),
            long.report(),
        );
    }
}

#[test]
fn raising_input_durability_disables_the_collector() {
    let session = drive_edit_session(InputDurability::RaisedToHigh, SHORT_SESSION);
    let long = drive_edit_session(InputDurability::RaisedToHigh, LONG_SESSION);

    for name in BODY_KEYED_INTERNED {
        let slots = session.slots(name);
        assert!(
            slots >= SHORT_SESSION as usize,
            "the control session retained only {slots} `{name}` slots across {SHORT_SESSION} \
             edits, so the corpus is no longer re-keying it once per edit and \
             `interned_slots_stay_bounded_across_an_edit_session` is measuring nothing. Fix the \
             corpus (see its doc comment), not this bound. Measured: {}",
            session.report()
        );
        // …and the leak really does trip the bound the primary test applies,
        // so that assertion cannot pass vacuously.
        assert!(
            long.slots(name) > slots * SLOT_GROWTH_BOUND,
            "with the collector disabled, `{name}` grew only {} -> {} across \
             {SHORT_SESSION} -> {LONG_SESSION} edits, which does not exceed the \
             {SLOT_GROWTH_BOUND}x bound `interned_slots_stay_bounded_across_an_edit_session` \
             applies — so that test would pass even with the leak restored and is no longer a \
             guard. Measured: short {} | long {}",
            slots,
            long.slots(name),
            session.report(),
            long.report(),
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
