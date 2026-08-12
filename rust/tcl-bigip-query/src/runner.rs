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

//! Query runner (`run_query`, `QueryOptions`, `QueryResult`), covering both
//! the per-file (non-merge) path and `--merge` (every loaded config as one
//! logical namespace).
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
//! queued). In `--merge` mode the program runs once over every source
//! (concatenating each root's projection), graph builtins walk references
//! across files, and colliding `(kind, full-path)` identities are refused.

use std::collections::HashMap;
use std::rc::Rc;

use tcl_bigip::parser::parse_bigip_conf;

use crate::ast::Expr;
use crate::edit_plan::{AppliedSource, apply};
use crate::errors::QueryError;
use crate::eval::{EvalContext, Root, evaluate_statement};
use crate::inputs::{InputSpec, parse_input};
use crate::value::Value;

/// Explicit, ambient-free configuration for a query run.
///
/// Mutation and merge are out of scope for the read-only runner;
/// `partitions` and `names`
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
    /// config + one side input renders with a banner) but
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
    /// `files` / `file` / `glob` / `grep` reader hook (the CLI injects a
    /// UCS-aware reader). Threaded onto [`EvalContext::files_reader`].
    pub files_reader: Option<crate::eval::FilesReader>,
    /// Treat every loaded config as one logical namespace (`--merge`). When
    /// `true`, the query runs once over every source (concatenating each
    /// root's projection), `refs` / `referenced_by` walk references across
    /// files, and two sources defining the same `(kind, full-path)` are
    /// refused. When `false` (the default) the query runs once per source.
    pub merge: bool,
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
            .field("files_reader", &self.files_reader.is_some())
            .field("merge", &self.merge)
            .finish()
    }
}

/// One bound structured side-input, pairing an input spec with its resolved
/// name.
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
    /// `(uri, values)` in source order, kept as an ordered `Vec` so the verb
    /// renders files in input order.
    pub values_per_file: Vec<(String, Vec<Value>)>,
    /// Per-file `AppliedSource` for mutating queries, in source order. Empty
    /// for read-only queries.
    pub edits_per_file: Vec<(String, AppliedSource)>,
    /// Whether the query *attempted* a mutation (queued any edit op). A
    /// read-only query is `false`.
    pub has_mutation: bool,
}

/// Derive a default `$name` from a URI.
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
/// its bound `$NAME`.
///
/// Each side-input's `source` is parsed per its [`InputSpec`] into a
/// [`Value`] and wrapped in [`Root::json`]; parse failures surface with the
/// same `{uri}: invalid {kind} input (...)` wording the runner uses.
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
    // insert is correct — the side-input name simply wins.
    for (name, root) in side_roots {
        bindings.insert(name.clone(), std::rc::Rc::clone(root));
    }
    bindings
}

/// Parse *query* and run it against each `(uri, source)` in *sources*.
///
/// The per-file (non-merge) path: the expression is parsed once and
/// shared across files; each source takes its
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
    // bind to `$NAME` but never iterate as the primary `.` input — they are
    // skipped by the per-file loop.
    let side_roots = build_side_roots(&opts.side_inputs)?;

    if opts.merge {
        return run_query_merged(&program, sources, &side_roots, opts);
    }

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
                merge_roots: Vec::new(),
                bindings: HashMap::new(),
                edits: crate::edit_plan::EditPlan::new(),
                probes_enabled: opts.enable_probes,
                ca_bundle: opts.ca_bundle.clone(),
                ucs_cert_reader: opts.ucs_cert_reader.clone(),
                files_reader: opts.files_reader.clone(),
                merge_graph: std::cell::RefCell::new(None),
            };
            accumulated_values.extend(evaluate_statement(stmt, &mut ctx)?);

            if ctx.edits.has_edits() {
                attempted_mutation = true;
                // Edits may target the iterating source or any named source the
                // query reached via `$name`. Apply across every loaded source
                // (with the iterating URI overridden by its post-edit text) —
                // `apply` partitions by URI. Cross-file edits land directly on
                // `result.edits_per_file`; the iterating source's own edit
                // folds into `current_source` and is written at end-of-URI.
                let mut sources_now: HashMap<String, String> = sources.iter().cloned().collect();
                sources_now.insert(uri.clone(), current_source.clone());
                let applied = apply(&ctx.edits, &sources_now)?;
                if let Some(self_applied) = applied.get(uri) {
                    current_source.clone_from(&self_applied.new_source);
                    accumulated_field_edits += self_applied.field_edits;
                    accumulated_rename_reports.extend(self_applied.rename_reports.iter().cloned());
                }
                // Cross-file edits (`$other.x.y = …` from inside this
                // per-file iteration) target a different source — merge them
                // into `result.edits_per_file`, in source order for stable
                // output, accumulating onto any prior entry.
                for (other_uri, other_src) in sources {
                    if other_uri == uri {
                        continue;
                    }
                    let Some(other_applied) = applied.get(other_uri) else {
                        continue;
                    };
                    merge_cross_file_edit(&mut result, other_uri, other_src, other_applied);
                }
            }
        }

        result
            .values_per_file
            .push((uri.clone(), accumulated_values));
        if attempted_mutation {
            result.has_mutation = true;
            // Fold this source's accumulated self-edit onto any entry a prior
            // iteration's cross-file edit already created for this URI.
            let (base_renames, base_edits) = take_existing_edit(&result, uri);
            let mut rename_reports = base_renames;
            rename_reports.extend(accumulated_rename_reports);
            upsert_edit(
                &mut result,
                uri,
                AppliedSource {
                    uri: uri.clone(),
                    original: source.clone(),
                    new_source: current_source,
                    rename_reports,
                    field_edits: base_edits + accumulated_field_edits,
                },
            );
        }
    }
    Ok(result)
}

/// Insert or replace the [`AppliedSource`] for *uri* in
/// `result.edits_per_file`, preserving first-insertion position — the
/// `Vec`-backed equivalent of `edits_per_file[uri] = …` dict write.
fn upsert_edit(result: &mut QueryResult, uri: &str, applied: AppliedSource) {
    if let Some(entry) = result.edits_per_file.iter_mut().find(|(u, _)| u == uri) {
        entry.1 = applied;
    } else {
        result.edits_per_file.push((uri.to_owned(), applied));
    }
}

/// Read (without removing) the accumulated rename reports + field-edit count
/// already recorded for *uri*, or `(empty, 0)`.
fn take_existing_edit(
    result: &QueryResult,
    uri: &str,
) -> (Vec<crate::rewrite::RenameReport>, usize) {
    result
        .edits_per_file
        .iter()
        .find(|(u, _)| u == uri)
        .map_or_else(
            || (Vec::new(), 0),
            |(_, a)| (a.rename_reports.clone(), a.field_edits),
        )
}

/// Merge a cross-file [`AppliedSource`] (an edit that originated while
/// iterating a *different* source) into `result.edits_per_file`, accumulating
/// onto any prior entry.
fn merge_cross_file_edit(
    result: &mut QueryResult,
    other_uri: &str,
    other_original: &str,
    other_applied: &AppliedSource,
) {
    let existing = result.edits_per_file.iter().find(|(u, _)| u == other_uri);
    let base_original = existing.map_or(other_original.to_owned(), |(_, a)| a.original.clone());
    let (mut rename_reports, base_edits) = existing.map_or_else(
        || (Vec::new(), 0),
        |(_, a)| (a.rename_reports.clone(), a.field_edits),
    );
    rename_reports.extend(other_applied.rename_reports.iter().cloned());
    upsert_edit(
        result,
        other_uri,
        AppliedSource {
            uri: other_uri.to_owned(),
            original: base_original,
            new_source: other_applied.new_source.clone(),
            rename_reports,
            field_edits: base_edits + other_applied.field_edits,
        },
    );
}

// --merge: every loaded config as one logical namespace

/// The `BigipConfig` field-declaration index for each collision table.
///
/// Collision detection iterates the fields in declaration order (then
/// source order within a field), so the collision list — and the `head[:5]`
/// slice the error message renders — depends on this ordering.
/// `generic_objects` (index 529) sorts after every typed table, matching the
/// "typed line then generic-objects line" output.
fn collision_field_index(table_name: &str) -> u32 {
    match table_name {
        "data_groups" => 0,
        "pools" => 1,
        "virtual_servers" => 2,
        "virtual_addresses" => 3,
        "ltm_dns_cache_records" => 24,
        "nodes" => 229,
        "profiles" => 230,
        "monitors" => 231,
        "snat_pools" => 232,
        "persistence" => 233,
        "rules" => 234,
        "policies" => 235,
        "gtm_monitors" => 440,
        "gtm_topologies" => 445,
        "generic_objects" => 529,
        // An unmodelled table name sorts after the modelled tables but before
        // generic_objects; an unknown name is a bug, but ordering it just
        // before generic_objects is the least surprising fallback.
        _ => 528,
    }
}

/// Hard-error if two roots define the same `(kind, full-path)`.
///
/// Merge mode walks every config as one namespace; an identical object
/// identity across files would yield ambiguous lookups and unsafe edits, so
/// the runner refuses rather than silently merging. The error text (and its
/// `head[:5]` / `... and N more` truncation) is reproduced byte-for-byte.
fn detect_collisions(roots: &[Rc<crate::eval::Root>]) -> Result<(), QueryError> {
    // `(field_index, label, full_path)` for every collision candidate, in
    // iteration order: roots in source order, then table fields by
    // ascending field index, then dict / source order within each field.
    let mut seen: HashMap<(String, String), String> = HashMap::new();
    // (label, full_path, prior_uri, uri)
    let mut collisions: Vec<(String, String, String, String)> = Vec::new();

    for root in roots {
        // Build (field_index, table_name, full_path) rows for this root, then
        // stable-sort by field_index so typed tables (low index) precede
        // generic_objects (529) while preserving source order within a field.
        let mut rows: Vec<(u32, &str, &str)> = Vec::new();
        for placed in &root.config.objects {
            rows.push((
                collision_field_index(placed.table_name),
                placed.table_name,
                placed.full_path.as_str(),
            ));
        }
        for (key, _obj) in &root.config.generic_objects {
            rows.push((
                collision_field_index("generic_objects"),
                "generic_objects",
                key.as_str(),
            ));
        }
        rows.sort_by_key(|(idx, _, _)| *idx);

        for (_idx, table_name, full_path) in rows {
            let key = (table_name.to_owned(), full_path.to_owned());
            if let Some(prior_uri) = seen.get(&key) {
                collisions.push((
                    table_name.to_owned(),
                    full_path.to_owned(),
                    prior_uri.clone(),
                    root.uri.clone(),
                ));
            } else {
                seen.insert(key, root.uri.clone());
            }
        }
    }

    if collisions.is_empty() {
        return Ok(());
    }
    let head = &collisions[..collisions.len().min(5)];
    let extra = collisions.len() - head.len();
    let mut lines: Vec<String> = head
        .iter()
        .map(|(attr, full_path, prior_uri, uri)| {
            format!(
                "  {full_path} ({}) in {prior_uri} and {uri}",
                attr.replace('_', " ")
            )
        })
        .collect();
    if extra > 0 {
        lines.push(format!("  ... and {extra} more"));
    }
    Err(QueryError::eval(format!(
        "--merge: refusing to merge configs that define the same object in \
         more than one source:\n{}",
        lines.join("\n")
    )))
}

/// Run *program* in `--merge` mode.
///
/// Builds every root once, refuses colliding object identities, then runs the
/// program a single time: each statement is evaluated once per root with
/// `merge_mode = true` and every root bound as an active root, and the
/// per-root values are concatenated. `.ltm.virtual[]` thus enumerates virtuals
/// from every source; `refs` / `referenced_by` walk the merged graph; edits
/// route back to their originating source via [`apply`]'s URI partitioning.
/// All values land under the first source's URI (the synthetic merged bucket),
/// matching `result.values_per_file[primary_uri]`.
fn run_query_merged(
    program: &crate::ast::Program,
    sources: &[(String, String)],
    side_roots: &HashMap<String, Rc<crate::eval::Root>>,
    opts: &QueryOptions,
) -> Result<QueryResult, QueryError> {
    let mut result = QueryResult::default();
    if sources.is_empty() {
        return Ok(result);
    }

    let primary_uri = sources[0].0.clone();
    // `current_sources` tracks post-edit text per URI as multi-statement
    // queries chain. Kept as a parallel vec in source order.
    let mut current_sources: Vec<(String, String)> = sources.to_vec();
    let mut accumulated_values: Vec<Value> = Vec::new();
    let mut accumulated_renames_per_uri: HashMap<String, Vec<crate::rewrite::RenameReport>> =
        HashMap::new();
    let mut accumulated_field_edits_per_uri: HashMap<String, usize> = HashMap::new();
    let mut attempted_mutation = false;

    // Up-front collision check against the initial parse (build the
    // roots once and run `_detect_collisions` before the statement loop).
    let initial_roots: Vec<Rc<crate::eval::Root>> = current_sources
        .iter()
        .map(|(uri, src)| build_root(uri, src, opts))
        .collect();
    detect_collisions(&initial_roots)?;

    for stmt in &program.statements {
        // Rebuild roots against current_sources so each statement sees the
        // post-edit text of every preceding statement.
        let step_roots: Vec<Rc<crate::eval::Root>> = current_sources
            .iter()
            .map(|(uri, src)| build_root(uri, src, opts))
            .collect();
        let named_roots = build_named_roots(&current_sources, side_roots, opts);

        // Evaluate the statement once per root and concatenate, with
        // `merge_mode = true` so graph builtins cross files. A shared
        // `EditPlan` collects every root's queued edits.
        let plan = eval_merge_statement(
            stmt,
            &step_roots,
            &named_roots,
            opts,
            &mut accumulated_values,
        )?;

        if plan.has_edits() {
            attempted_mutation = true;
            let sources_now: HashMap<String, String> = current_sources.iter().cloned().collect();
            let applied_per = apply(&plan, &sources_now)?;
            for (uri, applied) in &applied_per {
                if let Some(entry) = current_sources.iter_mut().find(|(u, _)| u == uri) {
                    entry.1.clone_from(&applied.new_source);
                }
                accumulated_renames_per_uri
                    .entry(uri.clone())
                    .or_default()
                    .extend(applied.rename_reports.iter().cloned());
                *accumulated_field_edits_per_uri
                    .entry(uri.clone())
                    .or_default() += applied.field_edits;
            }
        }
    }

    // All values flow into a single bucket keyed by the first source's URI.
    result
        .values_per_file
        .push((primary_uri, accumulated_values));
    if attempted_mutation {
        result.has_mutation = true;
        emit_merge_edits(
            &mut result,
            sources,
            &current_sources,
            &accumulated_renames_per_uri,
            &accumulated_field_edits_per_uri,
        );
    }
    Ok(result)
}

/// Evaluate one merge statement once per root (`merge_mode = true`, every root
/// bound active), pushing each root's values onto `accumulated_values` and
/// returning the combined [`EditPlan`] of every root's queued edits.
fn eval_merge_statement(
    stmt: &Expr,
    step_roots: &[Rc<crate::eval::Root>],
    named_roots: &HashMap<String, Rc<crate::eval::Root>>,
    opts: &QueryOptions,
    accumulated_values: &mut Vec<Value>,
) -> Result<crate::edit_plan::EditPlan, QueryError> {
    let mut plan = crate::edit_plan::EditPlan::new();
    for root in step_roots {
        let mut named_for_step = named_roots.clone();
        // Rebuild the named-root entry for this source against the step root
        // so `$self.x` reads post-edit state within a multi-statement query.
        for r in named_for_step.values_mut() {
            if r.uri == root.uri {
                *r = Rc::clone(root);
            }
        }
        let mut ctx = EvalContext {
            root: Rc::clone(root),
            named_roots: named_for_step,
            merge_mode: true,
            merge_roots: step_roots.to_vec(),
            bindings: HashMap::new(),
            edits: crate::edit_plan::EditPlan::new(),
            probes_enabled: opts.enable_probes,
            ca_bundle: opts.ca_bundle.clone(),
            ucs_cert_reader: opts.ucs_cert_reader.clone(),
            files_reader: opts.files_reader.clone(),
            merge_graph: std::cell::RefCell::new(None),
        };
        accumulated_values.extend(evaluate_statement(stmt, &mut ctx)?);
        if ctx.edits.has_edits() {
            plan.ops.extend(ctx.edits.ops);
            plan.prefix_rewrites.extend(ctx.edits.prefix_rewrites);
        }
    }
    Ok(plan)
}

/// Push the per-source [`AppliedSource`]s onto *result*, in source order,
/// skipping sources that ended unchanged — the merge-mode tail of
/// `_run_merge_statements`.
fn emit_merge_edits(
    result: &mut QueryResult,
    sources: &[(String, String)],
    current_sources: &[(String, String)],
    renames_per_uri: &HashMap<String, Vec<crate::rewrite::RenameReport>>,
    field_edits_per_uri: &HashMap<String, usize>,
) {
    for (uri, original) in sources {
        let new_source = current_sources
            .iter()
            .find(|(u, _)| u == uri)
            .map_or(original.clone(), |(_, s)| s.clone());
        let renames = renames_per_uri.get(uri).cloned().unwrap_or_default();
        let field_edits = field_edits_per_uri.get(uri).copied().unwrap_or(0);
        if new_source == *original && renames.is_empty() && field_edits == 0 {
            continue;
        }
        result.edits_per_file.push((
            uri.clone(),
            AppliedSource {
                uri: uri.clone(),
                original: original.clone(),
                new_source,
                rename_reports: renames,
                field_edits,
            },
        ));
    }
}
