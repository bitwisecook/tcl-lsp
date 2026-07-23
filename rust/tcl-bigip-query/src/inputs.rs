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

//! Structured-input parsers + the input-format registry for side-inputs.
//!
//! Side-inputs
//! (`--input-json` / `--input-jsonl` / `--input-csv` / `--input-f5log`,
//! plus user-defined formats) bind a file's parsed content to a DSL
//! `$NAME` variable; this module owns the per-format parsers and the
//! dispatch table the runner consults.
//!
//! Each parser turns the file's text into a [`Value`]:
//!
//! - `json`  — a single JSON value (dict / list / scalar).
//! - `jsonl` — one JSON value per non-blank line, as a list.
//! - `csv`   — a list of row-dicts (first row names columns unless
//!   `headers` is supplied).
//! - `f5log` — a list of structured BIG-IP log-event dicts.
//!
//! The `f5log` tokeniser is deterministic line-parsing logic (no regex tower
//! beyond stripping the optional syslog `<NNN>` PRI prefix).

use crate::builtins::json_to_value;
use crate::errors::QueryError;
use crate::value::Value;

/// Metadata for one registered input format.
pub struct InputFormatSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub details: &'static str,
}

/// How to parse a side-input URI.
///
/// `kind` names the format; `headers` carries the optional `csv_headers`
/// option (the only per-format knob the CLI exposes today).
#[derive(Debug, Clone, Default)]
pub struct InputSpec {
    pub kind: String,
    /// CSV column headers from `--input-csv NAME=PATH:hdr1,hdr2,…`.
    pub csv_headers: Option<Vec<String>>,
}

impl InputSpec {
    #[must_use]
    pub fn new(kind: impl Into<String>) -> Self {
        InputSpec {
            kind: kind.into(),
            csv_headers: None,
        }
    }

    #[must_use]
    pub fn with_csv_headers(kind: impl Into<String>, headers: Option<Vec<String>>) -> Self {
        InputSpec {
            kind: kind.into(),
            csv_headers: headers,
        }
    }
}

/// The four built-in input formats, sorted by name.
#[must_use]
pub fn list_input_formats() -> Vec<InputFormatSpec> {
    // Sorted by name: csv, f5log, json, jsonl.
    vec![
        InputFormatSpec {
            name: "csv",
            summary: "CSV — list of row-dicts; first row names columns unless 'headers' option is set.",
            details: "Option ``headers`` (tuple of column names) skips header-row discovery so every line of *source* is treated as data — useful for CSVs without a leading header row.",
        },
        InputFormatSpec {
            name: "f5log",
            summary: "F5 BIG-IP syslog — structured event dicts (timestamp / module / message / ...).",
            details: "",
        },
        InputFormatSpec {
            name: "json",
            summary: "JSON side-input — parsed as a single value (dict, list, scalar).",
            details: "",
        },
        InputFormatSpec {
            name: "jsonl",
            summary: "JSON Lines (NDJSON) — one JSON value per non-blank line.",
            details: "",
        },
        InputFormatSpec {
            name: "zone",
            summary: "DNS zone file (RFC 1035) — list of resource records (name / ttl / class / type / rdata; A/AAAA also carry 'address').",
            details: "Honours ``$ORIGIN`` / ``$TTL``, owner-name inheritance, and parenthesised (SOA) continuations. Names are expanded to FQDNs; use it to resolve config addresses to hostnames in a report or query.",
        },
    ]
}

/// Return whether *name* is a registered input format.
#[must_use]
pub fn is_registered(name: &str) -> bool {
    matches!(name, "json" | "jsonl" | "csv" | "f5log" | "zone")
}

/// Parse *source* according to *spec*, returning the navigable [`Value`].
///
/// Port of `runner._build_structured_root`'s dispatch: unknown kinds
/// raise a `QueryError` listing the registered formats; parse failures
/// are wrapped in the runner's `{uri}: invalid {kind} input ({exc})`
/// shape (the caller does that wrapping).
///
/// # Errors
///
/// Returns a [`QueryError`] for an unknown format kind or a parse
/// failure (the message is the bare parser error, to be wrapped by the
/// caller).
pub fn parse_input(source: &str, uri: &str, spec: &InputSpec) -> Result<Value, QueryError> {
    if !is_registered(&spec.kind) {
        let registered = list_input_formats()
            .iter()
            .map(|s| s.name)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(QueryError::eval(format!(
            "{uri}: unknown input format {} (registered: {registered})",
            crate::lexer::py_repr_str(&spec.kind)
        )));
    }
    match spec.kind.as_str() {
        // `_parse_json` raises `QueryError` directly — it bypasses the
        // runner's `{uri}: invalid {kind} input (...)` re-wrap, so the
        // message is already `{uri}: invalid JSON (...)`.
        "json" => parse_json(source, uri),
        // `parse_jsonl` / `parse_csv` raise `InputError`, which the runner
        // re-wraps as `{uri}: invalid {kind} input ({inner})`.
        "jsonl" => parse_jsonl(source, uri)
            .map_err(|e| QueryError::eval(format!("{uri}: invalid jsonl input ({e})"))),
        "csv" => parse_csv(source, spec.csv_headers.as_deref(), uri)
            .map_err(|e| QueryError::eval(format!("{uri}: invalid csv input ({e})"))),
        "f5log" => Ok(parse_f5log(source)),
        "zone" => parse_zone(source, uri)
            .map_err(|e| QueryError::eval(format!("{uri}: invalid zone input ({e})"))),
        _ => unreachable!("kind validated above"),
    }
}

// JSON

/// Parse a single JSON value.
///
/// # Errors
///
/// Returns a [`QueryError`] when the text is not valid JSON; the message
/// mirrors `{uri}: invalid JSON (...)`.
pub fn parse_json(source: &str, uri: &str) -> Result<Value, QueryError> {
    let parsed: serde_json::Value = serde_json::from_str(source).map_err(|e| {
        QueryError::eval(format!("{uri}: invalid JSON ({} at line {})", e, e.line()))
    })?;
    Ok(json_to_value(&parsed, 0))
}

// JSON Lines (NDJSON)

/// Parse newline-delimited JSON into a list of values.
///
/// Blank lines are ignored; each non-blank line must be a single JSON
/// value. Errors quote the line number.
///
/// # Errors
///
/// Returns a [`QueryError`] for a line that is not valid JSON; the
/// message
pub fn parse_jsonl(text: &str, source: &str) -> Result<Value, QueryError> {
    let mut out: Vec<Value> = Vec::new();
    for (line_no, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let parsed: serde_json::Value = serde_json::from_str(line).map_err(|e| {
            QueryError::eval(format!(
                "{source}: line {}: invalid JSON ({} at col {})",
                line_no + 1,
                e,
                e.column()
            ))
        })?;
        out.push(json_to_value(&parsed, 0));
    }
    Ok(Value::List(out))
}

// CSV

/// Parse CSV text into a list of row-dicts.
///
/// When *headers* is `None` the first row names the columns; otherwise
/// every row is data and *headers* names each column. Empty /
/// whitespace-only rows are skipped. Missing trailing columns become
/// empty strings; extras land in `_extra`.
///
/// # Errors
///
/// Returns a [`QueryError`] when there are no header columns (the
/// "no header columns" message).
pub fn parse_csv(
    text: &str,
    headers: Option<&[String]>,
    source: &str,
) -> Result<Value, QueryError> {
    let parsed_rows = read_csv_records(text);
    // Keep only rows where at least one cell is non-whitespace.
    let rows: Vec<Vec<String>> = parsed_rows
        .into_iter()
        .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
        .collect();
    if rows.is_empty() {
        return Ok(Value::List(Vec::new()));
    }
    let (headers, data_rows): (Vec<String>, &[Vec<String>]) = match headers {
        None => {
            let hdrs: Vec<String> = rows[0].iter().map(|h| h.trim().to_string()).collect();
            (hdrs, &rows[1..])
        }
        Some(h) => (h.to_vec(), &rows[..]),
    };
    if headers.is_empty() {
        return Err(QueryError::eval(format!(
            "{source}: no header columns — supply --input-csv NAME=PATH:hdr1,hdr2,…"
        )));
    }
    let width = headers.len();
    let mut out: Vec<Value> = Vec::new();
    for row in data_rows {
        let mut record: indexmap::IndexMap<String, Value> = indexmap::IndexMap::new();
        for (i, name) in headers.iter().enumerate() {
            let cell = row.get(i).cloned().unwrap_or_default();
            record.insert(name.clone(), Value::Str(cell));
        }
        if row.len() > width {
            let extra: Vec<Value> = row[width..].iter().map(|c| Value::Str(c.clone())).collect();
            record.insert("_extra".to_string(), Value::List(extra));
        }
        out.push(Value::Object(record));
    }
    Ok(Value::List(out))
}

/// Parse CSV text into rows of cells, following RFC 4180
/// defaults (`"`-quoting, `""` escapes a quote, embedded
/// commas / newlines inside quotes).
///
/// Records are split on `\r\n`,
/// `\r`, or `\n` (outside quotes) and a quoted field's embedded
/// newlines are treated literally. A trailing newline does not produce an extra
/// empty record.
fn read_csv_records(text: &str) -> Vec<Vec<String>> {
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut field = String::new();
    let mut row: Vec<String> = Vec::new();
    let mut in_quotes = false;
    let mut field_started = false;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let n = chars.len();
    while i < n {
        let c = chars[i];
        if in_quotes {
            if c == '"' {
                if i + 1 < n && chars[i + 1] == '"' {
                    field.push('"');
                    i += 2;
                    continue;
                }
                in_quotes = false;
                i += 1;
                continue;
            }
            field.push(c);
            i += 1;
            continue;
        }
        match c {
            '"' => {
                in_quotes = true;
                field_started = true;
                i += 1;
            }
            ',' => {
                row.push(std::mem::take(&mut field));
                field_started = true;
                i += 1;
            }
            '\r' | '\n' => {
                // Normalise \r\n / \r / \n to one record boundary.
                if c == '\r' && i + 1 < n && chars[i + 1] == '\n' {
                    i += 1;
                }
                row.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut row));
                field_started = false;
                i += 1;
            }
            _ => {
                field.push(c);
                field_started = true;
                i += 1;
            }
        }
    }
    // Flush the final field/record if any content was started and not
    // terminated by a trailing newline.
    if field_started || !field.is_empty() || !row.is_empty() {
        row.push(field);
        records.push(row);
    }
    records
}

// F5 BIG-IP logs

const F5_SEVERITIES: &[&str] = &[
    "emerg", "alert", "crit", "err", "warning", "notice", "info", "debug",
];

const SOURCE_TAGS: &[&str] = &["audit", "gtm", "ltm", "security", "tmm", "user"];

/// One parsed BIG-IP log line.
#[derive(Debug, Clone, Default)]
struct F5LogEvent {
    timestamp: String,
    host: String,
    severity: String,
    daemon: String,
    pid: Option<i64>,
    code: String,
    module: String,
    level: Option<i64>,
    message: String,
    raw: String,
}

/// Parse a BIG-IP log file into a list of event dicts. Blank lines are
/// skipped.
#[must_use]
pub fn parse_f5log(text: &str) -> Value {
    let mut events: Vec<Value> = Vec::new();
    for raw_line in text.lines() {
        if raw_line.trim().is_empty() {
            continue;
        }
        // `rstrip("\n")`; `.lines()` already strips the newline.
        let event = parse_one_f5log(raw_line);
        events.push(event_to_value(&event));
    }
    Value::List(events)
}

/// Tokenise a single line into an [`F5LogEvent`].
fn parse_one_f5log(line: &str) -> F5LogEvent {
    let raw = line.to_string();
    let body = strip_pri_prefix(line);
    let body = strip_source_tag(body);
    let (timestamp, body, ts_swaps_host_level) = take_timestamp(body);
    let Some(timestamp) = timestamp else {
        return F5LogEvent {
            raw,
            message: line.to_string(),
            ..Default::default()
        };
    };
    let host: String;
    let severity: String;
    let remainder: String;
    if ts_swaps_host_level {
        // ``MM-DD HH:MM:SS level host …`` (tmsh show sys log): take both
        // tokens and swap.
        let (first, rest) = take_token(body);
        let (second, rest2) = take_token(&rest);
        host = second;
        let first_lower = first.to_lowercase();
        severity = if F5_SEVERITIES.contains(&first_lower.as_str()) {
            first_lower
        } else {
            first
        };
        remainder = rest2;
    } else {
        let (h, rest) = take_token(body);
        host = h;
        let (sev, rest2) = take_severity(&rest);
        severity = sev;
        remainder = rest2;
    }
    let (daemon, pid, body) = take_daemon_pid(&remainder);
    let mut body = body.trim_start().to_string();
    if let Some(stripped) = body.strip_prefix(':') {
        body = stripped.trim_start().to_string();
    }
    let (code, module, level, body) = take_f5_code(&body);
    F5LogEvent {
        timestamp,
        host,
        severity,
        daemon,
        pid,
        code,
        module,
        level,
        message: body,
        raw,
    }
}

/// Strip a leading syslog `<NNN>` PRI prefix.
fn strip_pri_prefix(line: &str) -> &str {
    let bytes = line.as_bytes();
    if bytes.first() != Some(&b'<') {
        return line;
    }
    // `<` then 1..=3 digits then `>`.
    let mut i = 1;
    let mut digits = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() && digits < 3 {
        i += 1;
        digits += 1;
    }
    if digits >= 1 && i < bytes.len() && bytes[i] == b'>' {
        &line[i + 1..]
    } else {
        line
    }
}

/// Drop a leading `audit `/`gtm `/`ltm `/… facility tag.
fn strip_source_tag(text: &str) -> &str {
    let (head, rest) = take_token(text);
    if !head.is_empty() && SOURCE_TAGS.contains(&head.as_str()) {
        // Strip leading whitespace, returning a slice into the original `text`.
        let trimmed = rest.trim_start();
        // Find where `trimmed` begins within `text`.
        let offset = text.len() - trimmed.len();
        &text[offset..]
    } else {
        text
    }
}

/// Peel a timestamp off the head.
///
/// Returns `(timestamp_or_None, remainder, swap_host_level_flag)`.
fn take_timestamp(text: &str) -> (Option<String>, &str, bool) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    // RFC3339: starts with 4 digits + '-'.
    if len >= 19
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && (bytes[10] == b'T' || bytes[10] == b' ')
        && bytes[13] == b':'
        && bytes[16] == b':'
    {
        let mut i = 19;
        while i < len && bytes[i] != b' ' && bytes[i] != b'\t' {
            i += 1;
        }
        return (Some(text[..i].to_string()), &text[i..], false);
    }
    // tmsh short ISO: ``NN-NN HH:MM:SS`` (no year, no T).
    if len >= 14
        && bytes[0..2].iter().all(u8::is_ascii_digit)
        && bytes[2] == b'-'
        && bytes[3..5].iter().all(u8::is_ascii_digit)
        && bytes[5] == b' '
        && bytes[6..8].iter().all(u8::is_ascii_digit)
        && bytes[8] == b':'
        && bytes[9..11].iter().all(u8::is_ascii_digit)
        && bytes[11] == b':'
        && bytes[12..14].iter().all(u8::is_ascii_digit)
    {
        return (Some(text[..14].to_string()), &text[14..], true);
    }
    // Syslog: 3-letter month, space(s), 1-2 digit day, space, HH:MM:SS.
    if len < 14 {
        return (None, text, false);
    }
    if !bytes[..3].iter().all(u8::is_ascii_alphabetic) {
        return (None, text, false);
    }
    // `text.split(None, 3)` — split on runs of whitespace, max 4 parts.
    let parts = py_split_whitespace(text, 3);
    if parts.len() < 4 {
        return (None, text, false);
    }
    let (month, day, hms) = (&parts[0], &parts[1], &parts[2]);
    let hms_bytes = hms.as_bytes();
    if month.len() != 3
        || !day.bytes().all(|b| b.is_ascii_digit())
        || day.is_empty()
        || hms.len() != 8
        || hms_bytes[2] != b':'
        || hms_bytes[5] != b':'
    {
        return (None, text, false);
    }
    // Recover the literal source slice so double-space day alignment survives.
    let mut i = 3; // past month
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i += day.len();
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i += hms.len();
    (Some(text[..i].to_string()), &text[i..], false)
}

/// Split on whitespace runs, up to `maxsplit` splits (so at most
/// `maxsplit + 1` parts), leading whitespace ignored.
fn py_split_whitespace(text: &str, maxsplit: usize) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        // Skip leading whitespace.
        while i < n && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= n {
            break;
        }
        if parts.len() == maxsplit {
            // Remainder is the final part (with no trailing strip — trailing
            // content is kept, but the leading split-whitespace is stripped).
            parts.push(chars[i..].iter().collect());
            return parts;
        }
        let start = i;
        while i < n && !chars[i].is_whitespace() {
            i += 1;
        }
        parts.push(chars[start..i].iter().collect());
    }
    parts
}

/// Return `(first whitespace-delimited token, rest)`.
///
/// `lstrip()` first, then split at the next whitespace; `rest` keeps the
/// whitespace that followed the token.
fn take_token(text: &str) -> (String, String) {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return (String::new(), String::new());
    }
    let mut end = trimmed.len();
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_whitespace() {
            end = idx;
            break;
        }
    }
    (trimmed[..end].to_string(), trimmed[end..].to_string())
}

/// Peel a severity word when present.
fn take_severity(text: &str) -> (String, String) {
    let (head, rest) = take_token(text);
    let head_lower = head.to_lowercase();
    if F5_SEVERITIES.contains(&head_lower.as_str()) {
        (head_lower, rest)
    } else {
        (String::new(), text.to_string())
    }
}

/// Peel `daemon[pid]` or `daemon` off the head, conservatively.
fn take_daemon_pid(text: &str) -> (String, Option<i64>, String) {
    let stripped = text.trim_start();
    if stripped.is_empty() {
        return (String::new(), None, String::new());
    }
    let sb: Vec<char> = stripped.chars().collect();
    let n = sb.len();
    let mut i = 0;
    while i < n && sb[i] != '[' && sb[i] != ':' && !sb[i].is_whitespace() {
        i += 1;
    }
    if i == 0 {
        return (String::new(), None, text.to_string());
    }
    let daemon: String = sb[..i].iter().collect();
    if i < n && sb[i] == '[' {
        let mut j = i + 1;
        while j < n && sb[j].is_ascii_digit() {
            j += 1;
        }
        if j > i + 1 && j < n && sb[j] == ']' && j + 1 < n && sb[j + 1] == ':' {
            let pid_str: String = sb[i + 1..j].iter().collect();
            let pid = pid_str.parse::<i64>().ok();
            let rest: String = sb[j + 1..].iter().collect();
            return (daemon, pid, rest);
        }
        // `daemon[…` without a closing `]:` — roll back into the tail.
        return (String::new(), None, text.to_string());
    }
    if i < n && sb[i] == ':' {
        let rest: String = sb[i..].iter().collect();
        return (daemon, None, rest);
    }
    // No terminator — surface the original text.
    (String::new(), None, text.to_string())
}

/// Peel an F5 message code (`XXXXXXXX:N`) off the head when present
///
/// Returns `(code, module, level, rest)`.
fn take_f5_code(text: &str) -> (String, String, Option<i64>, String) {
    let bytes = text.as_bytes();
    if bytes.len() < 12 {
        return (String::new(), String::new(), None, text.to_string());
    }
    if bytes[8] != b':' || !is_hex(&text[..8]) {
        return (String::new(), String::new(), None, text.to_string());
    }
    if !bytes[9].is_ascii_digit() {
        return (String::new(), String::new(), None, text.to_string());
    }
    if bytes[10] != b':' {
        return (String::new(), String::new(), None, text.to_string());
    }
    let module = text[..8].to_string();
    let Some(level) = (text[9..10]).parse::<i64>().ok() else {
        return (String::new(), String::new(), None, text.to_string());
    };
    let code = format!("{module}:{level}");
    let rest = text[11..].trim_start().to_string();
    (code, module, Some(level), rest)
}

/// `true` when every char is a hex digit.
fn is_hex(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    text.chars().all(|ch| ch.is_ascii_hexdigit())
}

/// Return the event as an object value.
fn event_to_value(event: &F5LogEvent) -> Value {
    let mut m: indexmap::IndexMap<String, Value> = indexmap::IndexMap::new();
    m.insert("timestamp".to_string(), Value::Str(event.timestamp.clone()));
    m.insert("host".to_string(), Value::Str(event.host.clone()));
    m.insert("severity".to_string(), Value::Str(event.severity.clone()));
    m.insert("daemon".to_string(), Value::Str(event.daemon.clone()));
    m.insert("pid".to_string(), event.pid.map_or(Value::Null, Value::Int));
    m.insert("code".to_string(), Value::Str(event.code.clone()));
    m.insert("module".to_string(), Value::Str(event.module.clone()));
    m.insert(
        "level".to_string(),
        event.level.map_or(Value::Null, Value::Int),
    );
    m.insert("message".to_string(), Value::Str(event.message.clone()));
    m.insert("raw".to_string(), Value::Str(event.raw.clone()));
    Value::Object(m)
}

// DNS zone files (RFC 1035)

/// Parse a DNS zone file into a list of resource-record objects
/// `{name, ttl, class, type, rdata}` (A / AAAA records also carry `address`).
///
/// Honours `$ORIGIN` / `$TTL`, owner-name inheritance (a record beginning with
/// whitespace reuses the previous owner), `@` for the origin, relative-vs-FQDN
/// names, `;` comments, and parenthesised (SOA) multi-line continuations. Names
/// are expanded to fully-qualified form (trailing dot stripped). Lets a report
/// or query resolve config addresses to hostnames.
///
/// # Errors
/// Currently infallible for well-formed text; returns the bare parser error
/// (wrapped by the caller as `{uri}: invalid zone input (...)`).
pub fn parse_zone(source: &str, _uri: &str) -> Result<Value, QueryError> {
    let mut origin = String::new();
    let mut default_ttl = String::new();
    let mut last_owner = String::new();
    let mut out: Vec<Value> = Vec::new();

    for line in zone_logical_lines(source) {
        if line.trim().is_empty() {
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("$ORIGIN") {
            origin = zone_fqdn(rest.trim(), &origin);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("$TTL") {
            default_ttl = rest.trim().to_string();
            continue;
        }
        if trimmed.starts_with('$') {
            continue; // $INCLUDE / other directives — skipped leniently
        }

        // A line starting with whitespace inherits the previous owner name.
        let inherit = line.starts_with([' ', '\t']);
        let mut toks: Vec<&str> = line.split_whitespace().collect();
        if toks.is_empty() {
            continue;
        }
        let owner = if inherit {
            last_owner.clone()
        } else {
            let o = zone_fqdn(toks.remove(0), &origin);
            last_owner.clone_from(&o);
            o
        };

        // Optional TTL and/or class (either order) precede the type.
        let mut ttl = default_ttl.clone();
        let mut class = String::new();
        while let Some(&t) = toks.first() {
            if !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()) {
                ttl = t.to_string();
                toks.remove(0);
            } else if matches!(t.to_ascii_uppercase().as_str(), "IN" | "CH" | "HS" | "CS") {
                class = t.to_ascii_uppercase();
                toks.remove(0);
            } else {
                break;
            }
        }
        if toks.is_empty() {
            continue;
        }
        let rtype = toks.remove(0).to_ascii_uppercase();
        let rdata = toks.join(" ");

        let mut rec: indexmap::IndexMap<String, Value> = indexmap::IndexMap::new();
        rec.insert("name".to_string(), Value::Str(owner));
        rec.insert("ttl".to_string(), Value::Str(ttl));
        rec.insert(
            "class".to_string(),
            Value::Str(if class.is_empty() {
                "IN".to_string()
            } else {
                class
            }),
        );
        rec.insert("type".to_string(), Value::Str(rtype.clone()));
        rec.insert("rdata".to_string(), Value::Str(rdata.clone()));
        if rtype == "A" || rtype == "AAAA" {
            let addr = rdata.split_whitespace().next().unwrap_or("").to_string();
            rec.insert("address".to_string(), Value::Str(addr));
        }
        out.push(Value::Object(rec));
    }
    Ok(Value::List(out))
}

/// Strip everything from an unquoted `;` to end of line, preserving leading
/// whitespace (owner inheritance depends on it).
fn strip_zone_comment(line: &str) -> String {
    let mut out = String::new();
    let mut in_q = false;
    for c in line.chars() {
        match c {
            '"' => {
                in_q = !in_q;
                out.push(c);
            }
            ';' if !in_q => break,
            _ => out.push(c),
        }
    }
    out
}

/// Fold physical lines into logical records: strip comments and join lines
/// while parentheses are unbalanced (SOA spans several lines), then flatten the
/// parentheses away. Leading whitespace of each record's first physical line is
/// preserved.
fn zone_logical_lines(src: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut depth: i32 = 0;
    for raw in src.lines() {
        let line = strip_zone_comment(raw);
        let mut in_q = false;
        for c in line.chars() {
            match c {
                '"' => in_q = !in_q,
                '(' if !in_q => depth += 1,
                ')' if !in_q => depth -= 1,
                _ => {}
            }
        }
        if buf.is_empty() {
            buf.push_str(line.trim_end());
        } else {
            buf.push(' ');
            buf.push_str(line.trim());
        }
        if depth <= 0 {
            out.push(buf.replace(['(', ')'], " "));
            buf.clear();
            depth = 0;
        }
    }
    if !buf.is_empty() {
        out.push(buf.replace(['(', ')'], " "));
    }
    out
}

/// Expand a zone name to fully-qualified form against `origin` (trailing dot
/// stripped). `@` maps to the origin; a name ending in `.` is already absolute.
fn zone_fqdn(name: &str, origin: &str) -> String {
    let name = name.trim();
    let o = origin.trim_end_matches('.');
    if name == "@" || name.is_empty() {
        return o.to_string();
    }
    if let Some(abs) = name.strip_suffix('.') {
        return abs.to_string();
    }
    if o.is_empty() {
        name.to_string()
    } else {
        format!("{name}.{o}")
    }
}

#[cfg(test)]
mod zone_tests {
    use super::*;

    fn recs(v: &Value) -> &Vec<Value> {
        match v {
            Value::List(l) => l,
            _ => panic!("expected list"),
        }
    }
    fn field(v: &Value, k: &str) -> String {
        match v {
            Value::Object(m) => match m.get(k) {
                Some(Value::Str(s)) => s.clone(),
                _ => String::new(),
            },
            _ => String::new(),
        }
    }

    #[test]
    fn parses_common_records_with_origin_and_inheritance() {
        let zone = "\
$ORIGIN example.com.
$TTL 3600
@   IN SOA ns1.example.com. admin.example.com. (
        2024010101 ; serial
        7200 3600 1209600 3600 )
    IN NS  ns1.example.com.
www IN A   192.0.2.10
    IN AAAA 2001:db8::10
api CNAME www          ; relative target
mail 300 IN A 192.0.2.20
";
        let v = parse_zone(zone, "example.com.zone").unwrap();
        let r = recs(&v);
        // SOA, NS, A(www), AAAA(www via inheritance), CNAME(api), A(mail)
        assert_eq!(r.len(), 6, "records: {r:?}");
        assert_eq!(field(&r[0], "type"), "SOA");
        assert_eq!(field(&r[0], "name"), "example.com");
        assert_eq!(field(&r[1], "type"), "NS");
        assert_eq!(field(&r[1], "name"), "example.com"); // inherited @ owner
        assert_eq!(field(&r[2], "name"), "www.example.com");
        assert_eq!(field(&r[2], "type"), "A");
        assert_eq!(field(&r[2], "address"), "192.0.2.10");
        assert_eq!(field(&r[3], "name"), "www.example.com"); // whitespace-inherited
        assert_eq!(field(&r[3], "type"), "AAAA");
        assert_eq!(field(&r[3], "address"), "2001:db8::10");
        assert_eq!(field(&r[4], "type"), "CNAME");
        assert_eq!(field(&r[4], "name"), "api.example.com");
        assert_eq!(field(&r[5], "name"), "mail.example.com");
        assert_eq!(field(&r[5], "ttl"), "300");
    }

    #[test]
    fn registered_and_listed() {
        assert!(is_registered("zone"));
        assert!(list_input_formats().iter().any(|f| f.name == "zone"));
    }
}
