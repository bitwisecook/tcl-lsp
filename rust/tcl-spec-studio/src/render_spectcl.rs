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

//! Render a draft as a `SpecTcl` `.tclspec` pack — the inverse of the loader.
//!
//! The loader ([`crate::spectcl`]) turns a pack into live `CommandSpec`s, which
//! the studio seeds drafts from. This module closes the loop: draft in,
//! `.tclspec` text out, so a shipped spec can be *exported* to a pack, a pack
//! can be round-tripped through the studio's editor, and the studio's DSL pane
//! has something to show. The syntax is the frozen one in
//! `docs/design/spec-dsl-examples/README.md`, and the eleven `*.tclspec` ports
//! beside it are the formatting exemplars: one-screen simple commands, row
//! statements rather than nested blocks, `\`-continued option rows with the
//! prose flag last.
//!
//! ## Three rules the renderer follows
//!
//! 1. **Only non-default fields are emitted.** Every key is compared against
//!    [`crate::draft::default_command_draft`] /
//!    [`crate::draft::default_subcommand_draft`], so a rendered pack says
//!    exactly what its author set and nothing else — the same promise
//!    [`crate::render_rs`] makes with `..CommandSpec::DEFAULT`.
//! 2. **Catalogue spellings are verbatim.** A draft already holds the Rust
//!    variant spelling for every enum and flag (`VarWrite`, `BYTE_COMPILED`,
//!    `tcl9.0`), which is exactly what the DSL wants, so no field is
//!    re-spelled on the way out.
//! 3. **Nothing is dropped silently.** A field the draft holds but the DSL
//!    cannot carry is emitted as a `# TODO(spectcl):` line naming the field,
//!    its documented spelling, and why the value did not survive — see
//!    [`GAPS`]. A field whose value *names engine code* is emitted in the
//!    documented `-native ID` form, which is not a loss: the loader installs
//!    its family's abstention either way, and the draft records both the same.
//!
//! ## What round-trips
//!
//! `tests/spectcl_roundtrip.rs` is the gate: every command of every browsable
//! dialect is drafted, rendered here, loaded back through
//! [`crate::spectcl::load_pack`], and the two drafts are compared field by
//! field. Every field that legitimately differs is one of the [`GAPS`] below,
//! and the test fails on anything else.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Map, Value};

use crate::draft::{
    self, Draft, OPTION_DEPRECATION_FIX_HOOK_KEY, OPTION_HOOK_KEY, SOURCE_DIALECT_KEY,
};

/// The DSL **vocabulary** version a rendered pack declares — the word after
/// the pack name in `speclib <pack> <version> { … }`.
pub const DSL_VERSION: &str = "1.0";

/// Column the renderer tries to keep rows inside before continuing a row with
/// a `\`, matching the ports' own wrapping.
const WRAP_COLUMN: usize = 92;

// ---------------------------------------------------------------------------
// Why a field did not survive
// ---------------------------------------------------------------------------

/// Why a draft key cannot be written as SpecTcl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapKind {
    /// The DSL has a spelling and the loader reads it, but the **draft** holds
    /// no value to write: seeding records the field as
    /// [`UNRENDERABLE`](crate::draft::UNRENDERABLE_KEY) — "set, but the
    /// defining expression could not be recovered" — because the spec field is
    /// a function pointer or a reference to a named `&'static` descriptor.
    DraftOpaque,
    /// The draft holds the value and the design memo documents a spelling, but
    /// the loader has no reader for that property word yet, so writing it
    /// would only produce an unknown-property notice.
    LoaderGap,
    /// The design excludes the field from what a pack may author at all.
    Excluded,
}

impl GapKind {
    /// One clause explaining this kind, used in the `TODO` line.
    const fn why(self) -> &'static str {
        match self {
            Self::DraftOpaque => {
                "a draft records only that the field is set, never the expression that set it"
            }
            Self::LoaderGap => "the loader has no reader for that property word yet",
            Self::Excluded => "a pack may not author this field",
        }
    }
}

/// One field the renderer cannot carry, with its documented DSL spelling.
#[derive(Debug, Clone, Copy)]
pub struct Gap {
    /// The draft / schema key.
    pub key: &'static str,
    /// How `docs/design/spec-dsl-examples/README.md`'s coverage matrix spells
    /// the field, or `""` where it spells none.
    pub spelling: &'static str,
    /// Why the value did not survive.
    pub kind: GapKind,
}

/// Every field a draft can hold that a rendered pack cannot carry.
///
/// This is the renderer's half of the round-trip contract: the gate in
/// `tests/spectcl_roundtrip.rs` allows a rendered-then-reloaded draft to differ
/// from its source **only** on these keys, and reports any other difference as
/// a failure.
pub const GAPS: &[Gap] = &[
    // --- the value is not in the draft ------------------------------------
    Gap {
        key: "frame_effect",
        spelling: "frame_effect -level-word W -layout L",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "semantic_operation",
        spelling: "semantic_operation Invoke|{Intrinsic ID}|{StructuredLowering ID}",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "world_effects",
        spelling: "world_effects none|NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "state_transitions",
        spelling: "state_transitions NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "event_requires",
        spelling: "event_requires NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "case_list",
        spelling: "case_list NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "definition_body",
        spelling: "definition_body NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "object_class",
        spelling: "object_class NAME ?-superclass {…}? ?-allow-unknown? { … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "body_scope",
        spelling: "body_scope NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "bpf_op",
        spelling: "bpf_op -native ID",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "data_collection",
        spelling: "data_collection -native ID",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "event_requirement_forms",
        spelling: "event_requirement_form {word …} ?-only-in {E …}? ?{ … }?",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "side_switch_target",
        spelling: "side_switch_target Client|Server",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "event_handler_priority",
        spelling: "event_handler_priority -default N ?-warn-implicit?",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "result_stability",
        spelling: "result_stability Unknown|ReferentiallyTransparent|Volatile|{ReadsVersionedWorld {D …}}",
        kind: GapKind::DraftOpaque,
    },
    // --- the draft has it; the loader does not read it yet -----------------
    Gap {
        key: "return_elements",
        spelling: "return_elements {VARIANT payload …}",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "var_elements_effect",
        spelling: "var_elements_effect {VARIANT payload …}",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "representation_effect",
        spelling: "representation_effect {VARIANT payload …}",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "default_form_first_word",
        spelling: "default_form_first_word Integer",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "setter_constraints",
        spelling: "setter_constraint N -prefix P -code CODE -message {…}",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "format_string_type",
        spelling: "format_string_type Sprintf|Clock|Binary|Regsub",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "deprecation_fix",
        spelling: "deprecation_fix -replace WORD -description {…} -safety S",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "byte_array_payload",
        spelling: "byte_array_payload -replace-data-index N ?-message-flag-shift?",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "defines_symbol",
        spelling: "defines_symbol -name-arg N ?-detail-arg N? ?-requires-arg N? -kind KIND",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: OPTION_DEPRECATION_FIX_HOOK_KEY,
        spelling: "option … -deprecation-fix …",
        kind: GapKind::LoaderGap,
    },
    // --- excluded by design -------------------------------------------------
    Gap {
        key: "completion",
        spelling: "",
        kind: GapKind::Excluded,
    },
    Gap {
        key: "dispatch_dependencies",
        spelling: "",
        kind: GapKind::Excluded,
    },
    Gap {
        key: "command_forms",
        spelling: "",
        kind: GapKind::Excluded,
    },
    Gap {
        key: "subcommand_forms",
        spelling: "",
        kind: GapKind::Excluded,
    },
];

/// The [`Gap`] for `key`, if the renderer cannot carry it.
#[must_use]
pub fn gap(key: &str) -> Option<&'static Gap> {
    GAPS.iter().find(|gap| gap.key == key)
}

// ---------------------------------------------------------------------------
// Tcl words
// ---------------------------------------------------------------------------

/// Whether `text` can be written as a bare word.
fn bare_safe(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('#')
        && text.chars().all(|c| {
            c.is_ascii_alphanumeric() || "-_.:/+@=,<>*?!%^~|'`".contains(c) && c.is_ascii()
        })
}

/// Whether `{text}` is a well-formed braced word whose value is `text` byte
/// for byte.
///
/// The scan is Tcl's own: a backslash escapes the next character (so `\{` and
/// `\}` do not nest), the depth may never go negative, and a trailing
/// backslash would escape the closing brace. Anything else has to take the
/// quoted form.
fn brace_safe(text: &str) -> bool {
    let mut depth = 0i32;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if chars.next().is_none() {
                    return false;
                }
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

/// The quoted spelling of `text`, escaping everything Tcl would substitute.
///
/// The fallback for prose a braced word cannot hold — README's "one place the
/// format will bite an author writing about Tcl syntax".
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        if matches!(c, '\\' | '"' | '$' | '[') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// One word carrying `text` verbatim, in the lightest spelling that works.
fn word(text: &str) -> String {
    if bare_safe(text) {
        text.to_owned()
    } else if brace_safe(text) {
        format!("{{{text}}}")
    } else {
        quoted(text)
    }
}

/// A braced word carrying `text` verbatim — the prose and block spelling.
///
/// Unlike [`word`] this never degrades to a bare word, because a `detail` or
/// `description` is braced in every port whether or not it needs to be.
fn braced(text: &str) -> String {
    if brace_safe(text) {
        format!("{{{text}}}")
    } else {
        quoted(text)
    }
}

/// A Tcl list word holding `items`, each element carrying its text verbatim.
fn list_word<S: AsRef<str>>(items: &[S]) -> String {
    let inner = items
        .iter()
        .map(|item| word(item.as_ref()))
        .collect::<Vec<_>>()
        .join(" ");
    braced(&inner)
}

/// The elements of a JSON string array as a list word.
fn str_list_word(value: &Value) -> String {
    let items: Vec<&str> = as_array(value).iter().filter_map(Value::as_str).collect();
    list_word(&items)
}

fn as_array(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

// ---------------------------------------------------------------------------
// The output buffer
// ---------------------------------------------------------------------------

/// An indented line buffer with a one-slot blank-line separator.
#[derive(Debug, Default)]
struct Out {
    text: String,
    indent: usize,
    /// A blank line is owed before the next line, unless nothing follows.
    pending_blank: bool,
}

impl Out {
    fn line(&mut self, text: &str) {
        if self.pending_blank && !self.text.is_empty() {
            self.text.push('\n');
        }
        self.pending_blank = false;
        if !text.is_empty() {
            for _ in 0..self.indent {
                self.text.push_str("    ");
            }
            self.text.push_str(text);
        }
        self.text.push('\n');
    }

    /// Ask for one blank line before whatever comes next.
    fn gap(&mut self) {
        if !self.text.is_empty() {
            self.pending_blank = true;
        }
    }

    fn raw(&mut self, text: &str) {
        if self.pending_blank && !self.text.is_empty() {
            self.text.push('\n');
        }
        self.pending_blank = false;
        self.text.push_str(text);
    }

    fn indented<T>(&mut self, body: impl FnOnce(&mut Self) -> T) -> T {
        self.indent += 1;
        let out = body(self);
        self.indent -= 1;
        out
    }

    /// A `# …` comment, wrapped to a readable width.
    fn comment(&mut self, text: &str) {
        let width = 78usize.saturating_sub(self.indent * 4);
        let mut line = String::new();
        for token in text.split_whitespace() {
            if !line.is_empty() && line.len() + 1 + token.len() > width {
                let done = std::mem::take(&mut line);
                self.line(&format!("# {done}"));
                line.push_str("  ");
            }
            if !line.is_empty() && !line.ends_with("  ") {
                line.push(' ');
            }
            line.push_str(token);
        }
        if !line.is_empty() {
            self.line(&format!("# {line}"));
        }
    }

    /// One statement written as `word word …`, continued with `\` when it runs
    /// past [`WRAP_COLUMN`].
    ///
    /// `break_before` names the flag a long row is split in front of — the
    /// ports put the prose flag on its own continuation line and everything
    /// else on the first.
    fn row(&mut self, words: &[String], break_before: &str) {
        let flat = words.join(" ");
        if self.indent * 4 + flat.len() <= WRAP_COLUMN || break_before.is_empty() {
            self.line(&flat);
            return;
        }
        let Some(at) = words.iter().position(|w| w == break_before) else {
            self.line(&flat);
            return;
        };
        if at == 0 {
            self.line(&flat);
            return;
        }
        self.line(&format!("{} \\", words[..at].join(" ")));
        self.indented(|out| out.line(&words[at..].join(" ")));
    }
}

// ---------------------------------------------------------------------------
// Shared `values` tables
// ---------------------------------------------------------------------------

/// The pack-level `values NAME { … }` tables a render needs.
///
/// An `arg`/`option` row can spell a plain value set inline (`-values {a b}`),
/// but a value carrying a `detail`, a Tcl-version floor, or a completion code
/// has nowhere to put them on the row — so those become a shared table,
/// exactly as `string.tclspec`'s `is-classes` and `return.tclspec`'s
/// `return-codes` do.
#[derive(Debug, Default)]
struct ValueTables {
    /// `(name, values array)`, in declaration order.
    tables: Vec<(String, Value)>,
}

impl ValueTables {
    /// Register `values` under a name derived from `hint`, reusing an
    /// identical table when one is already declared.
    fn intern(&mut self, hint: &str, values: &Value) -> String {
        if let Some((name, _)) = self.tables.iter().find(|(_, table)| table == values) {
            return name.clone();
        }
        let base = slug(hint);
        let mut name = base.clone();
        let mut n = 2;
        while self.tables.iter().any(|(taken, _)| *taken == name) {
            name = format!("{base}-{n}");
            n += 1;
        }
        self.tables.push((name.clone(), values.clone()));
        name
    }

    fn render(&self, out: &mut Out) {
        for (name, values) in &self.tables {
            out.gap();
            out.line(&format!("values {name} {{"));
            out.indented(|out| {
                for value in as_array(values) {
                    let mut row = vec!["value".to_owned(), word(str_of(&value["value"]))];
                    if let Some(version) = value["min_tcl"].as_str() {
                        row.push("-min-tcl".to_owned());
                        row.push(tcl_version_word(version));
                    }
                    if let Some(code) = value["code"].as_i64() {
                        row.push("-code".to_owned());
                        row.push(code.to_string());
                    }
                    let detail = str_of(&value["detail"]);
                    if !detail.is_empty() {
                        row.push("-detail".to_owned());
                        row.push(braced(detail));
                    }
                    out.row(&row, "-detail");
                }
            });
            out.line("}");
        }
    }
}

/// A name safe to use as a pack-level declaration word.
fn slug(hint: &str) -> String {
    let mut out = String::with_capacity(hint.len());
    let mut last_dash = false;
    for c in hint.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_end_matches('-').to_owned();
    if trimmed.is_empty() {
        "values".to_owned()
    } else {
        trimmed
    }
}

/// The catalogue spelling of a `min_tcl` floor as the DSL's own version word.
///
/// A draft holds the `TclVersion` variant (`V8_6`); the DSL takes `tcl8.6`,
/// which is also what the shipped table's own key is.
fn tcl_version_word(variant: &str) -> String {
    let digits: String = variant
        .trim_start_matches('V')
        .chars()
        .map(|c| if c == '_' { '.' } else { c })
        .collect();
    format!("tcl{digits}")
}

fn str_of(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Rust-expression fields the draft *can* recover
// ---------------------------------------------------------------------------

/// Split `text` on top-level `sep`, honouring `{}` / `[]` / `()` nesting and
/// Rust string literals.
fn split_top(text: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut start = 0usize;
    for (at, c) in text.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' | '[' | '(' if !in_string => depth += 1,
            '}' | ']' | ')' if !in_string => depth -= 1,
            c if c == sep && depth == 0 && !in_string => {
                out.push(text[start..at].trim());
                start = at + c.len_utf8();
            }
            _ => {}
        }
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// The body of `prefix … suffix`, or `None` when `text` is not that shape.
fn between<'a>(text: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    text.trim()
        .strip_prefix(prefix)?
        .strip_suffix(suffix)
        .map(str::trim)
}

/// The `field: value` pairs of a `Name { … }` struct literal.
fn struct_fields(text: &str) -> Option<BTreeMap<&str, &str>> {
    let open = text.find('{')?;
    let body = text[open + 1..].trim_end().strip_suffix('}')?;
    let mut fields = BTreeMap::new();
    for item in split_top(body, ',') {
        let (key, value) = item.split_once(':')?;
        fields.insert(key.trim(), value.trim());
    }
    Some(fields)
}

/// The items of an `&[ … ]` slice literal.
fn slice_items(text: &str) -> Option<Vec<&str>> {
    let body = between(text, "&[", "]")?;
    Some(split_top(body, ','))
}

/// The value of a Rust string literal, unescaped.
fn rust_str(text: &str) -> Option<String> {
    let body = between(text, "\"", "\"")?;
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            other => out.push(other),
        }
    }
    Some(out)
}

/// The variant word of a `Path::Variant` expression, payload included.
fn variant_of(text: &str) -> &str {
    text.trim().rsplit("::").next().unwrap_or("").trim()
}

/// `Some(inner)` → `inner`; anything else is returned unchanged.
fn unwrap_some(text: &str) -> Option<&str> {
    between(text, "Some(", ")")
}

/// The dialect member names behind a rendered `DialectSet` expression.
///
/// [`crate::render_rs::dialect_set`] writes the readable aggregates
/// (`DialectSet::ALL_TCL.union(DialectSet::IRULES)`); this reads them back so
/// an option constraint keeps its gate.
fn dialect_names(expr: &str) -> Option<Vec<&'static str>> {
    const CONSTANTS: &[(&str, &[&str])] = &[
        (
            "ALL_TCL",
            &["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"],
        ),
        ("TCL85_PLUS", &["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]),
        ("TCL86_PLUS", &["tcl8.6", "tcl9.0", "tcl9.1"]),
        ("TCL90_PLUS", &["tcl9.0", "tcl9.1"]),
        ("TCL84", &["tcl8.4"]),
        ("TCL85", &["tcl8.5"]),
        ("TCL86", &["tcl8.6"]),
        ("TCL90", &["tcl9.0"]),
        ("TCL91", &["tcl9.1"]),
        ("IRULES", &["f5-irules"]),
        ("IAPPS", &["f5-iapps"]),
        ("TK", &["tk"]),
        ("EXPECT", &["expect"]),
        ("BPF", &["bpf"]),
        ("TMSH", &["f5-tmsh"]),
        ("BIGIP", &["f5-bigip"]),
    ];
    let mut names: Vec<&'static str> = Vec::new();
    for part in expr.replace(".union(", " ").replace(')', " ").split_whitespace() {
        let key = part.trim().rsplit("::").next()?;
        let (_, members) = CONSTANTS.iter().find(|(name, _)| *name == key)?;
        for member in *members {
            if !names.contains(member) {
                names.push(member);
            }
        }
    }
    (!names.is_empty()).then_some(names)
}

/// `var_write_typing`'s four spellings, from its rendered Rust expression.
fn var_write_typing_word(expr: &str) -> Option<String> {
    let variant = variant_of(expr.split(['(', '{']).next()?);
    Some(match variant {
        "ReturnValue" | "Destructured" => variant.to_owned(),
        "Fixed" => {
            let payload = between(expr, "VarWriteTyping::Fixed(", ")")?;
            format!("{{Fixed {}}}", variant_of(payload))
        }
        "ElementsOf" => {
            let fields = struct_fields(expr)?;
            format!("{{ElementsOf {}}}", fields.get("container_arg")?)
        }
        _ => return None,
    })
}

/// `repeat ROLE …` rows from a rendered `&[RepeatedArgLayout]`.
fn repeat_rows(expr: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let fields = struct_fields(item)?;
        let mut row = vec![
            "repeat".to_owned(),
            variant_of(fields.get("role")?).to_owned(),
        ];
        let start = *fields.get("start")?;
        if start != "0" {
            row.push("-from".to_owned());
            row.push(start.to_owned());
        }
        let stride = *fields.get("stride")?;
        if stride != "1" {
            row.push("-stride".to_owned());
            row.push(stride.to_owned());
        }
        let exclude = *fields.get("exclude_trailing")?;
        if exclude != "0" {
            row.push("-exclude-trailing".to_owned());
            row.push(exclude.to_owned());
        }
        if *fields.get("optional_leading_word")? == "true" {
            row.push("-optional-leading".to_owned());
        }
        if *fields.get("conditional_binding")? == "true" {
            row.push("-conditional".to_owned());
        }
        rows.push(row);
    }
    Some(rows)
}

/// `option_conflict {…}` rows from a rendered `&[OptionConstraint]`.
fn option_conflict_rows(expr: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let fields = struct_fields(item)?;
        let options: Option<Vec<String>> = slice_items(fields.get("options")?)?
            .into_iter()
            .map(rust_str)
            .collect();
        let mut row = vec!["option_conflict".to_owned(), list_word(&options?)];
        if let Some(inner) = unwrap_some(fields.get("dialects")?) {
            row.push("-dialects".to_owned());
            row.push(list_word(&dialect_names(inner)?));
        }
        rows.push(row);
    }
    Some(rows)
}

/// `oo_context_fact WORD FACT` rows from a rendered fact table.
fn oo_context_fact_rows(expr: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let pair = between(item, "(", ")")?;
        let parts = split_top(pair, ',');
        let [name, fact] = parts.as_slice() else {
            return None;
        };
        rows.push(vec![
            "oo_context_fact".to_owned(),
            word(&rust_str(name)?),
            variant_of(fact).to_owned(),
        ]);
    }
    Some(rows)
}

/// The `binds_handle` row's list argument, from a rendered
/// `Some(&HandleBindingSpec { … })`.
fn binds_handle_word(expr: &str) -> Option<String> {
    let fields = struct_fields(unwrap_some(expr)?)?;
    let source = |text: &str| -> Option<String> {
        let variant = variant_of(text.split('(').next()?);
        let payload = between(text, &format!("{variant}("), ")")?;
        Some(match variant {
            "Implicit" => format!("{{Implicit {}}}", word(&rust_str(payload)?)),
            other => format!("{{{other} {payload}}}"),
        })
    };
    let mut parts = vec![
        "-name-from".to_owned(),
        source(fields.get("name_from")?)?,
        "-class-from".to_owned(),
        source(fields.get("class_from")?)?,
    ];
    if let Some(keyword) = unwrap_some(fields.get("keyword")?) {
        let inner = struct_fields(keyword)?;
        parts.push("-keyword".to_owned());
        parts.push(format!(
            "{{{} {}}}",
            inner.get("at")?,
            word(&rust_str(inner.get("word")?)?)
        ));
    }
    Some(braced(&parts.join(" ")))
}

/// `versioned_arg_value N VALUE …` rows from a rendered gate table.
fn versioned_arg_value_rows(expr: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let fields = struct_fields(item)?;
        let mut row = vec![
            "versioned_arg_value".to_owned(),
            (*fields.get("index")?).to_owned(),
            word(&rust_str(fields.get("value")?)?),
        ];
        let lifecycle = struct_fields(fields.get("lifecycle")?)?;
        for (field, flag) in [
            ("introduced", "-introduced"),
            ("deprecated", "-deprecated"),
            ("retired", "-retired"),
        ] {
            if let Some(inner) = unwrap_some(lifecycle.get(field)?) {
                row.push(flag.to_owned());
                row.push(word(&rust_str(inner)?));
            }
        }
        rows.push(row);
    }
    Some(rows)
}

// ---------------------------------------------------------------------------
// Field-level emission
// ---------------------------------------------------------------------------

/// Everything one command or subcommand body needs while rendering.
struct Ctx<'a> {
    /// The default draft the body's fields are compared against.
    defaults: &'a Draft,
    /// The `-native` id prefix — `lsort`, or `string::is` for a subcommand.
    scope: String,
    tables: &'a mut ValueTables,
}

impl Ctx<'_> {
    /// Whether `draft`'s `key` differs from the default a fresh spec has.
    fn set(&self, draft: &Draft, key: &str) -> bool {
        let value = draft.get(key);
        value.is_some() && value != self.defaults.get(key)
    }

    fn native(&self, field: &str) -> Vec<String> {
        vec![
            field.to_owned(),
            "-native".to_owned(),
            format!("{}::{field}", self.scope),
        ]
    }
}

/// Whether `draft` records `key` as one whose defining expression seeding
/// could not recover.
fn unrecovered(draft: &Draft, key: &str) -> bool {
    draft
        .get(draft::UNRENDERABLE_KEY)
        .map(as_array)
        .is_some_and(|keys| keys.iter().any(|k| k.as_str() == Some(key)))
}

/// The `# TODO(spectcl):` line for a field that did not survive.
fn todo(out: &mut Out, key: &str) {
    let Some(gap) = gap(key) else {
        out.comment(&format!(
            "TODO(spectcl): `{key}` is set on the source spec and has no DSL spelling."
        ));
        return;
    };
    let spelling = if gap.spelling.is_empty() {
        String::new()
    } else {
        format!(" The DSL spells it `{}`, but ", gap.spelling)
    };
    let joiner = if spelling.is_empty() { " — " } else { "" };
    out.comment(&format!(
        "TODO(spectcl): `{key}` is set on the source spec.{spelling}{joiner}{}.",
        gap.kind.why()
    ));
}

/// Emit `key` when it is set, as a one-value property statement.
fn scalar(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str, value: String) {
    if ctx.set(draft, key) {
        out.line(&format!("{key} {value}"));
    }
}

/// Emit a boolean property in the DSL's bare-word / explicit-`no` spelling.
fn flag(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if !ctx.set(draft, key) {
        return;
    }
    if draft[key].as_bool() == Some(false) {
        out.line(&format!("{key} no"));
    } else {
        out.line(key);
    }
}

/// Emit a text / opt-text property.
fn text(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(value) = draft[key].as_str()
    {
        out.line(&format!("{key} {}", word(value)));
    }
}

/// Emit an enum property, whose draft value is already the catalogue spelling.
fn enum_word(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(value) = draft[key].as_str()
    {
        out.line(&format!("{key} {value}"));
    }
}

/// Emit a flag-set / dialect-set property.
fn set_word(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key) && draft[key].is_array() {
        out.line(&format!("{key} {}", str_list_word(&draft[key])));
    }
}

/// Emit a `&'static [&'static str]` property.
fn text_list(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    set_word(out, ctx, draft, key);
}

/// Emit an index-list property, keeping the tri-state (`{}` is *declared
/// empty*, absent is unset).
fn index_list(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if !ctx.set(draft, key) {
        return;
    }
    let Some(items) = draft[key].as_array() else {
        return;
    };
    let indices: Vec<String> = items.iter().map(ToString::to_string).collect();
    out.line(&format!("{key} {{{}}}", indices.join(" ")));
}

/// Emit a count property.
fn count(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(n) = draft[key].as_u64()
    {
        out.line(&format!("{key} {n}"));
    }
}

/// Emit a `RustExpr` field that the DSL cannot carry, as a `TODO`.
fn gap_todo(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key) || unrecovered(draft, key) {
        todo(out, key);
    }
}

/// Emit a hook field in the documented `-native ID` form.
///
/// Not a loss: the loader installs the family's abstention for a `-native`
/// hook exactly as it does for a Tcl body, and seeding records the reloaded
/// field the same way it recorded the shipped one — "set, expression not
/// recovered".
fn native_hook(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if unrecovered(draft, key) {
        out.line(&ctx.native(key).join(" "));
    }
}

/// Emit a closed-catalogue compiler hook, whose draft value *is* the variant
/// name the DSL takes.
fn catalogue_hook(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(id) = draft[key].as_str()
    {
        out.line(&format!("{key} -native {id}"));
    }
}

/// `arity N`, `N..M`, `N..`, `..M`, `..`, plus `-step` / `-also`.
fn arity_row(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "arity") {
        return;
    }
    let value = &draft["arity"];
    let min = value["min"].as_u64().unwrap_or(0);
    let max = value["max"].as_u64();
    let step = value["step"].as_u64().unwrap_or(0);
    let also = value["also_exact"].as_u64();
    let range = match (min, max) {
        (m, Some(x)) if m == x && step == 0 => m.to_string(),
        (0, None) => "..".to_owned(),
        (m, None) => format!("{m}.."),
        (0, Some(x)) => format!("..{x}"),
        (m, Some(x)) => format!("{m}..{x}"),
    };
    let mut row = format!("arity {range}");
    if step != 0 {
        let _ = write!(row, " -step {step}");
    }
    if let Some(also) = also {
        let _ = write!(row, " -also {also}");
    }
    out.line(&row);
}

/// The per-argument rows: six schema keys, one statement per index.
///
/// Indices are visited in the order the tables themselves list them — roles
/// first, then the tables that only ever accompany them — so a spec that
/// declares its roles out of index order keeps that order on the way back in.
fn arg_rows(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft) {
    let table = |key: &str| -> Vec<Value> { as_array(draft.get(key).unwrap_or(&Value::Null)).to_vec() };
    let roles = table("arg_roles");
    let types = table("arg_types");
    let values = table("arg_values");
    let presentation = table("arg_presentation");
    let prefixes = table("command_prefixes");
    let closed: Vec<u64> = as_array(draft.get("closed_value_args").unwrap_or(&Value::Null))
        .iter()
        .filter_map(Value::as_u64)
        .collect();

    let mut order: Vec<u64> = Vec::new();
    let mut note = |index: Option<u64>, order: &mut Vec<u64>| {
        if let Some(index) = index
            && !order.contains(&index)
        {
            order.push(index);
        }
    };
    for entry in roles
        .iter()
        .chain(&types)
        .chain(&values)
        .chain(&presentation)
        .chain(&prefixes)
    {
        note(entry["index"].as_u64(), &mut order);
    }
    for index in &closed {
        note(Some(*index), &mut order);
    }

    let at = |entries: &[Value], index: u64| -> Option<Value> {
        entries
            .iter()
            .find(|entry| entry["index"].as_u64() == Some(index))
            .cloned()
    };

    for index in order {
        let mut row = vec!["arg".to_owned(), index.to_string()];
        if let Some(entry) = at(&roles, index) {
            row.push("-role".to_owned());
            row.push(str_of(&entry["role"]).to_owned());
        }
        if let Some(entry) = at(&types, index) {
            if let Some(expected) = entry["expected"].as_str() {
                row.push("-type".to_owned());
                row.push(expected.to_owned());
            }
            if entry["shimmers"].as_bool() == Some(true) {
                row.push("-shimmers".to_owned());
            }
            let transparent = as_array(&entry["transparent_from"]);
            if !transparent.is_empty() {
                row.push("-transparent".to_owned());
                row.push(str_list_word(&entry["transparent_from"]));
            }
        }
        if let Some(entry) = at(&values, index) {
            push_values(&mut row, ctx, &entry["values"], &format!("{}-arg{index}", ctx.scope));
        }
        if closed.contains(&index) {
            row.push("-closed".to_owned());
        }
        if let Some(entry) = at(&presentation, index) {
            row.push("-layout".to_owned());
            row.push(str_of(&entry["presentation"]).to_owned());
        }
        if let Some(entry) = at(&prefixes, index) {
            row.push("-appends".to_owned());
            row.push(appended_arity_word(&entry["arity"]));
        }
        out.row(&row, "-detail");
    }
}

/// `{Exactly N}` / `{AtLeast N}` / `Unknown`.
fn appended_arity_word(value: &Value) -> String {
    let kind = str_of(&value["kind"]);
    match value["n"].as_u64() {
        Some(n) => format!("{{{kind} {n}}}"),
        None => kind.to_owned(),
    }
}

/// Push `-values {…}` or `-values-from NAME` onto a row.
///
/// The inline form carries the value words and nothing else, so a table whose
/// rows have a detail, a version floor, or a completion code is hoisted to a
/// pack-level `values` declaration instead.
fn push_values(row: &mut Vec<String>, ctx: &mut Ctx<'_>, values: &Value, hint: &str) {
    let entries = as_array(values);
    if entries.is_empty() {
        return;
    }
    let plain = entries.iter().all(|entry| {
        str_of(&entry["detail"]).is_empty() && entry["min_tcl"].is_null() && entry["code"].is_null()
    });
    if plain {
        let words: Vec<&str> = entries.iter().map(|entry| str_of(&entry["value"])).collect();
        row.push("-values".to_owned());
        row.push(list_word(&words));
    } else {
        let name = ctx.tables.intern(hint, values);
        row.push("-values-from".to_owned());
        row.push(name);
    }
}

/// One `option NAME …` row.
fn option_row(out: &mut Out, ctx: &mut Ctx<'_>, option: &Value) {
    let name = str_of(&option["name"]);
    let mut row = vec!["option".to_owned(), word(name)];
    let value = &option["value"];
    if !value.is_null() {
        row.push("-takes".to_owned());
        row.push(word(str_of(&value["hint"])));

        let arity = &value["arity"];
        match str_of(&arity["kind"]) {
            "Fixed" => {
                row.push("-arity".to_owned());
                row.push(format!("{{Fixed {}}}", arity["n"].as_u64().unwrap_or(0)));
            }
            "Hook" => {
                row.push("-arity-hook".to_owned());
                row.push("-native".to_owned());
                row.push(format!("{}::{}::arity", ctx.scope, name.trim_start_matches('-')));
            }
            _ => {}
        }
        let role = str_of(&value["role"]);
        if role != "Value" {
            row.push("-role".to_owned());
            row.push(role.to_owned());
        }
        if let Some(also) = value["also_role"].as_str() {
            row.push("-also-role".to_owned());
            row.push(also.to_owned());
        }
        let body_kind = str_of(&value["body_kind"]);
        if body_kind != "Plain" {
            row.push("-body-kind".to_owned());
            row.push(body_kind.to_owned());
        }
        push_values(
            &mut row,
            ctx,
            &value["values"],
            &format!("{}-{}", ctx.scope, name.trim_start_matches('-')),
        );
        if value["closed"].as_bool() == Some(true) {
            row.push("-closed".to_owned());
        }
        if let Some(domain) = integer_domain_word(&value["integer"]) {
            row.push("-integer".to_owned());
            row.push(domain);
        }
        let appended = &value["appended_arity"];
        if str_of(&appended["kind"]) != "Unknown" {
            row.push("-appends".to_owned());
            row.push(appended_arity_word(appended));
        }
    }
    if let Some(aliases) = option["aliases"].as_array()
        && !aliases.is_empty()
    {
        row.push("-aliases".to_owned());
        row.push(str_list_word(&option["aliases"]));
    }
    if let Some(dialects) = option["dialects"].as_array() {
        row.push("-dialects".to_owned());
        row.push(str_list_word(&Value::Array(dialects.clone())));
    }
    if let Some(n) = option["min_abbrev"].as_u64() {
        row.push("-min-abbrev".to_owned());
        row.push(n.to_string());
    }
    for (key, flag) in [
        ("introduced_version", "-introduced"),
        ("deprecated_version", "-deprecated"),
        ("retired_version", "-retired"),
    ] {
        if let Some(version) = option[key].as_str() {
            row.push(flag.to_owned());
            row.push(word(version));
        }
    }
    let detail = str_of(&option["detail"]);
    if !detail.is_empty() {
        row.push("-detail".to_owned());
        row.push(braced(detail));
    }
    out.row(&row, "-detail");
    if option[draft::OPTION_DEPRECATION_FIX_UNRECOVERABLE_KEY]
        .as_bool()
        .unwrap_or(false)
        || !option["deprecation_fix"].is_null()
    {
        out.indented(|out| {
            out.comment(&format!(
                "TODO(spectcl): `{name}` carries a lifecycle deprecation fix; \
                 the option row has no flag for one yet."
            ));
        });
    }
}

/// `{Range lo hi}` / `Any` / `Port`, with the `max` / `min` sentinels the DSL
/// documents.
fn integer_domain_word(value: &Value) -> Option<String> {
    match str_of(&value["kind"]) {
        "Any" => Some("Any".to_owned()),
        "Port" => Some("Port".to_owned()),
        "Range" => {
            let bound = |n: Option<i64>, sentinel: &str| -> String {
                match n {
                    Some(i64::MAX) => "max".to_owned(),
                    Some(i64::MIN) => "min".to_owned(),
                    Some(n) => n.to_string(),
                    None => sentinel.to_owned(),
                }
            };
            Some(format!(
                "{{Range {} {}}}",
                bound(value["lo"].as_i64(), "min"),
                bound(value["hi"].as_i64(), "max"),
            ))
        }
        _ => None,
    }
}

/// The `hover { … }` block.
fn hover_block(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "hover") {
        return;
    }
    let hover = &draft["hover"];
    if hover.is_null() {
        return;
    }
    out.gap();
    out.line("hover {");
    out.indented(|out| {
        let scalar_row = |out: &mut Out, key: &str, word_name: &str| {
            let text = str_of(&hover[key]);
            if !text.is_empty() {
                out.line(&format!("{word_name} {}", braced(text)));
            }
        };
        scalar_row(out, "summary", "summary");
        for synopsis in as_array(&hover["synopsis"]) {
            out.line(&format!("synopsis {}", braced(str_of(synopsis))));
        }
        // Three words are renamed from their Rust field names, and nothing
        // else in the DSL renames a key.
        scalar_row(out, "snippet", "description");
        scalar_row(out, "source", "source");
        // ONE `example` block, never one row per example: repeated rows join
        // with a single newline, which would flatten an `examples` string that
        // separates its examples with a blank line.
        scalar_row(out, "examples", "example");
        scalar_row(out, "return_value", "returns");
    });
    out.line("}");
}

// ---------------------------------------------------------------------------
// Command and subcommand bodies
// ---------------------------------------------------------------------------

/// Render the body of one `command NAME { … }` block.
#[allow(clippy::too_many_lines)]
fn command_body(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft) {
    // --- identity and availability -----------------------------------------
    set_word(out, ctx, draft, "dialects");
    set_word(out, ctx, draft, "traits");
    arity_row(out, ctx, draft);
    text(out, ctx, draft, "required_package");
    text(out, ctx, draft, "tcllib_package");
    text(out, ctx, draft, "implementation_namespace");
    text(out, ctx, draft, "introduced_version");
    text(out, ctx, draft, "deprecated_version");
    text(out, ctx, draft, "retired_version");
    flag(out, ctx, draft, "warn_missing_import");
    flag(out, ctx, draft, "is_namespace_exported");
    flag(out, ctx, draft, "unsafe_command");
    text_list(out, ctx, draft, "excluded_events");
    set_word(out, ctx, draft, "safe_on_uninit");
    gap_todo(out, ctx, draft, "deprecation_fix");

    // --- types and shape ---------------------------------------------------
    out.gap();
    enum_word(out, ctx, draft, "return_type");
    if ctx.set(draft, "var_write_typing")
        && let Some(value) = draft["var_write_typing"].as_str()
    {
        match var_write_typing_word(value) {
            Some(spelling) => out.line(&format!("var_write_typing {spelling}")),
            None => todo(out, "var_write_typing"),
        }
    }
    gap_todo(out, ctx, draft, "return_elements");
    gap_todo(out, ctx, draft, "var_elements_effect");
    gap_todo(out, ctx, draft, "representation_effect");
    enum_word(out, ctx, draft, "inferred_storage_type");
    enum_word(out, ctx, draft, "body_kind");
    enum_word(out, ctx, draft, "byte_array_effect");
    enum_word(out, ctx, draft, "pattern_type");
    gap_todo(out, ctx, draft, "format_string_type");
    gap_todo(out, ctx, draft, "byte_array_payload");

    arg_rows(out, ctx, draft);
    if ctx.set(draft, "repeated_args")
        && let Some(expr) = draft["repeated_args"].as_str()
    {
        match repeat_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "");
                }
            }
            None => todo(out, "repeated_args"),
        }
    }
    count(out, ctx, draft, "reserved_trailing_words");
    count(out, ctx, draft, "body_arg_implicit_args");
    if ctx.set(draft, "assigns_variable_at") {
        scalar(out, ctx, draft, "assigns_variable_at", draft["assigns_variable_at"].to_string());
    }
    if ctx.set(draft, "creates_instance_at") {
        scalar(out, ctx, draft, "creates_instance_at", draft["creates_instance_at"].to_string());
    }
    if ctx.set(draft, "defines_command_at") {
        scalar(out, ctx, draft, "defines_command_at", draft["defines_command_at"].to_string());
    }
    gap_todo(out, ctx, draft, "defines_symbol");

    // --- subcommand dispatch -----------------------------------------------
    flag(out, ctx, draft, "allow_unknown_subcommands");
    enum_word(out, ctx, draft, "prefix_matching");
    gap_todo(out, ctx, draft, "default_form_first_word");
    text_list(out, ctx, draft, "self_receiver_words");

    // --- hooks -------------------------------------------------------------
    out.gap();
    native_hook(out, ctx, draft, "arg_role_resolver");
    native_hook(out, ctx, draft, "command_prefix_resolver");
    native_hook(out, ctx, draft, "clause_shape_check");
    native_hook(out, ctx, draft, "const_fold");
    native_hook(out, ctx, draft, "const_fold_versioned");
    native_hook(out, ctx, draft, "literal_argument_validator");
    native_hook(out, ctx, draft, "context_gate");
    catalogue_hook(out, ctx, draft, "lowering_hook");
    catalogue_hook(out, ctx, draft, "codegen_hook");
    catalogue_hook(out, ctx, draft, "inline_codegen_hook");
    catalogue_hook(out, ctx, draft, "analyser_hook");
    gap_todo(out, ctx, draft, "semantic_operation");
    gap_todo(out, ctx, draft, "bpf_op");
    gap_todo(out, ctx, draft, "completion");
    gap_todo(out, ctx, draft, "dispatch_dependencies");
    gap_todo(out, ctx, draft, "result_stability");
    gap_todo(out, ctx, draft, "command_forms");

    // --- effects -----------------------------------------------------------
    out.gap();
    enum_word(out, ctx, draft, "command_table_effect");
    side_effect_rows(out, draft);
    gap_todo(out, ctx, draft, "frame_effect");
    gap_todo(out, ctx, draft, "world_effects");
    gap_todo(out, ctx, draft, "state_transitions");

    // --- taint and security ------------------------------------------------
    out.gap();
    text(out, ctx, draft, "taint_output_sink");
    text_list(out, ctx, draft, "taint_output_sink_subcommands");
    text(out, ctx, draft, "taint_log_sink");
    index_list(out, ctx, draft, "taint_network_sink_args");
    index_list(out, ctx, draft, "taint_code_sink_args");
    text_list(out, ctx, draft, "taint_interp_eval_subcommands");
    set_word(out, ctx, draft, "taint_source");
    set_word(out, ctx, draft, "taint_transform");
    set_word(out, ctx, draft, "taint_double_encode_colour");
    set_word(out, ctx, draft, "taint_sink_safe_colour");
    native_hook(out, ctx, draft, "taint_sink_gate");
    text_list(out, ctx, draft, "credential_options");
    text_list(out, ctx, draft, "sensitive_headers");
    gap_todo(out, ctx, draft, "setter_constraints");

    // --- iRules ------------------------------------------------------------
    gap_todo(out, ctx, draft, "event_requires");
    gap_todo(out, ctx, draft, "event_requirement_forms");
    gap_todo(out, ctx, draft, "data_collection");
    gap_todo(out, ctx, draft, "side_switch_target");
    gap_todo(out, ctx, draft, "event_handler_priority");

    // --- translation -------------------------------------------------------
    out.gap();
    if ctx.set(draft, "xc_translatable")
        && let Some(value) = draft["xc_translatable"].as_bool()
    {
        out.line(if value {
            "xc_translatable yes"
        } else {
            "xc_translatable no"
        });
    }
    text(out, ctx, draft, "xc_operation");
    text(out, ctx, draft, "deprecated_replacement");
    flag(out, ctx, draft, "deprecated_replacement_drop_in");

    // --- descriptors -------------------------------------------------------
    out.gap();
    gap_todo(out, ctx, draft, "definition_body");
    manufacturer_rows(out, draft);
    gap_todo(out, ctx, draft, "case_list");
    gap_todo(out, ctx, draft, "object_class");
    gap_todo(out, ctx, draft, "body_scope");
    if ctx.set(draft, "oo_context_facts")
        && let Some(expr) = draft["oo_context_facts"].as_str()
    {
        match oo_context_fact_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "");
                }
            }
            None => todo(out, "oo_context_facts"),
        }
    }
    if ctx.set(draft, "binds_handle")
        && let Some(expr) = draft["binds_handle"].as_str()
    {
        match binds_handle_word(expr) {
            Some(spelling) => out.line(&format!("binds_handle {spelling}")),
            None => todo(out, "binds_handle"),
        }
    }

    // --- options -----------------------------------------------------------
    option_block(out, ctx, draft);

    // --- documentation -----------------------------------------------------
    if ctx.set(draft, "forms") {
        out.gap();
        for form in as_array(&draft["forms"]) {
            let mut row = vec![
                "form".to_owned(),
                str_of(&form["kind"]).to_owned(),
                braced(str_of(&form["synopsis"])),
            ];
            if let Some(dialects) = form["dialects"].as_array() {
                row.push("-dialects".to_owned());
                row.push(str_list_word(&Value::Array(dialects.clone())));
            }
            out.row(&row, "");
        }
    }
    hover_block(out, ctx, draft);

    // --- subcommands -------------------------------------------------------
    for sub in as_array(draft.get("subcommands").unwrap_or(&Value::Null)) {
        let Some(body) = sub.as_object() else { continue };
        out.gap();
        subcommand_block(out, ctx, body);
    }
}

/// The `option …` / `option_conflict …` rows, which the ports keep together.
fn option_block(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft) {
    if ctx.set(draft, "options") {
        out.gap();
        for option in as_array(&draft["options"]).to_vec() {
            option_row(out, ctx, &option);
        }
    }
    if ctx.set(draft, "option_constraints")
        && let Some(expr) = draft["option_constraints"].as_str()
    {
        match option_conflict_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "");
                }
            }
            None => todo(out, "option_constraints"),
        }
    }
}

fn side_effect_rows(out: &mut Out, draft: &Draft) {
    for effect in as_array(draft.get("side_effects").unwrap_or(&Value::Null)) {
        let mut row = vec![
            "side_effect".to_owned(),
            str_of(&effect["target"]).to_owned(),
        ];
        if effect["reads"].as_bool() == Some(true) {
            row.push("-reads".to_owned());
        }
        if effect["writes"].as_bool() == Some(true) {
            row.push("-writes".to_owned());
        }
        let side = str_of(&effect["connection_side"]);
        if side != "None" {
            row.push("-side".to_owned());
            row.push(side.to_owned());
        }
        if let Some(dialects) = effect["dialects"].as_array() {
            row.push("-dialects".to_owned());
            row.push(str_list_word(&Value::Array(dialects.clone())));
        }
        out.row(&row, "");
    }
}

fn manufacturer_rows(out: &mut Out, draft: &Draft) {
    for method in as_array(draft.get("manufacturer_methods").unwrap_or(&Value::Null)) {
        let mut row = vec![
            "manufacturer".to_owned(),
            word(str_of(&method["keyword"])),
        ];
        if str_of(&method["visibility"]) == "Unexported" {
            row.push("-unexported".to_owned());
        }
        for (key, flag) in [
            ("names_instance_at", "-names-instance-at"),
            ("definition_body_at", "-definition-body-at"),
        ] {
            if let Some(n) = method[key].as_u64() {
                row.push(flag.to_owned());
                row.push(n.to_string());
            }
        }
        if let Some(n) = method["constructor_args_from"].as_u64()
            && n != 0
        {
            row.push("-constructor-args-from".to_owned());
            row.push(n.to_string());
        }
        out.row(&row, "");
    }
}

/// One `subcommand NAME { … }` block.
///
/// A subcommand saying only what its arity, detail, and synopsis are is
/// written on one line with `;` separators, the way
/// `irules-http-header.tclspec` writes fourteen of them.
#[allow(clippy::too_many_lines)]
fn subcommand_block(out: &mut Out, parent: &mut Ctx<'_>, sub: &Draft) {
    let name = str_of(sub.get("name").unwrap_or(&Value::Null));
    let defaults = draft::default_subcommand_draft();
    let mut ctx = Ctx {
        defaults: &defaults,
        scope: format!("{}::{name}", parent.scope),
        tables: parent.tables,
    };
    let ctx = &mut ctx;

    let mut body = Out {
        indent: 0,
        ..Out::default()
    };
    let out_body = &mut body;

    arity_row(out_body, ctx, sub);
    if ctx.set(sub, "detail") {
        out_body.line(&format!("detail {}", braced(str_of(&sub["detail"]))));
    }
    if ctx.set(sub, "synopsis") {
        out_body.line(&format!("synopsis {}", braced(str_of(&sub["synopsis"]))));
    }
    set_word(out_body, ctx, sub, "traits");
    set_word(out_body, ctx, sub, "dialects");
    text(out_body, ctx, sub, "introduced_version");
    text(out_body, ctx, sub, "deprecated_version");
    text(out_body, ctx, sub, "retired_version");
    set_word(out_body, ctx, sub, "safe_on_uninit");
    gap_todo(out_body, ctx, sub, "deprecation_fix");

    enum_word(out_body, ctx, sub, "return_type");
    if ctx.set(sub, "var_write_typing")
        && let Some(value) = sub["var_write_typing"].as_str()
    {
        match var_write_typing_word(value) {
            Some(spelling) => out_body.line(&format!("var_write_typing {spelling}")),
            None => todo(out_body, "var_write_typing"),
        }
    }
    gap_todo(out_body, ctx, sub, "return_elements");
    gap_todo(out_body, ctx, sub, "var_elements_effect");
    gap_todo(out_body, ctx, sub, "representation_effect");
    enum_word(out_body, ctx, sub, "inferred_storage_type");
    enum_word(out_body, ctx, sub, "body_kind");
    enum_word(out_body, ctx, sub, "byte_array_effect");
    enum_word(out_body, ctx, sub, "pattern_type");
    gap_todo(out_body, ctx, sub, "format_string_type");

    flag(out_body, ctx, sub, "pure");
    flag(out_body, ctx, sub, "mutator");
    flag(out_body, ctx, sub, "destructive");
    flag(out_body, ctx, sub, "returns_path");
    flag(out_body, ctx, sub, "is_unescape");
    flag(out_body, ctx, sub, "loop_list_header");
    flag(out_body, ctx, sub, "creates_scope_alias");
    flag(out_body, ctx, sub, "arg_values_accept_prefix");
    count(out_body, ctx, sub, "body_arg_implicit_args");
    if ctx.set(sub, "defines_command_at") {
        out_body.line(&format!("defines_command_at {}", sub["defines_command_at"]));
    }
    if ctx.set(sub, "max_leading_option_words") {
        out_body.line(&format!(
            "max_leading_option_words {}",
            sub["max_leading_option_words"]
        ));
    }
    if ctx.set(sub, "min_abbrev") {
        out_body.line(&format!("min_abbrev {}", sub["min_abbrev"]));
    }
    enum_word(out_body, ctx, sub, "prefix_matching");
    text(out_body, ctx, sub, "cfg_rewrite_name");

    arg_rows(out_body, ctx, sub);
    if ctx.set(sub, "repeated_args")
        && let Some(expr) = sub["repeated_args"].as_str()
    {
        match repeat_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out_body.row(&row, "");
                }
            }
            None => todo(out_body, "repeated_args"),
        }
    }

    native_hook(out_body, ctx, sub, "arg_role_resolver");
    native_hook(out_body, ctx, sub, "command_prefix_resolver");
    native_hook(out_body, ctx, sub, "const_fold");
    native_hook(out_body, ctx, sub, "const_fold_versioned");
    native_hook(out_body, ctx, sub, "literal_argument_validator");
    catalogue_hook(out_body, ctx, sub, "lowering_hook");
    catalogue_hook(out_body, ctx, sub, "codegen_hook");
    catalogue_hook(out_body, ctx, sub, "inline_codegen_hook");
    catalogue_hook(out_body, ctx, sub, "analyser_hook");
    gap_todo(out_body, ctx, sub, "semantic_operation");
    gap_todo(out_body, ctx, sub, "completion");
    gap_todo(out_body, ctx, sub, "dispatch_dependencies");
    gap_todo(out_body, ctx, sub, "result_stability");
    gap_todo(out_body, ctx, sub, "subcommand_forms");

    enum_word(out_body, ctx, sub, "command_table_effect");
    side_effect_rows(out_body, sub);
    gap_todo(out_body, ctx, sub, "world_effects");
    gap_todo(out_body, ctx, sub, "state_transitions");

    text(out_body, ctx, sub, "taint_output_sink");
    set_word(out_body, ctx, sub, "taint_transform");
    set_word(out_body, ctx, sub, "taint_double_encode_colour");
    if ctx.set(sub, "credential_arg") {
        out_body.line(&format!("credential_arg {}", sub["credential_arg"]));
    }
    text_list(out_body, ctx, sub, "sensitive_headers");
    text(out_body, ctx, sub, "xc_operation");

    option_block(out_body, ctx, sub);
    if ctx.set(sub, "versioned_arg_values")
        && let Some(expr) = sub["versioned_arg_values"].as_str()
    {
        match versioned_arg_value_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out_body.row(&row, "");
                }
            }
            None => todo(out_body, "versioned_arg_values"),
        }
    }
    for row in as_array(sub.get("sub_subcommands").unwrap_or(&Value::Null)) {
        let mut words = vec!["sub_subcommand".to_owned(), word(str_of(&row["name"]))];
        let detail = str_of(&row["detail"]);
        if !detail.is_empty() {
            words.push("-detail".to_owned());
            words.push(braced(detail));
        }
        let synopsis = str_of(&row["synopsis"]);
        if !synopsis.is_empty() {
            words.push("-synopsis".to_owned());
            words.push(braced(synopsis));
        }
        if let Some(dialects) = row["dialects"].as_array() {
            words.push("-dialects".to_owned());
            words.push(str_list_word(&Value::Array(dialects.clone())));
        }
        out_body.row(&words, "-detail");
    }
    hover_block(out_body, ctx, sub);

    let statements: Vec<&str> = body.text.lines().filter(|line| !line.is_empty()).collect();
    let one_liner = format!(
        "subcommand {} {{ {} }}",
        word(name),
        statements.join(" ; ")
    );
    let simple = statements.len() <= 3
        && !statements.iter().any(|line| line.ends_with('{'))
        && !one_liner.contains('\n');
    if simple && out.indent * 4 + one_liner.len() <= WRAP_COLUMN {
        out.line(&one_liner);
        return;
    }

    out.line(&format!("subcommand {} {{", word(name)));
    out.indented(|out| {
        let indent = "    ".repeat(out.indent);
        for line in body.text.lines() {
            if line.is_empty() {
                out.raw("\n");
            } else {
                out.raw(&format!("{indent}{line}\n"));
            }
        }
    });
    out.line("}");
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// Render one draft as a complete single-command `.tclspec` pack.
///
/// The pack takes its name from the dialect the draft was seeded from
/// ([`SOURCE_DIALECT_KEY`]) when it has one, which is what the ports do —
/// `speclib tcl 1.0`, `speclib f5-irules 1.0`.
#[must_use]
pub fn render(draft: &Draft) -> String {
    let pack = draft
        .get(SOURCE_DIALECT_KEY)
        .and_then(Value::as_str)
        .unwrap_or("pack")
        .to_owned();
    render_pack(std::slice::from_ref(draft), &pack)
}

/// Render a pack of drafts as one `.tclspec` file.
#[must_use]
pub fn render_pack(drafts: &[Draft], pack_name: &str) -> String {
    let mut tables = ValueTables::default();
    let mut bodies: Vec<(String, String)> = Vec::new();

    for draft in drafts {
        let name = str_of(draft.get("name").unwrap_or(&Value::Null)).to_owned();
        let defaults = draft::default_command_draft();
        let mut body = Out::default();
        {
            let mut ctx = Ctx {
                defaults: &defaults,
                scope: name.clone(),
                tables: &mut tables,
            };
            command_body(&mut body, &mut ctx, draft);
        }
        bodies.push((name, body.text));
    }

    let mut out = Out::default();
    out.comment(
        "Rendered from the tcl-lsp command registry by the spec studio's SpecTcl \
         renderer (rust/tcl-spec-studio/src/render_spectcl.rs).",
    );
    out.comment(
        "The syntax is docs/design/spec-dsl-examples/README.md. Every `TODO(spectcl)` \
         line below names a field the source spec sets that a pack cannot yet say.",
    );
    out.line("");
    out.line(&format!(
        "speclib {} {DSL_VERSION} {{",
        word(pack_name)
    ));
    out.indented(|out| {
        tables.render(out);
        for (name, body) in &bodies {
            out.gap();
            out.line(&format!("command {} {{", word(name)));
            out.indented(|out| {
                let indent = "    ".repeat(out.indent);
                for line in body.lines() {
                    if line.is_empty() {
                        out.raw("\n");
                    } else {
                        out.raw(&format!("{indent}{line}\n"));
                    }
                }
            });
            out.line("}");
        }
    });
    out.line("}");
    out.text
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn drafted(name: &str) -> Draft {
        let mut d = draft::default_command_draft();
        d.insert("name".into(), json!(name));
        d
    }

    #[test]
    fn a_default_draft_renders_a_pack_that_says_nothing_else() {
        let text = render_pack(&[drafted("mycommand")], "probe");
        assert!(text.contains("speclib probe 1.0 {"));
        assert!(text.contains("command mycommand {"));
        let statements: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('}'))
            .collect();
        assert_eq!(
            statements,
            vec!["speclib probe 1.0 {", "command mycommand {"],
            "a default draft must emit only what the author actually set:\n{text}"
        );
    }

    #[test]
    fn arity_takes_its_six_documented_spellings() {
        for (min, max, step, also, expected) in [
            (3u64, Some(3u64), 0u64, None, "arity 3"),
            (1, Some(2), 0, None, "arity 1..2"),
            (1, None, 0, None, "arity 1.."),
            (0, Some(2), 0, None, "arity ..2"),
            (0, None, 0, None, "arity .."),
            (3, None, 2, None, "arity 3.. -step 2"),
            (3, None, 2, Some(2), "arity 3.. -step 2 -also 2"),
        ] {
            let mut d = drafted("probe");
            d.insert(
                "arity".into(),
                json!({ "min": min, "max": max, "step": step, "also_exact": also }),
            );
            let text = render_pack(&[d], "probe");
            assert!(
                text.contains(expected),
                "expected `{expected}` in:\n{text}"
            );
        }
    }

    #[test]
    fn prose_that_braces_cannot_hold_takes_the_quoted_form() {
        assert_eq!(braced("plain text"), "{plain text}");
        assert_eq!(braced("a {nested} word"), "{a {nested} word}");
        // A lone opening brace, and a trailing backslash, both need quoting.
        assert_eq!(braced("a { lone"), "\"a { lone\"");
        assert_eq!(braced("trailing\\"), "\"trailing\\\\\"");
        assert_eq!(braced("$x [y]"), "{$x [y]}");
    }

    #[test]
    fn a_native_hook_is_named_after_the_field_it_fills() {
        let mut d = drafted("probe");
        d.insert(
            draft::UNRENDERABLE_KEY.into(),
            json!(["arg_role_resolver", "const_fold"]),
        );
        let text = render_pack(&[d], "probe");
        assert!(text.contains("arg_role_resolver -native probe::arg_role_resolver"));
        assert!(text.contains("const_fold -native probe::const_fold"));
    }

    #[test]
    fn a_field_the_dsl_cannot_carry_becomes_a_todo_naming_it() {
        let mut d = drafted("probe");
        d.insert(draft::UNRENDERABLE_KEY.into(), json!(["frame_effect"]));
        let text = render_pack(&[d], "probe");
        assert!(
            text.contains("TODO(spectcl): `frame_effect`"),
            "the TODO must name the field:\n{text}"
        );
        assert!(text.contains("frame_effect -level-word W -layout L"));
    }

    #[test]
    fn values_carrying_details_are_hoisted_to_a_shared_table() {
        let mut d = drafted("probe");
        d.insert(
            "arg_values".into(),
            json!([{
                "index": 0,
                "values": [
                    { "value": "alnum", "detail": "Any alphanumeric.", "min_tcl": null, "code": null },
                    { "value": "dict", "detail": "A dict.", "min_tcl": "V9_0", "code": null },
                ],
            }]),
        );
        let text = render_pack(&[d], "probe");
        assert!(text.contains("values probe-arg0 {"), "{text}");
        assert!(text.contains("value alnum -detail {Any alphanumeric.}"), "{text}");
        assert!(text.contains("value dict -min-tcl tcl9.0"), "{text}");
        assert!(text.contains("arg 0 -values-from probe-arg0"), "{text}");
    }

    #[test]
    fn a_plain_value_set_stays_on_the_row() {
        let mut d = drafted("probe");
        d.insert(
            "arg_values".into(),
            json!([{
                "index": 1,
                "values": [
                    { "value": "yes", "detail": "", "min_tcl": null, "code": null },
                    { "value": "no", "detail": "", "min_tcl": null, "code": null },
                ],
            }]),
        );
        let text = render_pack(&[d], "probe");
        assert!(text.contains("arg 1 -values {yes no}"), "{text}");
        assert!(!text.contains("values probe"), "{text}");
    }

    #[test]
    fn a_repeated_arg_layout_round_trips_through_its_rust_expression() {
        let rows = repeat_rows(
            "&[RepeatedArgLayout { role: ArgRole::LoopVarList, start: 0, stride: 2, \
             exclude_trailing: 1, optional_leading_word: false, conditional_binding: false }]",
        )
        .expect("the layout parses");
        assert_eq!(
            rows[0].join(" "),
            "repeat LoopVarList -stride 2 -exclude-trailing 1"
        );
    }

    #[test]
    fn an_option_constraint_keeps_its_dialect_gate() {
        let rows = option_conflict_rows(
            "&[OptionConstraint { options: &[\"-glob\", \"-regexp\"], \
             dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)) }]",
        )
        .expect("the constraint parses");
        assert_eq!(
            rows[0].join(" "),
            "option_conflict {-glob -regexp} -dialects {tcl8.4 tcl8.5 tcl8.6 tcl9.0 tcl9.1 f5-irules}"
        );
    }

    #[test]
    fn a_simple_subcommand_is_written_on_one_line() {
        let mut d = drafted("probe");
        let mut sub = draft::default_subcommand_draft();
        sub.insert("name".into(), json!("at"));
        sub.insert(
            "arity".into(),
            json!({ "min": 1, "max": 1, "step": 0, "also_exact": null }),
        );
        sub.insert("detail".into(), json!("Get header name by index."));
        d.insert("subcommands".into(), json!([Value::Object(sub)]));
        let text = render_pack(&[d], "probe");
        assert!(
            text.contains("subcommand at { arity 1 ; detail {Get header name by index.} }"),
            "{text}"
        );
    }
}
