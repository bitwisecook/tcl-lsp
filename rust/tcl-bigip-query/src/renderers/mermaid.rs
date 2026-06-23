//! `mermaid` renderer — Mermaid flowchart of a query result.
//!
//! Two modes pick themselves up automatically:
//!
//! - when every value is an [`ObjectRef`](crate::value::ObjectRef), it would
//!   build the BIG-IP object reference sub-graph reachable from those seeds
//!   (the same Mermaid `f5 graph --format mermaid` produces). That mode needs
//!   the originating source text for each seed's `config_uri`, but no source
//!   map is threaded into `output::render`: the renderer runs after the run
//!   completes, so the CLI dispatch path has no sources by render time and the
//!   object-graph attempt falls back to chain mode.
//! - otherwise emits a generic `graph LR` flowchart: one node per value,
//!   consecutive values connected with an arrow.
//!
//! Options:
//!
//! - `direction` — `LR` (default), `TB`, `RL`, `BT`.
//! - `reverse` — `true` to flip ObjectRef-mode edges (validated even though
//!   ObjectRef-mode is unreachable from the CLI).
//! - `max-depth` — integer BFS depth limit for ObjectRef-mode.

use std::collections::BTreeMap;

use crate::errors::QueryError;
use crate::output::cell_str;
use crate::value::Value;

const DIRECTIONS: [&str; 4] = ["LR", "RL", "TB", "BT"];

/// Render *values* as Mermaid — registry entry point.
///
/// # Errors
///
/// Returns [`QueryError::Renderer`] for an unknown `direction`, a non-boolean
/// `reverse`, or a non-integer `max-depth`.
pub fn render(values: &[Value], opts: &BTreeMap<String, String>) -> Result<String, QueryError> {
    let direction = opts
        .get("direction")
        .map_or_else(|| "LR".to_owned(), |s| s.to_uppercase());
    if !DIRECTIONS.contains(&direction.as_str()) {
        let valid = DIRECTIONS.join(", ");
        return Err(QueryError::Renderer(format!(
            "mermaid: unknown direction '{direction}' (expected one of {valid})"
        )));
    }

    let all_object_refs =
        !values.is_empty() && values.iter().all(|v| matches!(v, Value::ObjectRef(_)));
    if all_object_refs {
        // Validate the ObjectRef-mode options before attempting the graph,
        // then fall back to chain mode: the engine has no source map to walk
        // the reference graph from, since the CLI dispatch path has no sources
        // by render time.
        let _reverse = parse_bool(opts.get("reverse"), "reverse")?;
        let _max_depth =
            parse_optional_int(opts.get("max-depth").or_else(|| opts.get("max_depth")))?;
        return Ok(render_chain(values, &direction));
    }
    Ok(render_chain(values, &direction))
}

fn render_chain(values: &[Value], direction: &str) -> String {
    if values.is_empty() {
        return format!("graph {direction}\n");
    }
    let mut lines = vec![format!("graph {direction}")];
    for (i, v) in values.iter().enumerate() {
        lines.push(format!("  n{i}[\"{}\"]", node_label(v)));
    }
    for i in 0..values.len().saturating_sub(1) {
        lines.push(format!("  n{i} --> n{}", i + 1));
    }
    lines.join("\n") + "\n"
}

fn node_label(v: &Value) -> String {
    let text = match v {
        Value::ObjectRef(o) => {
            if o.full_path.is_empty() {
                o.kind.clone()
            } else {
                o.full_path.clone()
            }
        }
        Value::PathRef(p) => p.full_path.clone(),
        Value::Object(map) => {
            // `title` / `label` / `name`, else fall back to the cell string.
            let title = map
                .get("title")
                .or_else(|| map.get("label"))
                .or_else(|| map.get("name"));
            match title {
                Some(t) => py_str_value(t),
                None => cell_str(v),
            }
        }
        _ => cell_str(v),
    };
    text.replace('"', "'").replace('\n', " ")
}

/// Python `str(value)` for the title/label/name field.
fn py_str_value(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Null => "None".to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => crate::jsonfmt::py_float_repr(*f),
        Value::PathRef(p) => p.full_path.clone(),
        other => cell_str(other),
    }
}

/// Parse a boolean option.
fn parse_bool(v: Option<&String>, name: &str) -> Result<bool, QueryError> {
    let Some(s) = v else { return Ok(false) };
    match s.to_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Ok(true),
        "false" | "no" | "0" | "off" | "" => Ok(false),
        _ => Err(QueryError::Renderer(format!(
            "mermaid: {name} must be a boolean (got '{s}')"
        ))),
    }
}

/// Parse an optional integer option.
fn parse_optional_int(v: Option<&String>) -> Result<Option<i64>, QueryError> {
    match v {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => s.trim().parse::<i64>().map(Some).map_err(|_| {
            QueryError::Renderer(format!("mermaid: max-depth must be an integer (got '{s}')"))
        }),
    }
}
