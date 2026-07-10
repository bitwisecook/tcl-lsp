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

//! WASM command-parity gate (`RUST_ISSUE_006`).
//!
//! `AGENTS.md` § *WASM command parity* asserts a contract: **every core-Tcl
//! command in `tcl-registry` (the source of truth) needs runtime backing** — a
//! real Rust handler in `runtime/rust/`, an interpreter (init.tcl) fallback, or
//! an explicit *not required* classification. Nothing enforced it (the runtime
//! doesn't depend on `tcl-registry`, and the old Python coverage script is
//! retired), so a command could be added to the registry — or dropped from the
//! runtime — with the two silently diverging.
//!
//! This check re-establishes the contract as a drift gate wired into
//! `make xtask-check`:
//!
//! - the **source of truth** is [`tcl_registry`]'s core Tcl command specs
//!   (`required_package == None`, available in Tcl 9.0);
//! - the **backed** set is scanned from the runtime source
//!   (`register_builtin(b"…")`), canonicalised on a leading `::`, plus the
//!   [`HANDLER_EXTRA`] list for commands backed natively outside that literal
//!   scan (the `TclOO` metaclasses, the per-object `my`);
//! - the residue is accounted for by the committed classification below: the
//!   [`is_expr_operator`] predicate for the `expr` operator family, and the
//!   [`STDLIB`] / [`NOT_REQUIRED`] / [`KNOWN_UNBACKED`] lists.
//!
//! A generated report (`docs/generated/wasm-command-backing.md`) records the
//! per-command status; `--check` fails on report drift **and** on any core
//! command that is neither backed nor classified (a genuinely new gap) or any
//! stale classification entry (a name no longer in the registry, or now backed).

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};

use crate::util::repo_root;

/// The generated per-command backing report (committed, drift-checked).
const REPORT_PATH: &str = "docs/generated/wasm-command-backing.md";

/// Core commands the runtime backs natively but **not** through a
/// `register_builtin(b"…")` literal the [`scan_handlers`] pass can see, with a
/// note on the mechanism: `TclOO` metaclasses bootstrapped as objects
/// (`oo::class create ::oo::…` in `cmd_oo.rs`) and the per-object `my` command
/// created by method dispatch (`oo_register_my`). Names are canonical (no
/// leading `::`). Kept sorted.
const HANDLER_EXTRA: &[(&str, &str)] = &[
    (
        "my",
        "per-object command created by TclOO method dispatch (oo_register_my)",
    ),
    (
        "oo::abstract",
        "TclOO metaclass bootstrapped as an object in cmd_oo.rs",
    ),
    (
        "oo::class",
        "TclOO metaclass bootstrapped as an object in cmd_oo.rs",
    ),
    (
        "oo::configurable",
        "TclOO metaclass bootstrapped as an object in cmd_oo.rs",
    ),
    (
        "oo::object",
        "TclOO root class bootstrapped as an object in cmd_oo.rs",
    ),
    (
        "oo::singleton",
        "TclOO metaclass bootstrapped as an object in cmd_oo.rs",
    ),
];

/// Core commands backed by the **interpreter fallback** the runtime sources at
/// startup (the embedded real-C-Tcl-9 `init.tcl` / `package.tcl` / `parray.tcl`,
/// seeded into the WASM `MemFs`) rather than a Rust `register_builtin` handler —
/// the contract's "interpreter-fallback path". The note is the defining library
/// file. Names are canonical (no leading `::`). Kept sorted.
const STDLIB: &[(&str, &str)] = &[
    ("auto_execok", "init.tcl"),
    ("auto_import", "init.tcl"),
    ("auto_load", "init.tcl"),
    ("auto_load_index", "init.tcl"),
    ("auto_qualify", "init.tcl"),
    ("parray", "parray.tcl"),
    ("pkg::create", "package.tcl (alias to ::tcl::Pkg::Create)"),
    ("tclLog", "init.tcl"),
    ("tclPkgSetup", "package.tcl"),
    ("tclPkgUnknown", "package.tcl"),
    ("unknown", "init.tcl"),
];

/// Core commands the runtime deliberately does **not** back, with the reason —
/// the contract's explicit "not required" classification. The `expr` operators
/// (`+`, `tcl::mathop::*`, `eq`, …) are handled separately by
/// [`is_expr_operator`] rather than enumerated here. Names are canonical (no
/// leading `::`). Kept sorted.
const NOT_REQUIRED: &[(&str, &str)] = &[
    (
        "auto_mkindex",
        "dev-time auto-load index generation (auto.tcl); needs a host filesystem, auto.tcl is not embedded",
    ),
    (
        "auto_mkindex_old",
        "legacy auto-load index generation (auto.tcl); needs a host filesystem, auto.tcl is not embedded",
    ),
    (
        "auto_reset",
        "auto-load cache reset (auto.tcl); auto.tcl is not embedded",
    ),
    (
        "bgerror",
        "background-error hook; no default definition even in bare tclsh 9 (user/Tk-provided) and there is no WASM event loop",
    ),
    (
        "disabled_in_irules",
        "F5 iRules dialect artifact, not a real Tcl 9 command",
    ),
    (
        "exec",
        "external OS process; loop-registered as an explicit \"not supported under the WASM runtime\" stub (cmd_misc.rs)",
    ),
    (
        "fcopy",
        "event-loop channel copy; loop-registered as an explicit unsupported stub (cmd_misc.rs)",
    ),
    (
        "fileevent",
        "event loop; loop-registered as an explicit unsupported stub (cmd_misc.rs)",
    ),
    (
        "filename",
        "registry/documentation placeholder, not a real Tcl command",
    ),
    (
        "foreachLine",
        "non-core EDA/dialect file helper, absent from bare tclsh 9",
    ),
    (
        "http",
        "the `http` package (`package require http`), not a core command",
    ),
    (
        "load",
        "native library loading; loop-registered as an explicit unsupported stub (cmd_misc.rs)",
    ),
    ("memory", "TCL_MEM_DEBUG-only heap-debug command"),
    (
        "pkg_mkindex",
        "dev-time package-index generation; needs a host filesystem",
    ),
    (
        "re_quote",
        "non-core dialect helper, absent from bare tclsh 9",
    ),
    (
        "readFile",
        "non-core EDA/dialect file helper, absent from bare tclsh 9",
    ),
    (
        "regex::quote",
        "non-core dialect helper, absent from bare tclsh 9",
    ),
    (
        "regex_quote",
        "non-core dialect helper, absent from bare tclsh 9",
    ),
    (
        "regexp::quote",
        "non-core dialect helper, absent from bare tclsh 9",
    ),
    ("registry", "Windows registry package, platform-specific"),
    (
        "socket",
        "TCP sockets; loop-registered as an explicit unsupported stub (cmd_misc.rs)",
    ),
    (
        "tcl::process",
        "OS subprocess management; needs host processes",
    ),
    (
        "tcl_findLibrary",
        "library-bootstrap helper; not defined in the embedded Tcl 9 init.tcl and absent from bare tclsh 9",
    ),
    ("unload", "native library unloading (counterpart to load)"),
    (
        "writeFile",
        "non-core EDA/dialect file helper, absent from bare tclsh 9",
    ),
];

/// Core commands that *should* be backed but are not yet — real gaps tracked by
/// `RUST_ISSUE_007`. Allow-listed so the gate stays green while they are
/// implemented one by one; removing a name here (as it gains a handler) is the
/// visible progress marker. Names are canonical (no leading `::`). Kept sorted.
const KNOWN_UNBACKED: &[(&str, &str)] = &[
    ("chan", "core I/O ensemble"),
    (
        "classvariable",
        "TclOO method-body helper for class-shared variables",
    ),
    (
        "coroinject",
        "coroutine introspection (the runtime backs `coroutine`/`yield` but not this)",
    ),
    (
        "coroprobe",
        "coroutine introspection (the runtime backs `coroutine`/`yield` but not this)",
    ),
    ("timerate", "benchmarking command"),
    ("zlib", "core compression / checksums"),
];

/// Strip a single leading `::` so a registry name (`tcl::build-info`,
/// `tcl::process`) and the scanned handler / classification names
/// (`::tcl::build-info`, `::tcl::process`) canonicalise to the same key. The
/// registry carries both fully-qualified and bare spellings of some commands;
/// this collapses them for cross-referencing.
fn canon(name: &str) -> &str {
    name.strip_prefix("::").unwrap_or(name)
}

/// The `expr` operators, exposed in the registry as commands (`+`,
/// `tcl::mathop::+`, `eq`, `min`, …) but evaluated inside `expr`'s bytecode, not
/// dispatched as standalone runtime commands (bare `+` is not even a command in
/// tclsh). A stable, closed family — matched by predicate rather than spelling
/// out ~90 entries. `c` must already be [`canon`]ical.
fn is_expr_operator(c: &str) -> bool {
    const BARE: &[&str] = &[
        "!", "!=", "%", "&", "&&", "*", "**", "+", "-", "/", "<", "<<", "<=", "==", ">", ">=",
        ">>", "@", "^", "eq", "in", "max", "min", "ne", "ni", "|", "||", "~",
    ];
    c == "tcl::mathop" || c.starts_with("tcl::mathop::") || BARE.contains(&c)
}

/// The reason attached to every [`is_expr_operator`] command in the report.
const EXPR_OPERATOR_REASON: &str =
    "`expr` operator/function; evaluated inside `expr`, not a standalone runtime command";

/// One core command's backing status.
enum Status {
    /// A real `register_builtin` handler in the runtime.
    Handler,
    /// Backed natively but outside the `register_builtin` scan (`TclOO`
    /// metaclass, per-object `my`), with a note on the mechanism.
    HandlerNative(&'static str),
    /// Interpreter fallback (embedded init.tcl / package.tcl / parray.tcl).
    Stdlib(&'static str),
    /// Explicitly not required, with the reason.
    NotRequired(&'static str),
    /// A known, tracked gap (`RUST_ISSUE_007`).
    KnownGap(&'static str),
    /// Unaccounted for — a new gap the gate flags.
    Unclassified,
}

/// The core Tcl commands the runtime must back: `tcl-registry`'s built-in
/// (`required_package == None`) command specs that are available in **Tcl 9.0**
/// (the runtime's target). The dialect filter drops the tcl-family commands
/// modelled only for the embedded/EDA dialects (F5 iRules/iApps, Expect, the EDA
/// tools) — `readFile`, `foreachLine`, `disabled_in_irules`, … — which the core
/// runtime is not expected to provide. `None` dialects means "all dialects".
/// Namespaced ensemble *implementation* commands (`::tcl::…`) stay in the set so
/// their classification is explicit.
fn core_commands() -> BTreeSet<String> {
    use tcl_registry::dialects::DialectSet;
    tcl_registry::commands::tcl::tcl_command_specs()
        .iter()
        .filter(|s| s.required_package.is_none())
        .filter(|s| s.dialects.is_none_or(|d| d.contains(DialectSet::TCL90)))
        .map(|s| s.name.to_string())
        .collect()
}

/// Scan the runtime source for `register_builtin(b"NAME", …)` registrations.
fn scan_handlers(root: &std::path::Path) -> Result<BTreeSet<String>> {
    let src = root.join("runtime/rust/src");
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(&src).with_context(|| format!("reading {}", src.display()))? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        // `register_builtin(b"name", handler)` — capture the byte-string literal.
        let mut rest = text.as_str();
        while let Some(i) = rest.find("register_builtin(b\"") {
            rest = &rest[i + "register_builtin(b\"".len()..];
            if let Some(end) = rest.find('"') {
                // Canonicalise: `::tcl::build-info` (registered) → `tcl::build-info`
                // (the registry spelling).
                names.insert(canon(&rest[..end]).to_string());
                rest = &rest[end..];
            }
        }
    }
    Ok(names)
}

fn classify(name: &str, backed: &BTreeSet<String>) -> Status {
    let c = canon(name);
    if backed.contains(c) {
        return Status::Handler;
    }
    if let Some((_, why)) = HANDLER_EXTRA.iter().find(|(n, _)| *n == c) {
        return Status::HandlerNative(why);
    }
    if is_expr_operator(c) {
        return Status::NotRequired(EXPR_OPERATOR_REASON);
    }
    if let Some((_, why)) = STDLIB.iter().find(|(n, _)| *n == c) {
        return Status::Stdlib(why);
    }
    if let Some((_, why)) = NOT_REQUIRED.iter().find(|(n, _)| *n == c) {
        return Status::NotRequired(why);
    }
    if let Some((_, why)) = KNOWN_UNBACKED.iter().find(|(n, _)| *n == c) {
        return Status::KnownGap(why);
    }
    Status::Unclassified
}

/// Render the committed backing report from the current registry + runtime.
fn render_report(core: &BTreeSet<String>, backed: &BTreeSet<String>) -> String {
    let mut counts = [0usize; 6];
    let mut rows = String::new();
    for name in core {
        let (label, why, idx) = match classify(name, backed) {
            Status::Handler => ("handler", String::new(), 0),
            Status::HandlerNative(w) => ("handler (native)", w.to_string(), 1),
            Status::Stdlib(w) => ("stdlib", w.to_string(), 2),
            Status::NotRequired(w) => ("not-required", w.to_string(), 3),
            Status::KnownGap(w) => ("known-gap", w.to_string(), 4),
            Status::Unclassified => ("UNCLASSIFIED", String::new(), 5),
        };
        counts[idx] += 1;
        let _ = writeln!(rows, "| `{name}` | {label} | {why} |");
    }
    let mut out = String::new();
    let _ = write!(
        out,
        "# WASM command backing coverage\n\n\
         > Auto-generated by `cargo xtask command-backing`. Do not edit by hand.\n\
         > The gate (`make xtask-check`) fails on drift, on any `UNCLASSIFIED`\n\
         > command, or on a stale classification entry (`RUST_ISSUE_006`).\n\n\
         Source of truth: `tcl-registry` core command specs (`required_package == None`),\n\
         restricted to those available in Tcl 9.0. Backing: a `register_builtin`\n\
         handler in `runtime/rust/`, a native non-`register_builtin` registration\n\
         (TclOO metaclass, per-object `my`), an embedded-init.tcl (stdlib) fallback,\n\
         or an explicit *not required* classification.\n\n\
         | status | count |\n| --- | --- |\n\
         | handler | {} |\n| handler (native) | {} |\n| stdlib | {} |\n\
         | not-required | {} |\n| known-gap (`RUST_ISSUE_007`) | {} |\n\
         | **UNCLASSIFIED** | {} |\n| **total** | {} |\n\n\
         | command | backing | note |\n| --- | --- | --- |\n{rows}",
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        counts[4],
        counts[5],
        core.len(),
    );
    out
}

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let core = core_commands();
    let backed = scan_handlers(&root)?;

    // Gaps: a core command that is neither backed nor classified.
    let gaps: Vec<&String> = core
        .iter()
        .filter(|n| matches!(classify(n, &backed), Status::Unclassified))
        .collect();
    // Stale classification: a listed (canonical) name that no core command maps
    // to, or one that now has a `register_builtin` handler (so the classification
    // is redundant). `HANDLER_EXTRA` is checked for the "no longer core" case
    // only — it is *expected* to name backed commands.
    let core_canon: BTreeSet<&str> = core.iter().map(|n| canon(n)).collect();
    let mut stale: Vec<String> = Vec::new();
    for (n, _) in HANDLER_EXTRA {
        if !core_canon.contains(n) {
            stale.push(format!(
                "`{n}` — classified but not a core registry command"
            ));
        }
    }
    for (n, _) in STDLIB.iter().chain(NOT_REQUIRED).chain(KNOWN_UNBACKED) {
        if !core_canon.contains(n) {
            stale.push(format!(
                "`{n}` — classified but not a core registry command"
            ));
        } else if backed.contains(*n) {
            stale.push(format!("`{n}` — classified but now has a runtime handler"));
        }
    }

    let report = render_report(&core, &backed);
    let report_path = root.join(REPORT_PATH);

    if !check {
        fs::write(&report_path, &report)
            .with_context(|| format!("writing {}", report_path.display()))?;
        eprintln!("wrote {REPORT_PATH} ({} core commands)", core.len());
        eprintln!(
            "  handler + stdlib + not-required + known-gap + UNCLASSIFIED={}",
            gaps.len()
        );
        return Ok(ExitCode::SUCCESS);
    }

    let mut failed = false;
    let current = fs::read_to_string(&report_path).unwrap_or_default();
    if current != report {
        eprintln!("{REPORT_PATH} is stale — run `cargo xtask command-backing`");
        failed = true;
    }
    if !gaps.is_empty() {
        eprintln!(
            "{} core registry command(s) have no runtime backing and no classification\n\
             (add a `register_builtin` handler, or classify in `command_backing.rs`):",
            gaps.len()
        );
        for n in &gaps {
            eprintln!("  - {n}");
        }
        failed = true;
    }
    if !stale.is_empty() {
        eprintln!("stale command-backing classification entries:");
        for s in &stale {
            eprintln!("  - {s}");
        }
        failed = true;
    }
    if failed {
        return Ok(ExitCode::from(1));
    }
    eprintln!(
        "OK: all {} core registry commands are backed or classified.",
        core.len()
    );
    Ok(ExitCode::SUCCESS)
}
