//! Render evaluator output for the `f5 query` verb.
//!
//! Five flavours, picked by CLI flag: `auto` (default), `scf`, `raw`,
//! `paths`, `json`, plus the `table` / `table-lineart` grids. Unknown modes
//! fall through to the renderer registry; a mode that names neither a
//! built-in mode nor a registered renderer is a [`QueryError::Renderer`].

use std::collections::BTreeMap;

use crate::errors::QueryError;
use crate::jsonfmt;
use crate::renderers;
use crate::value::Value;

/// Flatten top-level `Stream`s into the value list — port of `output._flat`.
pub(crate) fn flat(values: &[Value]) -> Vec<Value> {
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

/// Render *values* in the named *mode* with no renderer options.
///
/// Thin wrapper over [`render_with_opts`] passing an empty option map; kept
/// so existing call sites (and the parity tests) need no change.
///
/// # Errors
///
/// See [`render_with_opts`].
pub fn render(values: &[Value], mode: &str) -> Result<String, QueryError> {
    render_with_opts(values, mode, &BTreeMap::new())
}

/// Render *values* in the named *mode*, threading per-renderer *opts*.
///
/// The built-in output modes (`auto` / `scf` / `raw` / `paths` / `json` /
/// `table` / `table-lineart`) ignore *opts*. Any other *mode* falls through
/// to the renderer registry (`mermaid` / `gantt` / `ascii-blocks`), which
/// consumes the `--render-opt KEY=VALUE` map. Port of `output.render`'s
/// dispatch, including the registry fall-through and the
/// `unknown output mode: …` error for an unregistered name.
///
/// # Errors
///
/// Returns [`QueryError::Renderer`] when `--raw` is asked to render a
/// non-scalar, when a renderer rejects its input / options, or when *mode*
/// names no built-in mode and no registered renderer.
pub fn render_with_opts(
    values: &[Value],
    mode: &str,
    opts: &BTreeMap<String, String>,
) -> Result<String, QueryError> {
    let values = flat(values);
    match mode {
        "auto" => Ok(render_auto(&values)),
        "scf" => Ok(render_scf(&values)),
        "raw" => render_raw(&values),
        "paths" => Ok(render_paths(&values)),
        "json" => Ok(render_json(&values)),
        "table" => Ok(render_table(&values, false)),
        "table-lineart" => Ok(render_table(&values, true)),
        // Fall through to the pluggable renderer registry.
        other => match renderers::lookup(other) {
            Some(spec) => (spec.impl_fn)(&values, opts),
            None => Err(QueryError::Renderer(format!(
                "unknown output mode: {other}"
            ))),
        },
    }
}

fn render_auto(values: &[Value]) -> String {
    if values.is_empty() {
        return String::new();
    }
    if values
        .iter()
        .all(|v| matches!(v, Value::ObjectRef(o) if o.stanza_slot.is_some()))
    {
        return render_scf(values);
    }
    if let Some(flat) = flatten_scalar_lists(values) {
        return render_raw_unchecked(&flat);
    }
    render_json(values)
}

fn render_scf(values: &[Value]) -> String {
    let mut parts = String::new();
    for v in values {
        if let Value::ObjectRef(o) = v
            && let Some(slot) = &o.stanza_slot
        {
            parts.push_str(&slot.raw_text);
            if !slot.raw_text.ends_with('\n') {
                parts.push('\n');
            }
            continue;
        }
        parts.push_str(&scalar_str(v));
        parts.push('\n');
    }
    parts
}

fn render_raw(values: &[Value]) -> Result<String, QueryError> {
    let mut out = String::new();
    for v in flatten_scalars(values) {
        if !is_scalar(&v) {
            let kind = if let Value::ObjectRef(o) = &v {
                format!("ObjectRef('{}')", o.kind)
            } else {
                v.type_name().to_string()
            };
            return Err(QueryError::Renderer(format!(
                "--raw cannot render {kind}; use --paths-only for object \
                 identities or --json / --scf for full objects"
            )));
        }
        out.push_str(&scalar_str(&v));
        out.push('\n');
    }
    Ok(out)
}

/// `--raw` body used by `auto` mode, which has already proved every value is
/// a scalar (or flattens to scalars), so it never raises.
fn render_raw_unchecked(values: &[Value]) -> String {
    let mut out = String::new();
    for v in flatten_scalars(values) {
        out.push_str(&scalar_str(&v));
        out.push('\n');
    }
    out
}

fn render_paths(values: &[Value]) -> String {
    let mut out = String::new();
    for v in flatten_scalars(values) {
        match &v {
            Value::ObjectRef(o) => out.push_str(&o.full_path),
            Value::PathRef(p) => out.push_str(&p.full_path),
            other => out.push_str(&scalar_str(other)),
        }
        out.push('\n');
    }
    out
}

fn render_json(values: &[Value]) -> String {
    jsonfmt::to_pretty(&Value::List(values.to_vec())) + "\n"
}

fn is_scalar(v: &Value) -> bool {
    matches!(
        v,
        Value::Str(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::Bool(_)
            | Value::PathRef(_)
            | Value::Null
    )
}

/// If every value is a scalar or a list of scalars, return the flat list;
/// otherwise `None`. Port of `output._flatten_scalar_lists`.
fn flatten_scalar_lists(values: &[Value]) -> Option<Vec<Value>> {
    let mut out = Vec::new();
    for v in values {
        if is_scalar(v) {
            out.push(v.clone());
            continue;
        }
        if let Value::List(items) = v {
            for item in items {
                if !is_scalar(item) {
                    return None;
                }
                out.push(item.clone());
            }
            continue;
        }
        return None;
    }
    Some(out)
}

/// Like [`flatten_scalar_lists`] but never refuses — port of
/// `output._flatten_scalars` (used by `--raw` / `--paths-only`).
fn flatten_scalars(values: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for v in values {
        if let Value::List(items) = v {
            out.extend(items.iter().cloned());
        } else {
            out.push(v.clone());
        }
    }
    out
}

/// Coerce a scalar to its display string — port of `output._scalar_str`.
fn scalar_str(v: &Value) -> String {
    match v {
        Value::Null | Value::Drop => "null".to_string(),
        Value::PathRef(p) => p.full_path.clone(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        other => py_str(other),
    }
}

/// Python `str(value)` for the scalar values the renderers reach.
fn py_str(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => jsonfmt::py_float_repr(*f),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Null => "None".to_string(),
        Value::PathRef(p) => p.full_path.clone(),
        Value::ObjectRef(o) => format!("ObjectRef(kind='{}', full_path='{}')", o.kind, o.full_path),
        other => other.describe(),
    }
}

fn render_table(values: &[Value], lineart: bool) -> String {
    if values.is_empty() {
        return String::new();
    }
    let mut headers: Vec<String> = Vec::new();
    let mut has_dict = false;
    for v in values {
        match v {
            Value::Object(map) => {
                has_dict = true;
                for k in map.keys() {
                    if !headers.contains(k) {
                        headers.push(k.clone());
                    }
                }
            }
            Value::ObjectRef(o) => {
                has_dict = true;
                for k in o.fields.keys() {
                    if !headers.contains(k) {
                        headers.push(k.clone());
                    }
                }
            }
            _ => {}
        }
    }

    let rows: Vec<Vec<String>> = if has_dict {
        values
            .iter()
            .map(|v| match v {
                Value::Object(map) => headers
                    .iter()
                    .map(|h| cell_str(map.get(h).unwrap_or(&Value::Str(String::new()))))
                    .collect(),
                Value::ObjectRef(o) => headers
                    .iter()
                    .map(|h| cell_str(o.fields.get(h).unwrap_or(&Value::Str(String::new()))))
                    .collect(),
                other => {
                    let mut row = vec![cell_str(other)];
                    row.extend(std::iter::repeat_n(
                        String::new(),
                        headers.len().saturating_sub(1),
                    ));
                    row
                }
            })
            .collect()
    } else {
        headers = vec!["value".to_string()];
        values.iter().map(|v| vec![cell_str(v)]).collect()
    };

    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let max_cell = rows.iter().map(|r| r[i].chars().count()).max().unwrap_or(0);
            h.chars().count().max(max_cell)
        })
        .collect();

    let (hc, vc, tl, tm, tr, ml, mm, mr, bl, bm, br) = if lineart {
        ("─", "│", "┌", "┬", "┐", "├", "┼", "┤", "└", "┴", "┘")
    } else {
        ("-", "|", "+", "+", "+", "+", "+", "+", "+", "+", "+")
    };

    let bar = |l: &str, mid: &str, r: &str| -> String {
        let cells: Vec<String> = widths.iter().map(|w| hc.repeat(w + 2)).collect();
        format!("{l}{}{r}", cells.join(mid))
    };
    let row_line = |cells: &[String]| -> String {
        let body: Vec<String> = cells
            .iter()
            .zip(&widths)
            .map(|(c, w)| format!(" {c:<width$} ", width = *w))
            .collect();
        format!("{vc}{}{vc}", body.join(vc))
    };

    let mut lines: Vec<String> = vec![bar(tl, tm, tr), row_line(&headers), bar(ml, mm, mr)];
    for row in &rows {
        lines.push(row_line(row));
    }
    lines.push(bar(bl, bm, br));
    lines.join("\n") + "\n"
}

/// Coerce a single table cell value to its display string — port of
/// `output._cell_str`.
pub(crate) fn cell_str(v: &Value) -> String {
    match v {
        Value::Null | Value::Drop => String::new(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::PathRef(p) => p.full_path.clone(),
        Value::ObjectRef(o) => {
            if o.full_path.is_empty() {
                o.kind.clone()
            } else {
                o.full_path.clone()
            }
        }
        Value::List(items) | Value::Stream(items) => {
            items.iter().map(cell_str).collect::<Vec<_>>().join(",")
        }
        Value::Object(_) => jsonfmt::to_compact(v),
        other => py_str(other),
    }
}
