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

//! Descriptor literals for grammars as data: Tk geometry, clause grammars, definition bodies with their members, runtime backing and semantic operations.

use super::field_values::surface_expr;
use super::{as_array, manufacturer_method_expr, rust_string, str_slice};
use serde_json::Value;

/// Render the data-only Tk geometry-manager descriptor.  This is deliberately
/// a struct literal rather than a named shared constant: the Studio draft
/// exposes every semantic field, so a user can safely adjust any manager
/// without dropping back to arbitrary Rust.
pub(super) fn tk_geometry_expr(value: &Value) -> Option<String> {
    let geometry = value.as_object()?;
    let policy = match geometry.get("container_policy").and_then(Value::as_str) {
        Some("Exclusive") => "Exclusive",
        Some("Independent") => "Independent",
        _ => return None,
    };
    let container_option = match geometry.get("container_option") {
        None | Some(Value::Null) => "None".to_owned(),
        Some(value) => format!("Some({})", rust_string(value.as_str()?)),
    };
    let placement_subcommand = match geometry.get("placement_subcommand") {
        None | Some(Value::Null) => "None".to_owned(),
        Some(value) => format!("Some({})", rust_string(value.as_str()?)),
    };
    let direct_form = geometry
        .get("direct_form")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let release_subcommands = str_slice(as_array(
        geometry.get("release_subcommands").unwrap_or(&Value::Null),
    ));
    Some(format!(
        "Some(TkGeometryManagerSpec {{ container_policy: \
         TkGeometryContainerPolicy::{policy}, container_option: {container_option}, \
         direct_form: {direct_form}, placement_subcommand: {placement_subcommand}, \
         release_subcommands: {release_subcommands} }})"
    ))
}

/// Render the plain-data clause-grammar descriptor as an inline literal:
/// every row and slot is a struct literal, so the whole expression is
/// promoted to `'static` exactly as the shipped grammars' constants are.
pub(super) fn clause_grammar_expr(value: &Value, indent: &str) -> Option<String> {
    let grammar = value.as_object()?;
    let inner = format!("{indent}    ");
    let rows: Option<Vec<String>> = as_array(grammar.get("rows").unwrap_or(&Value::Null))
        .iter()
        .map(clause_row_expr)
        .collect();
    let rows = rows?;
    let rows = if rows.is_empty() {
        "&[]".to_owned()
    } else {
        format!(
            "&[\n{}\n{inner}]",
            rows.iter()
                .map(|row| format!("{inner}    {row},"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    let tail = match grammar.get("tail") {
        None | Some(Value::Null) => "None".to_owned(),
        Some(row) => format!("Some({})", clause_row_expr(row)?),
    };
    let fallthrough = match grammar.get("fallthrough_body") {
        None | Some(Value::Null) => "None".to_owned(),
        Some(word) => format!("Some({})", rust_string(word.as_str()?)),
    };
    let default = match grammar.get("default_clause") {
        None | Some(Value::Null) => "None".to_owned(),
        Some(default) => {
            let row = match default.get("row") {
                None | Some(Value::Null) => "None".to_owned(),
                Some(row) => format!("Some({})", row.as_u64()?),
            };
            let final_only = default.get("final_only").and_then(Value::as_bool)?;
            format!("Some(DefaultClause {{ row: {row}, final_only: {final_only} }})")
        }
    };
    let selection = tcl_registry::clause_grammar::ClauseSelection::from_spelling(
        grammar.get("selection").and_then(Value::as_str)?,
    )?;
    let surface = surface_expr(grammar.get("surface").unwrap_or(&Value::Null));
    Some(format!(
        "Some(&ClauseGrammarSpec {{\n\
         {inner}head: {head},\n\
         {inner}rows: {rows},\n\
         {inner}tail: {tail},\n\
         {inner}fallthrough_body: {fallthrough},\n\
         {inner}default_clause: {default},\n\
         {inner}selection: ClauseSelection::{selection:?},\n\
         {inner}surface: {surface},\n\
         {indent}}})",
        head = clause_row_expr(grammar.get("head")?)?,
    ))
}

/// One clause row as a `ClauseRow` literal.
fn clause_row_expr(row: &Value) -> Option<String> {
    let keyword = match row.get("keyword") {
        None | Some(Value::Null) => "None".to_owned(),
        Some(word) => format!("Some({})", rust_string(word.as_str()?)),
    };
    let keyword_required = row.get("keyword_required").and_then(Value::as_bool)?;
    let slots: Option<Vec<String>> = as_array(row.get("slots").unwrap_or(&Value::Null))
        .iter()
        .map(clause_slot_expr)
        .collect();
    let slots = slots?.join(", ");
    let shape = match row.get("shape").and_then(Value::as_str)? {
        "repeated" => format!("ClauseRowShape::Repeated {{ slots: &[{slots}] }}"),
        "once" => format!("ClauseRowShape::Once {{ slots: &[{slots}] }}"),
        "group" => format!(
            "ClauseRowShape::Group {{ layout: {} }}",
            row.get("layout").and_then(Value::as_u64)?
        ),
        _ => return None,
    };
    let timing = match tcl_registry::clause_grammar::ClauseTiming::from_spelling(
        row.get("timing").and_then(Value::as_str)?,
    )? {
        tcl_registry::clause_grammar::ClauseTiming::LoopFixture(phase) => {
            format!("ClauseTiming::LoopFixture(LoopPhase::{phase:?})")
        }
        timing => format!("ClauseTiming::{timing:?}"),
    };
    let surface = surface_expr(row.get("surface").unwrap_or(&Value::Null));
    Some(format!(
        "ClauseRow {{ keyword: {keyword}, keyword_required: {keyword_required}, shape: {shape}, \
         timing: {timing}, surface: {surface} }}"
    ))
}

/// One clause slot as a `ClauseSlot` literal.
fn clause_slot_expr(slot: &Value) -> Option<String> {
    let role = slot.get("role").and_then(Value::as_str)?;
    let noise = match slot.get("noise") {
        None | Some(Value::Null) => "None".to_owned(),
        Some(word) => format!("Some({})", rust_string(word.as_str()?)),
    };
    let handler = match slot.get("handler") {
        None | Some(Value::Null) => "None".to_owned(),
        Some(word) => format!(
            "Some(HandlerMatch::{:?})",
            tcl_registry::clause_grammar::handler_from_spelling(word.as_str()?)?
        ),
    };
    let conditional = slot.get("conditional_binding").and_then(Value::as_bool)?;
    let optional = slot.get("optional").and_then(Value::as_bool)?;
    Some(format!(
        "ClauseSlot {{ role: ArgRole::{role}, noise: {noise}, handler: {handler}, \
         conditional_binding: {conditional}, optional: {optional} }}"
    ))
}

/// The registry constant a shipped grammar's name spells:
/// `tcloo-configurable` → `TCLOO_CONFIGURABLE_GRAMMAR`.
fn shipped_grammar_constant(name: &str) -> String {
    format!("{}_GRAMMAR", name.to_ascii_uppercase().replace('-', "_"))
}

/// Render a `definition_body` value: a shipped grammar's constant, or the
/// whole grammar as an inline literal — every member row, built-in method,
/// member-body command and manufacturer a struct literal — promoted to
/// `'static` exactly as the shipped grammars' constants are.
pub(super) fn definition_body_expr(value: &Value, indent: &str) -> Option<String> {
    if let Some(name) = value.as_str() {
        tcl_spectcl::SHIPPED_DEFINITION_BODIES
            .iter()
            .any(|(shipped, _)| *shipped == name)
            .then_some(())?;
        return Some(format!(
            "Some(&crate::definer::{})",
            shipped_grammar_constant(name)
        ));
    }
    let grammar = value.as_object()?;
    let inner = format!("{indent}    ");
    let rows = format!("{inner}    ");
    let list = |key: &str, render: &dyn Fn(&Value) -> Option<String>| -> Option<String> {
        let items: Option<Vec<String>> = as_array(grammar.get(key).unwrap_or(&Value::Null))
            .iter()
            .map(render)
            .collect();
        let items = items?;
        Some(if items.is_empty() {
            "&[]".to_owned()
        } else {
            format!(
                "&[\n{}\n{inner}]",
                items
                    .iter()
                    .map(|item| format!("{rows}{item},"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
    };
    let strs = |key: &str| str_slice(as_array(grammar.get(key).unwrap_or(&Value::Null)));
    let flag = |key: &str| grammar.get(key).and_then(Value::as_bool).unwrap_or(false);
    let unknown = match grammar.get("unknown_dispatch_method") {
        None | Some(Value::Null) => "None".to_owned(),
        Some(word) => format!("Some({})", rust_string(word.as_str()?)),
    };
    // The hint is a function pointer a draft cannot hold; the loader's own
    // placeholder is what a pack's `bare_word_construction` installs.
    let hint = if flag("bare_word_construction_hint") {
        "None /* TODO(studio): the bare-word construction hint is a function pointer */"
    } else {
        "None"
    };
    let source_hint =
        bare_word_source_hint_expr(grammar.get("bare_word_construction_source_hint"))?;
    let family = grammar.get("family").and_then(Value::as_str)?;
    Some(format!(
        "Some(&crate::definer::DefinitionBodyGrammar {{\n\
         {inner}family: crate::definer::DefinerFamily::{family},\n\
         {inner}members: {members},\n\
         {inner}implicit_vars: {implicit_vars},\n\
         {inner}implicit_receiver_vars: {implicit_receiver_vars},\n\
         {inner}member_body_namespace_path: {path},\n\
         {inner}builtin_type_methods: {type_methods},\n\
         {inner}builtin_object_methods: {object_methods},\n\
         {inner}builtin_terminating_methods: {terminating},\n\
         {inner}member_body_commands: {body_commands},\n\
         {inner}bare_word_construction: {bare},\n\
         {inner}bare_word_construction_hint: {hint},\n\
         {inner}bare_word_construction_source_hint: {source_hint},\n\
         {inner}dynamic_method_dispatch: {dynamic},\n\
         {inner}manufacturers: {manufacturers},\n\
         {inner}unknown_dispatch_method: {unknown},\n\
         {inner}property_accessor_methods: {accessors},\n\
         {indent}}})",
        members = list("members", &member_spec_expr)?,
        implicit_vars = strs("implicit_vars"),
        implicit_receiver_vars = strs("implicit_receiver_vars"),
        path = strs("member_body_namespace_path"),
        type_methods = strs("builtin_type_methods"),
        object_methods = list("builtin_object_methods", &builtin_object_method_expr)?,
        terminating = strs("builtin_terminating_methods"),
        body_commands = list("member_body_commands", &member_body_command_expr)?,
        bare = flag("bare_word_construction"),
        dynamic = flag("dynamic_method_dispatch"),
        manufacturers = list("manufacturers", &|entry| {
            Some(
                manufacturer_method_expr(entry, "")
                    .trim_end_matches(',')
                    .to_owned(),
            )
        })?,
        accessors = strs("property_accessor_methods"),
    ))
}

fn bare_word_source_hint_expr(value: Option<&Value>) -> Option<String> {
    Some(match value {
        None | Some(Value::Null) => "None".to_owned(),
        Some(hint) => {
            let values = hint.get("values")?.as_array()?;
            let prefixes = hint.get("prefixes")?.as_array()?;
            if !values.iter().all(Value::is_string)
                || !prefixes
                    .iter()
                    .all(|value| value.as_str().is_some_and(|prefix| !prefix.is_empty()))
            {
                return None;
            }
            format!(
                "Some(crate::definer::BareWordConstructionSourceHint {{ values: {}, prefixes: {} }})",
                str_slice(values),
                str_slice(prefixes)
            )
        }
    })
}

/// `Some(expr)` of a present key, `None` for a null one.
fn opt_expr(
    value: Option<&Value>,
    render: impl FnOnce(&Value) -> Option<String>,
) -> Option<String> {
    match value {
        None | Some(Value::Null) => Some("None".to_owned()),
        Some(value) => Some(format!("Some({})", render(value)?)),
    }
}

/// One member row as a `MemberSpec` literal.
fn member_spec_expr(member: &Value) -> Option<String> {
    let roles: Option<Vec<String>> = as_array(member.get("arg_roles").unwrap_or(&Value::Null))
        .iter()
        .map(|pair| {
            Some(format!(
                "({}, ArgRole::{})",
                pair.get("index")?.as_u64()?,
                pair.get("role")?.as_str()?
            ))
        })
        .collect();
    let optional = opt_expr(member.get("optional_argument"), |optional| {
        let values: Option<Vec<String>> = as_array(optional.get("values")?)
            .iter()
            .map(|value| {
                Some(format!(
                    "crate::definer::MemberOptionValue {{ value: {}, role: ArgRole::{}, \
                     surface: {}, declared_visibility: {} }}",
                    rust_string(value.get("value")?.as_str()?),
                    value.get("role")?.as_str()?,
                    surface_expr(value.get("surface").unwrap_or(&Value::Null)),
                    opt_expr(value.get("declared_visibility"), |visibility| {
                        Some(format!(
                            "crate::definer::DeclaredMemberVisibility::{}",
                            visibility.as_str()?
                        ))
                    })?,
                ))
            })
            .collect();
        Some(format!(
            "crate::definer::OptionalMemberArgument {{ position: {}, values: &[{}] }}",
            optional.get("position")?.as_u64()?,
            values?.join(", ")
        ))
    })?;
    let named = |key: &str, ty: &str| {
        opt_expr(member.get(key), |value| {
            Some(format!("crate::definer::{ty}::{}", value.as_str()?))
        })
    };
    let slot = opt_expr(member.get("slot"), |slot| {
        Some(format!(
            "crate::definer::SlotSpec {{ default_op: crate::definer::SlotOp::{}, dedup: {} }}",
            slot.get("default_op")?.as_str()?,
            slot.get("dedup")?.as_bool()?
        ))
    })?;
    let shift = opt_expr(member.get("wrapper_shift"), |shift| {
        let receiver = opt_expr(shift.get("receiver"), |receiver| {
            Some(format!(
                "crate::definer::MemberReceiver::{:?}",
                tcl_registry::definer::MemberReceiver::from_spelling(receiver.as_str()?)?
            ))
        })?;
        let visibility = opt_expr(shift.get("visibility"), |visibility| {
            Some(format!(
                "crate::definer::DeclaredMemberVisibility::{:?}",
                tcl_registry::definer::DeclaredMemberVisibility::from_spelling(
                    visibility.as_str()?
                )?
            ))
        })?;
        Some(format!(
            "crate::definer::WrapperShift {{ receiver: {receiver}, visibility: {visibility} }}"
        ))
    })?;
    Some(format!(
        "crate::definer::MemberSpec {{ keyword: {keyword}, arg_roles: &[{roles}], \
         optional_argument: {optional}, all_args_var: {all_vars}, all_args_ref: {all_refs}, \
         kind: crate::definer::MemberKind::{kind}, wrapper_block_body: {block}, \
         surface: {surface}, retraction: {retraction}, slot: {slot}, \
         visibility_effect: {visibility}, effect: {effect}, wrapper_shift: {shift} }}",
        keyword = rust_string(member.get("keyword")?.as_str()?),
        roles = roles?.join(", "),
        all_vars = member.get("all_args_var")?.as_bool()?,
        all_refs = named("all_args_ref", "MemberRefKind")?,
        kind = member.get("kind")?.as_str()?,
        block = member.get("wrapper_block_body")?.as_bool()?,
        surface = surface_expr(member.get("surface").unwrap_or(&Value::Null)),
        retraction = named("retraction", "MemberRetraction")?,
        visibility = named("visibility_effect", "MemberVisibility")?,
        effect = member_effect_expr(member.get("effect")?)?,
    ))
}

/// A member effect's draft value as a `MemberEffect` expression.
fn member_effect_expr(effect: &Value) -> Option<String> {
    use tcl_registry::definer::{
        CallableRole, InitTiming, MemberReceiver, RelationSlot, StateScope,
    };
    let spelled = |key: &str| effect.get(key).and_then(Value::as_str);
    let slot = |key: &str| -> Option<String> {
        match effect.get(key) {
            None | Some(Value::Null) => Some("None".to_owned()),
            Some(value) => Some(format!("Some({})", value.as_u64()?)),
        }
    };
    let index = |key: &str| effect.get(key).and_then(Value::as_u64);
    let path = "crate::definer::MemberEffect";
    Some(match spelled("kind")? {
        "callable" => format!(
            "{path}::Callable {{ receiver: crate::definer::MemberReceiver::{:?}, \
             role: crate::definer::CallableRole::{:?}, name_slot: {}, params_slot: {}, \
             body_slot: {} }}",
            MemberReceiver::from_spelling(spelled("receiver")?)?,
            CallableRole::from_spelling(spelled("role")?)?,
            slot("name_slot")?,
            slot("params_slot")?,
            slot("body_slot")?,
        ),
        "forward" => format!(
            "{path}::Forward {{ name_slot: {}, prefix_slot: {} }}",
            index("name_slot")?,
            index("prefix_slot")?
        ),
        "state-declaration" => format!(
            "{path}::StateDeclaration {{ scope: crate::definer::StateScope::{:?} }}",
            StateScope::from_spelling(spelled("scope")?)?
        ),
        "relation" => format!(
            "{path}::Relation {{ slot: crate::definer::RelationSlot::{:?} }}",
            RelationSlot::from_spelling(spelled("slot")?)?
        ),
        "init-script" => format!(
            "{path}::InitScript {{ body_slot: {}, timing: crate::definer::InitTiming::{:?} }}",
            index("body_slot")?,
            InitTiming::from_spelling(spelled("timing")?)?
        ),
        "visibility" => format!("{path}::Visibility"),
        "retraction" => format!("{path}::Retraction"),
        "configuration" => format!("{path}::Configuration"),
        _ => return None,
    })
}

/// One built-in object method as a `BuiltinObjectMethod` literal.
fn builtin_object_method_expr(method: &Value) -> Option<String> {
    let operation = match method.get("operation") {
        None | Some(Value::Null) => "None".to_owned(),
        Some(value) => format!(
            "Some(crate::definer::BuiltinObjectMethodOperation::{})",
            value.as_str()?
        ),
    };
    Some(format!(
        "crate::definer::BuiltinObjectMethod {{ name: {}, visibility: \
         crate::definer::MemberVisibility::{}, receiver: crate::definer::BuiltinMethodReceiver::{}, \
         operation: {operation}, detail: {} }}",
        rust_string(method.get("name")?.as_str()?),
        method.get("visibility")?.as_str()?,
        method.get("receiver")?.as_str()?,
        rust_string(method.get("detail")?.as_str()?),
    ))
}

/// One member-body command as a `MemberBodyCommand` literal. The draft holds
/// its handle binding as the `Some(&HandleBindingSpec { … })` the command-level
/// field takes; this field holds the spec by value.
fn member_body_command_expr(command: &Value) -> Option<String> {
    let binds = match command.get("binds_handle") {
        None | Some(Value::Null) => "None".to_owned(),
        Some(expr) => expr.as_str()?.replacen("Some(&", "Some(", 1),
    };
    Some(format!(
        "crate::definer::MemberBodyCommand {{ name: {}, detail: {}, binds_handle: {binds} }}",
        rust_string(command.get("name")?.as_str()?),
        rust_string(command.get("detail")?.as_str()?),
    ))
}

/// A `runtime_backing` value (the statement's spelling) as its
/// `RuntimeBacking` expression, or `None` for a spelling that does not read.
pub(super) fn runtime_backing_expr(value: &Value) -> Option<String> {
    use tcl_spectcl::BackingSyntax;
    Some(match BackingSyntax::parse_spelling(value.as_str()?).ok()? {
        BackingSyntax::None => "RuntimeBacking::None".to_owned(),
        BackingSyntax::HostNative => "RuntimeBacking::HostNative".to_owned(),
        BackingSyntax::ShippedBuiltin { identity } => {
            format!("RuntimeBacking::shipped({})", rust_string(&identity))
        }
        BackingSyntax::PackageSource {
            relative_path,
            evaluate,
        } => format!(
            "RuntimeBacking::package_source({}){}",
            rust_string(&relative_path),
            evaluated(evaluate)
        ),
        BackingSyntax::PackText { text, evaluate } => format!(
            "RuntimeBacking::pack_text({}){}",
            rust_string(&text),
            evaluated(evaluate)
        ),
    })
}

/// The call that carries the author's assertion that a Tcl body may be evaluated.
pub(super) const fn evaluated(evaluate: bool) -> &'static str {
    if evaluate { ".evaluated()" } else { "" }
}

/// A `semantic_operation` value (`{kind, detail}`) as its
/// `Some(SemanticOperationId::…)` expression.
pub(super) fn semantic_operation_expr(value: &Value) -> Option<String> {
    use tcl_registry::semantic_operation::SemanticOperationId;
    let kind = value.get("kind")?.as_str()?;
    let detail = value.get("detail").and_then(Value::as_str);
    let operation = tcl_spectcl::semantic_operations()
        .find(|operation| operation.kind_str() == kind && operation.detail_str() == detail)?;
    Some(match operation {
        SemanticOperationId::Invoke => "Some(SemanticOperationId::Invoke)".to_owned(),
        SemanticOperationId::Intrinsic(intrinsic) => {
            format!("Some(SemanticOperationId::Intrinsic(IntrinsicId::{intrinsic:?}))")
        }
        SemanticOperationId::StructuredLowering(lowering) => {
            format!("Some(SemanticOperationId::StructuredLowering(LoweringHookId::{lowering:?}))")
        }
    })
}
