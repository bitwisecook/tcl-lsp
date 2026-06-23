//! Refcount-contract lint.
//!
//! Walks every `pub export fn` in `runtime/zig/` and warns when an export
//! is missing a row in the refcount contract doc
//! (`docs/design/runtime/refcount-contract.md`). Output and exit codes
//! are stable and canonical:
//!
//! - exit 0: every export has a contract row, **or** there are gaps but
//!   `--strict` was not passed (warnings only);
//! - exit 1: gaps exist and `--strict` was passed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use regex::Regex;

use crate::util::repo_root;

/// Run the lint. `strict` escalates any gap to a non-zero exit.
pub fn run(strict: bool) -> Result<ExitCode> {
    let root = repo_root();
    let runtime = root.join("runtime/zig");
    let contract = root.join("docs/design/runtime/refcount-contract.md");

    let exports = collect_exports(&root, &runtime)?;
    let rows = collect_rows(&contract)?;

    let export_names: BTreeSet<&String> = exports.keys().collect();
    // `BTreeSet` iteration is sorted, so `missing` / `extra` come out in
    // sorted order.
    let missing: Vec<&String> = export_names
        .iter()
        .filter(|name| !rows.contains(**name))
        .copied()
        .collect();
    let extra: Vec<&String> = rows
        .iter()
        .filter(|name| !export_names.contains(name))
        .collect();

    let contract_rel = contract
        .strip_prefix(&root)
        .unwrap_or(&contract)
        .to_string_lossy()
        .replace('\\', "/");

    if !missing.is_empty() {
        eprintln!(
            "WARNING: {} runtime exports missing from {contract_rel}:",
            missing.len()
        );
        for name in &missing {
            let files = exports[*name].join(", ");
            eprintln!("  - {name}  ({files})");
        }
    }

    if !extra.is_empty() {
        eprintln!(
            "WARNING: {} contract rows reference functions not found in \
             runtime/zig/ — stale rows or typos:",
            extra.len()
        );
        for name in &extra {
            eprintln!("  - {name}");
        }
    }

    let has_gaps = !missing.is_empty() || !extra.is_empty();
    if has_gaps && strict {
        return Ok(ExitCode::from(1));
    }

    if has_gaps {
        // Rows that are genuine exports (`rows ∩ exports`).
        let matched = rows.len() - extra.len();
        let pct = 100 * matched / exports.len().max(1);
        eprintln!(
            "\n{} of {} runtime exports have contract rows ({pct}%).",
            rows.len(),
            exports.len()
        );
        return Ok(ExitCode::SUCCESS);
    }

    eprintln!(
        "OK: every runtime export has a contract row ({} exports / {} rows).",
        exports.len(),
        rows.len()
    );
    Ok(ExitCode::SUCCESS)
}

/// Walk `runtime/zig` and return `{export_name: [files containing it]}`,
/// in source order per name (one entry per regex-match occurrence, left to
/// right). Vendored C-bridge sources under `vendor/` are skipped.
fn collect_exports(root: &Path, runtime: &Path) -> Result<BTreeMap<String, Vec<String>>> {
    // `\bpub\s+export\s+fn\s+(name)\s*\(` — the export-declaration form.
    let fn_re = Regex::new(r"\bpub\s+export\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(")
        .expect("static regex is valid");

    let mut files = Vec::new();
    collect_zig_files(runtime, &mut files)?;
    files.retain(|path| {
        !path
            .to_string_lossy()
            .replace('\\', "/")
            .contains("vendor/")
    });
    files.sort();

    let mut exports: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in &files {
        let text =
            std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        let rel = file
            .strip_prefix(root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        for caps in fn_re.captures_iter(&text) {
            exports
                .entry(caps[1].to_owned())
                .or_default()
                .push(rel.clone());
        }
    }
    Ok(exports)
}

/// Recursively collect `*.zig` files under `dir` into `out`.
fn collect_zig_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("reading directory {}", dir.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            collect_zig_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "zig") {
            out.push(path);
        }
    }
    Ok(())
}

/// Parse the contract doc and return every function name that has a table
/// row whose first cell opens with the backtick-quoted function name.
fn collect_rows(contract: &Path) -> Result<BTreeSet<String>> {
    if !contract.exists() {
        return Ok(BTreeSet::new());
    }
    // `^\|\s*`+(name)` — a table row whose first cell opens with the
    // backtick-quoted function name; whatever follows is free-form.
    let row_re = Regex::new(r"^\|\s*`+([A-Za-z_][A-Za-z0-9_]*)").expect("static regex is valid");

    let text = std::fs::read_to_string(contract)
        .with_context(|| format!("reading {}", contract.display()))?;
    let mut rows = BTreeSet::new();
    for line in text.lines() {
        if let Some(caps) = row_re.captures(line) {
            rows.insert(caps[1].to_owned());
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_regex_matches_declaration_forms() {
        let re = Regex::new(r"\bpub\s+export\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(").unwrap();
        let src = "pub export fn tcl_var_get(obj: *Obj) void {\n\
                   pub  export  fn  upvar_resolve_depth (n: u32) void {\n\
                   fn not_exported() void {\n";
        let names: Vec<&str> = re
            .captures_iter(src)
            .map(|c| c.get(1).unwrap().as_str())
            .collect();
        assert_eq!(names, vec!["tcl_var_get", "upvar_resolve_depth"]);
    }

    #[test]
    fn row_regex_matches_single_and_double_backtick_rows() {
        let re = Regex::new(r"^\|\s*`+([A-Za-z_][A-Za-z0-9_]*)").unwrap();
        assert_eq!(
            re.captures("| `dispatch(ctx)` | retains |")
                .unwrap()
                .get(1)
                .unwrap()
                .as_str(),
            "dispatch"
        );
        assert_eq!(
            re.captures("|  ``eval_command(x)`` | borrows |")
                .unwrap()
                .get(1)
                .unwrap()
                .as_str(),
            "eval_command"
        );
        assert!(re.captures("not a table row").is_none());
        // A header separator row must not match.
        assert!(re.captures("| --- | --- |").is_none());
    }
}
