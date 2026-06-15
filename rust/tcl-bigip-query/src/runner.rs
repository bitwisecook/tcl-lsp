//! Query runner — port of the per-file (non-merge) path of
//! `dialects/f5/query/runner.py` (`run_query`, `QueryOptions`,
//! `QueryResult`).
//!
//! This is the pure, I/O-free orchestration layer the `f5 query` verb calls:
//! given a list of `(uri, source_text)` BIG-IP configs plus a query
//! expression and options, it parses the expression once, evaluates it
//! against each source in order, and returns a [`QueryResult`].
//!
//! Mutating assignments are supported — field-value writes
//! (`=` / `|=` / `+=` / `-=`) and identity-field rewrites / `rename*`
//! (token-bounded source rewrites). Each statement's queued
//! [`EditOp`](crate::edit_plan::EditOp)s / `PrefixRewrite`s are applied to the
//! running source after the statement evaluates, and the rewritten text plus
//! [`RenameReport`](crate::rewrite::RenameReport)s land on
//! [`QueryResult::edits_per_file`] (`has_mutation` flags whether any edit was
//! queued); `--merge` is not yet ported.

use std::collections::HashMap;

use tcl_bigip::parser::parse_bigip_conf;

use crate::edit_plan::{AppliedSource, apply};
use crate::errors::QueryError;
use crate::eval::{EvalContext, Root, evaluate_statement};
use crate::inputs::{InputSpec, parse_input};
use crate::value::Value;

/// Explicit, ambient-free configuration for a query run.
///
/// Port of the read-relevant subset of `runner.QueryOptions`. Mutation and
/// merge are out of scope for the read-only runner; `partitions` and `names`
/// are kept because they affect parsing and `$name` resolution. Network
/// probes (`--enable-probes` / `--ca-bundle` / the UCS reader) thread through
/// to the [`EvalContext`].
//
// `Debug` is hand-written because `ucs_cert_reader` is an `Rc<dyn Fn>`, which
// is not `Debug`; `Default` / `Clone` derive cleanly (`Option`/`Rc`).
#[derive(Clone, Default)]
pub struct QueryOptions {
    /// Explicit `$name -> uri` bindings (`--name N=PATH`). When empty, names
    /// are auto-derived from each URI's filename stem.
    pub names: HashMap<String, String>,
    /// Per-URI BIG-IP partition (`--partition PATH=PARTITION`). Defaults to
    /// `Common` when a URI is absent.
    pub partitions: HashMap<String, String>,
    /// Structured side-inputs (`--input-{json,jsonl,csv,f5log}` / `--input
    /// KIND`). Each entry binds `$name` to a JSON-backed [`Root`] parsed from
    /// `source` per its [`InputSpec`]. The `uri` is the side-input's file
    /// URI; it participates in the multi-file source count (so a single
    /// config + one side input renders with a banner, matching Python) but
    /// never iterates as the primary `.` input.
    pub side_inputs: Vec<SideInput>,
    /// Opt the query in to live network probes (`--enable-probes`). When
    /// `false`, every probe builtin raises the gating error. Threaded onto
    /// [`EvalContext::probes_enabled`].
    pub enable_probes: bool,
    /// CA bundle path for TLS-aware probes (`--ca-bundle`). Threaded onto
    /// [`EvalContext::ca_bundle`].
    pub ca_bundle: Option<String>,
    /// `ucs_cert` reader hook (the CLI injects a UCS-aware reader). Threaded
    /// onto [`EvalContext::ucs_cert_reader`].
    pub ucs_cert_reader: Option<crate::eval::UcsCertReader>,
}

impl std::fmt::Debug for QueryOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryOptions")
            .field("names", &self.names)
            .field("partitions", &self.partitions)
            .field("side_inputs", &self.side_inputs)
            .field("enable_probes", &self.enable_probes)
            .field("ca_bundle", &self.ca_bundle)
            .field("ucs_cert_reader", &self.ucs_cert_reader.is_some())
            .finish()
    }
}

/// One bound structured side-input — port of the runner's `input_specs` +
/// `side_resolved_names` pairing.
#[derive(Debug, Clone)]
pub struct SideInput {
    /// The `$NAME` the parsed value binds to.
    pub name: String,
    /// The side-input file's URI.
    pub uri: String,
    /// The raw file text.
    pub source: String,
    /// How to parse `source`.
    pub spec: InputSpec,
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

/// Parse every structured side-input into a JSON-backed [`Root`] keyed by
/// its bound `$NAME` — port of the runner's `_build_structured_root` loop.
///
/// Each side-input's `source` is parsed per its [`InputSpec`] into a
/// [`Value`] and wrapped in [`Root::json`]; parse failures surface with the
/// same `{uri}: invalid {kind} input (...)` wording the Python runner uses.
fn build_side_roots(
    side_inputs: &[SideInput],
) -> Result<HashMap<String, std::rc::Rc<Root>>, QueryError> {
    let mut roots = HashMap::with_capacity(side_inputs.len());
    for si in side_inputs {
        let value = parse_input(&si.source, &si.uri, &si.spec)?;
        roots.insert(si.name.clone(), Root::json(si.uri.clone(), value));
    }
    Ok(roots)
}

/// Build the `$name -> Root` bindings for every loaded source.
///
/// Port of `runner._build_named_roots`: explicit `--name N=PATH` win;
/// remaining sources fall back to filename-stem auto-naming, and a stem
/// collision binds the later source under its full URI instead so the
/// earlier name keeps working.
fn build_named_roots(
    sources: &[(String, String)],
    side_roots: &HashMap<String, std::rc::Rc<Root>>,
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
    // Side-input `$NAME` bindings win over (and never participate in) the
    // BIG-IP auto-naming: they're bound by explicit name only. The CLI has
    // already rejected name collisions between side inputs, so a plain
    // insert is faithful to `full_names = {**resolved_names, **json_names}`.
    for (name, root) in side_roots {
        bindings.insert(name.clone(), std::rc::Rc::clone(root));
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

    // Parse every structured side-input into a JSON-backed root once. These
    // bind to `$NAME` but never iterate as the primary `.` input — mirroring
    // the runner's `_is_json_source` skip of the per-file loop.
    let side_roots = build_side_roots(&opts.side_inputs)?;

    let mut result = QueryResult {
        values_per_file: Vec::with_capacity(sources.len()),
        edits_per_file: Vec::new(),
        has_mutation: false,
    };

    for (uri, source) in sources {
        let mut current_source = source.clone();
        let mut accumulated_values: Vec<Value> = Vec::new();
        let mut accumulated_field_edits = 0usize;
        let mut accumulated_rename_reports: Vec<crate::rewrite::RenameReport> = Vec::new();
        let mut attempted_mutation = false;

        for stmt in &program.statements {
            // Rebuild the root against the post-edit text so a multi-statement
            // `;` chain reads coherent intermediate state.
            let named_roots = build_named_roots(sources, &side_roots, opts);
            let root = build_root(uri, &current_source, opts);
            let mut ctx = EvalContext {
                root,
                named_roots,
                merge_mode: false,
                bindings: HashMap::new(),
                edits: crate::edit_plan::EditPlan::new(),
                probes_enabled: opts.enable_probes,
                ca_bundle: opts.ca_bundle.clone(),
                ucs_cert_reader: opts.ucs_cert_reader.clone(),
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
                    accumulated_rename_reports.extend(self_applied.rename_reports.iter().cloned());
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
                    rename_reports: accumulated_rename_reports,
                    field_edits: accumulated_field_edits,
                },
            ));
        }
    }
    Ok(result)
}
