//! The `query` verb — jq-flavoured DSL over BIG-IP configs (read-only).
//!
//! Port of the read-only emit path of `tooling/f5/verbs/query.py`
//! (`_run_query` + `_emit_values` + `_empty_match_exit_code`). Loads each
//! input to `(uri, source)` via the UCS/stdin-aware loader, runs the pure
//! `tcl_bigip_query::run_query` runner, and renders the per-file values with
//! `tcl_bigip_query::output::render`.
//!
//! Field-value mutations (`=` / `|=` / `+=` / `-=`) are supported: the
//! default is a unified-diff preview, `--write` prints the rewritten config,
//! and `--in-place` overwrites the input. Identity-field writes (`.name = …`)
//! and the `rename*` builtins are deferred (cleanly rejected) — they route
//! through the unported rename token-rewrite engine.
//!
//! Deferred (cleanly rejected / ignored): `--merge` cross-file ref-walking,
//! side-inputs, network probes, renderers, and the `--help-*` actions.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use tcl_bigip_io::read_path;
use tcl_bigip_query::value::Value;
use tcl_bigip_query::{QueryOptions, QueryResult, output, run_query};

use super::difflib;

/// The mutually-exclusive output-mode flags, resolved to an `output::render`
/// mode by [`OutputModeFlags::resolve`]. Mirrors the `--scf` / `--raw` /
/// `--paths-only` / `--json` / `--table` / `--table-lineart` group.
//
// Each field mirrors a distinct user-facing CLI flag (one of a mutually
// exclusive group), so a bool-per-flag is the faithful shape — not a state
// machine.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy)]
pub struct OutputModeFlags {
    pub scf: bool,
    pub raw: bool,
    pub paths_only: bool,
    pub json: bool,
    pub table: bool,
    pub table_lineart: bool,
}

impl OutputModeFlags {
    /// Resolve the selected flag to an `output::render` mode (default `auto`).
    #[must_use]
    pub fn resolve(self) -> &'static str {
        if self.scf {
            "scf"
        } else if self.raw {
            "raw"
        } else if self.paths_only {
            "paths"
        } else if self.json {
            "json"
        } else if self.table {
            "table"
        } else if self.table_lineart {
            "table-lineart"
        } else {
            "auto"
        }
    }
}

/// Parse `--name NAME=PATH` bindings into a `name -> path` map, validating the
/// identifier and that the path was supplied as a positional input. Mirrors
/// `query._parse_name_bindings`.
fn parse_name_bindings(
    raw: &[String],
    paths: &[String],
) -> Result<std::collections::HashMap<String, String>, String> {
    let name_re = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_-]*$").expect("valid regex");
    let mut bindings = std::collections::HashMap::new();
    for entry in raw {
        let Some((nm, pth)) = entry.split_once('=') else {
            return Err(format!("--name expects NAME=PATH (got '{entry}')"));
        };
        if !name_re.is_match(nm) {
            return Err(format!(
                "--name '{entry}': '{nm}' is not a valid DSL identifier \
                 (letters, digits, '_', '-'; cannot start with a digit)"
            ));
        }
        if bindings.contains_key(nm) {
            return Err(format!("--name {nm}: duplicate binding"));
        }
        if !paths.iter().any(|p| p == pth) {
            return Err(format!(
                "--name {nm}={pth}: path was not given as a positional \
                 input (so the runner would have no source text for it)"
            ));
        }
        bindings.insert(nm.to_owned(), pth.to_owned());
    }
    Ok(bindings)
}

/// Flatten top-level `Stream`s — port of `output._flat`, needed to build the
/// multi-file JSON envelope's inner value arrays.
fn flat(values: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for v in values {
        if let Value::Stream(items) = v {
            out.extend(items.iter().cloned());
        } else {
            out.push(v.clone());
        }
    }
    out
}

/// The non-output behavioural flags for a query run. `merge` / `write` /
/// `in_place` are deferred (cleanly rejected); `strict` selects the
/// empty-match exit code.
//
// Each field mirrors a distinct user-facing CLI flag, so a bool-per-flag is
// the faithful shape here.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy)]
pub struct QueryFlags {
    pub merge: bool,
    pub write: bool,
    pub in_place: bool,
    pub strict: bool,
}

/// `f5 query` (read-only).
pub fn run_query_verb(
    expression: Option<&str>,
    inputs: &[PathBuf],
    names: &[String],
    mode: &str,
    flags: QueryFlags,
) -> anyhow::Result<u8> {
    // Mirror `_run_query`'s up-front validation order and messages.
    let Some(expression) = expression else {
        eprintln!("error: no query expression supplied (positional or --from-file)");
        return Ok(2);
    };
    if inputs.is_empty() {
        eprintln!("error: no input files (pass '-' to read stdin)");
        return Ok(2);
    }

    if flags.merge {
        eprintln!("error: --merge is not yet supported in the Rust port");
        return Ok(2);
    }
    // `--in-place` requires a real path, not stdin (mirrors `_run_query`).
    if flags.in_place && inputs.iter().any(|p| p.as_os_str() == "-") {
        eprintln!("error: --in-place requires a path, not stdin");
        return Ok(2);
    }

    let path_strs: Vec<String> = inputs
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    let name_map = match parse_name_bindings(names, &path_strs) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    // Load each input to `(uri, source)` via the UCS/stdin-aware reader, in
    // source order. Reject duplicate URIs (mirrors `_run_query`).
    let opts = crate::cli::PassphraseArgs::default().to_options();
    let mut sources: Vec<(String, String)> = Vec::with_capacity(path_strs.len());
    let mut path_for_uri: Vec<(String, String)> = Vec::new();
    for path_str in &path_strs {
        let (uri, src) = match read_path(path_str, false, &opts) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("error: {e}");
                return Ok(2);
            }
        };
        if let Some((_, prev)) = path_for_uri.iter().find(|(u, _)| *u == uri) {
            let label = if path_str == "-" {
                "stdin"
            } else {
                path_str.as_str()
            };
            if prev == path_str {
                eprintln!("error: duplicate input {label}");
            } else {
                eprintln!("error: duplicate input {label} (already read as {prev})");
            }
            return Ok(2);
        }
        path_for_uri.push((uri.clone(), path_str.clone()));
        sources.push((uri, src));
    }

    // Translate `--name N=PATH` (path-string) to URI keys for the runner.
    let mut resolved_names = std::collections::HashMap::new();
    for (nm, path_str) in name_map {
        let uri = path_for_uri
            .iter()
            .find(|(_, p)| *p == path_str)
            .map(|(u, _)| u.clone());
        let Some(uri) = uri else {
            eprintln!(
                "error: --name {nm}={path_str}: path was not loaded \
                 (must also appear as a positional argument)"
            );
            return Ok(2);
        };
        resolved_names.insert(nm, uri);
    }

    let query_opts = QueryOptions {
        names: resolved_names,
        partitions: std::collections::HashMap::new(),
    };

    let result = match run_query(expression, &sources, &query_opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    if result.has_mutation {
        return emit_mutation(&result, &path_for_uri, flags);
    }

    emit_values(&result, sources.len(), mode, flags.strict)
}

/// Emit the result of a mutating query — port of `query._emit_mutation`.
///
/// Default: a unified-diff preview per changed file. `--write`: the rewritten
/// config to stdout. `--in-place`: overwrite each input. A no-op (no file
/// changed) exits 1, or 2 under `--strict`.
fn emit_mutation(
    result: &QueryResult,
    path_for_uri: &[(String, String)],
    flags: QueryFlags,
) -> anyhow::Result<u8> {
    let mut any_changed = false;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for (uri, applied) in &result.edits_per_file {
        // A "mutating" query that produced no actual textual change exits 1
        // (no-op), mirroring `f5 rename`.
        if applied.new_source == applied.original {
            continue;
        }
        any_changed = true;
        let path_str = path_for_uri
            .iter()
            .find(|(u, _)| u == uri)
            .map_or(uri.as_str(), |(_, p)| p.as_str());
        let rewritten = &applied.new_source;
        if flags.in_place && path_str != "-" {
            std::fs::write(Path::new(path_str), rewritten)?;
            continue;
        }
        if flags.write {
            write!(out, "{rewritten}")?;
            continue;
        }
        let from = path_str.to_owned();
        let to = format!("{path_str} (modified)");
        let a = difflib::splitlines_keepends(&applied.original);
        let b = difflib::splitlines_keepends(&applied.new_source);
        let diff = difflib::unified_diff(&a, &b, &from, &to);
        write!(out, "{diff}")?;
    }
    if !any_changed {
        if flags.strict {
            eprintln!(
                "error: --strict: mutating query produced no textual change \
                 (no matches).  Check the path / predicate."
            );
            return Ok(2);
        }
        return Ok(1);
    }
    Ok(0)
}

/// Render the per-file values — port of `_emit_values` (read path).
fn emit_values(
    result: &tcl_bigip_query::QueryResult,
    n_sources: usize,
    mode: &str,
    strict: bool,
) -> anyhow::Result<u8> {
    let multi = n_sources > 1;
    let use_banner = multi && mode != "json";
    let mut any_matched = false;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    // Multi-file `--json` → one top-level envelope of {uri, values}.
    if multi && mode == "json" {
        let envelope: Vec<Value> = result
            .values_per_file
            .iter()
            .map(|(uri, values)| {
                if !values.is_empty() {
                    any_matched = true;
                }
                Value::object([
                    ("uri".to_owned(), Value::Str(uri.clone())),
                    ("values".to_owned(), Value::List(flat(values))),
                ])
            })
            .collect();
        let rendered = tcl_bigip_query::jsonfmt::to_pretty(&Value::List(envelope));
        writeln!(out, "{rendered}")?;
        return Ok(empty_match_exit_code(any_matched, strict));
    }

    for (uri, values) in &result.values_per_file {
        if !values.is_empty() {
            any_matched = true;
        }
        if use_banner {
            writeln!(out, "# === {uri} ===")?;
        }
        match output::render(values, mode) {
            Ok(text) => write!(out, "{text}")?,
            Err(e) => {
                eprintln!("error: {e}");
                return Ok(2);
            }
        }
    }
    Ok(empty_match_exit_code(any_matched, strict))
}

/// Decide the read-only exit code — port of `_empty_match_exit_code`.
///
/// jq-style: `0` whether or not the query matched; `--strict` opts into
/// `exit 1 when nothing matched`.
fn empty_match_exit_code(any_matched: bool, strict: bool) -> u8 {
    u8::from(!any_matched && strict)
}
