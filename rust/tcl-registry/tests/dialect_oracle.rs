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

//! Live differential audit of the registry's dialect gating against real
//! `tclsh` binaries.
//!
//! For every curated command the registry knows, the availability the registry
//! reports for the Tcl 8.6 and Tcl 9.0 dialects must match whether a real
//! `tclsh8.6` / `tclsh9.0` actually provides that command. This ties
//! [`CommandRegistry::get_for_surface`] to ground truth — a registry that
//! mis-gates a 9.0-only command (or forgets to add one) fails here.
//!
//! Skips cleanly unless **both** an 8.6 and a 9.0 interpreter are on `PATH`
//! (the differential needs the boundary), so CI without a dual Tcl install is
//! unaffected.

use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use tcl_dialect::model::Family;
use tcl_dialect::model::{SurfaceQuery, surface_admits};
use tcl_registry::CommandRegistry;

/// Run `script` on `tclsh` via stdin, returning stdout (or `None` on spawn
/// failure).
fn run_tcl(tclsh: &str, script: &str) -> Option<String> {
    let mut child = Command::new(tclsh)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(script.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Find a `tclsh` on `PATH` whose `info patchlevel` starts with `prefix`
/// (e.g. `"8.6"` / `"9.0"`), trying the versioned name first.
fn find_tclsh(prefix: &str, versioned: &str) -> Option<String> {
    for cand in [versioned, "tclsh"] {
        if let Some(out) = run_tcl(cand, "puts -nonewline [info patchlevel]")
            && out.starts_with(prefix)
        {
            return Some(cand.to_string());
        }
    }
    None
}

/// Probe an interpreter for whether each name in `names` resolves to a command.
/// `TclOO` and `http` are loaded first so their commands are visible; a name
/// resolves iff `namespace which -command` reports it.
fn probe_existence(tclsh: &str, names: &[&str]) -> HashMap<String, bool> {
    let list = names.join(" ");
    let script = format!(
        "catch {{package require TclOO}}\n\
         catch {{package require http}}\n\
         foreach n {{{list}}} {{\n\
         \x20 puts \"$n [expr {{[namespace which -command $n] ne \"\" ? 1 : 0}}]\"\n\
         }}\n"
    );
    let out = run_tcl(tclsh, &script).unwrap_or_default();
    out.lines()
        .filter_map(|l| {
            let (n, v) = l.rsplit_once(' ')?;
            Some((n.to_string(), v.trim() == "1"))
        })
        .collect()
}

/// Curated commands the registry should know, spanning the 8.6/9.0 boundary
/// plus stable controls. `tcl::idna` is deliberately omitted: it is provided
/// lazily by the `http` package and is not reliably visible to
/// `namespace which`, so it is not a clean differential probe.
const PROBES: &[&str] = &[
    // New in Tcl 9.0 (must be absent in 8.6, present in 9.0).
    "lseq",
    "lpop",
    "lremove",
    "ledit",
    "coroinject",
    "coroprobe",
    "tcl::process",
    "oo::abstract",
    "oo::configurable",
    "oo::singleton",
    // New in Tcl 9.0: the floating-point classifier, whose own manual page
    // (`doc/fpclassify.n`) exists only in the 9.x trees.
    "fpclassify",
    // Removed in Tcl 9.0: `case`, obsolete since 7.0 and dropped along with
    // its manual page. The one probe running the gate in the *other*
    // direction — present in 8.6, absent in 9.0.
    "case",
    // Stable across both (present in 8.6 and 9.0).
    "foreach",
    "lassign",
    "lmap",
    "dict",
    "apply",
    "oo::class",
    "oo::define",
    "try",
    "throw",
];

/// Enumerate each ensemble's canonical subcommand set by triggering its
/// "unknown subcommand … must be …" error and parsing the alternatives list.
/// Returns `ensemble → set of subcommand names` for `tclsh`.
fn enumerate_ensemble_subcommands(tclsh: &str, ensembles: &[&str]) -> HashMap<String, Vec<String>> {
    let list = ensembles.join(" ");
    // `regsub` normalises the ", " / ", or " separators to spaces so `lsort`
    // yields a clean word list, which we print as `ens sub1 sub2 …`.
    let script = format!(
        "catch {{package require TclOO}}\n\
         foreach ens {{{list}}} {{\n\
         \x20 if {{[catch {{$ens __nope__}} e] && [regexp {{must be (.*)$}} $e -> tail]}} {{\n\
         \x20   regsub -all {{,? or }} $tail {{ }} tail\n\
         \x20   regsub -all {{,}} $tail {{ }} tail\n\
         \x20   puts \"$ens [lsort $tail]\"\n\
         \x20 }}\n\
         }}\n"
    );
    let out = run_tcl(tclsh, &script).unwrap_or_default();
    out.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let ens = it.next()?.to_string();
            Some((ens, it.map(str::to_string).collect()))
        })
        .collect()
}

/// Return whether an ensemble accepts a concrete subcommand spelling.
///
/// Tcl can retain deprecated compatibility spellings without advertising
/// them in the ensemble's "must be ..." error.  In particular, Tcl 9.0.4
/// accepts `interp slaves` but lists only the preferred `children` spelling.
/// A wrong-arity (or other non-dispatch) error therefore proves that the
/// subcommand resolved; only the ensemble's own unknown-option shapes mean it
/// is absent.
fn ensemble_accepts_subcommand(tclsh: &str, ensemble: &str, subcommand: &str) -> bool {
    let script = format!(
        "if {{[catch {{{ensemble} {subcommand}}} e]}} {{\n\
         \x20 set unknown [expr {{[string match {{unknown *subcommand*}} $e] ||\n\
         \x20                     [string match {{bad option *: must be *}} $e]}}]\n\
         \x20 puts -nonewline [expr {{!$unknown}}]\n\
         }} else {{\n\
         \x20 puts -nonewline 1\n\
         }}\n"
    );
    run_tcl(tclsh, &script).is_some_and(|out| out.trim() == "1")
}

/// Ensembles whose subcommand tables span the 8.6/9.0 boundary. Their C-level
/// subcommand sets are the ground truth; the registry's per-subcommand dialect
/// gating must reproduce exactly which are present in each version.
const AUDITED_ENSEMBLES: &[&str] = &[
    "string",
    "dict",
    "info",
    "file",
    "array",
    "chan",
    "clock",
    "namespace",
    "interp",
    "package",
    "binary",
    "encoding",
];

#[test]
fn registry_subcommand_dialect_gating_matches_tclsh_8_6_and_9_0() {
    let (Some(t86), Some(t90)) = (find_tclsh("8.6", "tclsh8.6"), find_tclsh("9.0", "tclsh9.0"))
    else {
        eprintln!("skipping subcommand dialect oracle: need both tclsh8.6 and tclsh9.0 on PATH");
        return;
    };
    let set86 = enumerate_ensemble_subcommands(&t86, AUDITED_ENSEMBLES);
    let set90 = enumerate_ensemble_subcommands(&t90, AUDITED_ENSEMBLES);
    let reg = CommandRegistry::build_default();

    // Effective subcommand availability: own `dialects` if set, else inherited
    // from the parent command.
    let sub_available = |cmd: &str, sub: &str, dialect: Option<SurfaceQuery<'_>>| -> Option<bool> {
        let parent = reg.get(cmd)?;
        let s = parent.subcommand(sub)?;
        Some(match s.surface {
            Some(ds) => surface_admits(ds, dialect.as_ref()),
            None => parent.supports_dialect(dialect),
        })
    };

    let mut mismatches: Vec<String> = Vec::new();
    let mut audited = 0usize;
    for &ens in AUDITED_ENSEMBLES {
        let (Some(s86), Some(s90)) = (set86.get(ens), set90.get(ens)) else {
            eprintln!("note: could not enumerate `{ens}` subcommands from tclsh (skipped)");
            continue;
        };
        // Union of the canonical names seen in either interpreter. Registry-only
        // names (prefix aliases like `dict getd`) are intentionally not in this
        // set, so they are not treated as spurious mismatches.
        let mut names: Vec<&String> = s86.iter().chain(s90.iter()).collect();
        names.sort_unstable();
        names.dedup();
        for sub in names {
            // Only audit subcommands the registry models; a name it doesn't know
            // is a completeness gap (out of scope for this gating differential).
            let (Some(got86), Some(got90)) = (
                sub_available(ens, sub, Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
                sub_available(ens, sub, Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
            ) else {
                continue;
            };
            audited += 1;
            // The advertised table is canonical, but C Tcl may also accept a
            // hidden compatibility spelling. Probe only when the spelling is
            // absent from the table so the common path stays cheap.
            let want86 = s86.contains(sub) || ensemble_accepts_subcommand(&t86, ens, sub);
            let want90 = s90.contains(sub) || ensemble_accepts_subcommand(&t90, ens, sub);
            if got86 != want86 {
                mismatches.push(format!(
                    "`{ens} {sub}`: registry available-in-8.6={got86}, tclsh8.6 says {want86}"
                ));
            }
            if got90 != want90 {
                mismatches.push(format!(
                    "`{ens} {sub}`: registry available-in-9.0={got90}, tclsh9.0 says {want90}"
                ));
            }
        }
    }
    assert!(
        audited > 100,
        "audit coverage suspiciously low ({audited} subcommands) — enumeration likely failed"
    );
    assert!(
        mismatches.is_empty(),
        "registry subcommand dialect gating diverges from tclsh ({audited} audited):\n{}",
        mismatches.join("\n")
    );
    eprintln!("subcommand dialect audit: {audited} subcommands checked, all match");
}

/// Enumerate a command's option/switch set by triggering its "bad option …
/// must be …" listing with a bogus flag. `triggers` maps command → a full
/// invocation whose first argument is an unknown flag. Returns command → set
/// of option names (each with its leading `-`).
fn enumerate_options(tclsh: &str, triggers: &[(&str, &str)]) -> HashMap<String, Vec<String>> {
    let body: String = triggers
        .iter()
        .map(|(cmd, trig)| format!("{cmd} {{{trig}}}"))
        .collect::<Vec<_>>()
        .join(" ");
    // For each (cmd, trigger) run the trigger, capture the error, and parse the
    // "must be …" alternatives (comma / "or" separated) into a word list.
    let script = format!(
        "foreach {{cmd trig}} {{{body}}} {{\n\
         \x20 catch {{uplevel #0 $trig}} e\n\
         \x20 if {{[regexp {{(?:bad|unknown) (?:option|switch)[^:]*: must be (.*?)$}} $e -> tail]}} {{\n\
         \x20   regsub -all {{,? or }} $tail {{ }} tail\n\
         \x20   regsub -all {{,}} $tail {{ }} tail\n\
         \x20   puts \"$cmd [lsort -unique $tail]\"\n\
         \x20 }}\n\
         }}\n"
    );
    let out = run_tcl(tclsh, &script).unwrap_or_default();
    out.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let cmd = it.next()?.to_string();
            // Keep only real `-flag` tokens (drop the `--` end-of-options marker
            // and any stray non-flag word).
            Some((
                cmd,
                it.filter(|w| w.starts_with('-') && *w != "--")
                    .map(str::to_string)
                    .collect(),
            ))
        })
        .collect()
}

/// Commands with version-sensitive option sets, each paired with an invocation
/// whose first argument is a bogus flag so the interpreter lists its real
/// options. Spans the 8.6/9.0 boundary (`lsearch -stride` and `regsub
/// -command` are 9.0-only; `lsort -stride` is in both).
const OPT_TRIGGERS: &[(&str, &str)] = &[
    ("lsearch", "lsearch -__nope__ x y"),
    ("regsub", "regsub -__nope__ a b c"),
    ("regexp", "regexp -__nope__ a b"),
    ("lsort", "lsort -__nope__ x"),
];

#[test]
fn registry_option_dialect_gating_matches_tclsh_8_6_and_9_0() {
    let (Some(t86), Some(t90)) = (find_tclsh("8.6", "tclsh8.6"), find_tclsh("9.0", "tclsh9.0"))
    else {
        eprintln!("skipping option dialect oracle: need both tclsh8.6 and tclsh9.0 on PATH");
        return;
    };
    let opt86 = enumerate_options(&t86, OPT_TRIGGERS);
    let opt90 = enumerate_options(&t90, OPT_TRIGGERS);
    let reg = CommandRegistry::build_default();

    let mut mismatches: Vec<String> = Vec::new();
    let mut audited = 0usize;
    for &(cmd, _) in OPT_TRIGGERS {
        let (Some(o86), Some(o90)) = (opt86.get(cmd), opt90.get(cmd)) else {
            eprintln!("note: could not enumerate `{cmd}` options from tclsh (skipped)");
            continue;
        };
        let Some(spec) = reg.get(cmd) else { continue };
        // The registry's declared options (no dialect filter). Only these are
        // audited; an option tclsh has that the registry does not declare is a
        // completeness gap, out of scope for a gating differential.
        let declared: Vec<&'static str> = spec.switch_names(None);
        let in86 = spec.switch_names(Some(SurfaceQuery::core(Family::Tcl, "8.6")));
        let in90 = spec.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")));
        for opt in &declared {
            // `--` is the end-of-options marker: version-invariant and listed
            // inconsistently by tclsh across commands, so it is not a gating
            // fact worth auditing.
            if *opt == "--" {
                continue;
            }
            audited += 1;
            let got86 = in86.contains(opt);
            let got90 = in90.contains(opt);
            let want86 = o86.iter().any(|o| o == opt);
            let want90 = o90.iter().any(|o| o == opt);
            if got86 != want86 {
                mismatches.push(format!(
                    "`{cmd} {opt}`: registry available-in-8.6={got86}, tclsh8.6 says {want86}"
                ));
            }
            if got90 != want90 {
                mismatches.push(format!(
                    "`{cmd} {opt}`: registry available-in-9.0={got90}, tclsh9.0 says {want90}"
                ));
            }
        }
    }
    assert!(
        audited > 20,
        "option audit coverage suspiciously low ({audited}) — enumeration likely failed"
    );
    assert!(
        mismatches.is_empty(),
        "registry option dialect gating diverges from tclsh ({audited} audited):\n{}",
        mismatches.join("\n")
    );
    eprintln!("option dialect audit: {audited} options checked, all match");
}

#[test]
fn registry_dialect_gating_matches_tclsh_8_6_and_9_0() {
    let (Some(t86), Some(t90)) = (find_tclsh("8.6", "tclsh8.6"), find_tclsh("9.0", "tclsh9.0"))
    else {
        eprintln!("skipping dialect oracle: need both tclsh8.6 and tclsh9.0 on PATH");
        return;
    };

    let have86 = probe_existence(&t86, PROBES);
    let have90 = probe_existence(&t90, PROBES);
    let reg = CommandRegistry::build_default();

    let mut mismatches: Vec<String> = Vec::new();
    for &name in PROBES {
        // Only audit commands the registry actually models; a command it does
        // not know at all is a *completeness* gap, not a *gating* bug, and is
        // out of scope for this differential.
        if reg.get(name).is_none() {
            eprintln!("note: registry does not model `{name}` (completeness gap, skipped)");
            continue;
        }
        let want86 = have86.get(name).copied().unwrap_or(false);
        let want90 = have90.get(name).copied().unwrap_or(false);
        let got86 = reg
            .get_for_surface(name, Some(SurfaceQuery::core(Family::Tcl, "8.6")))
            .is_some();
        let got90 = reg
            .get_for_surface(name, Some(SurfaceQuery::core(Family::Tcl, "9.0")))
            .is_some();
        if got86 != want86 {
            mismatches.push(format!(
                "`{name}`: registry says available-in-8.6={got86}, tclsh8.6 says {want86}"
            ));
        }
        if got90 != want90 {
            mismatches.push(format!(
                "`{name}`: registry says available-in-9.0={got90}, tclsh9.0 says {want90}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "registry dialect gating diverges from tclsh:\n{}",
        mismatches.join("\n")
    );
}
