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

//! `value-transfers` — the value-transfer drift gate
//! (`docs/design/compiler/value-transfers-migration.md` § *The drift gate
//! and the generated inventory*).
//!
//! Per-command knowledge on the value axis belongs to the registry's
//! `CommandSemantics` declarations; the compiler, analyser, and language
//! server are generic consumers. Three parts keep that honest:
//!
//! 1. **A source lint.** A Tcl command or subcommand name used to
//!    *recognise an invocation* — compared with `==` / `!=` against a
//!    `command`, `canonical_command`, `cmd`, `cmd_name`, `head`, `sub`, or
//!    `subcommand` binding, listed in a `matches!` on one, or as a
//!    string-literal arm of a `match` on one — and a `match` arm on a
//!    catalogued evaluator id (`NativeEvalId::…`) outside the registry. A
//!    reviewed site carries `// value-transfer-ok: <axis> — <reason>` on
//!    the line or in the comment block directly above it; a file whose
//!    sites all belong to one axis may carry one
//!    `// value-transfer-ok(file): <axis> — <reason>` in its header. The
//!    axis names the registry field the fact belongs to, or `irreducible`
//!    with why, and the generated inventory lists every waiver by axis. The
//!    lint is a warning mechanism: a renamed binding or a helper table can
//!    evade it, so the registry's contract tests and ownership review are
//!    the gate's other half. Every scanned file is held to one of two
//!    rules. A *clean* file — each file slice 1 of the migration touched,
//!    and each file a later slice adds — has every site waived or gone.
//!    Every other file is *ratcheted*: its count of unwaived sites is
//!    pinned here, `--check` fails when the count rises, and the pin is
//!    lowered beside the review that removes or waives the file's sites,
//!    never raised and never added. The migration plan's ledger lists
//!    each ratcheted file with its pin and the slice, or the axis
//!    migration, that reviews it; the gate holds the two in agreement.
//! 2. **A registry enumeration**, written to
//!    `docs/generated/value-transfers.md`: every command, subcommand, and
//!    declaring form with its declaration state, route, route owner, target
//!    roles, and a *gap* column for a command that writes variables or
//!    declares purity and has no route. `--check` fails on drift, on a
//!    variable-writing command with no semantics that no classification
//!    names, and on a stale classification.
//! 3. **A pinned-set test** — `rust/tcl-registry/tests/value_transfers.rs`
//!    — over every loadable dialect and the shipped `.tclspec` packs, so a
//!    route cannot appear, vanish, or move without that file changing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use tcl_dialect::model::SpecSurface;
use tcl_registry::forms::CommandForm;
use tcl_registry::spec::{CommandSpec, SubCommand};
use tcl_registry::value_transfer::{EvalRoute, EvaluatorOwner, resolve_semantics};
use tcl_registry::{ArgRole, CommandRegistry, ResultStability, Traits};

use crate::util::repo_root;

const REPORT_PATH: &str = "docs/generated/value-transfers.md";

/// The design page whose ledger lists every ratcheted file with its pin.
const LEDGER_PATH: &str = "docs/design/compiler/value-transfers-migration.md";

/// The roots the lint scans: the compiler crate and the analysis and
/// tooling tiers. The registry and this gate are outside them by
/// construction — they are where the dispatch lives.
const LINT_ROOTS: &[&str] = &[
    "rust/tcl-compiler/src",
    "rust/tcl-lsp-core/src",
    "rust/tcl-mcp/src",
    "rust/tcl-cli/src",
    "rust/tcl-diagram/src",
    "rust/tcl-irules/src",
    "rust/tcl-irule-test/src",
    "rust/tcl-bigip/src",
    "rust/tcl-sslictcl/src",
    "rust/tcl-syntax/src",
];

/// The files the lint holds clean: every recogniser-shaped site is waived
/// or gone. Slice 1 touched each of these; a slice adds the files it
/// touches, and a file never leaves the list.
const CLEAN_FILES: &[&str] = &[
    "rust/tcl-compiler/src/analyser/bounds_checks.rs",
    "rust/tcl-compiler/src/analyser/diagnostics/usage.rs",
    "rust/tcl-compiler/src/analyser/diagnostics/var_command.rs",
    "rust/tcl-compiler/src/cfg_builder/mod.rs",
    "rust/tcl-compiler/src/command_binding.rs",
    "rust/tcl-compiler/src/compilation_unit.rs",
    "rust/tcl-compiler/src/dataflow_graph.rs",
    "rust/tcl-compiler/src/intervals.rs",
    "rust/tcl-compiler/src/ir_helpers.rs",
    "rust/tcl-compiler/src/lib.rs",
    "rust/tcl-compiler/src/optimiser/chain_fold.rs",
    "rust/tcl-compiler/src/optimiser/propagation.rs",
    "rust/tcl-compiler/src/sccp.rs",
    "rust/tcl-compiler/src/shimmer/commit.rs",
    "rust/tcl-compiler/src/shimmer/mod.rs",
    "rust/tcl-compiler/src/specialise_factories.rs",
    "rust/tcl-compiler/src/static_loops.rs",
    "rust/tcl-compiler/src/value_transfer.rs",
    "rust/tcl-compiler/src/word_subst.rs",
    "rust/tcl-lsp-core/src/document_links.rs",
];

/// The ratchet over every other scanned file: the pinned count of unwaived
/// recogniser-shaped sites, as of the slice that last reviewed the file.
/// `--check` fails when a file's count rises above its pin, or when a file
/// not listed here gains a site. A pin is lowered beside the review that
/// removes or waives its sites — with the file's ledger row in
/// [`LEDGER_PATH`] — and is never raised or added: a site that moves to a
/// new file is reviewed there.
const RATCHET: &[(&str, usize)] = &[
    ("rust/tcl-cli/src/commands/minimize.rs", 1),
    ("rust/tcl-compiler/src/analyser/class_lattice.rs", 3),
    ("rust/tcl-compiler/src/analyser/commands.rs", 4),
    ("rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs", 2),
    ("rust/tcl-compiler/src/analyser/diagnostics/helpers.rs", 5),
    ("rust/tcl-compiler/src/analyser/diagnostics/security.rs", 2),
    ("rust/tcl-compiler/src/analyser/diagnostics/validity.rs", 2),
    ("rust/tcl-compiler/src/analyser/irules_event_checks.rs", 7),
    ("rust/tcl-compiler/src/analyser/param_traits.rs", 1),
    ("rust/tcl-compiler/src/auto_path_eval.rs", 3),
    ("rust/tcl-compiler/src/codegen/cmd_subst.rs", 3),
    ("rust/tcl-compiler/src/codegen/emitter/bytecoded.rs", 4),
    ("rust/tcl-compiler/src/codegen/emitter/loop_blocks.rs", 2),
    ("rust/tcl-compiler/src/codegen/statements.rs", 2),
    ("rust/tcl-compiler/src/codegen/structured.rs", 2),
    ("rust/tcl-compiler/src/connection_scope.rs", 1),
    ("rust/tcl-compiler/src/inline_uplevel.rs", 1),
    ("rust/tcl-compiler/src/interprocedural.rs", 2),
    ("rust/tcl-compiler/src/irules_checks.rs", 3),
    ("rust/tcl-compiler/src/lowering/mod.rs", 1),
    ("rust/tcl-compiler/src/lowering/structured.rs", 2),
    ("rust/tcl-compiler/src/optimiser/end_offset.rs", 1),
    ("rust/tcl-compiler/src/place_bridge.rs", 2),
    ("rust/tcl-compiler/src/shimmer/thunking.rs", 1),
    ("rust/tcl-compiler/src/ssa.rs", 1),
    ("rust/tcl-compiler/src/taint.rs", 3),
    ("rust/tcl-compiler/src/uri_split.rs", 6),
    ("rust/tcl-compiler/src/var_escape/handlers.rs", 2),
    ("rust/tcl-compiler/src/var_escape/helpers.rs", 1),
    ("rust/tcl-compiler/src/var_escape/slot_resolution.rs", 6),
    ("rust/tcl-compiler/src/var_scoping.rs", 1),
    ("rust/tcl-irules/src/lib.rs", 1),
    ("rust/tcl-lsp-core/src/oo_body.rs", 1),
    ("rust/tcl-mcp/src/irule_gen.rs", 5),
    ("rust/tcl-mcp/src/irule_test.rs", 2),
    ("rust/tcl-sslictcl/src/bin/sslictcl-data.rs", 4),
];

const WAIVER: &str = "value-transfer-ok:";
const FILE_WAIVER: &str = "value-transfer-ok(file):";

/// The axes a waiver may name: a registry field the fact belongs to, the
/// value axis itself while a site awaits its slice, or the sanctioned
/// exception.
const AXES: &[&str] = &[
    "dataflow",
    "options",
    "arg_roles",
    "definition_body",
    "traits",
    "substitution",
    "case_list",
    "return_type",
    "special_vars",
    "events",
    "side_effects",
    "frame_effect",
    "native_lowering",
    "abbrev",
    "completion",
    "irreducible",
    "shape",
];

/// The bindings a Tcl command or subcommand name is compared against.
const HEAD_BINDINGS: &[&str] = &[
    "command",
    "canonical_command",
    "cmd",
    "cmd_name",
    "head",
    "sub",
    "subcommand",
];

/// Run the gate: the lint, then the enumeration, written or verified.
pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let lint = lint(&root);
    let mut failed = false;
    if !lint.clean_hits.is_empty() {
        failed = true;
        eprintln!(
            "value-transfers: {} site(s) recognise a command by name in a file the gate holds \
             clean. Per-command knowledge on the value axis is a registry `CommandSemantics` \
             declaration the consumer reaches through the resolved invocation; mark a \
             reviewed site `// value-transfer-ok: <axis> — <reason>` naming the registry \
             field it awaits, or `irreducible — <why>`:",
            lint.clean_hits.len()
        );
        for hit in &lint.clean_hits {
            eprintln!("  {hit}");
        }
    }
    let ledger = fs::read_to_string(root.join(LEDGER_PATH))
        .with_context(|| format!("reading {LEDGER_PATH}, the ratchet's ledger"))?;
    let rows = enumerate()?;
    for problem in ratchet_verdicts(&lint, RATCHET, CLEAN_FILES)
        .into_iter()
        .chain(ledger_verdicts(&ledger, RATCHET))
        .chain(lint.problems.iter().cloned())
        .chain(classification_problems(&rows))
    {
        failed = true;
        eprintln!("value-transfers: {problem}");
    }
    let report = render_report(&rows, &lint);
    let path = root.join(REPORT_PATH);
    if check {
        let on_disk = fs::read_to_string(&path).unwrap_or_default();
        if on_disk != report {
            failed = true;
            eprintln!(
                "value-transfers: {REPORT_PATH} is stale — run `cargo xtask value-transfers`"
            );
        }
    } else {
        fs::write(&path, &report).with_context(|| format!("writing {REPORT_PATH}"))?;
    }
    if failed {
        return Ok(ExitCode::FAILURE);
    }
    println!(
        "value-transfers: OK ({} file(s) clean, {} site(s) waived, {} site(s) pinned across {} \
         ratcheted file(s), {} inventory row(s))",
        CLEAN_FILES.len(),
        lint.waived.len(),
        lint.ratchet_hits.values().map(Vec::len).sum::<usize>(),
        lint.ratchet_hits.len(),
        rows.len()
    );
    Ok(ExitCode::SUCCESS)
}

// The lint

/// One recognised site.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    path: String,
    line: usize,
    snippet: String,
}

impl std::fmt::Display for Site {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.path, self.line, self.snippet)
    }
}

/// A waived site, with the axis and reason its waiver names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Waived {
    axis: String,
    site: Site,
    reason: String,
    /// Whether the waiver is the file's, not the site's.
    file_level: bool,
}

#[derive(Debug, Default)]
struct Lint {
    /// Unwaived sites in a file the gate holds clean.
    clean_hits: Vec<Site>,
    /// Unwaived sites in every other file, by path, in line order.
    ratchet_hits: BTreeMap<String, Vec<Site>>,
    waived: Vec<Waived>,
    problems: Vec<String>,
}

fn lint(root: &Path) -> Lint {
    let mut out = Lint::default();
    let mut files = Vec::new();
    for dir in LINT_ROOTS {
        collect_rs_files(&root.join(dir), &mut files);
    }
    files.sort();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if rel.contains("/tests/") || rel.ends_with("/tests.rs") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let clean = CLEAN_FILES.contains(&rel.as_str());
        let file_waiver = file_waiver(&text);
        if let Some((axis, _)) = &file_waiver
            && !AXES.contains(&axis.as_str())
        {
            out.problems
                .push(format!("{rel}: file waiver names unknown axis `{axis}`"));
        }
        for (line, snippet) in scan(&text) {
            let site = Site {
                path: rel.clone(),
                line,
                snippet,
            };
            if let Some((axis, reason)) = site_waiver(&text, line) {
                if !AXES.contains(&axis.as_str()) {
                    out.problems
                        .push(format!("{rel}:{line}: waiver names unknown axis `{axis}`"));
                }
                out.waived.push(Waived {
                    axis,
                    site,
                    reason,
                    file_level: false,
                });
            } else if let Some((axis, reason)) = &file_waiver {
                out.waived.push(Waived {
                    axis: axis.clone(),
                    site,
                    reason: reason.clone(),
                    file_level: true,
                });
            } else if clean {
                out.clean_hits.push(site);
            } else {
                out.ratchet_hits.entry(rel.clone()).or_default().push(site);
            }
        }
    }
    out.clean_hits.sort();
    out.waived.sort();
    out
}

/// Recursively collect `.rs` files under `dir` (skipping `target/`).
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Whether `text[..at]` ends with a head binding, optionally followed by an
/// `.as_str()` / `.as_deref()` / `.as_ref()` view.
fn ends_with_head_binding(before: &str) -> bool {
    let before = before.trim_end();
    let before = ["as_str()", "as_deref()", "as_ref()"]
        .iter()
        .find_map(|view| before.strip_suffix(view).and_then(|b| b.strip_suffix('.')))
        .unwrap_or(before);
    let ident_start = before
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map_or(0, |i| i + 1);
    let ident = &before[ident_start..];
    HEAD_BINDINGS.contains(&ident)
}

/// Whether `text[at..]` starts with a head binding (after optional `&` /
/// `*` and a path prefix).
fn starts_with_head_binding(after: &str) -> bool {
    let after = after.trim_start().trim_start_matches(['&', '*']);
    let end = after
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
        .unwrap_or(after.len());
    let last = after[..end].rsplit('.').next().unwrap_or("");
    HEAD_BINDINGS.contains(&last)
}

/// Whether a line is a string-literal `match` arm: `"x" =>`, or
/// `"x" | "y" =>`, with an optional guard.
fn is_literal_arm(line: &str) -> bool {
    let t = line.trim_start();
    if !t.starts_with('"') {
        return false;
    }
    let mut rest = t;
    loop {
        let Some(after_open) = rest.strip_prefix('"') else {
            return false;
        };
        let Some(close) = after_open.find('"') else {
            return false;
        };
        rest = after_open[close + 1..].trim_start();
        if let Some(r) = rest.strip_prefix('|') {
            rest = r.trim_start();
            continue;
        }
        return rest.starts_with("=>") || rest.starts_with("if ");
    }
}

/// Whether a line is a tuple-pattern `match` arm naming a string literal:
/// `("set", _) | (_, "::set")`, with the guard and `=>` on this line or the
/// next.
fn is_tuple_literal_arm(line: &str) -> bool {
    let t = line.trim_start();
    if !t.starts_with('(') {
        return false;
    }
    let mut rest = t;
    let mut named = false;
    loop {
        let Some(inner) = rest.strip_prefix('(') else {
            return false;
        };
        let Some(close) = inner.find(')') else {
            return false;
        };
        named |= inner[..close].contains('"');
        rest = inner[close + 1..].trim_start();
        if let Some(r) = rest.strip_prefix('|') {
            rest = r.trim_start();
            continue;
        }
        return named && (rest.is_empty() || rest.starts_with("=>") || rest.starts_with("if "));
    }
}

/// Where the item a `#[cfg(test)]` at `index` annotates ends: `None` when
/// it is an inline test module, whose end is the file's; else the index of
/// the item's last line — a module declaration's own line, the line that
/// closes the item's braces, or the one that ends it with `;`.
fn test_item_end(lines: &[&str], index: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut opened = false;
    for (at, raw) in lines.iter().enumerate().skip(index + 1) {
        let line = code_part(raw);
        let t = line.trim_start();
        if !opened && (t.is_empty() || t.starts_with("//") || t.starts_with("#[")) {
            continue;
        }
        if !opened && (t.starts_with("mod ") || t.starts_with("pub mod ")) {
            return t.trim_end().ends_with(';').then_some(at);
        }
        for c in code_braces(line) {
            if c == '{' {
                depth += 1;
                opened = true;
            } else {
                depth = depth.saturating_sub(1);
            }
        }
        if (opened && depth == 0) || (!opened && t.trim_end().ends_with(';')) {
            return Some(at);
        }
    }
    None
}

/// The braces of `line` outside its string and character literals, in
/// order.
fn code_braces(line: &str) -> Vec<char> {
    let mut out = Vec::new();
    let mut in_string = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if in_string => {
                chars.next();
            }
            '"' => in_string = !in_string,
            '\'' if !in_string => {
                // A character literal `'{'` or `'\''`; a lifetime has no
                // closing quote within three characters.
                let rest: String = chars.clone().take(3).collect();
                if rest.chars().nth(1) == Some('\'') {
                    chars.next();
                    chars.next();
                } else if rest.starts_with('\\') && rest.chars().nth(2) == Some('\'') {
                    chars.next();
                    chars.next();
                    chars.next();
                }
            }
            '{' | '}' if !in_string => out.push(c),
            _ => {}
        }
    }
    out
}

/// Yield `(1-based line, trimmed line text)` for every recogniser-shaped
/// site in `text`. A `#[cfg(test)]` module ends the scan; any other
/// `#[cfg(test)]` item — a test-only helper — is skipped alone.
fn scan(text: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut hit = |index: usize| {
        let snippet: String = lines[index].trim().chars().take(96).collect();
        if out.last().is_none_or(|(l, _)| *l != index + 1) {
            out.push((index + 1, snippet));
        }
    };
    let mut skip_to: Option<usize> = None;
    for (index, raw) in lines.iter().enumerate() {
        if skip_to.is_some_and(|end| index <= end) {
            continue;
        }
        if raw.trim_start().starts_with("#[cfg(test)]") {
            match test_item_end(&lines, index) {
                Some(end) => {
                    skip_to = Some(end);
                    continue;
                }
                None => break,
            }
        }
        let line = code_part(raw);
        if line.trim_start().starts_with("//") {
            continue;
        }
        // `<head> == "lit"` / `<head> != "lit"`, and the reverse.
        let mut from = 0;
        while let Some(off) = line[from..].find(['=', '!']) {
            let at = from + off;
            from = at + 1;
            let Some(op_end) = line.get(at..at + 2) else {
                break;
            };
            if op_end != "==" && op_end != "!=" {
                continue;
            }
            let before = &line[..at];
            let after = &line[at + 2..];
            let literal_after = after.trim_start().starts_with('"');
            let literal_before = before.trim_end().ends_with('"');
            if (literal_after && ends_with_head_binding(before))
                || (literal_before && starts_with_head_binding(after))
            {
                hit(index);
            }
        }
        // `matches!(<head>, "lit" | …)`.
        if let Some(pos) = line.find("matches!(") {
            let args = &line[pos + "matches!(".len()..];
            if let Some(comma) = args.find(',')
                && ends_with_head_binding(&args[..comma])
                && args[comma + 1..].trim_start().starts_with('"')
            {
                hit(index);
            }
        }
        // `match <head> {` with a string-literal arm below.
        let t = line.trim_start();
        if let Some(subject) = t.strip_prefix("match ")
            && let Some(brace) = subject.rfind('{')
            && ends_with_head_binding(&subject[..brace])
            && lines[index + 1..]
                .iter()
                .take(48)
                .any(|l| is_literal_arm(code_part(l)))
        {
            hit(index);
        }
        // A tuple-pattern arm naming a literal, in a `match (…) {` whose
        // tuple holds a head binding: each arm recognises its command.
        if is_tuple_literal_arm(line)
            && lines[..index]
                .iter()
                .rev()
                .take(48)
                .map(|l| code_part(l).trim_start())
                .find_map(|l| l.strip_prefix("match ("))
                .and_then(|subject| subject.rfind(')').map(|close| &subject[..close]))
                .is_some_and(|tuple| tuple.split(',').any(ends_with_head_binding))
        {
            hit(index);
        }
        // A `match` arm on a catalogued evaluator id.
        if line.contains("NativeEvalId::") && line.contains("=>") {
            hit(index);
        }
    }
    out
}

/// The line with a trailing `//` comment removed, when the comment is not
/// inside a string literal.
fn code_part(line: &str) -> &str {
    let mut in_string = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_string => i += 1,
            b'"' => in_string = !in_string,
            b'/' if !in_string && bytes.get(i + 1) == Some(&b'/') => return &line[..i],
            _ => {}
        }
        i += 1;
    }
    line
}

/// The `(axis, reason)` a waiver marker carries.
fn parse_waiver(after_marker: &str) -> (String, String) {
    let text = after_marker.trim();
    let (axis, reason) = text
        .split_once(" — ")
        .or_else(|| text.split_once(" - "))
        .map_or((text, ""), |(a, r)| (a, r));
    (axis.trim().to_owned(), reason.trim().to_owned())
}

/// The site waiver for `line_no`: on the line, in the comment block
/// directly above it, or — for a `match` arm on an evaluator id — in the
/// comment block above the enclosing `match`.
fn site_waiver(text: &str, line_no: usize) -> Option<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let idx = line_no.saturating_sub(1);
    let marker = |l: &str| {
        l.find(WAIVER)
            .filter(|_| !l.contains(FILE_WAIVER))
            .map(|i| parse_waiver(&l[i + WAIVER.len()..]))
    };
    if let Some(found) = lines.get(idx).and_then(|l| marker(l)) {
        return Some(found);
    }
    let block_above = |at: usize| {
        lines[..at.min(lines.len())]
            .iter()
            .rev()
            .take_while(|l| l.trim_start().starts_with("//"))
            .find_map(|l| marker(l))
    };
    if let Some(found) = block_above(idx) {
        return Some(found);
    }
    if lines.get(idx).is_some_and(|l| l.contains("NativeEvalId::")) {
        let indent = |l: &str| l.len() - l.trim_start().len();
        let arm_indent = lines.get(idx).map_or(0, |l| indent(l));
        let enclosing = lines[..idx]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, l)| indent(l) < arm_indent && l.trim_start().starts_with("match "))
            .map(|(i, _)| i)?;
        return block_above(enclosing);
    }
    None
}

/// The file-level waiver in the file's leading comment, when it has one.
fn file_waiver(text: &str) -> Option<(String, String)> {
    text.lines().take(80).find_map(|l| {
        l.find(FILE_WAIVER)
            .map(|i| parse_waiver(&l[i + FILE_WAIVER.len()..]))
    })
}

// The ratchet

/// The ratchet's verdicts on `lint` against `pins` (the pinned count per
/// file) and `clean` (the files that carry no pin): a count above its pin,
/// with the file's sites; a pin above its count, which is stale; a pin on
/// a clean file; a file pinned twice.
fn ratchet_verdicts(lint: &Lint, pins: &[(&str, usize)], clean: &[&str]) -> Vec<String> {
    let mut verdicts = Vec::new();
    let mut pinned: BTreeMap<&str, usize> = BTreeMap::new();
    for (path, pin) in pins {
        if pinned.insert(path, *pin).is_some() {
            verdicts.push(format!("RATCHET pins `{path}` twice"));
        }
        if clean.contains(path) {
            verdicts.push(format!(
                "`{path}` is held clean and carries no pin; remove it from RATCHET"
            ));
        }
    }
    for (path, sites) in &lint.ratchet_hits {
        let pin = pinned.get(path.as_str()).copied().unwrap_or(0);
        if sites.len() > pin {
            let listed: Vec<String> = sites.iter().map(|site| format!("  {site}")).collect();
            verdicts.push(format!(
                "`{path}` has {} unwaived site(s) against a pin of {pin}; the count may only \
                 fall — move the fact to the registry, or waive a reviewed site with \
                 `// value-transfer-ok: <axis> — <reason>`:\n{}",
                sites.len(),
                listed.join("\n")
            ));
        }
    }
    for (path, pin) in &pinned {
        let count = lint.ratchet_hits.get(*path).map_or(0, Vec::len);
        if count < *pin {
            verdicts.push(format!(
                "`{path}` has {count} unwaived site(s) against a pin of {pin}; lower the pin in \
                 RATCHET (rust/xtask/src/value_transfers.rs) and in its ledger row in \
                 {LEDGER_PATH}"
            ));
        }
    }
    verdicts
}

/// The ledger's verdicts: its ratchet table — the rows of `text` whose
/// first cell is a backticked `rust/…/*.rs` path and whose second cell is
/// a count — held equal to `pins`, so the plan says what the gate enforces.
fn ledger_verdicts(text: &str, pins: &[(&str, usize)]) -> Vec<String> {
    let mut verdicts = Vec::new();
    let mut ledger: BTreeMap<&str, usize> = BTreeMap::new();
    for line in text.lines() {
        let mut cells = line.split('|').map(str::trim);
        let Some((cell, count)) = cells.nth(1).zip(cells.next()) else {
            continue;
        };
        let Some(path) = cell.strip_prefix('`').and_then(|c| c.strip_suffix('`')) else {
            continue;
        };
        let Ok(count) = count.parse::<usize>() else {
            continue;
        };
        if !path.starts_with("rust/")
            || Path::new(path).extension().is_none_or(|e| e != "rs")
            || path.contains(['`', ' '])
        {
            continue;
        }
        if ledger.insert(path, count).is_some() {
            verdicts.push(format!(
                "{LEDGER_PATH} lists `{path}` twice in the ratchet table"
            ));
        }
    }
    for (path, pin) in pins {
        match ledger.get(path) {
            None => verdicts.push(format!(
                "{LEDGER_PATH} has no ratchet row for `{path}`, pinned at {pin}"
            )),
            Some(count) if count != pin => verdicts.push(format!(
                "{LEDGER_PATH} lists `{path}` at {count}; RATCHET pins it at {pin}"
            )),
            Some(_) => {}
        }
    }
    for path in ledger.keys() {
        if !pins.iter().any(|(pinned, _)| pinned == path) {
            verdicts.push(format!(
                "{LEDGER_PATH} has a ratchet row for `{path}`, which RATCHET does not pin"
            ));
        }
    }
    verdicts
}

// The enumeration

/// One inventory row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Row {
    command: String,
    scope: &'static str,
    dialects: BTreeSet<String>,
    semantics: String,
    route: String,
    owner: String,
    enabled: bool,
    targets: String,
    gaps: Vec<&'static str>,
    /// Whether a variable-writing role is declared and no semantics is.
    unclassified_write: bool,
}

fn enumerate() -> Result<Vec<Row>> {
    let mut rows: BTreeMap<(String, &'static str), Row> = BTreeMap::new();
    for profile in tcl_dialect::DialectProfile::all()
        .iter()
        .chain(std::iter::once(crate::environment::profile_for_dialect(
            "tk",
        )))
    {
        let registry = crate::environment::store_for_profile(profile);
        collect_registry(&mut rows, profile.name, registry);
    }
    let packs = tcl_spectcl::bundled::load_from(&repo_root().join("specs"));
    if packs.is_empty() {
        bail!("the shipped EDA loadables must be present for the value-transfer inventory");
    }
    for profile in tcl_dialect::DialectProfile::all() {
        let registry = tcl_spectcl::bundled::registry_for_dialect_from(profile.name, &packs);
        collect_registry(&mut rows, profile.name, &registry);
    }
    Ok(rows.into_values().collect())
}

fn collect_registry(
    rows: &mut BTreeMap<(String, &'static str), Row>,
    dialect: &str,
    registry: &CommandRegistry,
) {
    let mut names: Vec<_> = registry.command_names().collect();
    names.sort_unstable();
    for name in names {
        let Some(spec) = registry.get(name) else {
            continue;
        };
        if !visible_in(spec.surface, dialect) {
            continue;
        }
        push_row(rows, dialect, spec, None, None);
        for sub in spec.subcommands {
            if visible_in(sub.surface.or(spec.surface), dialect) {
                push_row(rows, dialect, spec, Some(sub), None);
            }
        }
        for form in spec.command_forms {
            if visible_in(form.surface.or(spec.surface), dialect) && !form.semantics.is_inherited()
            {
                push_row(rows, dialect, spec, None, Some(form));
            }
        }
    }
}

fn visible_in(surface: Option<&'static [SpecSurface]>, dialect: &str) -> bool {
    let Some(profile) = tcl_dialect::DialectProfile::find(dialect) else {
        return true;
    };
    surface
        .is_none_or(|have| tcl_dialect::model::surface_admits(have, Some(&profile.surface_query())))
}

fn push_row(
    rows: &mut BTreeMap<(String, &'static str), Row>,
    dialect: &str,
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    form: Option<&CommandForm>,
) {
    let (command, scope) = match (sub, form) {
        (Some(sub), _) => (format!("{} {}", spec.name, sub.name), "subcommand"),
        (None, Some(form)) => (format!("{} (form {})", spec.name, form.name), "form"),
        (None, None) => (spec.name.to_owned(), "command"),
    };
    let key = (command.clone(), scope);
    if let Some(row) = rows.get_mut(&key) {
        row.dialects.insert(dialect.to_owned());
        return;
    }
    let resolved = resolve_semantics(spec, sub, form);
    let (route, owner, enabled) = describe_route(resolved.route());
    let semantics = match resolved.semantics() {
        Some(semantics) => format!(
            "{} · `{}`",
            resolved.origin().as_str(),
            semantics.identity()
        ),
        None => resolved.origin().as_str().to_owned(),
    };
    let (targets, writes) = target_roles(spec, sub, form);
    let has_semantics = resolved.semantics().is_some();
    let traits = spec.traits | sub.map_or_else(Traits::empty, |s| s.traits);
    let pure = traits.contains(Traits::PURE)
        || sub.is_some_and(|s| s.pure)
        || traits.contains(Traits::CSE_CANDIDATE)
        || matches!(
            sub.and_then(|s| s.result_stability)
                .or(spec.result_stability),
            Some(ResultStability::ReferentiallyTransparent)
        );
    let mut gaps = Vec::new();
    if writes && !has_semantics {
        gaps.push("writes a variable, no semantics");
    }
    if writes && has_semantics && !enabled {
        gaps.push("descriptor without a route");
    }
    if pure && !enabled {
        gaps.push("pure, no route");
    }
    if traits.contains(Traits::READS_BEFORE_WRITE) && !enabled && !writes {
        gaps.push("reads before write, no route");
    }
    rows.insert(
        key,
        Row {
            command,
            scope,
            dialects: BTreeSet::from([dialect.to_owned()]),
            semantics,
            route,
            owner,
            enabled,
            targets,
            gaps,
            unclassified_write: writes && !has_semantics,
        },
    );
}

fn describe_route(route: Option<EvalRoute>) -> (String, String, bool) {
    match route {
        None => ("—".to_owned(), "—".to_owned(), false),
        Some(EvalRoute::Direct { id }) => {
            let owner = match id.owner() {
                EvaluatorOwner::Registry => "registry".to_owned(),
                EvaluatorOwner::Transitional { retires_in_slice } => {
                    format!("compiler, transitional until slice {retires_in_slice}")
                }
            };
            (format!("direct `{}`", id.as_str()), owner, true)
        }
        Some(EvalRoute::Expression { language }) => (
            format!("expression `{}`", language.as_str()),
            "compiler engine adapter (registry argument assembly lands in slice 3)".to_owned(),
            true,
        ),
        Some(EvalRoute::Implementation(capability)) => (
            format!("implementation `{}`", capability.identity.id),
            "bounded engine".to_owned(),
            true,
        ),
        Some(EvalRoute::None { reason }) => {
            (format!("none ({})", reason.as_str()), "—".to_owned(), false)
        }
    }
}

/// The variable-writing roles the effective descriptor declares, and
/// whether there are any.
fn target_roles(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    form: Option<&CommandForm>,
) -> (String, bool) {
    let (static_roles, resolver_roles): (&[(u8, ArgRole)], &[ArgRole]) = match (form, sub) {
        (Some(form), _) => (form.arg_roles, &[]),
        (None, Some(sub)) => (sub.arg_roles, sub.arg_role_resolver_roles),
        (None, None) => (spec.arg_roles, spec.arg_role_resolver_roles),
    };
    let mut parts: Vec<String> = static_roles
        .iter()
        .filter(|(_, role)| *role == ArgRole::VarWrite)
        .map(|(index, _)| format!("`VarWrite@{index}`"))
        .collect();
    if resolver_roles.contains(&ArgRole::VarWrite) {
        parts.push("`VarWrite` (resolver)".to_owned());
    }
    if sub.is_none()
        && form.is_none()
        && let Some(at) = spec.assigns_variable_at
    {
        parts.push(format!("`assigns_variable_at {at}`"));
    }
    let writes = !parts.is_empty();
    (
        if writes {
            parts.join(", ")
        } else {
            "—".to_owned()
        },
        writes,
    )
}

// Classification of the gaps

/// A variable-writing command with no semantics, classified by the slice of
/// the migration plan (`value-transfers-migration.md` § *The slices*) that
/// gives it one. Keyed by the inventory's command column (`cmd` or
/// `cmd sub`); a key ending in ` *` covers every subcommand of an ensemble.
/// An entry no row matches is stale and fails the gate. Slice 2 left no
/// entry of its own: `set` and the `dict` keyed updates declare their
/// semantics, `append` and `lappend` derive theirs, and `const`, `lset`,
/// `ledit` and `lpop` wait for the existence rung and the new list cores.
const KNOWN_GAPS: &[(&str, &str)] = &[
    // Slice 10, completion paths.
    (
        "catch",
        "slice 10 — the completion protocol's result and options writes",
    ),
    // Slice 13, proc-level transfer summaries and the binders they carry.
    (
        "global",
        "slice 13 — the binder's `Name` outcome with its frame level",
    ),
    (
        "variable",
        "slice 13 — the binder's `Name` outcome with its frame level",
    ),
    (
        "my variable",
        "slice 13 — the binder's `Name` outcome with its frame level",
    ),
    (
        "sharedvar",
        "slice 13 — the iRules connection-scoped binder, a `Name` outcome",
    ),
    (
        "info default",
        "slice 13 — the parameter default the transfer summary carries",
    ),
    // Slice 7, broader execution: the list cell updates over new shared
    // cores, then tcllib and the Tcl-level library procedures moving to
    // SpecTcl with a declared implementation.
    (
        "lset",
        "slice 7 — the list cell updates over new shared cores",
    ),
    (
        "ledit",
        "slice 7 — the list cell updates over new shared cores",
    ),
    (
        "lpop",
        "slice 7 — the list cell updates over new shared cores",
    ),
    (
        "base32::core::define",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "base32::core::valid",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "cmdline::getKnownOpt",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "cmdline::getKnownOptions",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "cmdline::typedGetopt",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "cmdline::typedGetoptions",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "fileutil::foreachLine",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "fileutil::test",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "math::statistics::filter",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "math::statistics::map",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "math::statistics::samplescount",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "struct::list filterfor",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "struct::list foreachperm",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "struct::list mapfor",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "tie::tie",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "tie::untie",
        "slice 7 — a declared implementation when the tcllib specs move to SpecTcl",
    ),
    (
        "tcl_findLibrary",
        "slice 7 — a declared implementation for the Tcl-level library procedure",
    ),
    (
        "tcltest::normalizePath",
        "slice 7 — a declared implementation for the Tcl-level library procedure",
    ),
];

fn classification_problems(rows: &[Row]) -> Vec<String> {
    let mut problems = Vec::new();
    let by_name: BTreeMap<&str, &Row> = rows.iter().map(|r| (r.command.as_str(), r)).collect();
    for (name, _) in KNOWN_GAPS {
        let covered = if let Some(prefix) = name.strip_suffix(" *") {
            rows.iter()
                .any(|r| r.command.starts_with(&format!("{prefix} ")) && r.unclassified_write)
        } else {
            by_name.get(name).is_some_and(|r| r.unclassified_write)
        };
        if !covered {
            problems.push(format!(
                "stale gap classification `{name}`: no variable-writing command without semantics matches it"
            ));
        }
    }
    for row in rows.iter().filter(|r| r.unclassified_write) {
        if gap_classification(row).is_none() {
            problems.push(format!(
                "`{}` writes a variable and declares no semantics; classify it in KNOWN_GAPS \
                 (rust/xtask/src/value_transfers.rs) with the slice that gives it one",
                row.command
            ));
        }
    }
    problems
}

fn gap_classification(row: &Row) -> Option<&'static str> {
    KNOWN_GAPS
        .iter()
        .find(|(name, _)| {
            name.strip_suffix(" *")
                .map_or(*name == row.command, |prefix| {
                    row.command.starts_with(&format!("{prefix} "))
                })
        })
        .map(|(_, why)| *why)
}

// The report

fn render_report(rows: &[Row], lint: &Lint) -> String {
    let mut out = String::new();
    out.push_str("<!-- Generated by `cargo xtask value-transfers`; do not edit. -->\n\n");
    out.push_str("# Value transfers — the registry inventory\n\n");
    out.push_str(
        "Every command, subcommand, and declaring form, resolved through the value-transfer \
         declaration states (`docs/design/compiler/value-transfers.md` § *One invocation, one \
         context*), with the evaluator route it names and who implements it. *Semantics* is the \
         declaration state and the specialisation's identity; *Route* the declared route; *Owner* \
         who implements it — the registry, or the compiler's transitional handler with the slice \
         of the migration plan that retires it; *Enabled* whether the route evaluates at all: \
         descriptor availability and enabled evaluation are separate columns. *Targets* lists the \
         variable-writing roles the effective descriptor declares. *Gap* names what the row still \
         lacks. A row with no semantics and no gap is counted per dialect below rather than listed.\n\n",
    );
    render_declarations(&mut out, rows);
    render_silent_rows(&mut out, rows);
    render_waivers(&mut out, lint);
    render_ratchet(&mut out, lint);
    out
}

fn render_declarations(out: &mut String, rows: &[Row]) {
    let _ = writeln!(out, "## Declarations and gaps\n");
    let _ = writeln!(
        out,
        "| Command | Scope | Dialects | Semantics | Route | Owner | Enabled | Targets | Gap | Classified |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|---|");
    for row in rows
        .iter()
        .filter(|r| r.semantics != "none" || !r.gaps.is_empty())
    {
        let dialects: Vec<&str> = row.dialects.iter().map(String::as_str).collect();
        let classified = if row.unclassified_write {
            gap_classification(row).unwrap_or("**unclassified**")
        } else {
            "—"
        };
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row.command,
            row.scope,
            dialects.join(", "),
            row.semantics,
            row.route,
            row.owner,
            if row.enabled { "yes" } else { "no" },
            row.targets,
            if row.gaps.is_empty() {
                "—".to_owned()
            } else {
                row.gaps.join("; ")
            },
            classified,
        );
    }
}

fn render_silent_rows(out: &mut String, rows: &[Row]) {
    let mut silent: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows
        .iter()
        .filter(|r| r.semantics == "none" && r.gaps.is_empty())
    {
        for dialect in &row.dialects {
            *silent.entry(dialect.as_str()).or_default() += 1;
        }
    }
    let _ = writeln!(out, "\n## Rows with nothing to say on the value axis\n");
    let _ = writeln!(
        out,
        "Commands and subcommands that declare no semantics, write no variable, and declare no purity, per dialect.\n"
    );
    let _ = writeln!(out, "| Dialect | Rows |");
    let _ = writeln!(out, "|---|---|");
    for (dialect, count) in &silent {
        let _ = writeln!(out, "| {dialect} | {count} |");
    }
}

fn render_waivers(out: &mut String, lint: &Lint) {
    let _ = writeln!(
        out,
        "\n## Hand-written command knowledge outside the registry\n"
    );
    let _ = writeln!(
        out,
        "Every reviewed site the source lint found, with the waiver that names the axis it \
         belongs to. A file-level waiver covers every site in its file.\n"
    );
    let _ = writeln!(out, "| Axis | Site | Waiver | Reason |");
    let _ = writeln!(out, "|---|---|---|---|");
    for waived in &lint.waived {
        let _ = writeln!(
            out,
            "| {} | `{}:{}` | {} | {} |",
            waived.axis,
            waived.site.path,
            waived.site.line,
            if waived.file_level { "file" } else { "site" },
            waived.reason.replace('|', "\\|"),
        );
    }
}

fn render_ratchet(out: &mut String, lint: &Lint) {
    let _ = writeln!(out, "\n## The ratchet\n");
    let clean: Vec<String> = CLEAN_FILES.iter().map(|file| format!("`{file}`")).collect();
    let _ = writeln!(
        out,
        "The files the gate holds clean, with every site waived or gone: {}.\n\nEvery other \
         scanned file with an unwaived recogniser-shaped site, and its count, which is the pin in \
         `rust/xtask/src/value_transfers.rs`. The count may only fall: a slice lowers the pin \
         beside the review that removes or waives the file's sites, and the ledger in \
         `docs/design/compiler/value-transfers-migration.md` names that slice or axis migration.\n",
        clean.join(", ")
    );
    let _ = writeln!(out, "| File | Unwaived sites |");
    let _ = writeln!(out, "|---|---|");
    for (path, sites) in &lint.ratchet_hits {
        let _ = writeln!(out, "| `{path}` | {} |", sites.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_the_comparison_shapes() {
        for bad in [
            "if command == \"unset\" {",
            "if cmd != \"expr\" && cmd != \"::expr\" {",
            "if lifted.command != \"expr\" {",
            "if \"foreach\" == head {",
            "} if matches!(command.as_str(), \"foreach\" | \"lmap\")",
            "if sub == \"length\"",
        ] {
            assert_eq!(scan(bad).len(), 1, "{bad}");
        }
    }

    #[test]
    fn flags_a_match_with_literal_arms() {
        let src = "match command.as_str() {\n    \"set\" if values.len() == 1 => 1,\n    \"append\" | \"lappend\" => 2,\n    _ => 3,\n}\n";
        assert_eq!(scan(src).len(), 1);
    }

    #[test]
    fn ignores_non_head_bindings_and_comments() {
        for ok in [
            "if name == \"x\" {",
            "// command == \"unset\"",
            "let s = \"cmd == \\\"x\\\"\";",
            "match kind { Kind::A => 1, _ => 2 }",
            "if command == other {",
        ] {
            assert!(scan(ok).is_empty(), "{ok}");
        }
    }

    #[test]
    fn a_test_module_ends_the_scan() {
        let src = "fn f(command: &str) -> bool { command == \"a\" }\n#[cfg(test)]\nmod tests { fn g(command: &str) -> bool { command == \"b\" } }\n";
        assert_eq!(scan(src).len(), 1);
    }

    #[test]
    fn a_test_only_item_is_skipped_alone() {
        let src = "#[cfg(test)]\npub(crate) fn helper(command: &str) -> bool {\n    let _ = \"}\";\n    command == \"a\"\n}\n#[cfg(test)]\nuse std::fmt;\n#[cfg(test)]\nmod support;\nfn f(command: &str) -> bool { command == \"b\" }\n#[cfg(test)]\nmod tests {\n    fn g(command: &str) -> bool { command == \"c\" }\n}\n";
        let hits = scan(src);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].0, 10);
    }

    #[test]
    fn flags_a_tuple_match_arm_naming_a_literal() {
        let src = "match (command.as_str(), canonical) {\n    (\"set\", _) | (_, \"::set\")\n        if args.len() == 2 =>\n    {\n    }\n    (\"array\", _) | (_, \"::array\") if ok => {}\n    _ => {}\n}\nmatch (kind, other) {\n    (\"x\", _) => {}\n    _ => {}\n}\n";
        let hits: Vec<usize> = scan(src).into_iter().map(|(line, _)| line).collect();
        assert_eq!(hits, [2, 6]);
    }

    #[test]
    fn site_waivers_are_found_on_the_line_and_in_the_block_above() {
        let src = "// value-transfer-ok: options — pending value_word_count\nif cmd == \"switch\" {}\nif head == \"x\" {} // value-transfer-ok: irreducible — grammar\nif sub == \"y\" {}\n";
        let hits = scan(src);
        assert_eq!(hits.len(), 3);
        assert_eq!(
            site_waiver(src, hits[0].0),
            Some(("options".to_owned(), "pending value_word_count".to_owned()))
        );
        assert_eq!(
            site_waiver(src, hits[1].0),
            Some(("irreducible".to_owned(), "grammar".to_owned()))
        );
        assert_eq!(site_waiver(src, hits[2].0), None);
    }

    #[test]
    fn an_evaluator_id_arm_is_waived_by_its_enclosing_match() {
        let src = "fn t(id: NativeEvalId) {\n    // value-transfer-ok: dataflow — the transitional table\n    match id {\n        NativeEvalId::ListOfArgs => {}\n        NativeEvalId::ListLength => {}\n    }\n}\n";
        let hits = scan(src);
        assert_eq!(hits.len(), 2);
        for (line, _) in hits {
            assert!(site_waiver(src, line).is_some(), "line {line}");
        }
    }

    #[test]
    fn a_file_waiver_names_its_axis() {
        let src = "//! docs\n// value-transfer-ok(file): options — every site awaits value_word_count\nfn f() {}\n";
        assert_eq!(
            file_waiver(src),
            Some((
                "options".to_owned(),
                "every site awaits value_word_count".to_owned()
            ))
        );
        assert_eq!(site_waiver(src, 3), None);
    }

    fn lint_with(hits: &[(&str, usize)]) -> Lint {
        let mut lint = Lint::default();
        for (path, count) in hits {
            let sites = (1..=*count)
                .map(|line| Site {
                    path: (*path).to_owned(),
                    line,
                    snippet: String::new(),
                })
                .collect();
            lint.ratchet_hits.insert((*path).to_owned(), sites);
        }
        lint
    }

    #[test]
    fn a_count_above_its_pin_fails_and_names_the_sites() {
        let lint = lint_with(&[("rust/x/a.rs", 3)]);
        let verdicts = ratchet_verdicts(&lint, &[("rust/x/a.rs", 2)], &[]);
        assert_eq!(verdicts.len(), 1, "{verdicts:?}");
        assert!(verdicts[0].contains("3 unwaived site(s) against a pin of 2"));
        assert_eq!(verdicts[0].matches("rust/x/a.rs:").count(), 3);
    }

    #[test]
    fn a_pin_above_its_count_is_stale() {
        let lint = lint_with(&[("rust/x/a.rs", 1)]);
        let pins = [("rust/x/a.rs", 2), ("rust/x/gone.rs", 1)];
        let verdicts = ratchet_verdicts(&lint, &pins, &[]);
        assert_eq!(verdicts.len(), 2, "{verdicts:?}");
        assert!(verdicts.iter().all(|v| v.contains("lower the pin")));
        assert!(ratchet_verdicts(&lint, &[("rust/x/a.rs", 1)], &[]).is_empty());
    }

    #[test]
    fn an_unpinned_file_gains_no_site_and_a_clean_file_carries_no_pin() {
        let lint = lint_with(&[("rust/x/new.rs", 1)]);
        let verdicts = ratchet_verdicts(&lint, &[("rust/x/clean.rs", 0)], &["rust/x/clean.rs"]);
        assert!(
            verdicts
                .iter()
                .any(|v| v.contains("`rust/x/new.rs` has 1 unwaived site(s) against a pin of 0")),
            "{verdicts:?}"
        );
        assert!(
            verdicts.iter().any(|v| v.contains("held clean")),
            "{verdicts:?}"
        );
    }

    #[test]
    fn the_ledger_carries_every_pin_at_its_count() {
        let doc = "| File | Sites | Reviewed in |\n|---|---|---|\n| `rust/x/a.rs` | 2 | slice 2 |\n| `rust/x/b.rs` | 5 | slice 5 |\n| `rust/x/c.rs` | 1 | slice 8 |\n| `lset` | 2 | not a path |\n| `rust/x/analyser/`, `lowering/` | 28 | a tier row |\n";
        let verdicts = ledger_verdicts(
            doc,
            &[("rust/x/a.rs", 2), ("rust/x/b.rs", 4), ("rust/x/d.rs", 1)],
        );
        assert_eq!(verdicts.len(), 3, "{verdicts:?}");
        assert!(verdicts.iter().any(|v| v.contains("`rust/x/b.rs` at 5")));
        assert!(
            verdicts
                .iter()
                .any(|v| v.contains("no ratchet row for `rust/x/d.rs`"))
        );
        assert!(
            verdicts
                .iter()
                .any(|v| v.contains("`rust/x/c.rs`, which RATCHET does not pin"))
        );
        let agreed = [("rust/x/a.rs", 2), ("rust/x/b.rs", 5), ("rust/x/c.rs", 1)];
        assert!(ledger_verdicts(doc, &agreed).is_empty());
    }

    #[test]
    fn every_file_is_clean_or_at_its_pin() {
        let root = repo_root();
        let lint = lint(&root);
        assert!(
            lint.clean_hits.is_empty(),
            "unwaived sites in a clean file: {:?}",
            lint.clean_hits
        );
        assert!(lint.problems.is_empty(), "{:?}", lint.problems);
        let ledger = fs::read_to_string(root.join(LEDGER_PATH)).expect("the ledger page");
        let verdicts: Vec<String> = ratchet_verdicts(&lint, RATCHET, CLEAN_FILES)
            .into_iter()
            .chain(ledger_verdicts(&ledger, RATCHET))
            .collect();
        assert!(verdicts.is_empty(), "{verdicts:#?}");
    }
}
