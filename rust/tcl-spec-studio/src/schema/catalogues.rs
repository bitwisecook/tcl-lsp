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

//! The variant catalogues the form's pickers read: enum variants, semantic operations, definition bodies and the custom catalogues.

use crate::catalogue;
use serde_json::{Value, json};
use tcl_registry::traits::Trait;

fn catalogue_entries(id: &str, items: &[catalogue::Variant]) -> Value {
    Value::Array(
        items
            .iter()
            .map(|variant| {
                json!({
                    "key": variant.key,
                    "doc": variant.doc,
                    "group": (id == "traits")
                        .then(|| Trait::from_name(variant.key).map(|item| item.category().label()))
                        .flatten(),
                    "example": crate::examples::variant_example(
                        id,
                        variant.key,
                        variant.doc,
                    ),
                })
            })
            .collect(),
    )
}

fn custom_variant(id: &str, key: &str, doc: &str) -> Value {
    json!({
        "key": key,
        "doc": doc,
        "example": crate::examples::variant_example(id, key, doc),
    })
}

/// The semantic-operation vocabulary as a catalogue: one entry per
/// operation, keyed `KIND` or `KIND DETAIL` in the operation's own spellings
/// (`invoke`, `intrinsic list-length`, `structured-lowering expr`) — the
/// draft's `{kind, detail}` joined by a space.
fn semantic_operation_catalogue() -> Value {
    Value::Array(
        tcl_spectcl::semantic_operations()
            .map(|operation| {
                let key = operation.detail_str().map_or_else(
                    || operation.kind_str().to_owned(),
                    |detail| format!("{} {detail}", operation.kind_str()),
                );
                let doc = match operation.detail_str() {
                    None => "generic Tcl argv invocation — the conservative fallback".to_owned(),
                    Some(detail) => format!("the `{detail}` {}", operation.kind_str()),
                };
                custom_variant("semanticOperation", &key, &doc)
            })
            .collect(),
    )
}

/// The shipped definer grammars a `definition_body` can name.
fn definition_body_catalogue() -> Value {
    Value::Array(
        tcl_spectcl::SHIPPED_DEFINITION_BODIES
            .iter()
            .map(|(name, grammar)| {
                let doc = format!(
                    "the shipped {:?} grammar ({} members)",
                    grammar.family,
                    grammar.members.len()
                );
                custom_variant("definitionBody", name, &doc)
            })
            .collect(),
    )
}

fn custom_catalogues() -> [(&'static str, Value); 7] {
    [
        ("semanticOperation", semantic_operation_catalogue()),
        ("definitionBody", definition_body_catalogue()),
        (
            "optionPlacement",
            json!([
                custom_variant(
                    "optionPlacement",
                    "Leading",
                    "option parsing stops at the first non-option word",
                ),
                custom_variant(
                    "optionPlacement",
                    "Anywhere",
                    "options are recognised between positional words, up to `--`",
                ),
            ]),
        ),
        (
            "defaultFormFirstWord",
            json!([
                custom_variant(
                    "defaultFormFirstWord",
                    "Integer",
                    "an integer first word selects the default form",
                ),
                custom_variant(
                    "defaultFormFirstWord",
                    "WindowPath",
                    "a first word beginning with `.` (a window path) selects the default form",
                ),
                custom_variant(
                    "defaultFormFirstWord",
                    "GridContent",
                    "a first word beginning with `.`, `x` or `^` selects the default form",
                ),
            ]),
        ),
        (
            "prefixMatching",
            json!([
                custom_variant(
                    "prefixMatching",
                    "Enabled",
                    "any unique prefix resolves (Tcl_GetIndexFromObj)",
                ),
                custom_variant(
                    "prefixMatching",
                    "Strict",
                    "only the exact spelling resolves (TCL_INDEX_STRICT)",
                ),
            ]),
        ),
        (
            "appendedArity",
            json!([
                custom_variant(
                    "appendedArity",
                    "Exactly",
                    "exactly N arguments are appended"
                ),
                custom_variant(
                    "appendedArity",
                    "OneOf",
                    "one count from a finite set is appended, and every count in it must be accepted"
                ),
                custom_variant(
                    "appendedArity",
                    "AtLeast",
                    "at least N arguments are appended"
                ),
                custom_variant("appendedArity", "Unknown", "indeterminate — no arity check"),
            ]),
        ),
        (
            "optionArity",
            json!([
                custom_variant("optionArity", "One", "consumes one value word"),
                custom_variant(
                    "optionArity",
                    "Fixed",
                    "consumes a fixed number of value words",
                ),
            ]),
        ),
    ]
}

/// The variant catalogues the form's pickers read, keyed by catalogue id.
#[must_use]
pub fn catalogues() -> Value {
    let standard: [(&str, &[catalogue::Variant]); 32] = [
        ("argRole", catalogue::ARG_ROLES),
        ("tclType", catalogue::TCL_TYPES),
        ("bodyKind", catalogue::BODY_KINDS),
        ("scriptTiming", catalogue::SCRIPT_TIMINGS),
        ("scriptLookupScope", catalogue::SCRIPT_LOOKUP_SCOPES),
        ("irulesSourceFlow", catalogue::IRULES_SOURCE_FLOWS),
        ("variableScope", catalogue::VARIABLE_SCOPES),
        ("argPresentation", catalogue::ARG_PRESENTATIONS),
        ("storageType", catalogue::STORAGE_TYPES),
        ("byteArrayEffect", catalogue::BYTE_ARRAY_EFFECTS),
        (
            "sourceDeprecationAdvice",
            catalogue::SOURCE_DEPRECATION_ADVICE,
        ),
        ("commandTableEffect", catalogue::COMMAND_TABLE_EFFECTS),
        ("patternType", catalogue::PATTERN_TYPES),
        ("sourcePathOperation", catalogue::SOURCE_PATH_OPERATIONS),
        (
            "sourceConfigurationReference",
            catalogue::SOURCE_CONFIGURATION_REFERENCES,
        ),
        ("sourceIndexBounds", catalogue::SOURCE_INDEX_BOUNDS),
        ("channelConfiguration", catalogue::CHANNEL_CONFIGURATIONS),
        (
            "taintTransformCondition",
            catalogue::TAINT_TRANSFORM_CONDITIONS,
        ),
        ("taintNumericCoercion", catalogue::TAINT_NUMERIC_COERCIONS),
        ("formatType", catalogue::FORMAT_TYPES),
        ("formKind", catalogue::FORM_KINDS),
        ("definedSymbolKind", catalogue::DEFINED_SYMBOL_KINDS),
        ("sideEffectTarget", &catalogue::SIDE_EFFECT_TARGETS),
        ("connectionSide", catalogue::CONNECTION_SIDES),
        ("loweringHook", catalogue::LOWERING_HOOKS),
        ("codegenHook", catalogue::CODEGEN_HOOKS),
        ("inlineCodegenHook", catalogue::INLINE_CODEGEN_HOOKS),
        ("analyserHook", catalogue::ANALYSER_HOOKS),
        ("returnTypeHook", catalogue::RETURN_TYPE_HOOKS),
        ("traits", &catalogue::TRAITS),
        ("taintColour", &catalogue::TAINT_COLOURS),
        ("dialects", &catalogue::DIALECTS),
    ];
    let mut values = serde_json::Map::new();
    values.extend(standard.map(|(id, variants)| (id.to_owned(), catalogue_entries(id, variants))));
    values.extend(custom_catalogues().map(|(id, value)| (id.to_owned(), value)));
    Value::Object(values)
}
