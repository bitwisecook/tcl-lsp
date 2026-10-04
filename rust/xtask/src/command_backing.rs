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

//! WASM command-parity gate.
//!
//! `AGENTS.md` § *WASM command parity* asserts a contract: **every core-Tcl
//! command in `tcl-registry` (the source of truth) needs runtime backing** — a
//! handler in `runtime/rust/`, a definition in the Tcl library the runtime
//! embeds, or an explicit statement that nothing executes it there. A spec
//! states which through [`tcl_registry::RuntimeBacking`], and this gate holds
//! the statement to what the runtime says it registered:
//!
//! - the **declared** fact is each core Tcl command spec's `runtime_backing`
//!   (`required_package == None`, available at Tcl 9.0 or later);
//! - the **reported** fact is [`tcl_runtime::interp::Interp::backing_report`], asked of
//!   a fresh interpreter of the WASM runtime; [`tcl_vm::Vm::backing_report`]
//!   answers the same question for the bytecode VM and is reported beside it;
//! - a disagreement is drift and fails the gate: a command declared a shipped
//!   builtin that the runtime does not register, a command declared to be
//!   defined by a library file that the runtime does not report from that
//!   file, or a command declared as backed by nothing that the runtime
//!   registers after all;
//! - the one exception is [`KNOWN_UNBACKED`], the commands declared a shipped
//!   builtin that the runtime does not yet back. A waiver that no longer
//!   applies — its command is backed, or is not a core command — fails too.
//!
//! The generated report (`docs/generated/wasm-command-backing.md`) is a
//! rendering of those answers; `--check` fails when it is stale.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;
use tcl_dialect::model::Family;
use tcl_dialect::model::surface_admits_from;
use tcl_registry::{BodySource, RuntimeBacking};
use tcl_runtime_api::{BackingReport, RegisteredBacking};

use anyhow::{Context, Result};
use regex::Regex;

use crate::util::repo_root;

/// The generated per-command backing report (committed, drift-checked).
const REPORT_PATH: &str = "docs/generated/wasm-command-backing.md";

/// The one list the gate keeps: core commands declared a shipped builtin that
/// the WASM runtime does not yet back — real gaps, each with its reason.
/// Allow-listed so the gate stays green while they are implemented one by one;
/// removing an entry (as its commands gain a handler) is the visible progress
/// marker. Names are canonical (no leading `::`), kept sorted. An entry ending
/// in `::` is a namespace prefix and covers every core command under it.
///
/// The Tcl 9.1 entries (`divmod`, `frexp`, `lfilter`, `modf`, `remquo`, `timer`,
/// `unicode`) are visible to this gate because [`core_commands`] admits every
/// command available at 9.0 or later; they are genuinely unbacked.
const KNOWN_UNBACKED: &[(&str, &str)] = &[
    (
        "callback",
        "TclOO oo::Helpers::callback (issue #923 idx 51) — builds a command prefix that re-enters a method of the current object; needs a live method frame, no runtime handler (same situation as `link`)",
    ),
    (
        "divmod",
        "TIP 745 (Tcl 9.1) combined quotient/remainder list command; not yet implemented in runtime/rust",
    ),
    (
        "fpclassify",
        "Tcl 9.0 floating-point classifier (TIP 521); not yet implemented in runtime/rust",
    ),
    (
        "frexp",
        "TIP 745 (Tcl 9.1) IEEE-754 mantissa/exponent split; not yet implemented in runtime/rust",
    ),
    (
        "lfilter",
        "Tcl 9.1 list-filter command; not yet implemented in runtime/rust",
    ),
    (
        "link",
        "TclOO oo::Helpers::link (issue #923 idx 113) — installs a per-object-namespace alias to a method via the object's own command table, not a standalone dispatchable command; no runtime handler",
    ),
    (
        "modf",
        "TIP 745 (Tcl 9.1) integer/fractional split; not yet implemented in runtime/rust",
    ),
    (
        "mymethod",
        "TclOO oo::Helpers::mymethod (issue #923 idx 51) — `callback` under its Tcllib-compatibility name; same missing runtime handler",
    ),
    (
        "oo::Helpers::",
        "qualified `::oo::Helpers::*` spelling (issue #1026): runtime/rust registers only the bare, method-context name its dispatch installs, so a direct qualified call is `invalid command name`",
    ),
    (
        "remquo",
        "TIP 745 (Tcl 9.1) IEEE remainder with low quotient bits; not yet implemented in runtime/rust",
    ),
    (
        "tcl::dict::",
        "standalone `::tcl::dict::*` ensemble-implementation spelling (issue #923 idx 105): runtime/rust backs only the `dict` ensemble head, not the qualified name — a direct call is `invalid command name`",
    ),
    (
        "tcl::unsupported::grapheme",
        "Tcl 9.1.0 grapheme-cluster ensemble; needs UAX #29 segmentation tables not yet in runtime/rust",
    ),
    (
        "tcl::zipfs",
        "ZIP virtual filesystem — no runtime implementation yet; pre-existing gap, unrelated to issue #923",
    ),
    (
        "timer",
        "Tcl 9.1 timer command; not yet implemented in runtime/rust",
    ),
    (
        "unicode",
        "Tcl 9.1 Unicode-introspection ensemble; not yet implemented in runtime/rust",
    ),
    (
        "zipfs",
        "ZIP virtual filesystem — no runtime implementation yet; pre-existing gap, unrelated to issue #923",
    ),
];

/// Strip a single leading `::` so a spec's name (`tcl::build-info`,
/// `::tcl::dict::get`) and a runtime's answer canonicalise to the same key. The
/// registry carries both fully-qualified and bare spellings of some commands;
/// this collapses them for cross-referencing.
fn canon(name: &str) -> &str {
    name.strip_prefix("::").unwrap_or(name)
}

/// The reason [`KNOWN_UNBACKED`] gives for the core command `name`, exactly or
/// through a namespace prefix.
fn waiver(name: &str) -> Option<&'static str> {
    let name = canon(name);
    KNOWN_UNBACKED
        .iter()
        .find(|(entry, _)| {
            if entry.ends_with("::") {
                name.starts_with(entry) && name.len() > entry.len()
            } else {
                *entry == name
            }
        })
        .map(|(_, reason)| *reason)
}

/// One core command: its name as the registry spells it, and the backing its
/// spec declares.
struct CoreCommand {
    name: &'static str,
    declared: RuntimeBacking,
}

/// The core Tcl commands the runtime must back: `tcl-registry`'s built-in
/// (`required_package == None`) command specs that are available at **Tcl
/// 9.0 or later** (the runtime's target — `runtime/rust` also backs TIP 745
/// (Tcl 9.1) `::tcl::mathfunc::*` commands like `gamma`/`cbrt`/`fma`, so the
/// gate must consider a 9.1-only-gated command "core" too, or a regression
/// in that backing would go undetected). The dialect filter drops the
/// tcl-family commands modelled only for the embedded/EDA dialects (F5
/// iRules/iApps, Expect, the EDA tools) — `readFile`, `foreachLine`,
/// `disabled_in_irules`, … — which the core runtime is not expected to
/// provide. `None` dialects means "all dialects". Namespaced ensemble
/// *implementation* commands (`::tcl::…`) stay in the set so their
/// declaration is explicit.
///
/// Adversarial-review finding, twice over: the question is "9.0 **or
/// later**", and asking about the 9.0 point alone silently drops every
/// command introduced in 9.1 — not "classified as a gap", genuinely
/// invisible to the gate. [`surface_admits_from`] is the question this
/// wants.
///
/// A name the registry holds more than one spec for is one command, and its
/// specs must declare the same backing.
fn core_commands() -> Result<BTreeMap<&'static str, CoreCommand>> {
    let mut core: BTreeMap<&'static str, CoreCommand> = BTreeMap::new();
    for spec in tcl_registry::commands::tcl::tcl_command_specs()
        .iter()
        .filter(|spec| spec.required_package.is_none())
        .filter(|spec| {
            spec.surface
                .is_none_or(|rows| surface_admits_from(rows, Family::Tcl, "9.0"))
        })
    {
        let name: &'static str = spec.name;
        let declared = spec.runtime_backing;
        if let Some(existing) = core.get(name) {
            anyhow::ensure!(
                existing.declared == declared,
                "`{name}` has specs that declare different backings: {:?} and {declared:?}",
                existing.declared
            );
        } else {
            core.insert(name, CoreCommand { name, declared });
        }
    }
    Ok(core)
}

/// What each runtime reports, asked of a fresh interpreter of each.
struct Reports {
    wasm: BackingReport,
    vm: BackingReport,
}

impl Reports {
    fn ask() -> Self {
        Self {
            wasm: BackingReport::from_entries(tcl_runtime::interp::Interp::new().backing_report()),
            vm: BackingReport::from_entries(tcl_vm::Vm::new().backing_report()),
        }
    }
}

/// How one core command's declaration stands against what the WASM runtime
/// reports.
#[derive(Debug, PartialEq, Eq)]
enum Standing {
    /// The runtime bears the declaration out.
    Backed,
    /// The command is declared a shipped builtin the runtime does not back, and
    /// [`KNOWN_UNBACKED`] says why.
    KnownGap(&'static str),
    /// A declaration the runtime contradicts, and no waiver covers.
    Drift(String),
}

/// The word a report prints for a runtime's answer. A build without the
/// numeric tower says its handlers need it, and a report must read the same
/// whichever build of the runtime the gate was asked of, so those print as the
/// handlers they are in a build with it.
fn reported_label(backing: RegisteredBacking) -> String {
    match backing {
        RegisteredBacking::Stdlib { file } => format!("stdlib {file}"),
        RegisteredBacking::NeedsNumericTower => RegisteredBacking::Builtin.label().to_owned(),
        other => other.label().to_owned(),
    }
}

/// The spelling a report prints for a declaration.
fn declared_label(declared: RuntimeBacking) -> String {
    match declared {
        RuntimeBacking::ShippedBuiltin { .. } => "shipped-builtin".to_owned(),
        RuntimeBacking::TclBody {
            source: BodySource::PackageSource { relative_path },
            ..
        } => format!("tcl-body {relative_path}"),
        RuntimeBacking::TclBody {
            source: BodySource::PackText { .. },
            ..
        } => "tcl-body (pack text)".to_owned(),
        RuntimeBacking::HostNative => "host-native".to_owned(),
        RuntimeBacking::None => "none".to_owned(),
    }
}

/// Hold one declaration to the WASM runtime's answer.
fn stand(command: &CoreCommand, wasm: RegisteredBacking) -> Standing {
    let name = command.name;
    let disagree = |expected: &str| {
        Standing::Drift(format!(
            "`{name}` declares {} but the runtime reports {}; {expected}",
            declared_label(command.declared),
            reported_label(wasm),
        ))
    };
    match command.declared {
        RuntimeBacking::ShippedBuiltin { .. } | RuntimeBacking::HostNative => {
            if matches!(
                wasm,
                RegisteredBacking::Builtin
                    | RegisteredBacking::Object
                    | RegisteredBacking::NeedsNumericTower
            ) {
                Standing::Backed
            } else if let Some(reason) = waiver(name) {
                Standing::KnownGap(reason)
            } else {
                disagree("add a runtime handler, or waive it in `KNOWN_UNBACKED`")
            }
        }
        RuntimeBacking::TclBody {
            source: BodySource::PackageSource { relative_path },
            ..
        } => match wasm {
            RegisteredBacking::Stdlib { file } if file == relative_path => Standing::Backed,
            _ => disagree("the embedded library must define it from that file"),
        },
        RuntimeBacking::TclBody { .. } => {
            disagree("a core command's body comes from a library file")
        }
        RuntimeBacking::None => {
            if wasm.executes() {
                disagree("declare its backing on the spec, or remove the registration")
            } else {
                Standing::Backed
            }
        }
    }
}

/// Waivers that no longer apply: an exact entry that names no core command or
/// a command the runtime now backs, and a prefix entry under which every core
/// command is backed or which covers none.
fn stale_waivers(core: &BTreeMap<&'static str, CoreCommand>, wasm: &BackingReport) -> Vec<String> {
    let mut stale = Vec::new();
    for (entry, _) in KNOWN_UNBACKED {
        let covered: Vec<&CoreCommand> = core
            .values()
            .filter(|command| {
                let name = canon(command.name);
                if entry.ends_with("::") {
                    name.starts_with(entry) && name.len() > entry.len()
                } else {
                    name == *entry
                }
            })
            .collect();
        if covered.is_empty() {
            stale.push(format!(
                "`{entry}` — waived but not a core registry command"
            ));
            continue;
        }
        let gaps = covered
            .iter()
            .filter(|command| {
                !matches!(
                    wasm.of(command.name),
                    RegisteredBacking::Builtin
                        | RegisteredBacking::Object
                        | RegisteredBacking::NeedsNumericTower
                )
            })
            .count();
        if gaps == 0 {
            stale.push(format!(
                "`{entry}` — waived but the runtime now backs {}",
                if entry.ends_with("::") {
                    "every command under it"
                } else {
                    "it"
                }
            ));
        }
    }
    stale
}

/// Render the committed backing report from the declared facts and the two
/// runtimes' answers.
fn render_report(core: &BTreeMap<&'static str, CoreCommand>, reports: &Reports) -> String {
    let mut tally: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut rows = String::new();
    for command in core.values() {
        let wasm = reports.wasm.of(command.name);
        let vm = reports.vm.of(command.name);
        let standing = stand(command, wasm);
        let (status, note) = match (&standing, command.declared, wasm) {
            (Standing::KnownGap(reason), _, _) => (
                format!("{} (known gap)", reported_label(wasm)),
                (*reason).to_owned(),
            ),
            (Standing::Drift(_), _, _) => {
                (format!("{} (DRIFT)", reported_label(wasm)), String::new())
            }
            (_, _, RegisteredBacking::Unsupported) => (
                reported_label(wasm),
                "registered as an explicit not-supported stub".to_owned(),
            ),
            _ => (reported_label(wasm), String::new()),
        };
        *tally
            .entry((declared_label(command.declared), status.clone()))
            .or_default() += 1;
        let _ = writeln!(
            rows,
            "| `{}` | {} | {status} | {} | {note} |",
            command.name,
            declared_label(command.declared),
            reported_label(vm),
        );
    }
    let mut counts = String::new();
    for ((declared, wasm), count) in &tally {
        let _ = writeln!(counts, "| {declared} | {wasm} | {count} |");
    }
    let mut out = String::new();
    let _ = write!(
        out,
        "# WASM command backing coverage\n\n\
         > Auto-generated by `cargo xtask command-backing`. Do not edit by hand.\n\
         > The gate (`make xtask-check`) fails on drift between a command's\n\
         > declared backing and what the WASM runtime reports, on a waiver that\n\
         > no longer applies, and on a stale report.\n\n\
         Source of truth: `tcl-registry` core command specs (`required_package == None`),\n\
         restricted to those available at Tcl 9.0 or later. `backing` is the spec's own\n\
         `runtime_backing`. `wasm` is what `runtime/rust` reports for the name\n\
         (`Interp::backing_report`, built with the embedded Tcl library), and `vm` what\n\
         `tcl-vm` reports (`Vm::backing_report`); a build of the WASM runtime without the\n\
         numeric tower reports the handlers that need it as needing it, and this report\n\
         prints them as the handlers they are. A command declared a shipped builtin that\n\
         the WASM runtime lacks is a *known gap* when `KNOWN_UNBACKED` in\n\
         `rust/xtask/src/command_backing.rs` says so; every other disagreement fails the gate.\n\n\
         | backing | wasm | count |\n| --- | --- | --- |\n{counts}\
         | **total** | | {} |\n\n\
         | command | backing | wasm | vm | note |\n| --- | --- | --- | --- | --- |\n{rows}",
        core.len(),
    );
    out
}

/// Read the runtime file ensemble declaration and dispatch arms. This is
/// deliberately source-level: the backing report names commands, not their
/// subcommands, so it cannot prove that the registry's subcommand and arity
/// surface for `file` is implemented.
type FileArityMap = BTreeMap<String, (u16, u16)>;
type FileSurface = (BTreeSet<String>, FileArityMap);

fn scan_file_surface(root: &std::path::Path) -> Result<FileSurface> {
    let path = root.join("runtime/rust/src/cmd_fs.rs");
    let text = fs::read_to_string(&path)?;
    let table = text
        .split("const FILE_RUNTIME_ARITIES:")
        .nth(1)
        .and_then(|s| s.split("];\n\nfn file_cmd").next())
        .unwrap_or("");
    let arity_re = Regex::new(r#"\(\s*"([^"]+)"\s*,\s*(\d+)\s*,\s*(\d+)\s*\)"#)
        .expect("static file arity regex");
    let mut arities = BTreeMap::new();
    for cap in arity_re.captures_iter(table) {
        arities.insert(
            cap[1].to_owned(),
            (cap[2].parse().unwrap_or(0), cap[3].parse().unwrap_or(0)),
        );
    }
    let dispatch = text
        .split("match sub.as_slice()")
        .nth(1)
        .and_then(|s| s.split("_ => unreachable!").next())
        .unwrap_or("");
    let arm_re = Regex::new(r#"b"([a-zA-Z][a-zA-Z0-9_]*)"\s*(?:\||=>)"#)
        .expect("static file dispatch regex");
    let arms = arm_re
        .captures_iter(dispatch)
        .map(|c| c[1].to_owned())
        .collect();
    Ok((arms, arities))
}

fn file_fidelity_errors(root: &std::path::Path) -> Result<Vec<String>> {
    let (arms, arities) = scan_file_surface(root)?;
    let specs = tcl_registry::commands::tcl::tcl_command_specs();
    let file = specs
        .iter()
        .find(|s| s.name == "file")
        .context("registry has no file command")?;
    let registry: BTreeMap<String, (u16, u16)> = file
        .subcommands
        .iter()
        .map(|s| (s.name.to_owned(), (s.arity.min, s.arity.max)))
        .collect();
    Ok(compare_file_surface(&arms, &arities, &registry))
}

fn compare_file_surface(
    arms: &BTreeSet<String>,
    arities: &BTreeMap<String, (u16, u16)>,
    registry: &BTreeMap<String, (u16, u16)>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for (name, &(min, max)) in registry {
        if !arms.contains(name) {
            errors.push(format!("file {name}: no runtime dispatch arm"));
        }
        match arities.get(name) {
            None => errors.push(format!("file {name}: no runtime arity declaration")),
            Some(&(rmin, rmax)) if (rmin, rmax) != (min, max) => errors.push(format!(
                "file {name}: runtime arity {rmin}..={rmax} != registry {min}..={max}"
            )),
            _ => {}
        }
    }
    for name in arities.keys() {
        if !registry.contains_key(name) {
            errors.push(format!(
                "file {name}: runtime arity has no registry subcommand"
            ));
        }
    }
    errors
}

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let core = core_commands()?;
    let reports = Reports::ask();
    let file_fidelity = file_fidelity_errors(&root)?;

    let drift: Vec<String> = core
        .values()
        .filter_map(
            |command| match stand(command, reports.wasm.of(command.name)) {
                Standing::Drift(why) => Some(why),
                Standing::Backed | Standing::KnownGap(_) => None,
            },
        )
        .collect();
    let stale = stale_waivers(&core, &reports.wasm);
    let report = render_report(&core, &reports);
    let report_path = root.join(REPORT_PATH);

    let tower_gated = reports
        .wasm
        .iter()
        .filter(|(_, backing)| *backing == RegisteredBacking::NeedsNumericTower)
        .count();
    if tower_gated > 0 {
        eprintln!(
            "note: the WASM runtime was built without the numeric tower (libtommath); \
             {tower_gated} commands that need it are taken as the handlers a build with it registers"
        );
    }

    if !check {
        fs::write(&report_path, &report)
            .with_context(|| format!("writing {}", report_path.display()))?;
        eprintln!("wrote {REPORT_PATH} ({} core commands)", core.len());
        eprintln!(
            "  {} disagreement(s) between declared backing and the WASM runtime",
            drift.len()
        );
        return Ok(ExitCode::SUCCESS);
    }

    let mut failed = false;
    let current = fs::read_to_string(&report_path).unwrap_or_default();
    if current != report {
        eprintln!("{REPORT_PATH} is stale — run `cargo xtask command-backing`");
        failed = true;
    }
    if !drift.is_empty() {
        eprintln!(
            "{} core registry command(s) declare a backing the WASM runtime does not bear out:",
            drift.len()
        );
        for why in &drift {
            eprintln!("  - {why}");
        }
        failed = true;
    }
    if !stale.is_empty() {
        eprintln!("stale `KNOWN_UNBACKED` waivers:");
        for entry in &stale {
            eprintln!("  - {entry}");
        }
        failed = true;
    }
    if !file_fidelity.is_empty() {
        eprintln!("runtime file command fidelity failures:");
        for error in &file_fidelity {
            eprintln!("  - {error}");
        }
        failed = true;
    }
    if failed {
        return Ok(ExitCode::from(1));
    }
    eprintln!(
        "OK: all {} core registry commands declare a backing the WASM runtime bears out.",
        core.len()
    );
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::{
        CoreCommand, KNOWN_UNBACKED, Reports, Standing, canon, compare_file_surface, core_commands,
        file_fidelity_errors, render_report, reported_label, scan_file_surface, stale_waivers,
        stand, waiver,
    };
    use crate::util::repo_root;
    use std::collections::{BTreeMap, BTreeSet};
    use tcl_registry::RuntimeBacking;
    use tcl_runtime_api::{BackingReport, RegisteredBacking};

    fn command(name: &'static str, declared: RuntimeBacking) -> CoreCommand {
        CoreCommand { name, declared }
    }

    fn shipped(name: &'static str) -> CoreCommand {
        command(name, RuntimeBacking::shipped(name))
    }

    #[test]
    fn file_surface_matches_registry_subcommands_and_arities() {
        let root = repo_root();
        let (arms, arities) = scan_file_surface(&root).expect("scan file runtime surface");
        assert!(arms.contains("atime"));
        assert_eq!(arities.get("stat"), Some(&(1, 2)));
        assert!(
            file_fidelity_errors(&root)
                .expect("compare file runtime surface")
                .is_empty()
        );
    }

    #[test]
    fn file_fidelity_rejects_missing_arm_and_wrong_arity() {
        let arms = BTreeSet::from(["copy".to_owned()]);
        let arities = std::collections::BTreeMap::from([("copy".to_owned(), (1, 2))]);
        let registry = std::collections::BTreeMap::from([
            ("copy".to_owned(), (2, u16::MAX)),
            ("link".to_owned(), (1, 2)),
        ]);
        let errors = compare_file_surface(&arms, &arities, &registry);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("file copy: runtime arity"))
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("file link: no runtime dispatch arm"))
        );
    }

    /// A command declared a shipped builtin that the runtime does not register
    /// is drift unless a waiver covers it, exactly or through a namespace
    /// prefix; a waiver covers the command only while the runtime lacks it.
    #[test]
    fn a_declared_builtin_the_runtime_lacks_is_drift_unless_waived() {
        let unwaived = shipped("no_such_core_command");
        for absent in [
            RegisteredBacking::Absent,
            RegisteredBacking::Unsupported,
            RegisteredBacking::Stdlib { file: "init.tcl" },
        ] {
            assert!(
                matches!(stand(&unwaived, absent), Standing::Drift(_)),
                "{absent:?}"
            );
        }
        for backed in [
            RegisteredBacking::Builtin,
            RegisteredBacking::Object,
            RegisteredBacking::NeedsNumericTower,
        ] {
            assert_eq!(stand(&unwaived, backed), Standing::Backed, "{backed:?}");
        }

        for name in ["divmod", "::tcl::dict::get", "oo::Helpers::link"] {
            let gap = shipped(name);
            assert!(
                matches!(
                    stand(&gap, RegisteredBacking::Absent),
                    Standing::KnownGap(_)
                ),
                "{name}: waived while absent"
            );
            assert_eq!(
                stand(&gap, RegisteredBacking::Builtin),
                Standing::Backed,
                "{name}: backed, the waiver no longer matters to the command"
            );
        }
        assert!(
            waiver("tcl::dict::").is_none(),
            "a prefix covers what is under it"
        );
    }

    /// A command declared as backed by nothing is drift when the runtime
    /// registers it after all, and not when it only refuses or is absent.
    #[test]
    fn a_declared_none_the_runtime_registers_is_drift() {
        let none = command("no_such_core_command", RuntimeBacking::None);
        for registered in [
            RegisteredBacking::Builtin,
            RegisteredBacking::Object,
            RegisteredBacking::Stdlib { file: "init.tcl" },
            RegisteredBacking::NeedsNumericTower,
        ] {
            assert!(
                matches!(stand(&none, registered), Standing::Drift(_)),
                "{registered:?}"
            );
        }
        for not_executed in [RegisteredBacking::Absent, RegisteredBacking::Unsupported] {
            assert_eq!(
                stand(&none, not_executed),
                Standing::Backed,
                "{not_executed:?}"
            );
        }
    }

    /// A command declared to be defined by a library file is backed only when
    /// the runtime reports a definition from that file.
    #[test]
    fn a_declared_library_body_must_be_reported_from_that_file() {
        let body = command(
            "no_such_core_command",
            RuntimeBacking::package_source("init.tcl"),
        );
        assert_eq!(
            stand(&body, RegisteredBacking::Stdlib { file: "init.tcl" }),
            Standing::Backed
        );
        for other in [
            RegisteredBacking::Stdlib {
                file: "package.tcl",
            },
            RegisteredBacking::Builtin,
            RegisteredBacking::Absent,
        ] {
            assert!(
                matches!(stand(&body, other), Standing::Drift(_)),
                "{other:?}"
            );
        }
    }

    /// A waiver stops applying when its command is backed, when nothing under
    /// its prefix is left unbacked, and when it names no core command.
    #[test]
    fn a_waiver_that_no_longer_applies_is_stale() {
        let mut core: BTreeMap<&'static str, CoreCommand> = BTreeMap::new();
        for name in [
            "divmod",
            "::tcl::dict::get",
            "::tcl::dict::set",
            "oo::Helpers::link",
        ] {
            core.insert(name, shipped(name));
        }
        let report = |backed: &[&str]| {
            BackingReport::from_entries(
                backed
                    .iter()
                    .map(|name| ((*name).to_owned(), RegisteredBacking::Builtin)),
            )
        };
        let names = |stale: &[String]| stale.join("\n");

        let none_backed = names(&stale_waivers(&core, &report(&[])));
        assert!(!none_backed.contains("`divmod` —"), "{none_backed}");
        assert!(!none_backed.contains("`tcl::dict::` —"), "{none_backed}");
        assert!(
            none_backed.contains("`callback` — waived but not a core"),
            "{none_backed}"
        );

        let closed = names(&stale_waivers(&core, &report(&["divmod"])));
        assert!(
            closed.contains("`divmod` — waived but the runtime now backs it"),
            "{closed}"
        );

        let one_of_two = names(&stale_waivers(&core, &report(&["::tcl::dict::get"])));
        assert!(!one_of_two.contains("`tcl::dict::` —"), "{one_of_two}");
        let both = names(&stale_waivers(
            &core,
            &report(&["::tcl::dict::get", "tcl::dict::set"]),
        ));
        assert!(
            both.contains(
                "`tcl::dict::` — waived but the runtime now backs every command under it"
            ),
            "{both}"
        );
    }

    /// A build of the WASM runtime without the numeric tower reports its
    /// tower-gated handlers as needing it, and the report reads the same.
    #[test]
    fn a_build_without_the_tower_renders_its_handlers_as_handlers() {
        assert_eq!(
            reported_label(RegisteredBacking::NeedsNumericTower),
            reported_label(RegisteredBacking::Builtin)
        );
        assert_eq!(
            reported_label(RegisteredBacking::Stdlib { file: "init.tcl" }),
            "stdlib init.tcl"
        );
    }

    /// The waiver list is sorted and canonical, so a reader finds an entry and
    /// an entry means one thing.
    #[test]
    fn the_waivers_are_sorted_and_canonical() {
        let names: Vec<&str> = KNOWN_UNBACKED.iter().map(|(name, _)| *name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "kept sorted");
        for name in names {
            assert_eq!(
                canon(name),
                name,
                "{name} is written without a leading `::`"
            );
        }
    }

    /// The drift guard this module's own `--check` flag enforces, run directly
    /// as a unit test (mirroring the `committed_X_matches_generated` pattern
    /// every sibling generator in this crate already has): fails if
    /// `docs/generated/wasm-command-backing.md` is stale relative to the specs
    /// and what the two runtimes report.
    #[test]
    fn committed_wasm_command_backing_matches_generated() {
        let core = core_commands().expect("the core commands");
        let report = render_report(&core, &Reports::ask());
        let committed = std::fs::read_to_string(repo_root().join(super::REPORT_PATH))
            .expect("reading docs/generated/wasm-command-backing.md");
        assert_eq!(
            committed, report,
            "docs/generated/wasm-command-backing.md is stale — run `cargo xtask command-backing`"
        );
    }

    /// Every core command's declaration is borne out by the WASM runtime, or
    /// waived, and no waiver is stale — exactly what `--check` reports.
    #[test]
    fn every_core_command_declares_what_the_wasm_runtime_bears_out() {
        let core = core_commands().expect("the core commands");
        let reports = Reports::ask();
        let drift: Vec<String> = core
            .values()
            .filter_map(
                |command| match stand(command, reports.wasm.of(command.name)) {
                    Standing::Drift(why) => Some(why),
                    Standing::Backed | Standing::KnownGap(_) => None,
                },
            )
            .collect();
        assert!(drift.is_empty(), "drift: {drift:#?}");
        let stale = stale_waivers(&core, &reports.wasm);
        assert!(stale.is_empty(), "stale waivers: {stale:#?}");
        let names: BTreeSet<&str> = core.keys().copied().collect();
        assert!(names.len() > 300, "the core set is {}", names.len());
    }
}
