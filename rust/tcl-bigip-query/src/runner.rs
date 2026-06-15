//! Read-only query runner — port of the read path of
//! `dialects/f5/query/runner.py` (`run_query`, `QueryOptions`,
//! `QueryResult`).
//!
//! This is the pure, I/O-free orchestration layer the `f5 query` verb calls:
//! given a list of `(uri, source_text)` BIG-IP configs plus a query
//! expression and options, it parses the expression once, evaluates it
//! against each source in order, and returns a [`QueryResult`].
//!
//! Only the **read** path is ported. Mutating queries (`Assignment` nodes)
//! are detected via [`QueryResult::has_mutation`] so the verb can reject them
//! cleanly; the engine itself already errors on assignment evaluation.

use std::collections::HashMap;

use tcl_bigip::parser::parse_bigip_conf;

use crate::ast::{Expr, Program};
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
    /// Whether the query *attempted* a mutation (any `Assignment` in the
    /// AST). A read-only query is `false`; a mutating one is `true` so the
    /// verb can reject it.
    pub has_mutation: bool,
}

/// Return `true` when any statement in *program* contains an assignment.
///
/// Mirrors the `has_mutation` bookkeeping in `run_query`: the Python runner
/// flips it when an edit op is queued. In the read-only port no mutating
/// builtin is registered, so an `Assignment` node is the only mutation
/// surface — detect it statically.
fn program_has_mutation(program: &Program) -> bool {
    program.statements.iter().any(expr_has_assignment)
}

fn expr_has_assignment(node: &Expr) -> bool {
    match node {
        Expr::Assignment { .. } => true,
        Expr::Literal { .. } | Expr::Identity { .. } | Expr::Variable { .. } => false,
        Expr::ObjectLiteral { entries, .. } => entries.iter().any(|(_, v)| expr_has_assignment(v)),
        Expr::ListLiteral { inner, .. } => inner.as_deref().is_some_and(expr_has_assignment),
        Expr::PathExpr { head, steps, .. } => {
            head.as_deref().is_some_and(expr_has_assignment)
                || steps.iter().any(|step| match step {
                    crate::ast::PathStep::Subscript {
                        index: Some(index), ..
                    } => expr_has_assignment(index),
                    _ => false,
                })
        }
        Expr::Call { args, .. } => args.iter().any(expr_has_assignment),
        Expr::BinOp { lhs, rhs, .. } => expr_has_assignment(lhs) || expr_has_assignment(rhs),
        Expr::UnaryOp { operand, .. } => expr_has_assignment(operand),
        Expr::Pipe { lhs, rhs, .. } => expr_has_assignment(lhs) || expr_has_assignment(rhs),
        Expr::LetBinding { source, body, .. } => {
            expr_has_assignment(source) || expr_has_assignment(body)
        }
        Expr::CommaStream { parts, .. } => parts.iter().any(expr_has_assignment),
        Expr::IfThenElse {
            cond,
            then_body,
            elifs,
            else_body,
            ..
        } => {
            expr_has_assignment(cond)
                || expr_has_assignment(then_body)
                || elifs
                    .iter()
                    .any(|(c, b)| expr_has_assignment(c) || expr_has_assignment(b))
                || else_body.as_deref().is_some_and(expr_has_assignment)
        }
    }
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
/// Read-only port of `runner.run_query`: the expression is parsed once and
/// shared across files; each source takes its turn as the primary input (the
/// `.` of a top-level statement) in source order. Variables (`$name`) bind
/// to every loaded source so cross-file lookups resolve.
///
/// # Errors
///
/// Returns the [`QueryError`] from [`parse_query`](crate::parse_query) when
/// the expression fails to parse, or any evaluation error raised against a
/// source (faithful to `run_query`'s parse-error → `QueryError` behaviour).
pub fn run_query(
    query: &str,
    sources: &[(String, String)],
    opts: &QueryOptions,
) -> Result<QueryResult, QueryError> {
    let program = crate::parse_query(query)?;
    let has_mutation = program_has_mutation(&program);

    let mut result = QueryResult {
        values_per_file: Vec::with_capacity(sources.len()),
        has_mutation,
    };

    // Mutating queries are detected but not evaluated by the read-only
    // runner; return early so the verb can reject them. (The engine would
    // also error on assignment evaluation, but returning here keeps the
    // contract explicit and side-effect-free.)
    if has_mutation {
        return Ok(result);
    }

    for (uri, source) in sources {
        let named_roots = build_named_roots(sources, opts);
        let root = build_root(uri, source, opts);
        let mut ctx = EvalContext {
            root,
            named_roots,
            merge_mode: false,
            bindings: HashMap::new(),
        };
        let mut values: Vec<Value> = Vec::new();
        for stmt in &program.statements {
            values.extend(evaluate_statement(stmt, &mut ctx)?);
        }
        result.values_per_file.push((uri.clone(), values));
    }
    Ok(result)
}
