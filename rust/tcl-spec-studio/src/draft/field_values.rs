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

//! The draft form of one field value: lifecycles, dialect sets, arity, role and type maps, argument values, side effects, documentation and declared semantics.

use super::Unrecovered;
use super::options_and_forms::deprecation_fix_value;
use crate::catalogue;
use serde_json::{Map, Value, json};
use tcl_dialect::model::SpecSurface;
use tcl_registry::BodyInterpreter;
use tcl_registry::arg_role::{AppendedArity, ArgRole};
use tcl_registry::arity::Arity;
use tcl_registry::hooks::ArgTypeHint;
use tcl_registry::hover::{ArgValue, CallbackTaintInput, HoverSnippet, IntegerDomain};
use tcl_registry::lifecycle::Lifecycle;
use tcl_registry::presentation::ArgPresentation;
use tcl_registry::side_effects::SideEffect;
use tcl_registry::taint::{SetterConstraint, TaintColour};
use tcl_registry::traits::Traits;
use tcl_registry::value_transfer::{
    DeclaredEvaluation, DeclaredSemantics, EvalRoute, SemanticType, SemanticsDeclaration,
};

/// Validate authored receiver rows before either renderer emits the field.
/// Invalid draft data must not silently select Combined or argument zero.
pub(crate) fn selected_variable_receivers(
    values: &[Value],
) -> Option<
    Vec<(
        u8,
        tcl_registry::resolved_invocation::VariableReceiverOperandForm,
    )>,
> {
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm;
    let mut selected = Vec::with_capacity(values.len());
    for value in values {
        let (index, form) = if let Some(row) = value.as_object() {
            if row.len() != 2 {
                return None;
            }
            let index = u8::try_from(row.get("index")?.as_u64()?).ok()?;
            let form = match row.get("form")?.as_str()? {
                "Combined" => VariableReceiverOperandForm::Combined,
                "TraceSubject" => VariableReceiverOperandForm::TraceSubject,
                _ => return None,
            };
            (index, form)
        } else {
            (
                u8::try_from(value.as_u64()?).ok()?,
                VariableReceiverOperandForm::Combined,
            )
        };
        if selected.iter().any(|&(previous, _)| previous == index) {
            return None;
        }
        selected.push((index, form));
    }
    Some(selected)
}

pub(super) fn opt_str(value: Option<&'static str>) -> Value {
    value.map_or(Value::Null, |s| json!(s))
}

/// Write a [`Lifecycle`] into a draft under the canonical keys used by every
/// registry surface. Returns whether every field round-tripped; a custom
/// callback cannot reveal its Rust function path and must be supplied by the
/// author.
pub(crate) fn insert_lifecycle(d: &mut Map<String, Value>, lifecycle: Lifecycle) -> bool {
    let (deprecation_fix, complete) = deprecation_fix_value(lifecycle.deprecation_fix);
    d.insert("introduced_version".into(), opt_str(lifecycle.introduced));
    d.insert("deprecated_version".into(), opt_str(lifecycle.deprecated));
    d.insert("retired_version".into(), opt_str(lifecycle.retired));
    d.insert("deprecation_fix".into(), deprecation_fix);
    complete
}

pub(super) fn str_list(values: &[&'static str]) -> Value {
    Value::Array(values.iter().map(|s| json!(s)).collect())
}

pub(super) fn index_list(values: &[u8]) -> Value {
    Value::Array(values.iter().map(|n| json!(n)).collect())
}

pub(super) fn opt_index(value: Option<u8>) -> Value {
    value.map_or(Value::Null, |n| json!(n))
}

/// A facts domain's completeness as the draft states it: `{level}` plus,
/// when complete, the `effects -complete` window words (issue #1712).
pub(crate) fn knowledge<T>(knowledge: &tcl_registry::knowledge::Knowledge<T>) -> Value {
    let mut out = serde_json::Map::new();
    out.insert("level".into(), json!(knowledge.level().as_str()));
    if let Some(surfaces) = knowledge.complete_surfaces() {
        out.insert(
            "complete".into(),
            Value::Array(
                surfaces
                    .iter()
                    .map(|surface| {
                        tcl_registry::knowledge::complete_window_words(surface)
                            .map_or(Value::Null, |words| json!(words))
                    })
                    .collect(),
            ),
        );
    }
    Value::Object(out)
}

pub(super) fn dialects(value: Option<&'static [SpecSurface]>) -> Value {
    match value {
        None => Value::Null,
        // The draft's vocabulary is catalogue dialect names; the registry
        // owns the one projection from rows onto them. That projection names
        // every Jim row `jim`, so a row open-ended at one Jim release is
        // spelt `jimX.Y+` here, the form the loader reads back.
        Some(rows) => {
            let mut names: Vec<String> = rows
                .iter()
                .flat_map(|row| match (row.provider, row.windows) {
                    (
                        tcl_dialect::model::SpecProvider::Core(tcl_dialect::model::Family::Jim),
                        [(from, None)],
                    ) => vec![format!("jim{from}+")],
                    _ => tcl_registry::model::surface::dialect_names_for_rows(
                        std::slice::from_ref(row),
                    ),
                })
                .collect();
            names.sort_unstable();
            names.dedup();
            Value::Array(names.into_iter().map(|name| json!(name)).collect())
        }
    }
}

pub(super) fn traits(value: Traits) -> Value {
    Value::Array(
        catalogue::trait_keys(value)
            .into_iter()
            .map(|k| json!(k))
            .collect(),
    )
}

pub(super) fn taint(value: Option<TaintColour>) -> Value {
    match value {
        None => Value::Null,
        Some(colour) => Value::Array(
            catalogue::taint_keys(colour)
                .into_iter()
                .map(|k| json!(k))
                .collect(),
        ),
    }
}

pub(crate) fn arity(value: Arity) -> Value {
    json!({
        "min": value.min,
        "count": value.count.as_str(),
        "max": if value.is_unlimited() { Value::Null } else { json!(value.max) },
        "step": value.step,
        "also_exact": value.also_exact.map_or(Value::Null, |n| json!(n)),
    })
}

/// One [`tcl_registry::arity::ArityWindow`] as a draft value — the shape the
/// signature had over one span of the owning package's releases.
pub(crate) fn arity_windows(windows: &[tcl_registry::arity::ArityWindow]) -> Value {
    Value::Array(
        windows
            .iter()
            .map(|window| {
                json!({
                    "arity": arity(window.arity),
                    "lifecycle": {
                        "introduced": opt_str(window.lifecycle.introduced),
                        "deprecated": opt_str(window.lifecycle.deprecated),
                        "retired": opt_str(window.lifecycle.retired),
                    },
                })
            })
            .collect::<Vec<_>>(),
    )
}

/// One list of [`tcl_registry::stamp_window::StampWindow`]s as a draft value —
/// the stamps a command or subcommand carries over spans of the Tcl releases,
/// each as the value the plain stamp's own draft key holds.
pub(crate) fn stamp_windows<T: Copy>(
    windows: &[tcl_registry::stamp_window::StampWindow<T>],
    value: impl Fn(T) -> Value,
) -> Value {
    Value::Array(
        windows
            .iter()
            .map(|window| {
                json!({
                    "value": value(window.value),
                    "lifecycle": {
                        "introduced": opt_str(window.lifecycle.introduced),
                        "deprecated": opt_str(window.lifecycle.deprecated),
                        "retired": opt_str(window.lifecycle.retired),
                    },
                })
            })
            .collect::<Vec<_>>(),
    )
}

/// A hook id as the draft holds it: its catalogue variant's name.
pub(super) fn hook_name<T: std::fmt::Debug + Copy + 'static>(hook: T) -> Value {
    json!(catalogue::variant_name(&hook))
}

pub(super) fn appended_arity(value: AppendedArity) -> Value {
    match value {
        AppendedArity::Exactly(n) => json!({ "kind": "Exactly", "n": n }),
        AppendedArity::AtLeast(n) => json!({ "kind": "AtLeast", "n": n }),
        // `AppendedArity` is `#[non_exhaustive]`; an unrecognised variant
        // degrades to the arity-inert `Unknown` rather than being dropped.
        _ => json!({ "kind": "Unknown" }),
    }
}

pub(super) fn role_map(entries: &[(u8, ArgRole)]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(i, role)| json!({ "index": i, "role": catalogue::variant_name(role) }))
            .collect(),
    )
}

pub(super) fn variable_receiver_positions(
    receivers: Option<
        &[(
            u8,
            tcl_registry::resolved_invocation::VariableReceiverOperandForm,
        )],
    >,
) -> Value {
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm;
    receivers.map_or(Value::Null, |receivers| {
        Value::Array(
            receivers
                .iter()
                .map(|&(index, form)| match form {
                    VariableReceiverOperandForm::Combined => json!(index),
                    VariableReceiverOperandForm::TraceSubject => {
                        json!({ "index": index, "form": "TraceSubject" })
                    }
                })
                .collect(),
        )
    })
}

pub(super) fn role_list(entries: &[ArgRole]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|role| json!(catalogue::variant_name(role)))
            .collect(),
    )
}

pub(super) fn body_interpreter(value: BodyInterpreter) -> Value {
    match value {
        BodyInterpreter::Current => json!({ "kind": "Current" }),
        BodyInterpreter::Argument(index) => json!({ "kind": "Argument", "index": index }),
    }
}

pub(super) fn presentation_map(entries: &[(u8, ArgPresentation)]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(i, p)| json!({ "index": i, "presentation": catalogue::variant_name(p) }))
            .collect(),
    )
}

pub(super) fn prefix_map(entries: &[(u8, AppendedArity)]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(i, a)| json!({ "index": i, "arity": appended_arity(*a) }))
            .collect(),
    )
}

pub(super) fn callback_taint_map(entries: &[(u8, &'static [CallbackTaintInput])]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(index, inputs)| {
                json!({
                    "index": index,
                    "inputs": inputs.iter().map(|input| input.spelling()).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

pub(crate) fn arg_type_map(entries: &[(u8, ArgTypeHint)]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(i, hint)| {
                json!({
                    "index": i,
                    "expected": hint.expected.map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
                    "shimmers": hint.shimmers,
                    "transparent_from": Value::Array(
                        hint.transparent_from
                            .iter()
                            .map(|t| json!(catalogue::variant_name(t)))
                            .collect(),
                    ),
                })
            })
            .collect(),
    )
}

/// Draft form of one literal argument value, plus whether it round-tripped.
///
/// The value rides **two** version axes and carries both: `min_tcl` is the
/// Tcl-core rung it needs, the lifecycle keys are the owning package's
/// releases (`hover.rs`'s `ArgValue`).
pub(crate) fn arg_value(value: &ArgValue) -> (Value, bool) {
    let mut d = Map::new();
    d.insert("value".into(), json!(value.value));
    d.insert("detail".into(), json!(value.detail));
    d.insert(
        "min_tcl".into(),
        value
            .min_tcl
            .map_or(Value::Null, |v| json!(catalogue::variant_name(&v))),
    );
    let complete = insert_lifecycle(&mut d, value.lifecycle);
    d.insert("code".into(), value.code.map_or(Value::Null, |c| json!(c)));
    (Value::Object(d), complete)
}

/// Draft rows for one nested table, noting `key` when a row's lifecycle
/// carried a callback quick-fix whose Rust path seeding cannot name.
pub(super) fn nested_rows<T>(
    items: &[T],
    key: &'static str,
    lost: &mut Unrecovered,
    row: impl Fn(&T) -> (Value, bool),
) -> Value {
    Value::Array(
        items
            .iter()
            .map(|item| {
                let (value, complete) = row(item);
                if !complete {
                    lost.note(key);
                }
                value
            })
            .collect(),
    )
}

pub(super) fn arg_value_map(
    entries: &[(u8, &'static [ArgValue])],
    lost: &mut Unrecovered,
) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(i, values)| {
                json!({
                    "index": i,
                    "values": nested_rows(values, "arg_values", lost, arg_value),
                })
            })
            .collect(),
    )
}

pub(super) fn integer_domain(value: Option<IntegerDomain>) -> Value {
    match value {
        None => Value::Null,
        Some(IntegerDomain::Any) => json!({ "kind": "Any" }),
        Some(IntegerDomain::Range(lo, hi)) => json!({ "kind": "Range", "lo": lo, "hi": hi }),
        Some(IntegerDomain::Port) => json!({ "kind": "Port" }),
    }
}

pub(crate) fn side_effect(effect: &SideEffect) -> (Value, bool) {
    let mut d = Map::new();
    d.insert(
        "target".into(),
        json!(catalogue::variant_name(&effect.target)),
    );
    d.insert("reads".into(), json!(effect.reads));
    d.insert("writes".into(), json!(effect.writes));
    d.insert(
        "connection_side".into(),
        json!(catalogue::variant_name(&effect.connection_side)),
    );
    d.insert("surface".into(), dialects(effect.surface));
    let complete = insert_lifecycle(&mut d, effect.lifecycle);
    (Value::Object(d), complete)
}

pub(crate) fn setter_constraint(constraint: &SetterConstraint) -> Value {
    json!({
        "arg_index": constraint.arg_index,
        "required_prefix": constraint.required_prefix,
        "code": constraint.code.as_str(),
        "message": constraint.message,
    })
}

/// `[{code, carrier: {line, needle} | null, notes: [{line, needle, label}]}]`.
pub(crate) fn documentation_examples(
    examples: &[tcl_registry::documentation::DocumentationExample],
) -> Value {
    Value::Array(
        examples
            .iter()
            .map(|example| {
                json!({
                    "code": example.code,
                    "carrier": example.carrier.map_or(Value::Null, |carrier| {
                        json!({ "line": carrier.line, "needle": carrier.needle })
                    }),
                    "notes": example
                        .annotations
                        .iter()
                        .map(|note| json!({
                            "line": note.line,
                            "needle": note.needle,
                            "label": note.label,
                        }))
                        .collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

pub(crate) fn hover(value: Option<HoverSnippet>) -> Value {
    match value {
        None => Value::Null,
        Some(h) => json!({
            "summary": h.summary,
            "synopsis": str_list(h.synopsis),
            "snippet": h.snippet,
            "source": h.source,
            "examples": h.examples,
            "return_value": h.return_value,
        }),
    }
}

/// `-semantic T`: the DSL spelling of a declared result or yield type — the
/// Tcl type in lower case, or the vendor semantic verbatim.
fn semantic_type_word(semantic: SemanticType) -> String {
    match semantic {
        SemanticType::Tcl(ty) => catalogue::variant_name(&ty).to_lowercase(),
        SemanticType::Vendor(name) => name.to_owned(),
    }
}

/// The `semantics { … }` / `evaluate …` declaration at one scope: the plan is
/// plain data all the way down (like [`command_object_facts`]'s `object_class`
/// one level up) and round-trips in full, *except* a declared implementation's
/// body — carried only by the loader's pack-hook table, never on `CommandSpec`
/// — and an option-level `-evaluate` decline, not yet carried back onto its
/// option row.
/// Those two shapes stay `lost`, exactly as a shipped, compiled-in
/// specialisation — nameable by [`tcl_registry::value_transfer::CommandSemantics::identity`],
/// never reconstructable — already was. An implementation derived from the
/// command's reference body is neither: the body is on the `runtime_backing` row.
pub(super) fn semantics_value(declaration: SemanticsDeclaration, lost: &mut Unrecovered) -> Value {
    let declared = match declaration {
        SemanticsDeclaration::Inherited => return Value::Null,
        SemanticsDeclaration::Declined => return json!("none"),
        SemanticsDeclaration::Declared(semantics) => semantics,
    };
    let Some(declared) = declared.as_declared() else {
        // A shipped, compiled-in specialisation: nameable, not
        // reconstructable, and no pack could have written it either.
        return Value::Null;
    };
    let has_body = matches!(
        declared.evaluation,
        DeclaredEvaluation::Implementation(_)
            | DeclaredEvaluation::Route(EvalRoute::Implementation(_))
    );
    // An implementation the registry derived from the command's reference body
    // is not the draft's to carry: the `runtime_backing` row holds the body it
    // came from, and the next load derives it again.
    if has_body
        && declared.option_declines.is_empty()
        && tcl_registry::value_transfer::reference_body::is_derived(declared)
    {
        return Value::Null;
    }
    if has_body || !declared.option_declines.is_empty() {
        lost.note("semantics");
        return Value::Null;
    }
    declared_semantics_json(declared)
}

/// [`semantics_value`]'s fully-recoverable case.
fn declared_semantics_json(declared: &DeclaredSemantics) -> Value {
    let structure = declared.structure;
    let effects: Vec<&str> = structure.effects.iter().map(|e| e.as_str()).collect();
    let stores = structure.stores.map(|stores| {
        json!({
            "targets": stores.targets,
            "outcome": stores.outcome.as_str(),
        })
    });
    let iterate = structure.iterate.map(|iterate| {
        let kind = match iterate.kind {
            tcl_registry::value_transfer::IterableWord::List => "list".to_owned(),
            tcl_registry::value_transfer::IterableWord::Dict => "dict".to_owned(),
            tcl_registry::value_transfer::IterableWord::Vendor(name) => name.to_owned(),
        };
        json!({
            "binder": iterate.binder,
            "iterable": iterate.iterable,
            "kind": kind,
            "body": iterate.body,
            "yields": iterate.yields.map(semantic_type_word),
            "cardinality": iterate.cardinality,
            "zero_iterations_bind": iterate.zero_iterations_bind,
        })
    });
    let evaluation = match declared.evaluation {
        DeclaredEvaluation::Route(EvalRoute::None { reason }) => json!({
            "kind": "none",
            "reason": reason.as_str(),
        }),
        DeclaredEvaluation::Route(EvalRoute::Direct { id }) => json!({
            "kind": "direct",
            "id": catalogue::variant_name(&id),
        }),
        DeclaredEvaluation::Route(EvalRoute::Expression { language }) => json!({
            "kind": "expression",
            "language": language.as_str(),
        }),
        // Filtered out by `semantics_value` before this is reached.
        DeclaredEvaluation::Route(EvalRoute::Implementation(_))
        | DeclaredEvaluation::Implementation(_) => json!({"kind": "none", "reason": "unauthored"}),
    };
    json!({
        "effects": effects,
        "result": structure.result.map(semantic_type_word),
        "stores": stores,
        "iterate": iterate,
        "evaluation": evaluation,
    })
}
