//! `gantt` renderer — ASCII Gantt of timestamped state transitions.
//!
//! Accepts the row shape `tsv(.timestamp, .label, .state)` produces, plus a
//! few friendlier alternatives:
//!
//! - a string — split on newlines, each non-blank non-`#` line split on `\t`
//!   into three cells;
//! - a list of 3-tuples / 3-lists `(timestamp, label, state)`;
//! - a list of dicts with `timestamp` / `label` / `state` (or `member` /
//!   `state`) keys.
//!
//! Options (via `--render-opt KEY=VALUE`):
//!
//! - `unit-minutes` — minutes per output character, a positive divisor of 60
//!   (default 5).
//! - `year` — year to assume for yearless syslog timestamps (default a
//!   leap-safe year so `Feb 29` parses).

use std::collections::BTreeMap;

use chrono::{Duration, NaiveDateTime, Timelike};

use crate::errors::QueryError;
use crate::value::Value;

const VALID_UNITS: [i64; 12] = [1, 2, 3, 4, 5, 6, 10, 12, 15, 20, 30, 60];
const LEAP_SAFE_YEAR: i64 = 2020;

/// Render *values* as an ASCII Gantt chart — registry entry point.
///
/// # Errors
///
/// Returns [`QueryError::Renderer`] for a bad `unit-minutes` / `year` option,
/// an unparseable timestamp, or an unsupported row shape.
pub fn render(values: &[Value], opts: &BTreeMap<String, String>) -> Result<String, QueryError> {
    let unit_minutes = parse_unit_minutes(
        opts.get("unit-minutes")
            .or_else(|| opts.get("unit_minutes")),
    )?;
    let year = parse_year(opts.get("year"))?;
    let mut rows: Vec<(String, String, String)> = Vec::new();
    coerce_rows(values, &mut rows)?;
    render_gantt(&rows, unit_minutes, year)
}

/// Render *rows* as an ASCII Gantt chart.
fn render_gantt(
    rows: &[(String, String, String)],
    unit_minutes: i64,
    year: i64,
) -> Result<String, QueryError> {
    // Preserve first-seen member order for the events map, but the output is
    // emitted in sorted-member order.
    let mut members: Vec<String> = Vec::new();
    let mut by_member: BTreeMap<String, Vec<(NaiveDateTime, String)>> = BTreeMap::new();
    for (ts, member, state) in rows {
        let dt = parse_timestamp(ts, year)?;
        if !by_member.contains_key(member) {
            members.push(member.clone());
        }
        by_member
            .entry(member.clone())
            .or_default()
            .push((dt, state.clone()));
    }

    if by_member.is_empty() {
        return Ok("(no events)\n".to_owned());
    }

    let flat: Vec<NaiveDateTime> = by_member.values().flatten().map(|(t, _)| *t).collect();
    let min_dt = *flat.iter().min().expect("non-empty");
    let max_dt = *flat.iter().max().expect("non-empty");

    let unit_i64 = unit_minutes;
    let unit = unit_minutes;
    let start_minute = (i64::from(min_dt.minute()) / unit) * unit;
    let start = min_dt
        .with_minute(u32::try_from(start_minute).expect("0..60"))
        .and_then(|d| d.with_second(0))
        .expect("valid minute/second");
    let end = max_dt
        .with_minute(0)
        .and_then(|d| d.with_second(0))
        .expect("valid")
        + Duration::hours(1);
    let total_seconds = (end - start).num_seconds();
    let width = usize::try_from(total_seconds / (unit_i64 * 60)).unwrap_or(0) + 1;

    let col = |dt: NaiveDateTime| -> usize {
        usize::try_from((dt - start).num_seconds() / (unit_i64 * 60)).unwrap_or(0)
    };

    // Ruler — hour labels every `60 / unit` columns, offset by 22.
    let mut ruler: Vec<char> = vec![' '; 22 + width];
    let minutes_per_hour_char = (60 / unit) as usize;
    let mut h_col = 0usize;
    while h_col < width {
        let hour = (start + Duration::minutes((h_col as i64) * unit)).hour();
        let s = format!("{hour:02}");
        for (i, c) in s.chars().enumerate() {
            if 22 + h_col + i < ruler.len() {
                ruler[22 + h_col + i] = c;
            }
        }
        h_col += minutes_per_hour_char;
    }

    let mut out: Vec<String> = Vec::new();
    out.push(format!(
        "members down/up over time (1 char = {unit_minutes} min)"
    ));
    out.push(rstrip(&ruler.iter().collect::<String>()));
    out.push(" ".repeat(22) + "+" + &"-".repeat(width - 1));

    // `sorted(by_member)` — BTreeMap already yields keys in sorted order.
    for (member, events_ref) in &by_member {
        let mut events = events_ref.clone();
        // Sort the (datetime, state) tuples for each member.
        events.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let mut state_row: Vec<char> = vec![' '; width];
        let mut cur = "UP".to_owned();
        let mut prev_col = 0usize;
        for (dt, s) in &events {
            let cc = col(*dt);
            for cell in state_row.iter_mut().take(cc).skip(prev_col) {
                *cell = if cur == "UP" { ' ' } else { '#' };
            }
            cur.clone_from(s);
            if cc < state_row.len() {
                state_row[cc] = if s == "DOWN" { 'v' } else { '^' };
            }
            prev_col = cc + 1;
        }
        for cell in state_row.iter_mut().take(width).skip(prev_col) {
            *cell = if cur == "UP" { ' ' } else { '#' };
        }
        let stripped = strip_common(member);
        let name: String = stripped.chars().take(20).collect();
        let padded = format!("{name:20}");
        out.push(format!(
            "{padded}  |{}",
            rstrip(&state_row.iter().collect::<String>())
        ));
    }

    Ok(out.join("\n") + "\n")
}

fn strip_common(member: &str) -> &str {
    member.strip_prefix("/Common/").unwrap_or(member)
}

/// `str.rstrip()` with no args — strips trailing ASCII/Unicode whitespace.
fn rstrip(s: &str) -> String {
    s.trim_end().to_owned()
}

fn parse_timestamp(s: &str, year: i64) -> Result<NaiveDateTime, QueryError> {
    let input = format!("{year} {s}");
    NaiveDateTime::parse_from_str(&input, "%Y %b %d %H:%M:%S")
        .map_err(|e| QueryError::Renderer(format!("gantt: cannot parse timestamp '{s}': {e}")))
}

fn parse_unit_minutes(value: Option<&String>) -> Result<i64, QueryError> {
    let raw = value.cloned().unwrap_or_else(|| "5".to_owned());
    let ivalue = parse_py_int(&raw).map_err(|()| {
        QueryError::Renderer(format!(
            "gantt: unit-minutes must be an integer (got '{raw}')"
        ))
    })?;
    if !VALID_UNITS.contains(&ivalue) {
        let valid: Vec<String> = VALID_UNITS.iter().map(ToString::to_string).collect();
        return Err(QueryError::Renderer(format!(
            "gantt: unit-minutes must be a positive divisor of 60 (one of {}); got {ivalue}",
            valid.join(", ")
        )));
    }
    Ok(ivalue)
}

fn parse_year(value: Option<&String>) -> Result<i64, QueryError> {
    let Some(raw) = value else {
        return Ok(LEAP_SAFE_YEAR);
    };
    parse_py_int(raw)
        .map_err(|()| QueryError::Renderer(format!("gantt: year must be an integer (got '{raw}')")))
}

/// Integer-from-string semantics: strips surrounding whitespace, accepts an
/// optional sign, rejects floats / trailing junk.
fn parse_py_int(s: &str) -> Result<i64, ()> {
    s.trim().parse::<i64>().map_err(|_| ())
}

fn coerce_rows(
    values: &[Value],
    out: &mut Vec<(String, String, String)>,
) -> Result<(), QueryError> {
    for v in values {
        match v {
            Value::Stream(items) => coerce_rows(items, out)?,
            Value::Str(s) => rows_from_text(s, out)?,
            Value::List(items) => {
                if items.len() == 3 && items.iter().all(is_row_cell) {
                    out.push((cell(&items[0]), cell(&items[1]), cell(&items[2])));
                } else {
                    coerce_rows(items, out)?;
                }
            }
            Value::Object(map) => {
                let ts = first_present(map, &["timestamp", "ts", "time"]);
                let label = first_present(map, &["label", "member", "name"]);
                let state = map.get("state").filter(|x| !is_none(x));
                if let (Some(ts), Some(label), Some(state)) = (ts, label, state) {
                    out.push((cell(ts), cell(label), cell(state)));
                } else {
                    let mut keys: Vec<&String> = map.keys().collect();
                    keys.sort();
                    let keys_repr = py_str_list(&keys);
                    return Err(QueryError::Renderer(format!(
                        "gantt: dict rows must carry 'timestamp', 'label' (or 'member'), and 'state' keys; got {keys_repr}"
                    )));
                }
            }
            Value::ObjectRef(_) => {
                return Err(QueryError::Renderer(
                    "gantt: ObjectRef rows are not supported; project the (timestamp, label, state) fields explicitly via `tsv(...)` or `{timestamp, label, state}`".to_owned(),
                ));
            }
            other => {
                return Err(QueryError::Renderer(format!(
                    "gantt: unsupported row shape {}",
                    type_name_for_gantt(other)
                )));
            }
        }
    }
    Ok(())
}

/// Falsy-or-missing-aware key lookup; the chain
/// `v.get("timestamp") or v.get("ts") or v.get("time")` skips falsy values.
fn first_present<'a>(
    map: &'a indexmap::IndexMap<String, Value>,
    keys: &[&str],
) -> Option<&'a Value> {
    for k in keys {
        if let Some(v) = map.get(*k)
            && crate::value::truthy(v)
        {
            return Some(v);
        }
    }
    None
}

fn is_none(v: &Value) -> bool {
    matches!(v, Value::Null)
}

/// Accepts a `str`, `int`, `float`, or `PathRef` cell — a bool counts as int.
fn is_row_cell(v: &Value) -> bool {
    matches!(
        v,
        Value::Str(_) | Value::Int(_) | Value::Float(_) | Value::Bool(_) | Value::PathRef(_)
    )
}

fn rows_from_text(text: &str, out: &mut Vec<(String, String, String)>) -> Result<(), QueryError> {
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            return Err(QueryError::Renderer(format!(
                "gantt: expected at least 3 tab-separated cells, got {} in '{line}'",
                parts.len()
            )));
        }
        out.push((
            parts[0].to_owned(),
            parts[1].to_owned(),
            parts[2].to_owned(),
        ));
    }
    Ok(())
}

fn cell(v: &Value) -> String {
    match v {
        Value::PathRef(p) => p.full_path.clone(),
        Value::Null => String::new(),
        Value::Str(s) => s.clone(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => crate::jsonfmt::py_float_repr(*f),
        other => crate::output::cell_str(other),
    }
}

/// The value's type name for the unsupported-shape error.
fn type_name_for_gantt(v: &Value) -> &'static str {
    v.type_name()
}

/// Render a list of dict keys in repr style as a sorted str list, e.g.
/// `['member', 'state']`.
fn py_str_list(keys: &[&String]) -> String {
    let inner: Vec<String> = keys.iter().map(|k| format!("'{k}'")).collect();
    format!("[{}]", inner.join(", "))
}
