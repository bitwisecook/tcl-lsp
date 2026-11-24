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

//! The Rust expression recovered for a descriptor field the draft cannot hold as data.

use super::field_values::dialects;
use super::options_and_forms::deprecation_fix_expr;
use crate::catalogue;
use crate::render_rs::rust_string;
use serde_json::{Value, json};
use tcl_dialect::model::SpecSurface;
use tcl_registry::handle_binding::{
    HandleBindingSpec, HandleClassSource, HandleKeyword, HandleName,
};
use tcl_registry::lifecycle::Lifecycle;
use tcl_registry::option_effect::{EffectAxis, FamilyBase, OptionEffectFamily};
use tcl_registry::remote_method::{MethodWord, RemoteDispatch, RemoteFamily, RemoteMethodRole};
use tcl_registry::repeated::RepeatedArgLayout;
use tcl_registry::representation::RepresentationEffect;
use tcl_registry::spec::{BytePayloadSpec, OoContextFact, OptionRelation, VersionedArgValue};
use tcl_registry::symbol_def::SymbolDef;
use tcl_registry::types::{ReturnElements, VarElementsEffect, VarWriteTyping};

/// The Rust expression for a [`VarWriteTyping`], fully qualified.
///
/// Written out rather than derived from `Debug`: the `Fixed` payload is itself
/// an enum, and `Debug` prints its variant without a path (`Fixed(String)`),
/// which does not compile. The exhaustive `match` also means a new variant
/// breaks the build here rather than silently rendering a bad spec.
pub(super) fn var_write_typing_expr(value: VarWriteTyping) -> String {
    match value {
        VarWriteTyping::ReturnValue => "VarWriteTyping::ReturnValue".to_owned(),
        VarWriteTyping::Fixed(t) => format!(
            "VarWriteTyping::Fixed({})",
            catalogue::qualified_variant("TclType", &t)
        ),
        VarWriteTyping::Destructured => "VarWriteTyping::Destructured".to_owned(),
        VarWriteTyping::ElementsOf { container_arg } => {
            format!("VarWriteTyping::ElementsOf {{ container_arg: {container_arg} }}")
        }
    }
}

/// The Rust expression for a [`ReturnElements`], wrapped in `Some(…)` for the
/// `Option`-typed field that holds it.
pub(super) fn return_elements_expr(value: ReturnElements) -> String {
    let inner = match value {
        ReturnElements::ListOfArgs { from } => format!("ListOfArgs {{ from: {from} }}"),
        ReturnElements::DictOfPairs { from } => format!("DictOfPairs {{ from: {from} }}"),
        ReturnElements::ElementOf { container_arg } => {
            format!("ElementOf {{ container_arg: {container_arg} }}")
        }
        ReturnElements::SubListOf { container_arg } => {
            format!("SubListOf {{ container_arg: {container_arg} }}")
        }
    };
    format!("Some(ReturnElements::{inner})")
}

/// The Rust expression for a [`VarElementsEffect`], wrapped in `Some(…)`.
pub(super) fn var_elements_effect_expr(value: VarElementsEffect) -> String {
    let inner = match value {
        VarElementsEffect::SetsArrayElementsFromList { values_at } => {
            format!("SetsArrayElementsFromList {{ values_at: {values_at} }}")
        }
        VarElementsEffect::AppendsListElements { values_from } => {
            format!("AppendsListElements {{ values_from: {values_from} }}")
        }
        VarElementsEffect::SetsDictValue => "SetsDictValue".to_owned(),
        VarElementsEffect::ExtendsDictValuesByName { values_from } => {
            format!("ExtendsDictValuesByName {{ values_from: {values_from} }}")
        }
        VarElementsEffect::ListifiesDictValue => "ListifiesDictValue".to_owned(),
    };
    format!("Some(VarElementsEffect::{inner})")
}

/// The Rust expression for an `Option<u8>` field of a descriptor literal.
fn opt_u8_expr(value: Option<u8>) -> String {
    value.map_or_else(|| "None".to_owned(), |n| format!("Some({n})"))
}

/// The Rust expression for an `Option<&'static str>` field of a descriptor
/// literal.
fn opt_str_expr(value: Option<&'static str>) -> String {
    value.map_or_else(
        || "None".to_owned(),
        |s| format!("Some({})", rust_string(s)),
    )
}

/// The Rust expression for a [`Lifecycle`], spelled as a struct literal.
///
/// The literal names all three releases rather than chaining the
/// `introduced_in(…).deprecated_from(…)` builders, so a release the type gains
/// later cannot be dropped silently — [`crate::coverage`] checks the rendered
/// text mentions every field.
fn lifecycle_expr(value: Lifecycle) -> Option<String> {
    let deprecation_fix = match value.deprecation_fix {
        Some(hook) => deprecation_fix_expr(hook)?,
        None => "None".to_owned(),
    };
    Some(format!(
        "Lifecycle {{ introduced: {}, deprecated: {}, retired: {}, deprecation_fix: {} }}",
        opt_str_expr(value.introduced),
        opt_str_expr(value.deprecated),
        opt_str_expr(value.retired),
        deprecation_fix,
    ))
}

/// The Rust expression for one [`RepeatedArgLayout`].
///
/// Written as a full struct literal rather than one of the `every` / `strided`
/// constructors: a constructor hides the fields it defaults, and hiding a field
/// is exactly the drift this module exists to prevent.
fn repeated_arg_layout_expr(layout: RepeatedArgLayout) -> String {
    format!(
        "RepeatedArgLayout {{ role: {}, start: {}, stride: {}, exclude_trailing: {}, \
         optional_leading_word: {}, conditional_binding: {} }}",
        catalogue::qualified_variant("ArgRole", &layout.role),
        layout.start,
        layout.stride,
        layout.exclude_trailing,
        layout.optional_leading_word,
        layout.conditional_binding,
    )
}

/// The Rust expression for a `&'static [RepeatedArgLayout]` field.
pub(super) fn repeated_args_expr(layouts: &[RepeatedArgLayout]) -> String {
    let items: Vec<String> = layouts
        .iter()
        .copied()
        .map(repeated_arg_layout_expr)
        .collect();
    format!("&[{}]", items.join(", "))
}

/// The Rust expression for a [`HandleName`].
fn handle_name_expr(name: HandleName) -> String {
    match name {
        HandleName::Word(i) => format!("HandleName::Word({i})"),
        HandleName::Implicit(v) => format!("HandleName::Implicit({})", rust_string(v)),
    }
}

/// The Rust expression for a [`HandleClassSource`].
fn handle_class_source_expr(source: HandleClassSource) -> String {
    match source {
        HandleClassSource::Word(i) => format!("HandleClassSource::Word({i})"),
        HandleClassSource::ConstructionValue(i) => {
            format!("HandleClassSource::ConstructionValue({i})")
        }
    }
}

/// The Rust expression for a [`HandleKeyword`].
fn handle_keyword_expr(keyword: HandleKeyword) -> String {
    format!(
        "HandleKeyword {{ at: {}, word: {} }}",
        keyword.at,
        rust_string(keyword.word)
    )
}

/// The Rust expression for a [`HandleBindingSpec`] reference, wrapped in
/// `Some(&…)`.
///
/// The whole descriptor is plain data (two indices, a fieldless-payload enum,
/// and an optional keyword), so it round-trips: `&`-borrowing a constant struct
/// literal promotes to the `&'static` the field wants.
pub(super) fn handle_binding_expr(spec: &HandleBindingSpec) -> String {
    format!(
        "Some(&HandleBindingSpec {{ name_from: {}, class_from: {}, keyword: {} }})",
        handle_name_expr(spec.name_from),
        handle_class_source_expr(spec.class_from),
        spec.keyword.map_or_else(
            || "None".to_owned(),
            |k| format!("Some({})", handle_keyword_expr(k))
        ),
    )
}

/// The Rust expression for a [`RemoteMethodRole`] reference, wrapped in
/// `Some(&…)`.
///
/// Like [`handle_binding_expr`], the whole descriptor is plain data — two
/// fieldless-payload enums and a couple of indices — so `&`-borrowing the
/// struct literal promotes to the `&'static` the field wants and the value
/// round-trips through the draft.
pub(super) fn remote_method_expr(role: RemoteMethodRole) -> String {
    let inner = match role {
        RemoteMethodRole::OpensHandle(spec) => format!(
            "RemoteMethodRole::OpensHandle(RemoteHandleSpec {{ family: {}, scope_arg: {}, \
             extension_arg: {}, exact_argc: {} }})",
            remote_family_expr(spec.family),
            spec.scope_arg,
            spec.extension_arg,
            spec.exact_argc,
        ),
        RemoteMethodRole::CallsMethod(spec) => format!(
            "RemoteMethodRole::CallsMethod(RemoteCallSpec {{ family: {}, handle_arg: {}, \
             method: {}, dispatch: {} }})",
            remote_family_expr(spec.family),
            spec.handle_arg,
            match spec.method {
                MethodWord::At(index) => format!("MethodWord::At({index})"),
                MethodWord::AfterOptions(index) => format!("MethodWord::AfterOptions({index})"),
            },
            match spec.dispatch {
                RemoteDispatch::Synchronous => "RemoteDispatch::Synchronous",
                RemoteDispatch::Notification => "RemoteDispatch::Notification",
            },
        ),
    };
    format!("Some(&{inner})")
}

/// The Rust expression for one [`RemoteFamily`] variant.
fn remote_family_expr(family: RemoteFamily) -> &'static str {
    match family {
        RemoteFamily::IRulesLxNode => "RemoteFamily::IRulesLxNode",
    }
}

/// The Rust expression for a [`BytePayloadSpec`], wrapped in `Some(…)`.
pub(super) fn byte_payload_expr(spec: BytePayloadSpec) -> String {
    format!(
        "Some(BytePayloadSpec {{ replace_data_index: {}, message_flag_shift: {} }})",
        spec.replace_data_index, spec.message_flag_shift,
    )
}

/// The Rust expression for a [`SymbolDef`], wrapped in `Some(…)`.
pub(super) fn symbol_def_expr(def: SymbolDef) -> String {
    format!(
        "Some(SymbolDef {{ name_arg: {}, detail_arg: {}, requires_arg: {}, kind: {} }})",
        def.name_arg,
        opt_u8_expr(def.detail_arg),
        opt_u8_expr(def.requires_arg),
        catalogue::qualified_variant("DefinedSymbolKind", &def.kind),
    )
}

/// The Rust expression for a `&'static [(&'static str, OoContextFact)]` field.
pub(super) fn oo_context_facts_expr(facts: &[(&'static str, OoContextFact)]) -> String {
    let items: Vec<String> = facts
        .iter()
        .map(|(word, fact)| {
            format!(
                "({}, {})",
                rust_string(word),
                catalogue::qualified_variant("OoContextFact", fact)
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

/// The Rust expression for a `&'static [VersionedArgValue]` field.
pub(super) fn versioned_arg_values_expr(gates: &[VersionedArgValue]) -> Option<String> {
    let items: Option<Vec<String>> = gates
        .iter()
        .map(|gate| {
            Some(format!(
                "VersionedArgValue {{ index: {}, value: {}, lifecycle: {} }}",
                gate.index,
                rust_string(gate.value),
                lifecycle_expr(gate.lifecycle)?,
            ))
        })
        .collect();
    Some(format!("&[{}]", items?.join(", ")))
}

pub(super) fn representation_effect_expr(effect: RepresentationEffect) -> String {
    match effect {
        RepresentationEffect::None => "Some(RepresentationEffect::None)".to_owned(),
        RepresentationEffect::ObserveValue => "Some(RepresentationEffect::ObserveValue)".to_owned(),
        RepresentationEffect::CoerceExpressionValues { arguments_from } => format!(
            "Some(RepresentationEffect::CoerceExpressionValues {{ arguments_from: {arguments_from} }})"
        ),
        RepresentationEffect::CoerceNumericValues { arguments_from } => format!(
            "Some(RepresentationEffect::CoerceNumericValues {{ arguments_from: {arguments_from} }})"
        ),
        RepresentationEffect::CoerceOrdinaryList { operand } => {
            format!("Some(RepresentationEffect::CoerceOrdinaryList {{ operand: {operand} }})")
        }
        RepresentationEffect::CoerceOrdinaryDictionary { operand } => {
            format!("Some(RepresentationEffect::CoerceOrdinaryDictionary {{ operand: {operand} }})")
        }
        RepresentationEffect::CoerceOrdinaryListPairs { variables_from } => format!(
            "Some(RepresentationEffect::CoerceOrdinaryListPairs {{ variables_from: {variables_from} }})"
        ),
        RepresentationEffect::CoerceOrdinaryListIndices {
            operand,
            indices_from,
        } => format!(
            "Some(RepresentationEffect::CoerceOrdinaryListIndices {{ operand: {operand}, indices_from: {indices_from} }})"
        ),
        RepresentationEffect::CoerceOrdinaryListRange {
            operand,
            first,
            last,
        } => format!(
            "Some(RepresentationEffect::CoerceOrdinaryListRange {{ operand: {operand}, first: {first}, last: {last} }})"
        ),
        RepresentationEffect::CopyOnWriteContainerMutation {
            variable_arg,
            minimum_arguments,
        } => format!(
            "Some(RepresentationEffect::CopyOnWriteContainerMutation {{ variable_arg: {variable_arg}, minimum_arguments: {minimum_arguments} }})"
        ),
    }
}

fn dialect_set_expr(set: &'static [SpecSurface]) -> String {
    let members = dialects(Some(set));
    crate::render_rs::dialect_set(
        members
            .as_array()
            .expect("a present dialect set always seeds an array"),
    )
}

/// The Rust expression for one [`tcl_registry::OptionTerm`].
fn relation_term_expr(term: tcl_registry::OptionTerm) -> String {
    use tcl_registry::OptionTerm as T;
    match term {
        T::Option(name) => format!("OptionTerm::Option({})", rust_string(name)),
        T::OptionValue(name, value) => format!(
            "OptionTerm::OptionValue({}, {})",
            rust_string(name),
            rust_string(value)
        ),
        T::Argument(index) => format!("OptionTerm::Argument({index})"),
        T::ArgumentValue(index, value) => {
            format!("OptionTerm::ArgumentValue({index}, {})", rust_string(value))
        }
    }
}

/// The Rust expression for a `&'static [OptionRelation]` field.
///
/// A full struct literal per relation, `lifecycle` included: a relation can
/// itself be gated — it only exists once both its operands do.
pub(super) fn option_relations_expr(constraints: &[OptionRelation]) -> Option<String> {
    let items: Option<Vec<String>> = constraints
        .iter()
        .map(|relation| {
            let terms = relation
                .terms
                .iter()
                .copied()
                .map(relation_term_expr)
                .collect::<Vec<_>>()
                .join(", ");
            let subject = relation.subject.map_or_else(
                || "None".to_owned(),
                |term| format!("Some({})", relation_term_expr(term)),
            );
            let dialects = relation.surface.map_or_else(
                || "None".to_owned(),
                |set| format!("Some({})", dialect_set_expr(set)),
            );
            let message = relation.message.map_or_else(
                || "None".to_owned(),
                |text| format!("Some({})", rust_string(text)),
            );
            Some(format!(
                "OptionRelation {{ kind: RelationKind::{:?}, mode: RelationMode::{:?}, \
                 subject: {subject}, terms: &[{terms}], surface: {dialects}, \
                 lifecycle: {}, message: {message} }}",
                relation.kind,
                relation.mode,
                lifecycle_expr(relation.lifecycle)?,
            ))
        })
        .collect();
    Some(format!("&[{}]", items?.join(", ")))
}

/// The Rust expression for one [`EffectAxis`].
fn effect_axis_expr(axis: EffectAxis) -> String {
    match axis {
        EffectAxis::Substitution(kind) => format!(
            "EffectAxis::Substitution({})",
            catalogue::qualified_variant("SubstitutionKind", &kind)
        ),
        EffectAxis::PatternLanguage(kind) => format!(
            "EffectAxis::PatternLanguage({})",
            catalogue::qualified_variant("PatternType", &kind)
        ),
        EffectAxis::CaseSensitivity => "EffectAxis::CaseSensitivity".to_owned(),
        EffectAxis::Selection(mode) => format!(
            "EffectAxis::Selection({})",
            catalogue::qualified_variant("CaseMatchMode", &mode)
        ),
    }
}

/// The Rust expression for a `&'static [OptionEffectFamily]` field.
pub(super) fn option_effect_families_expr(families: &[OptionEffectFamily]) -> String {
    let items: Vec<String> = families
        .iter()
        .map(|family| {
            let base = match family.base {
                FamilyBase::AllOn => "FamilyBase::AllOn".to_owned(),
                FamilyBase::AllOff => "FamilyBase::AllOff".to_owned(),
                FamilyBase::Only(axis) => format!("FamilyBase::Only({})", effect_axis_expr(axis)),
            };
            let surface = family.surface.map_or_else(
                || "None".to_owned(),
                |set| format!("Some({})", dialect_set_expr(set)),
            );
            format!(
                "OptionEffectFamily {{ name: {}, base: {base}, combine: FamilyCombine::{:?}, \
                 surface: {surface} }}",
                rust_string(family.name),
                family.combine,
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

/// A draft value for a descriptor field: the rendered Rust expression when the
/// field is set, `null` when it is at its default.
///
/// `null` is what the schema's `RustExpr` editor shows as "unset", and what the
/// renderer reads as "leave it to `..DEFAULT`" — so an unset descriptor stays
/// out of the emitted spec exactly as before.
pub(super) fn expr_or_null(set: bool, render: impl FnOnce() -> String) -> Value {
    if set { json!(render()) } else { Value::Null }
}
