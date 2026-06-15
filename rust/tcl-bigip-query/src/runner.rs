//! Query runner — port of the per-file (non-merge) path of
//! `dialects/f5/query/runner.py` (`run_query`, `QueryOptions`,
//! `QueryResult`).
//!
//! This is the pure, I/O-free orchestration layer the `f5 query` verb calls:
//! given a list of `(uri, source_text)` BIG-IP configs plus a query
//! expression and options, it parses the expression once, evaluates it
//! against each source in order, and returns a [`QueryResult`].
//!
//! Mutating **field-value** assignments are supported: each statement's queued
//! [`EditOp`](crate::edit_plan::EditOp)s are applied to the running source
//! after the statement evaluates, and the rewritten text lands on
//! [`QueryResult::edits_per_file`] (`has_mutation` flags whether any edit was
//! queued). Identity-field rewrites / `rename*` are surfaced as a clear
//! [`QueryError::Edit`] by the edit-plan engine; `--merge` is not yet ported.

use std::collections::HashMap;

use tcl_bigip::parser::parse_bigip_conf;

use crate::edit_plan::{AppliedSource, apply};
use crate::errors::QueryError;
use crate::eval::{EvalContext, Root, evaluate_statement};
use crate::value::Value;

/// Explicit, ambient-free configuration for a query run.
///
/// Port of the read-relevant subset of `runner.QueryOptions`. Mutation,
/// merge, side-inputs, and network probes are out of scope for the
/// read-only runner; `partitions` and `names` are kept because they affect
/// parsing and `$name` resolution.
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    /// Explicit `$name -> uri` bindings (`--name N=PATH`). When empty, names
    /// are auto-derived from each URI's filename stem.
    pub names: HashMap<String, String>,
    /// Per-URI BIG-IP partition (`--partition PATH=PARTITION`). Defaults to
    /// `Common` when a URI is absent.
    pub partitions: HashMap<String, String>,
}

/// The combined output of a single read-only `run_query` invocation.
///
/// Port of `runner.QueryResult` (read fields only): per-file values in
/// source order, plus a `has_mutation` flag the verb uses to reject mutating
/// queries cleanly.
#[derive(Debug, Default)]
pub struct QueryResult {
    /// `(uri, values)` in source order — port of `values_per_file`, kept as
    /// an ordered `Vec` so the verb renders files in input order.
    pub values_per_file: Vec<(String, Vec<Value>)>,
    /// Per-file `AppliedSource` for mutating queries, in source order — port
    /// of `edits_per_file`. Empty for read-only queries.
    pub edits_per_file: Vec<(String, AppliedSource)>,
    /// Whether the query *attempted* a mutation (queued any edit op). A
    /// read-only query is `false`.
    pub has_mutation: bool,
}

/// Derive a default `$name` from a URI (port of `runner._filename_stem`).
///
/// Strips any directory prefix and the trailing extension; `-` (stdin)
/// becomes `stdin`.
fn filename_stem(uri: &str) -> String {
    if uri == "-" {
        return "stdin".to_owned();
    }
    let base = uri.rsplit('/').next().unwrap_or(uri);
    let stem = match base.rsplit_once('.') {
        Some((head, _)) if !head.is_empty() => head,
        _ => base,
    };
    if stem.is_empty() {
        uri.to_owned()
    } else {
        stem.to_owned()
    }
}

/// Build a BIG-IP root for *uri* / *source* using the per-URI partition.
fn build_root(uri: &str, source: &str, opts: &QueryOptions) -> std::rc::Rc<Root> {
    let partition = opts.partitions.get(uri).map_or("Common", String::as_str);
    let config = parse_bigip_conf(source, partition);
    Root::bigip(uri.to_owned(), source.to_owned(), config)
}

/// Build the `$name -> Root` bindings for every loaded source.
///
/// Port of `runner._build_named_roots`: explicit `--name N=PATH` win;
/// remaining sources fall back to filename-stem auto-naming, and a stem
/// collision binds the later source under its full URI instead so the
/// earlier name keeps working.
fn build_named_roots(
    sources: &[(String, String)],
    opts: &QueryOptions,
) -> HashMap<String, std::rc::Rc<Root>> {
    let mut bindings: HashMap<String, std::rc::Rc<Root>> = HashMap::new();
    let mut used: std::collections::HashSet<&str> = std::collections::HashSet::new();

    if !opts.names.is_empty() {
        for (nm, uri) in &opts.names {
            if let Some((u, src)) = sources.iter().find(|(u, _)| u == uri) {
                bindings.insert(nm.clone(), build_root(u, src, opts));
                used.insert(u.as_str());
            }
        }
    }
    for (uri, src) in sources {
        if used.contains(uri.as_str()) {
            continue;
        }
        let stem = filename_stem(uri);
        if bindings.contains_key(&stem) {
            // Collision: keep the earlier binding, expose this source by URI.
            bindings.insert(uri.clone(), build_root(uri, src, opts));
            continue;
        }
        bindings.insert(stem, build_root(uri, src, opts));
    }
    bindings
}

/// Parse *query* and run it against each `(uri, source)` in *sources*.
///
/// Port of the per-file (non-merge) path of `runner.run_query`: the
/// expression is parsed once and shared across files; each source takes its
/// turn as the primary input (the `.` of a top-level statement) in source
/// order. Variables (`$name`) bind to every loaded source so cross-file
/// lookups resolve.
///
/// Mutating field-edit assignments queue [`EditOp`](crate::edit_plan::EditOp)s
/// on the per-statement context; after each statement the queued edits are
/// applied to the running source text, so a `;`-separated chain sees the
/// post-edit source on the next statement. The rewritten text lands on
/// [`QueryResult::edits_per_file`].
///
/// # Errors
///
/// Returns the [`QueryError`] from [`parse_query`](crate::parse_query) when
/// the expression fails to parse, any evaluation error raised against a
/// source, or any edit-apply error (identity-field write, overlapping edits,
/// non-writable compound value).
pub fn run_query(
    query: &str,
    sources: &[(String, String)],
    opts: &QueryOptions,
) -> Result<QueryResult, QueryError> {
    let program = crate::parse_query(query)?;

    let mut result = QueryResult {
        values_per_file: Vec::with_capacity(sources.len()),
        edits_per_file: Vec::new(),
        has_mutation: false,
    };

    for (uri, source) in sources {
        let mut current_source = source.clone();
        let mut accumulated_values: Vec<Value> = Vec::new();
        let mut accumulated_field_edits = 0usize;
        let mut attempted_mutation = false;

        for stmt in &program.statements {
            // Rebuild the root against the post-edit text so a multi-statement
            // `;` chain reads coherent intermediate state.
            let named_roots = build_named_roots(sources, opts);
            let root = build_root(uri, &current_source, opts);
            let mut ctx = EvalContext {
                root,
                named_roots,
                merge_mode: false,
                bindings: HashMap::new(),
                edits: crate::edit_plan::EditPlan::new(),
            };
            accumulated_values.extend(evaluate_statement(stmt, &mut ctx)?);

            if ctx.edits.has_edits() {
                attempted_mutation = true;
                // Edits target the iterating source (cross-file `$other.x`
                // edits are a deferred edge case); apply against the running
                // text for this URI.
                let mut sources_now: HashMap<String, String> = HashMap::new();
                sources_now.insert(uri.clone(), current_source.clone());
                let applied = apply(&ctx.edits, &sources_now)?;
                if let Some(self_applied) = applied.get(uri) {
                    current_source.clone_from(&self_applied.new_source);
                    accumulated_field_edits += self_applied.field_edits;
                }
            }
        }

        result
            .values_per_file
            .push((uri.clone(), accumulated_values));
        if attempted_mutation {
            result.has_mutation = true;
            result.edits_per_file.push((
                uri.clone(),
                AppliedSource {
                    uri: uri.clone(),
                    original: source.clone(),
                    new_source: current_source,
                    field_edits: accumulated_field_edits,
                },
            ));
        }
    }
    Ok(result)
}
