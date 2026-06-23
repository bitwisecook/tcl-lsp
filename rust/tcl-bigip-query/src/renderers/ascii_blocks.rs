//! `ascii-blocks` renderer — Unicode line-art block diagrams.
//!
//! Walks a nested `{title, rows}` (or `{label, children}`) tree and lays it
//! out as a stack of boxes with `┌─┬─┐` borders.
//!
//! Keys (synonyms):
//!
//! - `title` / `label` / `name` — the box header text.
//! - `rows` / `children` / `items` — list of nested boxes.
//! - `note` / `detail` — optional second line in the box header.
//!
//! Options:
//!
//! - `min-width` — minimum interior width of a box, default 12.
//! - `style` — `"rounded"` (default), `"square"`, or `"ascii"`.

use std::collections::BTreeMap;

use indexmap::IndexMap;

use crate::errors::QueryError;
use crate::value::Value;

/// One box-drawing glyph set.
struct Glyphs {
    tl: &'static str,
    tr: &'static str,
    bl: &'static str,
    br: &'static str,
    h: &'static str,
    v: &'static str,
    stem: &'static str,
    branch: &'static str,
    last: &'static str,
    elbow: &'static str,
}

const ROUNDED: Glyphs = Glyphs {
    tl: "╭",
    tr: "╮",
    bl: "╰",
    br: "╯",
    h: "─",
    v: "│",
    stem: "│",
    branch: "├",
    last: "╰",
    elbow: "─",
};
const SQUARE: Glyphs = Glyphs {
    tl: "┌",
    tr: "┐",
    bl: "└",
    br: "┘",
    h: "─",
    v: "│",
    stem: "│",
    branch: "├",
    last: "└",
    elbow: "─",
};
const ASCII: Glyphs = Glyphs {
    tl: "+",
    tr: "+",
    bl: "+",
    br: "+",
    h: "-",
    v: "|",
    stem: "|",
    branch: "+",
    last: "+",
    elbow: "-",
};

struct Box {
    title: String,
    note: String,
    children: Vec<Box>,
}

/// Render *values* as nested ASCII block diagrams — registry entry point.
///
/// # Errors
///
/// Returns [`QueryError::Renderer`] for an unknown `style`, a non-integer
/// `min-width`, a dict block missing its title, a non-list `rows`, or an
/// unsupported block shape.
pub fn render(values: &[Value], opts: &BTreeMap<String, String>) -> Result<String, QueryError> {
    let style_name = opts
        .get("style")
        .map_or_else(|| "rounded".to_owned(), Clone::clone);
    let glyphs = match style_name.as_str() {
        "rounded" => &ROUNDED,
        "square" => &SQUARE,
        "ascii" => &ASCII,
        _ => {
            // The sorted style names joined by ", " → "ascii, rounded, square".
            return Err(QueryError::Renderer(format!(
                "ascii-blocks: unknown style '{style_name}' (expected one of ascii, rounded, square)"
            )));
        }
    };
    let min_width = parse_int(
        opts.get("min-width").or_else(|| opts.get("min_width")),
        "min-width",
    )?;

    let flattened = flatten(values);
    let mut boxes = Vec::with_capacity(flattened.len());
    for v in &flattened {
        boxes.push(coerce_box(v)?);
    }
    if boxes.is_empty() {
        return Ok("(no blocks)\n".to_owned());
    }
    let parts: Vec<String> = boxes
        .iter()
        .map(|b| render_box(b, glyphs, min_width))
        .collect();
    Ok(parts.join("\n"))
}

fn flatten(values: &[Value]) -> Vec<Value> {
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

fn coerce_box(v: &Value) -> Result<Box, QueryError> {
    match v {
        Value::ObjectRef(o) => Ok(Box {
            title: if o.full_path.is_empty() {
                o.kind.clone()
            } else {
                o.full_path.clone()
            },
            note: o.kind.clone(),
            children: Vec::new(),
        }),
        Value::Object(map) => {
            let title = first_truthy(map, &["title", "label", "name"]);
            let Some(title) = title else {
                let keys_repr = sorted_keys_repr(map);
                return Err(QueryError::Renderer(format!(
                    "ascii-blocks: dict block must carry 'title' (or 'label' / 'name'); got {keys_repr}"
                )));
            };
            let note =
                first_truthy(map, &["note", "detail"]).map_or_else(String::new, py_str_value);
            let kids_raw = map
                .get("rows")
                .filter(|x| truthy(x))
                .or_else(|| map.get("children").filter(|x| truthy(x)))
                .or_else(|| map.get("items").filter(|x| truthy(x)));
            // Only a list / tuple is accepted here — a `Stream` (not a list)
            // is rejected.
            let children = match kids_raw {
                None => Vec::new(),
                Some(Value::List(items)) => {
                    let mut kids = Vec::with_capacity(items.len());
                    for k in items {
                        kids.push(coerce_box(k)?);
                    }
                    kids
                }
                Some(other) => {
                    return Err(QueryError::Renderer(format!(
                        "ascii-blocks: 'rows' / 'children' / 'items' must be a list; got {}",
                        other.type_name()
                    )));
                }
            };
            Ok(Box {
                title: py_str_value(title),
                note,
                children,
            })
        }
        Value::Str(s) => Ok(Box {
            title: s.clone(),
            note: String::new(),
            children: Vec::new(),
        }),
        other => Err(QueryError::Renderer(format!(
            "ascii-blocks: unsupported block shape {}",
            other.type_name()
        ))),
    }
}

fn render_box(b: &Box, glyphs: &Glyphs, min_width: i64) -> String {
    let interior = box_interior_lines(b);
    let min_w = usize::try_from(min_width.max(0)).unwrap_or(0);
    let max_cell = interior
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(min_w);
    let width = min_w.max(max_cell);
    let h = glyphs.h;
    let v = glyphs.v;
    let top = format!("{}{}{}", glyphs.tl, h.repeat(width + 2), glyphs.tr);
    let bot = format!("{}{}{}", glyphs.bl, h.repeat(width + 2), glyphs.br);
    let body_lines: Vec<String> = interior
        .iter()
        .map(|line| {
            let pad = width.saturating_sub(line.chars().count());
            format!("{v} {line}{} {v}", " ".repeat(pad))
        })
        .collect();
    let mut lines = vec![top];
    lines.extend(body_lines);
    lines.push(bot);

    if b.children.is_empty() {
        return lines.join("\n") + "\n";
    }
    let mut out = lines;
    let n = b.children.len();
    for (i, child) in b.children.iter().enumerate() {
        let is_last = i == n - 1;
        out.push(glyphs.stem.to_owned());
        let child_text = render_box(child, glyphs, min_width);
        let child_lines: Vec<&str> = child_text.split('\n').collect();
        // Line-splitting drops a trailing empty element from the
        // final newline; mirror that by ignoring the empty tail.
        let child_lines: Vec<&str> = if child_lines.last() == Some(&"") {
            child_lines[..child_lines.len() - 1].to_vec()
        } else {
            child_lines
        };
        if child_lines.is_empty() {
            continue;
        }
        let elbow = if is_last { glyphs.last } else { glyphs.branch };
        out.push(format!("{elbow}{} {}", glyphs.elbow, child_lines[0]));
        let cont = if is_last { " " } else { glyphs.stem };
        for line in &child_lines[1..] {
            out.push(format!("{cont}  {line}"));
        }
    }
    out.join("\n") + "\n"
}

fn box_interior_lines(b: &Box) -> Vec<String> {
    let mut lines = vec![b.title.clone()];
    if !b.note.is_empty() {
        lines.push(b.note.clone());
    }
    lines
}

fn parse_int(value: Option<&String>, name: &str) -> Result<i64, QueryError> {
    let raw = value.cloned().unwrap_or_else(|| "12".to_owned());
    raw.trim().parse::<i64>().map_err(|_| {
        QueryError::Renderer(format!(
            "ascii-blocks: {name} must be an integer (got '{raw}')"
        ))
    })
}

/// `v.get(k1) or v.get(k2) or ...` — the first truthy value among *keys*.
fn first_truthy<'a>(map: &'a IndexMap<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    for k in keys {
        if let Some(v) = map.get(*k)
            && truthy(v)
        {
            return Some(v);
        }
    }
    None
}

fn truthy(v: &Value) -> bool {
    crate::value::truthy(v)
}

/// String coercion of a value, for the `title` / `note` fields.
fn py_str_value(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Null => "None".to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => crate::jsonfmt::py_float_repr(*f),
        Value::PathRef(p) => p.full_path.clone(),
        other => crate::output::cell_str(other),
    }
}

fn sorted_keys_repr(map: &IndexMap<String, Value>) -> String {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    let inner: Vec<String> = keys.iter().map(|k| format!("'{k}'")).collect();
    format!("[{}]", inner.join(", "))
}
