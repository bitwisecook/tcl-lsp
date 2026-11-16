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

//! DSL words and rows recovered from the rendered Rust expressions a draft holds.

use super::str_of;
use super::tcl_words::{
    as_array, available_flag, braced, dialect_words, list_word, push_word, word,
};
use serde_json::Value;
use std::collections::BTreeMap;

// Rust-expression fields the draft *can* recover

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
pub(super) fn between<'a>(text: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
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
    tcl_spectcl::catalogue::rust_path_leaf(text.trim()).trim()
}

/// `Some(inner)` → `inner`; anything else is returned unchanged.
fn unwrap_some(text: &str) -> Option<&str> {
    between(text, "Some(", ")")
}

/// The dialect member names behind a rendered surface expression.
///
/// [`crate::render_rs::dialect_set`] writes the readable aggregates
/// (`SpecSurface::ALL_TCL_AND_IRULES`); this reads them back so
/// an option constraint keeps its gate.
pub(super) fn dialect_names(expr: &str) -> Option<Vec<String>> {
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
        ("SPECTCL", &["spectcl"]),
        ("SSLICTCL", &["sslictcl"]),
        ("JIM", &["jim"]),
    ];
    // A row list is `SpecSurface::CONST` or `surface![CONST, CONST…]`.
    let inner = expr
        .trim()
        .strip_prefix("surface![")
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(expr);
    let mut names: Vec<String> = Vec::new();
    for part in split_top(inner, ',') {
        if let Some(word) = open_jim_row_word(part) {
            if !names.contains(&word) {
                names.push(word);
            }
            continue;
        }
        let key = tcl_spectcl::catalogue::rust_path_leaf(part);
        let (_, members) = CONSTANTS.iter().find(|(name, _)| *name == key)?;
        for member in *members {
            if !names.iter().any(|name| name == member) {
                names.push((*member).to_owned());
            }
        }
    }
    (!names.is_empty()).then_some(names)
}

/// The `jimX.Y+` word of a Jim core row open-ended at one release, the row
/// [`crate::render_rs::dialect_set`] writes for that word.
fn open_jim_row_word(row: &str) -> Option<String> {
    let arguments = row.split_once("core_in(")?.1.strip_suffix(')')?;
    let [family, intervals] = split_top(arguments, ',').try_into().ok()?;
    if tcl_spectcl::catalogue::rust_path_leaf(family) != "Jim" {
        return None;
    }
    let interval = between(intervals, "&[(", ")]")?;
    let [release, end] = split_top(interval, ',').try_into().ok()?;
    let release = between(release, "\"", "\"")?;
    (end == "None").then(|| format!("jim{release}+"))
}

/// `var_write_typing`'s four spellings, from its rendered Rust expression.
pub(super) fn var_write_typing_word(expr: &str) -> Option<String> {
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

/// `{ListOfArgs N}` … from a rendered `Option<ReturnElements>`.
///
/// The `{VARIANT payload …}` rule the coverage matrix gives for the three
/// element-structure facts: the variant word, then its fields in declaration
/// order.
pub(super) fn return_elements_word(expr: &str) -> Option<String> {
    let inner = unwrap_some(expr)?;
    let variant = variant_of(inner.split(['(', '{']).next()?);
    let fields = struct_fields(inner)?;
    Some(match variant {
        "ListOfArgs" | "DictOfPairs" => format!("{{{variant} {}}}", fields.get("from")?),
        "ElementOf" | "SubListOf" => format!("{{{variant} {}}}", fields.get("container_arg")?),
        _ => return None,
    })
}

/// `{AppendsListElements N}` / `SetsDictValue` / … from a rendered
/// `Option<VarElementsEffect>`.
pub(super) fn var_elements_effect_word(expr: &str) -> Option<String> {
    let inner = unwrap_some(expr)?;
    let variant = variant_of(inner.split(['(', '{']).next()?);
    Some(match variant {
        "SetsDictValue" | "ListifiesDictValue" => variant.to_owned(),
        "AppendsListElements" | "ExtendsDictValuesByName" => {
            format!(
                "{{{variant} {}}}",
                struct_fields(inner)?.get("values_from")?
            )
        }
        _ => return None,
    })
}

/// Representation effect variant and payload from a rendered optional descriptor.
pub(super) fn representation_effect_word(expr: &str) -> Option<String> {
    let inner = unwrap_some(expr)?;
    let variant = variant_of(inner.split(['(', '{']).next()?);
    Some(match variant {
        "None" => "None".to_owned(),
        "CoerceExpressionValues" | "CoerceNumericValues" => {
            let fields = struct_fields(inner)?;
            format!("{{{variant} {}}}", fields.get("arguments_from")?)
        }
        "CoerceOrdinaryList" | "CoerceOrdinaryDictionary" => {
            let fields = struct_fields(inner)?;
            format!("{{{variant} {}}}", fields.get("operand")?)
        }
        "CoerceOrdinaryListPairs" => {
            let fields = struct_fields(inner)?;
            format!("{{{variant} {}}}", fields.get("variables_from")?)
        }
        "CoerceOrdinaryListIndices" => {
            let fields = struct_fields(inner)?;
            format!(
                "{{{variant} {} {}}}",
                fields.get("operand")?,
                fields.get("indices_from")?
            )
        }
        "CoerceOrdinaryListRange" => {
            let fields = struct_fields(inner)?;
            format!(
                "{{{variant} {} {} {}}}",
                fields.get("operand")?,
                fields.get("first")?,
                fields.get("last")?
            )
        }
        "CopyOnWriteContainerMutation" => {
            let fields = struct_fields(inner)?;
            format!(
                "{{{variant} {} {}}}",
                fields.get("variable_arg")?,
                fields.get("minimum_arguments")?
            )
        }
        _ => return None,
    })
}

/// The `byte_array_effect` word, including the one variant that carries a
/// payload.
///
/// The draft holds the `Debug` spelling, so the five fieldless effects arrive
/// as a bare name and `Rebinarifies` as a struct literal.
pub(super) fn byte_array_effect_word(value: &str) -> Option<String> {
    if !value.contains('{') {
        return Some(value.to_owned());
    }
    let variant = variant_of(value.split('{').next()?);
    if variant != "Rebinarifies" {
        return None;
    }
    Some(format!(
        "{{Rebinarifies {}}}",
        struct_fields(value)?.get("value_arg")?
    ))
}

/// `byte_array_payload -replace-data-index N ?-message-flag-shift?`.
pub(super) fn byte_array_payload_row(expr: &str) -> Option<Vec<String>> {
    let fields = struct_fields(unwrap_some(expr)?)?;
    let mut row = vec![
        "byte_array_payload".to_owned(),
        "-replace-data-index".to_owned(),
        (*fields.get("replace_data_index")?).to_owned(),
    ];
    if *fields.get("message_flag_shift")? == "true" {
        row.push("-message-flag-shift".to_owned());
    }
    Some(row)
}

/// `defines_symbol -name-arg N ?-detail-arg N? ?-requires-arg N? -kind KIND`.
pub(super) fn defines_symbol_row(expr: &str) -> Option<Vec<String>> {
    let fields = struct_fields(unwrap_some(expr)?)?;
    let mut row = vec![
        "defines_symbol".to_owned(),
        "-name-arg".to_owned(),
        (*fields.get("name_arg")?).to_owned(),
    ];
    for (flag, key) in [
        ("-detail-arg", "detail_arg"),
        ("-requires-arg", "requires_arg"),
    ] {
        if let Some(index) = unwrap_some(fields.get(key)?) {
            row.push(flag.to_owned());
            row.push(index.to_owned());
        }
    }
    row.push("-kind".to_owned());
    row.push(variant_of(fields.get("kind")?).to_owned());
    Some(row)
}

/// `deprecation_fix -replace WORD ?-replace-arg N? ?-replace-invocation?
/// -description {…} -safety S`.
///
/// The coverage matrix's `-replace WORD -description {…} -safety S` covers the
/// matched-word variant; the two positional variants add one word each so the
/// same statement says which of the three it is. `Custom { resolver }` names a
/// registry callback, which the matrix marks reference-only, and has no
/// spelling at all — it never reaches here, because the draft records it as
/// unrecoverable.
pub(super) fn deprecation_fix_row(expr: &str) -> Option<Vec<String>> {
    let inner = unwrap_some(expr)?;
    let variant = variant_of(inner.split(['(', '{']).next()?);
    let fields = struct_fields(inner)?;
    let mut row = vec!["deprecation_fix".to_owned()];
    let mut lost = false;
    row.push("-replace".to_owned());
    push_word(
        &mut row,
        &mut lost,
        word(&rust_str(fields.get("replacement")?)?),
    );
    match variant {
        "ReplaceMatchedWord" => {}
        "ReplaceArgument" => {
            row.push("-replace-arg".to_owned());
            row.push((*fields.get("index")?).to_owned());
        }
        "ReplaceInvocation" => row.push("-replace-invocation".to_owned()),
        _ => return None,
    }
    row.push("-description".to_owned());
    push_word(
        &mut row,
        &mut lost,
        braced(&rust_str(fields.get("description")?)?),
    );
    row.push("-safety".to_owned());
    row.push(variant_of(fields.get("safety")?).to_owned());
    (!lost).then_some(row)
}

/// `setter_constraint N -prefix P -code CODE -message {…}` rows.
pub(super) fn setter_constraint_rows(value: &Value) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for constraint in as_array(value) {
        let mut row = vec![
            "setter_constraint".to_owned(),
            constraint.get("arg_index")?.as_u64()?.to_string(),
            "-prefix".to_owned(),
        ];
        let mut lost = false;
        push_word(
            &mut row,
            &mut lost,
            word(str_of(constraint.get("required_prefix")?)),
        );
        row.push("-code".to_owned());
        row.push(str_of(constraint.get("code")?).to_owned());
        row.push("-message".to_owned());
        push_word(
            &mut row,
            &mut lost,
            braced(str_of(constraint.get("message")?)),
        );
        if lost {
            return None;
        }
        rows.push(row);
    }
    Some(rows)
}

/// `repeat ROLE …` rows from a rendered `&[RepeatedArgLayout]`.
pub(super) fn repeat_rows(expr: &str) -> Option<Vec<Vec<String>>> {
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

/// Construct a relation's inner Tcl list value from its Rust expression.
fn relation_term_word(expr: &str) -> Option<String> {
    let expr = expr.trim();
    let open = expr.find('(')?;
    let inner = expr[open + 1..].strip_suffix(')')?;
    let variant = tcl_spectcl::catalogue::rust_path_leaf(&expr[..open]).trim();
    let parts = split_top(inner, ',');
    let words: Vec<String> = match (variant, parts.as_slice()) {
        ("Option", [name]) => vec![rust_str(name)?],
        ("OptionValue", [name, value]) => vec![rust_str(name)?, rust_str(value)?],
        ("Argument", [index]) => vec!["arg".to_owned(), index.trim().to_owned()],
        ("ArgumentValue", [index, value]) => {
            vec!["arg".to_owned(), index.trim().to_owned(), rust_str(value)?]
        }
        _ => return None,
    };
    // The loader parses the term as a list after reading the outer source word.
    Some(tcl_syntax::list::join_list(&words))
}

/// The `.tclspec` axis and value words from a rendered [`EffectAxis`]
/// expression (`EffectAxis::Substitution(SubstitutionKind::Backslashes)`,
/// `EffectAxis::CaseSensitivity`, …) — the inverse of `draft.rs`'s
/// `effect_axis_expr`.
fn effect_axis_words(expr: &str) -> Option<(&'static str, Option<String>)> {
    let expr = expr.trim();
    if expr == "EffectAxis::CaseSensitivity" {
        return Some(("case-sensitivity", None));
    }
    if let Some(inner) = between(expr, "EffectAxis::Substitution(", ")") {
        let word = match variant_of(inner) {
            "Backslashes" => "backslashes",
            "Commands" => "commands",
            "Variables" => "variables",
            _ => return None,
        };
        return Some(("substitution", Some(word.to_owned())));
    }
    if let Some(inner) = between(expr, "EffectAxis::PatternLanguage(", ")") {
        let word = match variant_of(inner) {
            "Glob" => "glob",
            "Regex" => "regex",
            _ => return None,
        };
        return Some(("pattern-language", Some(word.to_owned())));
    }
    if let Some(inner) = between(expr, "EffectAxis::Selection(", ")") {
        let word = match variant_of(inner) {
            "Exact" => "exact",
            "Glob" => "glob",
            "Regexp" => "regexp",
            "Other" => "other",
            _ => return None,
        };
        return Some(("selection", Some(word.to_owned())));
    }
    None
}

/// The `option_effect_family NAME { … }` rows from a rendered
/// `&[OptionEffectFamily]` — the inverse of `draft.rs`'s
/// `option_effect_families_expr`.
pub(super) fn option_effect_family_rows(expr: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let fields = struct_fields(item)?;
        let name = word(&rust_str(fields.get("name")?)?)?;
        let base_expr = (*fields.get("base")?).trim();
        let base = if base_expr == "FamilyBase::AllOn" {
            "all-on".to_owned()
        } else if base_expr == "FamilyBase::AllOff" {
            "all-off".to_owned()
        } else {
            let inner = between(base_expr, "FamilyBase::Only(", ")")?;
            let (axis, value) = effect_axis_words(inner)?;
            value.map_or_else(
                || format!("{{only {axis}}}"),
                |v| format!("{{only {axis} {v}}}"),
            )
        };
        let combine = match variant_of(fields.get("combine")?) {
            "Accumulate" => "accumulate",
            "LastWins" => "last-wins",
            _ => return None,
        };
        let mut block = vec![
            "base".to_owned(),
            base,
            "combine".to_owned(),
            combine.to_owned(),
        ];
        if let Some(inner) = unwrap_some(fields.get("surface")?) {
            let members = dialect_names(inner)?;
            let members: Vec<&str> = members.iter().map(String::as_str).collect();
            // `-introduced V` is the one gate a family may declare — the
            // lowest release the surface admits, on the family's own (Tcl
            // core) axis; anything else the loader could not have produced.
            let mut versions: Vec<&str> = members
                .iter()
                .filter_map(|member| member.strip_prefix("tcl"))
                .collect();
            versions.sort_by(|a, b| {
                tcl_dialect::model::Version::parse(a)
                    .ok()
                    .cmp(&tcl_dialect::model::Version::parse(b).ok())
            });
            let lowest = versions.first()?;
            if versions.len() != members.len() {
                return None;
            }
            block.push("-introduced".to_owned());
            block.push((*lowest).to_owned());
        }
        rows.push(vec![
            "option_effect_family".to_owned(),
            name,
            braced(&block.join(" "))?,
        ]);
    }
    Some(rows)
}

/// An option row's `-effect` value: `{disables|selects AXIS VALUE}`,
/// `{suppresses-role ROLE}`, `{reserves-trailing-words N}`, or
/// `ends-options` — read off the drafted, structured `effect` object (never
/// `null`, checked by the caller). `None` for an unreadable kind.
pub(super) fn option_effect_block(effect: &Value) -> Option<String> {
    match str_of(&effect["kind"]) {
        kind @ ("disables" | "selects") => {
            let axis = str_of(&effect["axis"]);
            Some(match effect["value"].as_str() {
                Some(value) => format!("{{{kind} {axis} {value}}}"),
                None => format!("{{{kind} {axis}}}"),
            })
        }
        "suppresses-role" => Some(format!("{{suppresses-role {}}}", str_of(&effect["role"]))),
        "reserves-trailing-words" => Some(format!(
            "{{reserves-trailing-words {}}}",
            effect["n"].as_u64().unwrap_or(0)
        )),
        "ends-options" => Some("ends-options".to_owned()),
        _ => None,
    }
}

/// The four E-R14 option-relation rows from a rendered `&[OptionRelation]`.
///
/// `option_conflict` keeps its 1.x shape exactly — statement word then the
/// term list — so an unchanged pack round-trips unchanged; the other three
/// take the subject between them.
pub(super) fn option_conflict_rows(expr: &str, availability: bool) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let fields = struct_fields(item)?;
        let terms: Option<Vec<String>> = slice_items(fields.get("terms")?)?
            .into_iter()
            .map(relation_term_word)
            .collect();
        let terms = list_word(&terms?)?;
        // R12: the SpecTcl vocabulary has no word for an inference edge —
        // `option_requires` reads as an assertion, and rendering an Infer
        // relation with it would export the opposite of what the row says.
        // No option relation is Infer today; refuse rather than assume.
        if fields.get("mode").map(|m| variant_of(m)) == Some("Infer") {
            return None;
        }
        let kind = fields
            .get("kind")
            .map_or("MutuallyExclusive", |k| variant_of(k));
        let statement = match kind {
            "MutuallyExclusive" => "option_conflict",
            "Requires" => "option_requires",
            "RequiresOneOf" => "option_requires_one_of",
            "Forbids" => "option_forbids",
            _ => return None,
        };
        let mut row = vec![statement.to_owned()];
        if kind != "MutuallyExclusive" {
            row.push(match fields.get("subject").and_then(|s| unwrap_some(s)) {
                Some(inner) => word(&relation_term_word(inner)?)?,
                None => "{}".to_owned(),
            });
        }
        row.push(terms);
        if let Some(inner) = unwrap_some(fields.get("surface")?) {
            let members = dialect_names(inner)?;
            let members: Vec<&str> = members.iter().map(String::as_str).collect();
            if let Some(spelling) = availability.then(|| available_flag(&members)).flatten() {
                row.push("-available".to_owned());
                row.push(spelling);
            } else {
                row.push("-dialects".to_owned());
                row.push(dialect_words(&members)?);
            }
        }
        if let Some(inner) = fields.get("message").and_then(|m| unwrap_some(m)) {
            row.push("-message".to_owned());
            row.push(word(&rust_str(inner)?)?);
        }
        push_literal_lifecycle_flags(&mut row, fields.get("lifecycle").copied())?;
        rows.push(row);
    }
    Some(rows)
}

/// `oo_context_fact WORD FACT` rows from a rendered fact table.
pub(super) fn oo_context_fact_rows(expr: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let pair = between(item, "(", ")")?;
        let parts = split_top(pair, ',');
        let [name, fact] = parts.as_slice() else {
            return None;
        };
        rows.push(vec![
            "oo_context_fact".to_owned(),
            word(&rust_str(name)?)?,
            variant_of(fact).to_owned(),
        ]);
    }
    Some(rows)
}

/// The `binds_handle` row's list argument, from a rendered
/// `Some(&HandleBindingSpec { … })`.
pub(super) fn binds_handle_word(expr: &str) -> Option<String> {
    let fields = struct_fields(unwrap_some(expr)?)?;
    let source = |text: &str| -> Option<String> {
        // `HandleName::Word(0)` / `HandleClassSource::ConstructionValue(1)` —
        // the path has to come off before the payload can be read.
        let (variant, payload) = variant_of(text).split_once('(')?;
        let payload = payload.strip_suffix(')')?;
        Some(match variant {
            "Implicit" => format!("{{Implicit {}}}", word(&rust_str(payload)?)?),
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
            word(&rust_str(inner.get("word")?)?)?
        ));
    }
    braced(&parts.join(" "))
}

/// Push the three lifecycle flags out of a rendered `Lifecycle { … }` literal.
///
/// `None` for a literal this reader cannot take apart, which makes the whole
/// row a reported loss rather than a half-written one; an absent field is the
/// registry's own `..DEFAULT`, so it contributes no flag.
fn push_literal_lifecycle_flags(row: &mut Vec<String>, literal: Option<&str>) -> Option<()> {
    let Some(literal) = literal else {
        return Some(());
    };
    let fields = struct_fields(literal)?;
    for (field, flag) in [
        ("introduced", "-introduced"),
        ("deprecated", "-deprecated"),
        ("retired", "-retired"),
    ] {
        let Some(value) = fields.get(field) else {
            continue;
        };
        if let Some(inner) = unwrap_some(value) {
            row.push(flag.to_owned());
            row.push(word(&rust_str(inner)?)?);
        }
    }
    Some(())
}

/// `versioned_arg_value N VALUE …` rows from a rendered gate table.
pub(super) fn versioned_arg_value_rows(expr: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let fields = struct_fields(item)?;
        let mut row = vec![
            "versioned_arg_value".to_owned(),
            (*fields.get("index")?).to_owned(),
            word(&rust_str(fields.get("value")?)?)?,
        ];
        push_literal_lifecycle_flags(&mut row, fields.get("lifecycle").copied())?;
        rows.push(row);
    }
    Some(rows)
}
