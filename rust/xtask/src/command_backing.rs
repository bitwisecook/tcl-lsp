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
//!   (`required_package == None`, available at Tcl 9.0 or later);
//! - the **backed** set is scanned from the runtime source: literal
//!   `register_builtin(b"…")` calls and `register_spec_builtin` calls whose
//!   [`tcl_registry::CommandSpec`] comes from a literal registry lookup,
//!   canonicalised on a leading `::`, plus the [`HANDLER_EXTRA`] list for
//!   commands backed natively outside those scans (the `TclOO` metaclasses,
//!   the per-object `my`);
//! - the residue is accounted for by the committed classification below: the
//!   [`is_expr_operator`] predicate for the `expr` operator family, the
//!   [`is_mathop_command`] predicate for the qualified `tcl::mathop::*`
//!   commands, and the [`STDLIB`] / [`NOT_REQUIRED`] / [`KNOWN_UNBACKED`]
//!   lists.
//!
//! A generated report (`docs/generated/wasm-command-backing.md`) records the
//! per-command status; `--check` fails on report drift **and** on any core
//! command that is neither backed nor classified (a genuinely new gap) or any
//! stale classification entry (a name no longer in the registry, or now backed).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use regex::Regex;

use crate::util::repo_root;

/// The generated per-command backing report (committed, drift-checked).
const REPORT_PATH: &str = "docs/generated/wasm-command-backing.md";

/// Core commands the runtime backs natively but **not** through a
/// `register_builtin(b"…")` literal the [`scan_handlers`] pass can see, with a
/// note on the mechanism: `TclOO` metaclasses bootstrapped as objects
/// (`oo::class create ::oo::…` in `cmd_oo.rs`) and the per-object `my` command
/// created by method dispatch (`oo_register_my`). Names are canonical (no
/// leading `::`). Kept sorted.
///
/// The standalone `::tcl::dict::*` spellings (issue #923 idx 105) are **not**
/// here: `runtime/rust` backs only the `dict` ensemble head, so a direct
/// `::tcl::dict::get …` call is `invalid command name` there — a genuine gap
/// classified by [`is_tcl_dict_qualified`] as [`Status::KnownGap`], not hidden
/// as if `dict`'s handler backed it (Codex review, PR #1020).
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
/// the contract's explicit "not required" classification. The bare `expr`
/// operators (`+`, `eq`, …) are handled separately by [`is_expr_operator`],
/// and the qualified `tcl::mathop::*` command spellings by
/// [`is_mathop_command`], rather than enumerated here. Names are canonical
/// (no leading `::`). Kept sorted.
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

/// Core commands that *should* be backed but are not yet — real gaps, each
/// with its own reason below. The original sweep that opened this allow-list
/// closed with it empty, so every entry here is a newer, independent
/// addition rather than a reopening of that work. Allow-listed so
/// the gate stays green while they are implemented one by one; removing a
/// name here (as it gains a handler) is the visible progress marker. Names
/// are canonical (no leading `::`). Kept sorted.
///
/// The Tcl 9.1 entries below (everything but `link`/`tcl::zipfs`/`zipfs`)
/// only became visible to this gate once `core_commands()`'s
/// `TCL90`→`TCL90_PLUS` fix stopped silently excluding every 9.1-only-gated
/// command (adversarial review of PR #1008) — they were always genuinely
/// unbacked, just invisible to the check before that fix.
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
        "remquo",
        "TIP 745 (Tcl 9.1) IEEE remainder with low quotient bits; not yet implemented in runtime/rust",
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

/// Strip a single leading `::` so a registry name (`tcl::build-info`,
/// `tcl::process`) and the scanned handler / classification names
/// (`::tcl::build-info`, `::tcl::process`) canonicalise to the same key. The
/// registry carries both fully-qualified and bare spellings of some commands;
/// this collapses them for cross-referencing.
fn canon(name: &str) -> &str {
    name.strip_prefix("::").unwrap_or(name)
}

/// The `expr` operators, exposed in the registry as **bare** commands (`+`,
/// `eq`, …) but evaluated inside `expr`'s bytecode, not dispatched as
/// standalone runtime commands (bare `+` is not even a command in tclsh).
/// `c` must already be [`canon`]ical.
///
/// This intentionally does **not** match the qualified `tcl::mathop::*`
/// spellings any more — those are real, separately-callable runtime commands
/// (see [`is_mathop_command`]), so classifying them here as "not required"
/// would hide a broken/missing runtime install from this gate (issue #983's
/// #987 residual: the WASM-parity check couldn't have caught a broken
/// `tcl::mathop` install because it never looked for one).
///
/// The bare spellings are derived from `tcl_syntax::expr::operators` (issue
/// #983's unification) rather than a hand-typed list — that list used to
/// carry a bare `max`/`min` entry left over from the same historical
/// `mathop_generated.rs` bug documented in that file's own header
/// (`max`/`min` were never real `::tcl::mathop` members; they're
/// `expr` math *functions*, registered only under the qualified
/// `tcl::mathfunc::` spellings, never bare — so this predicate never
/// actually matched a real registry command for them; dead weight, not a
/// live bug, since the WASM-parity scan only ever calls this on names the
/// registry actually carries).
fn is_expr_operator(c: &str) -> bool {
    c == "tcl::mathop"
        || tcl_syntax::expr::operators::ALL_BIN_OPS
            .iter()
            .any(|op| op.spec().spelling == c && op.spec().mathop_shape.is_some())
        || tcl_syntax::expr::operators::ALL_UNARY_OPS
            .iter()
            .any(|op| op.spec().spelling == c && op.spec().mathop_shape.is_some())
}

/// The reason attached to every [`is_expr_operator`] command in the report.
const EXPR_OPERATOR_REASON: &str =
    "`expr` operator/function; evaluated inside `expr`, not a standalone runtime command";

/// Whether `c` is a `tcl::mathop::<op>` (or `::`-qualified) command for a
/// real `expr` operator with a mathop command form — derived from
/// [`tcl_syntax::expr::operators`] (issue #983/#987's unification). `c` must
/// already be [`canon`]ical.
///
/// Unlike [`is_expr_operator`]'s bare operator spellings (grammar-only, never
/// a real command), `::tcl::mathop::*` commands genuinely **are** backed —
/// `runtime/rust/src/cmd_mathop.rs`'s `install()` registers a real handler
/// for every operator with a mathop shape — just via the same dynamic-
/// name-construction pattern `cmd_mathfunc.rs`'s `install()` uses
/// (`register_builtin(&full, …)` builds `full` from a runtime string, not a
/// `register_builtin(b"…")` literal [`scan_handlers`] can see). So this is
/// [`Status::HandlerNative`], not [`Status::NotRequired`] — real backing the
/// literal scan just can't detect. Before this predicate existed, qualified
/// `tcl::mathop::*` names fell through to [`is_expr_operator`]'s bare-prefix
/// check and were wrongly classified `NotRequired`, so a broken or missing
/// runtime `tcl::mathop` install could never fail this gate.
fn is_mathop_command(c: &str) -> bool {
    let Some(name) = c
        .strip_prefix("tcl::mathop::")
        .or_else(|| c.strip_prefix("::tcl::mathop::"))
    else {
        return false;
    };
    tcl_syntax::expr::operators::ALL_BIN_OPS
        .iter()
        .any(|op| op.spec().spelling == name && op.spec().mathop_shape.is_some())
        || tcl_syntax::expr::operators::ALL_UNARY_OPS
            .iter()
            .any(|op| op.spec().spelling == name && op.spec().mathop_shape.is_some())
}

/// The reason attached to every [`is_mathop_command`] command in the report.
const MATHOP_COMMAND_REASON: &str = "`::tcl::mathop::*` command, registered by cmd_mathop.rs::install()'s dynamic-name loop \
     (register_builtin(&full, …) — not a literal the scan can see)";

/// Whether `c` is a `tcl::mathfunc::<name>` (or `::`-qualified) command for
/// a real `expr` math function — derived from
/// [`tcl_syntax::expr::mathfunc::added_in`] (issue #983's unification).
/// `c` must already be [`canon`]ical.
///
/// Unlike [`is_expr_operator`]'s bare mathop spellings, `::tcl::mathfunc::*`
/// commands genuinely **are** backed — `runtime/rust/src/cmd_mathfunc.rs`'s
/// `install()` registers a real handler for every function — just via the
/// same dynamic-name-construction pattern `cmd_mathop.rs`'s `install()`
/// uses (`register_builtin(&full, …)` builds `full` from a runtime string,
/// not a `register_builtin(b"…")` literal [`scan_handlers`] can see). So
/// this is [`Status::HandlerNative`], not [`Status::NotRequired`] — real
/// backing the literal scan just can't detect.
fn is_mathfunc_command(c: &str) -> bool {
    let Some(name) = c
        .strip_prefix("tcl::mathfunc::")
        .or_else(|| c.strip_prefix("::tcl::mathfunc::"))
    else {
        return c == "tcl::mathfunc";
    };
    tcl_syntax::expr::mathfunc::added_in(name).is_some()
}

/// The reason attached to every [`is_mathfunc_command`] command in the
/// report.
const MATHFUNC_COMMAND_REASON: &str = "`::tcl::mathfunc::*` command, registered by cmd_mathfunc.rs::install()'s dynamic-name loop \
     (register_builtin(&full, …) — not a literal the scan can see)";

/// Whether `c` is a standalone `::tcl::dict::*` ensemble-implementation
/// spelling (issue #923 idx 105). These are real, separately-callable commands
/// in C Tcl (the `dict` ensemble's default map targets), so the registry
/// carries them — but `runtime/rust` implements only the `dict` ensemble head
/// (`register_builtin(b"dict", …)`), not the qualified spellings: a direct
/// `::tcl::dict::get …` call raises `invalid command name` there. Classified as
/// a genuine, visible runtime gap ([`Status::KnownGap`]) rather than hidden
/// under [`HANDLER_EXTRA`] as if `dict`'s handler backed them (Codex review,
/// PR #1020).
fn is_tcl_dict_qualified(c: &str) -> bool {
    c.strip_prefix("::")
        .unwrap_or(c)
        .strip_prefix("tcl::dict::")
        .is_some_and(|sub| !sub.is_empty())
}

/// The reason attached to every [`is_tcl_dict_qualified`] command in the
/// report.
const TCL_DICT_QUALIFIED_REASON: &str = "standalone `::tcl::dict::*` ensemble-implementation spelling (issue #923 idx 105): \
     runtime/rust backs only the `dict` ensemble head, not the qualified name — a direct call is `invalid command name`";

/// Whether `c` is a qualified `::oo::Helpers::*` spelling (issue #1026).
///
/// These are real commands in C Tcl — `info commands ::oo::Helpers::link`
/// answers under tclsh 9.0.4 — which is why the registry carries them
/// alongside their method-context-only bare twins. `runtime/rust` registers
/// only the bare names its method dispatch installs, so calling the
/// qualified spelling there is `invalid command name`: the same genuine,
/// visible gap [`is_tcl_dict_qualified`] records for `::tcl::dict::*`,
/// rather than hiding it under [`HANDLER_EXTRA`] as if the bare handler
/// backed it.
fn is_oo_helpers_qualified(c: &str) -> bool {
    c.strip_prefix("::")
        .unwrap_or(c)
        .strip_prefix("oo::Helpers::")
        .is_some_and(|sub| !sub.is_empty())
}

/// The reason attached to every [`is_oo_helpers_qualified`] command in the
/// report.
const OO_HELPERS_QUALIFIED_REASON: &str = "qualified `::oo::Helpers::*` spelling (issue #1026): runtime/rust registers only the bare, \
     method-context name its dispatch installs, so a direct qualified call is `invalid command name`";

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
    /// A known, tracked gap.
    KnownGap(&'static str),
    /// Unaccounted for — a new gap the gate flags.
    Unclassified,
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
/// classification is explicit.
///
/// Adversarial-review finding: this used to check `d.contains(TCL90)` — a
/// single-version bit that a `TCL91`-only gate (like TIP 745's) never has
/// set, since 9.1-only is a *distinct* bit from 9.0, not a superset of it.
/// That silently dropped every 9.1-only-gated command from this set
/// entirely — not "classified as a gap", genuinely invisible to the gate.
/// `intersects(TCL90_PLUS)` is the established idiom this repo already uses
/// elsewhere for "is this gate compatible with dialect X or later"
/// (`tcl-registry`'s `registry.rs`/`profile_queries.rs`).
fn core_commands() -> BTreeSet<String> {
    use tcl_dialect::DialectSet;
    tcl_registry::commands::tcl::tcl_command_specs()
        .iter()
        .filter(|s| s.required_package.is_none())
        .filter(|s| {
            s.dialects
                .is_none_or(|d| d.intersects(DialectSet::TCL90_PLUS))
        })
        .map(|s| s.name.to_string())
        .collect()
}

/// The `register_spec_builtin` form recognised by [`scan_handler_source`],
/// compiled once rather than once per scanned runtime source file.
static SPEC_REGISTRATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)register_spec_builtin\(\s*[A-Za-z_][A-Za-z0-9_]*\.get\(\s*"([^"]+)"\s*\)"#)
        .expect("static direct register_spec_builtin regex")
});

/// Scan one runtime source file for registry-backed command registrations.
///
/// Ordinary handlers expose their name directly through
/// `register_builtin(b"NAME", …)`. Semantically attested handlers instead pass
/// a registry-owned `CommandSpec` to `register_spec_builtin`; for those, the
/// scanner requires the exact literal `registry.get("NAME")` expression to be
/// nested in that call. This recognises the mechanism without correlating
/// binding names across scopes or enumerating commands that use it.
fn scan_handler_source(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    let mut rest = text;
    while let Some(i) = rest.find("register_builtin(b\"") {
        rest = &rest[i + "register_builtin(b\"".len()..];
        if let Some(end) = rest.find('"') {
            names.insert(canon(&rest[..end]).to_string());
            rest = &rest[end..];
        }
    }

    for captures in SPEC_REGISTRATION_RE.captures_iter(text) {
        if let Some(name) = captures.get(1).map(|capture| capture.as_str()) {
            names.insert(canon(name).to_string());
        }
    }

    names
}

/// Scan the runtime source for literal and registry-spec registrations.
fn scan_handlers(root: &std::path::Path) -> Result<BTreeSet<String>> {
    let src = root.join("runtime/rust/src");
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(&src).with_context(|| format!("reading {}", src.display()))? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        names.extend(scan_handler_source(&text));
    }
    Ok(names)
}

/// Read the runtime file ensemble declaration and dispatch arms. This is
/// deliberately source-level: the standalone runtime cannot be made a normal
/// workspace dependency of xtask, while a literal registration alone cannot
/// prove that the registry's subcommand/arity surface is implemented.
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

fn classify(name: &str, backed: &BTreeSet<String>) -> Status {
    let c = canon(name);
    if backed.contains(c) {
        return Status::Handler;
    }
    if let Some((_, why)) = HANDLER_EXTRA.iter().find(|(n, _)| *n == c) {
        return Status::HandlerNative(why);
    }
    if is_mathfunc_command(c) {
        return Status::HandlerNative(MATHFUNC_COMMAND_REASON);
    }
    if is_mathop_command(c) {
        return Status::HandlerNative(MATHOP_COMMAND_REASON);
    }
    if is_tcl_dict_qualified(c) {
        return Status::KnownGap(TCL_DICT_QUALIFIED_REASON);
    }
    if is_oo_helpers_qualified(c) {
        return Status::KnownGap(OO_HELPERS_QUALIFIED_REASON);
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
         > command, or on a stale classification entry.\n\n\
         Source of truth: `tcl-registry` core command specs (`required_package == None`),\n\
         restricted to those available at Tcl 9.0 or later. Backing: a literal `register_builtin`\n\
         handler or registry-derived `register_spec_builtin` handler in `runtime/rust/`,\n\
         a native registration outside those scans (TclOO metaclass, per-object `my`),\n\
         or an explicit *not required* classification.\n\n\
         | status | count |\n| --- | --- |\n\
         | handler | {} |\n| handler (native) | {} |\n| stdlib | {} |\n\
         | not-required | {} |\n| known-gap | {} |\n\
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
    let file_fidelity = file_fidelity_errors(&root)?;

    // Gaps: a core command that is neither backed nor classified.
    let gaps: Vec<&String> = core
        .iter()
        .filter(|n| matches!(classify(n, &backed), Status::Unclassified))
        .collect();
    // Stale classification: a listed (canonical) name that no core command maps
    // to, or one that now has a scanned runtime handler (so the classification
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
             (add a runtime handler registration, or classify in `command_backing.rs`):",
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
        "OK: all {} core registry commands are backed or classified.",
        core.len()
    );
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::{
        HANDLER_EXTRA, KNOWN_UNBACKED, NOT_REQUIRED, STDLIB, Status, canon, classify,
        compare_file_surface, file_fidelity_errors, scan_file_surface,
    };
    use super::{core_commands, render_report, scan_handler_source, scan_handlers};
    use crate::util::repo_root;
    use std::collections::BTreeSet;

    #[test]
    fn scanner_requires_an_exact_runtime_registration() {
        let source = r#"
            interp.register_builtin(b"::literal", literal_handler);

            interp.register_spec_builtin(
                registry.get("string").expect("core string spec"),
                string_handler,
            );

            {
                let spec = registry.get("dict").expect("core dict spec");
                inspect(spec);
            }
            let spec = registry.get("list").expect("core list spec");
            interp.register_spec_builtin(spec, list_handler);
        "#;

        assert_eq!(
            scan_handler_source(source),
            BTreeSet::from(["literal".to_owned(), "string".to_owned()]),
        );
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

    /// The drift guard this module's own `--check` flag enforces, run
    /// directly as a unit test (mirroring the `committed_X_matches_generated`
    /// pattern every sibling generator in this crate already has —
    /// adversarial-review finding: this file had none, unlike
    /// `gen_zed_queries.rs`/`gen_editor_catalogs.rs`/`gen_ai.rs`/etc). Fails
    /// if `docs/generated/wasm-command-backing.md` is stale relative to the
    /// live registry + runtime scan, or if any core command is genuinely
    /// unaccounted for.
    #[test]
    fn committed_wasm_command_backing_matches_generated() {
        let root = repo_root();
        let core = core_commands();
        let backed = scan_handlers(&root).expect("scan runtime/rust for register_builtin calls");
        let report = render_report(&core, &backed);
        let committed = std::fs::read_to_string(root.join(super::REPORT_PATH))
            .expect("reading docs/generated/wasm-command-backing.md");
        assert_eq!(
            committed, report,
            "docs/generated/wasm-command-backing.md is stale — run `cargo xtask command-backing`"
        );
    }

    /// Every core command must be backed or classified — an `Unclassified`
    /// result here is exactly the drift-gate failure `--check` reports as
    /// a genuinely new gap.
    #[test]
    fn every_core_command_is_backed_or_classified() {
        let root = repo_root();
        let core = core_commands();
        let backed = scan_handlers(&root).expect("scan runtime/rust for register_builtin calls");
        let gaps: Vec<&String> = core
            .iter()
            .filter(|n| matches!(classify(n, &backed), Status::Unclassified))
            .collect();
        assert!(
            gaps.is_empty(),
            "unclassified core commands (add a register_builtin handler, or classify in \
             command_backing.rs): {gaps:?}"
        );
    }

    /// No classification list entry names a command that's stopped being
    /// "core" (dropped from the registry) or that's since gained a real
    /// runtime handler (a stale, now-redundant classification) — the same
    /// staleness `--check` reports.
    #[test]
    fn no_classification_entry_is_stale() {
        let root = repo_root();
        let core = core_commands();
        let backed = scan_handlers(&root).expect("scan runtime/rust for register_builtin calls");
        let core_canon: BTreeSet<&str> = core.iter().map(|n| canon(n)).collect();
        for (n, _) in HANDLER_EXTRA {
            assert!(
                core_canon.contains(n),
                "`{n}` in HANDLER_EXTRA is classified but not a core registry command"
            );
        }
        for (n, _) in STDLIB.iter().chain(NOT_REQUIRED).chain(KNOWN_UNBACKED) {
            assert!(
                core_canon.contains(n),
                "`{n}` is classified but not a core registry command"
            );
            assert!(
                !backed.contains(*n),
                "`{n}` is classified but now has a runtime handler — remove the classification"
            );
        }
    }
}
