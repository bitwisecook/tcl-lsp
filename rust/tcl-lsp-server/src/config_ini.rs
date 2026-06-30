//! INI config-file parsing for full parity with the Python server's
//! configuration system (`shared/user_config.py` + `server/settings.py`),
//! minus the Python-interpreter selection.
//!
//! The Python server reads the *same* settings from three layers, lowest to
//! highest precedence:
//!
//! 1. the global user config — `config.ini`, `[global]` section
//!    (platform-native location, see [`crate::core_tcl_install::user_config_path`]),
//! 2. the editor's `workspace/configuration` payload,
//! 3. the per-workspace project config — `.tcl-lsp.ini`, `[project]` section.
//!
//! [`settings_from_ini`] parses one INI file into the *same* JSON shape the
//! editor delivers a `tclLsp` section as, so the existing
//! [`Backend::apply_global_config`](crate::Backend) applies a file layer
//! exactly as it applies the editor layer. [`merge_settings`] deep-merges the
//! layers (later wins, sections merged key-by-key) — the port of
//! `merge_settings_layers`.

use serde_json::{Map, Value};

/// Which precedence layer a file occupies, selecting its top-level section.
/// A `[global]` section in a project file (or `[project]` in the global file)
/// is ignored, matching Python — the section name must match the file's role
/// so copying a file between locations never silently changes its layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// The user `config.ini` — top-level keys live under `[global]`.
    Global,
    /// A project `.tcl-lsp.ini` — top-level keys live under `[project]`.
    Project,
}

impl Layer {
    fn top_section(self) -> &'static str {
        match self {
            Layer::Global => "global",
            Layer::Project => "project",
        }
    }
}

/// One parsed INI section: its name and ordered `(key, raw_value)` pairs.
struct Section {
    name: String,
    entries: Vec<(String, String)>,
}

/// Parse INI `content` into ordered sections, joining `configparser`-style
/// indented continuation lines with `\n`. Comment lines (`#` / `;`) and blank
/// lines are skipped; keys before any section header are ignored.
fn parse_ini(content: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    for raw_line in content.lines() {
        let line = raw_line.trim_end();
        let stripped = line.trim_start();
        if stripped.starts_with('[') && stripped.ends_with(']') {
            let name = stripped[1..stripped.len() - 1].trim().to_owned();
            sections.push(Section {
                name,
                entries: Vec::new(),
            });
            continue;
        }
        let Some(section) = sections.last_mut() else {
            continue;
        };
        // Continuation: an indented, non-empty line continues the current key's
        // value — configparser semantics. Keys are written at the section's base
        // (unindented) column, so an indented line is always a continuation,
        // even when it contains `:` / `=` (e.g. a `mylib::send` command name or
        // a regex pattern with a colon).
        let indented = line.starts_with([' ', '\t']);
        // No key to continue (indented line right after a header) falls through
        // and is treated as a normal `key = value` if it parses.
        if indented
            && !stripped.is_empty()
            && let Some(last) = section.entries.last_mut()
        {
            last.1.push('\n');
            last.1.push_str(stripped);
            continue;
        }
        if stripped.is_empty() || stripped.starts_with('#') || stripped.starts_with(';') {
            continue;
        }
        // `key = value` or `key: value`.
        if let Some(idx) = stripped.find(['=', ':']) {
            let key = stripped[..idx].trim().to_owned();
            let value = stripped[idx + 1..].trim().to_owned();
            if !key.is_empty() {
                section.entries.push((key, value));
            }
        }
    }
    sections
}

fn section_value<'a>(sections: &'a [Section], name: &str, key: &str) -> Option<&'a str> {
    sections
        .iter()
        .find(|s| s.name == name)?
        .entries
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn has_section(sections: &[Section], name: &str) -> bool {
    sections.iter().any(|s| s.name == name)
}

/// Split a comma-or-whitespace-separated value into tokens (Python's
/// `_parse_comma_list`).
fn parse_comma_list(raw: &str) -> Vec<String> {
    raw.replace(',', " ")
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Split a newline-list value (configparser continuation) into trimmed,
/// non-empty lines; falls back to comma-splitting a one-liner — the rule
/// `_read_top_level_section` uses for `libraryPaths`.
fn parse_path_list(raw: &str) -> Vec<String> {
    let lines: Vec<String> = raw
        .lines()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() <= 1 && raw.contains(',') {
        return parse_comma_list(raw);
    }
    lines
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Some(true),
        "false" | "no" | "0" | "off" => Some(false),
        _ => None,
    }
}

/// Parse one INI file's `content` into the `tclLsp`-section JSON shape the
/// editor delivers (so [`Backend::apply_global_config`](crate::Backend) can
/// apply it). `layer` selects the `[global]` vs `[project]` top-level section.
///
/// Mirrors `shared/user_config.py::get_all_settings`. Only keys actually
/// present are emitted, so absent settings keep their built-in defaults.
#[must_use]
pub fn settings_from_ini(content: &str, layer: Layer) -> Value {
    let sections = parse_ini(content);
    let mut out = Map::new();

    // Top-level section ([global] / [project]): dialect, extraCommands,
    // libraryPaths.
    let top = layer.top_section();
    if let Some(dialect) = section_value(&sections, top, "dialect") {
        let dialect = dialect.trim();
        if !dialect.is_empty() {
            out.insert("dialect".to_owned(), Value::String(dialect.to_owned()));
        }
    }
    if let Some(raw) = section_value(&sections, top, "extraCommands") {
        let cmds = parse_comma_list(raw);
        if !cmds.is_empty() {
            out.insert(
                "extraCommands".to_owned(),
                Value::Array(cmds.into_iter().map(Value::String).collect()),
            );
        }
    }
    if let Some(raw) = section_value(&sections, top, "libraryPaths") {
        let paths = parse_path_list(raw);
        if !paths.is_empty() {
            out.insert(
                "libraryPaths".to_owned(),
                Value::Array(paths.into_iter().map(Value::String).collect()),
            );
        }
    }

    insert_diagnostics(&sections, &mut out);
    insert_optimiser(&sections, &mut out);

    // [shimmer] / [xcDiagnostics]: enabled.
    for sect in ["shimmer", "xcDiagnostics"] {
        if let Some(b) = section_value(&sections, sect, "enabled").and_then(parse_bool) {
            out.insert(sect.to_owned(), json_enabled(b));
        }
    }

    // [features]: every key → bool.
    if let Some(section) = sections.iter().find(|s| s.name == "features") {
        let mut feat = Map::new();
        for (k, v) in &section.entries {
            if let Some(b) = parse_bool(v) {
                feat.insert(k.clone(), Value::Bool(b));
            }
        }
        if !feat.is_empty() {
            out.insert("features".to_owned(), Value::Object(feat));
        }
    }

    insert_formatting(&sections, &mut out);

    // [style] `line_length` → the W111 threshold; `nonAscii` → the W108
    // non-ASCII mode.  These are distinct from the formatter width above,
    // mirroring the Python `[style]` section mapping in `shared/user_config.py`.
    let mut style = Map::new();
    if let Some(len) =
        section_value(&sections, "style", "line_length").and_then(|v| v.trim().parse::<u64>().ok())
    {
        style.insert("lineLength".to_owned(), Value::from(len));
    }
    if let Some(mode) = section_value(&sections, "style", "nonAscii") {
        let mode = mode.trim();
        if !mode.is_empty() {
            style.insert("nonAscii".to_owned(), Value::String(mode.to_owned()));
        }
    }
    if !style.is_empty() {
        out.insert("style".to_owned(), Value::Object(style));
    }

    Value::Object(out)
}

/// `{ "enabled": b }` — the shape the `[shimmer]` / `[xcDiagnostics]` sections
/// map onto.
fn json_enabled(b: bool) -> Value {
    let mut m = Map::new();
    m.insert("enabled".to_owned(), Value::Bool(b));
    Value::Object(m)
}

/// `[diagnostics]`: `disabled` codes → `{CODE: false}` plus
/// `generic_variable_patterns` → `genericVariablePatterns`.
fn insert_diagnostics(sections: &[Section], out: &mut Map<String, Value>) {
    if !has_section(sections, "diagnostics") {
        return;
    }
    let mut diag = Map::new();
    if let Some(raw) = section_value(sections, "diagnostics", "disabled") {
        for code in parse_comma_list(raw) {
            diag.insert(code, Value::Bool(false));
        }
    }
    if let Some(raw) = section_value(sections, "diagnostics", "generic_variable_patterns") {
        let patterns: Vec<Value> = raw
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| Value::String(l.to_owned()))
            .collect();
        if !patterns.is_empty() {
            diag.insert("genericVariablePatterns".to_owned(), Value::Array(patterns));
        }
    }
    if !diag.is_empty() {
        out.insert("diagnostics".to_owned(), Value::Object(diag));
    }
}

/// `[optimiser]`: `enabled`, `profile`, and `disabled` codes → `{CODE: false}`.
fn insert_optimiser(sections: &[Section], out: &mut Map<String, Value>) {
    if !has_section(sections, "optimiser") {
        return;
    }
    let mut opt = Map::new();
    if let Some(b) = section_value(sections, "optimiser", "enabled").and_then(parse_bool) {
        opt.insert("enabled".to_owned(), Value::Bool(b));
    }
    if let Some(profile) = section_value(sections, "optimiser", "profile") {
        let profile = profile.trim();
        if !profile.is_empty() {
            opt.insert("profile".to_owned(), Value::String(profile.to_owned()));
        }
    }
    if let Some(raw) = section_value(sections, "optimiser", "disabled") {
        for code in parse_comma_list(raw) {
            opt.insert(code, Value::Bool(false));
        }
    }
    if !opt.is_empty() {
        out.insert("optimiser".to_owned(), Value::Object(opt));
    }
}

/// `[formatting]` → the formatter config object (camelCase editor keys). Each
/// `snake_case` INI key maps to the matching `FormatterConfig` field the server
/// consumes; integers, booleans, and strings are coerced per key.
fn insert_formatting(sections: &[Section], out: &mut Map<String, Value>) {
    let mut fmt = Map::new();
    // Integer-valued keys → camelCase.
    for (ini_key, json_key) in [
        ("max_line_length", "maxLineLength"),
        ("goal_line_length", "goalLineLength"),
        ("indent_size", "indentSize"),
        ("continuation_indent", "continuationIndent"),
        ("blank_lines_between_procs", "blankLinesBetweenProcs"),
        ("blank_lines_between_blocks", "blankLinesBetweenBlocks"),
        ("max_consecutive_blank_lines", "maxConsecutiveBlankLines"),
    ] {
        if let Some(n) = section_value(sections, "formatting", ini_key)
            .and_then(|v| v.trim().parse::<u64>().ok())
        {
            fmt.insert(json_key.to_owned(), Value::from(n));
        }
    }
    // `max_line_length` also feeds the legacy `lineLength` the server reads for
    // the willSaveWaitUntil resolved width.
    if let Some(n) = fmt.get("maxLineLength").cloned() {
        fmt.insert("lineLength".to_owned(), n);
    }
    // String-valued keys.
    for (ini_key, json_key) in [
        ("indent_style", "indentStyle"),
        ("brace_style", "braceStyle"),
        ("line_ending", "lineEnding"),
    ] {
        if let Some(s) = section_value(sections, "formatting", ini_key) {
            let s = s.trim();
            if !s.is_empty() {
                fmt.insert(json_key.to_owned(), Value::String(s.to_owned()));
            }
        }
    }
    // Boolean-valued keys.
    for (ini_key, json_key) in [
        ("space_between_braces", "spaceBetweenBraces"),
        ("space_after_comment_hash", "spaceAfterCommentHash"),
        ("trim_trailing_whitespace", "trimTrailingWhitespace"),
        ("enforce_braced_variables", "enforceBracedVariables"),
        ("ensure_final_newline", "ensureFinalNewline"),
        ("expand_single_line_bodies", "expandSingleLineBodies"),
    ] {
        if let Some(b) = section_value(sections, "formatting", ini_key).and_then(parse_bool) {
            fmt.insert(json_key.to_owned(), Value::Bool(b));
        }
    }
    if !fmt.is_empty() {
        out.insert("formatting".to_owned(), Value::Object(fmt));
    }
}

/// Deep-merge two `tclLsp`-shape settings objects: `high` overrides `low`, but
/// nested objects (sections) merge key-by-key rather than replace wholesale.
/// The port of `merge_settings_layers` — call lowest-priority first.
#[must_use]
pub fn merge_settings(low: &Value, high: &Value) -> Value {
    match (low, high) {
        (Value::Object(lo), Value::Object(hi)) => {
            let mut out = lo.clone();
            for (k, hv) in hi {
                let merged = match out.get(k) {
                    Some(lv) if lv.is_object() && hv.is_object() => merge_settings(lv, hv),
                    _ => hv.clone(),
                };
                out.insert(k.clone(), merged);
            }
            Value::Object(out)
        }
        // A non-object high layer (or low not an object) replaces.
        _ => high.clone(),
    }
}

#[cfg(test)]
mod tests;
