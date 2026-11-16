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

//! Render a draft as a `tcl-registry` command-spec source file.
//!
//! The output is a complete, drop-in `rust/tcl-registry/src/commands/<pack>/`
//! module: the project's AGPL copyright banner, a module doc line, the
//! imports the emitted literals need, hoisted `const` arrays for the option /
//! form / subcommand tables, and a `spec()` returning the `CommandSpec`.
//!
//! Only fields that differ from `CommandSpec::DEFAULT` are emitted; the rest
//! come from the trailing `..CommandSpec::DEFAULT`, matching how every
//! hand-written spec in the registry is laid out.

use std::fmt::Write as _;

use serde_json::Value;

use crate::draft::{self, Draft, UNRENDERABLE_KEY};
use crate::schema::{self, FieldKind, FieldSchema};

/// The project's AGPL-3.0 copyright banner, as it appears at the top of every
/// original Rust source file in the repository.
pub const COPYRIGHT_BANNER: &str = "\
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
// SPDX-License-Identifier: AGPL-3.0-or-later";

/// Escape `s` as the body of a Rust double-quoted string literal.
///
/// Shared with [`crate::draft`], which renders the same literals when it seeds
/// a descriptor field as a Rust expression — one escaper, so a `"` in a
/// keyword word cannot compile in one path and not the other.
pub(crate) fn rust_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn as_str(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

fn as_u64(value: &Value) -> u64 {
    value.as_u64().unwrap_or_default()
}

fn as_bool(value: &Value) -> bool {
    value.as_bool().unwrap_or_default()
}

fn as_array(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

/// Render a `&'static [&'static str]` literal.
fn str_slice(values: &[Value]) -> String {
    let items: Vec<String> = values.iter().map(|v| rust_string(as_str(v))).collect();
    format!("&[{}]", items.join(", "))
}

/// Render the data-only Tk geometry-manager descriptor.  This is deliberately
/// a struct literal rather than a named shared constant: the Studio draft
/// exposes every semantic field, so a user can safely adjust any manager
/// without dropping back to arbitrary Rust.
fn tk_geometry_expr(value: &Value) -> Option<String> {
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
fn clause_grammar_expr(value: &Value, indent: &str) -> Option<String> {
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
fn definition_body_expr(value: &Value, indent: &str) -> Option<String> {
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
fn runtime_backing_expr(value: &Value) -> Option<String> {
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
const fn evaluated(evaluate: bool) -> &'static str {
    if evaluate { ".evaluated()" } else { "" }
}

/// A `semantic_operation` value (`{kind, detail}`) as its
/// `Some(SemanticOperationId::…)` expression.
fn semantic_operation_expr(value: &Value) -> Option<String> {
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

/// A surface key's value as an `Option<&[SpecSurface]>` expression.
fn surface_expr(value: &Value) -> String {
    match value.as_array() {
        Some(items) => format!("Some({})", dialect_set(items)),
        None => "None".to_owned(),
    }
}

/// Render a `&'static [u8]` literal.
fn index_slice(values: &[Value]) -> String {
    let items: Vec<String> = values.iter().map(|v| as_u64(v).to_string()).collect();
    format!("&[{}]", items.join(", "))
}

fn variable_receivers_expr(values: &[Value]) -> String {
    let Some(selected) = draft::selected_variable_receivers(values) else {
        return "VARIABLE_RECEIVERS_REQUIRE_UNIQUE_U8_INDICES_AND_VALID_FORMS".to_owned();
    };
    let rows = selected.iter().map(|(index, form)| {
        format!("({index}, tcl_registry::resolved_invocation::VariableReceiverOperandForm::{form:?})")
    }).collect::<Vec<_>>();
    format!("Some(&[{}])", rows.join(", "))
}

/// Render a slice of catalogue enum variants such as `&[ArgRole::Body]`.
fn enum_slice(ty: &str, values: &[Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|value| tcl_spectcl::catalogue::rust_path(ty, as_str(value)))
        .collect();
    format!("&[{}]", items.join(", "))
}

fn body_interpreter_expr(value: &Value) -> Option<String> {
    match value.get("kind")?.as_str()? {
        "Current" => Some("BodyInterpreter::Current".to_owned()),
        "Argument" => Some(format!(
            "BodyInterpreter::Argument({})",
            value.get("index")?.as_u64()?
        )),
        _ => None,
    }
}

/// Render a bitflag union such as `Traits::A.union(Traits::B)`, or
/// `TYPE::empty()` when no bits are set.
///
/// `.union()` rather than `|`: the option / subcommand tables are hoisted into
/// `const` items, and `bitflags`' `BitOr` impl is not `const`, so a `|` chain
/// there fails to compile ("cannot call non-const operator in constants").
/// `union` is a `const fn`, so it is correct in both a `const` and the `fn`
/// body that returns the spec.
fn flag_union(ty: &str, values: &[Value]) -> String {
    let mut names = values
        .iter()
        .map(|v| tcl_spectcl::catalogue::rust_path(ty, as_str(v)));
    let Some(head) = names.next() else {
        return format!("{ty}::empty()");
    };
    names.fold(head, |acc, name| acc + ".union(" + &name + ")")
}

/// Render a surface row list from the canonical dialect-name list a draft
/// holds.
///
/// Emits the readable aggregate constants (`SpecSurface::ALL_TCL`,
/// `SpecSurface::TCL85_PLUS`) when the list matches one exactly, so the
/// output reads like the hand-written specs rather than a five-row list.
pub(crate) fn dialect_set(values: &[Value]) -> String {
    const AGGREGATES: &[(&str, &[&str])] = &[
        (
            "ALL_TCL",
            &["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"],
        ),
        ("TCL85_PLUS", &["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]),
        ("TCL86_PLUS", &["tcl8.6", "tcl9.0", "tcl9.1"]),
        ("TCL90_PLUS", &["tcl9.0", "tcl9.1"]),
    ];
    let names: Vec<&str> = values.iter().map(as_str).collect();
    if names.is_empty() {
        return "None".to_owned();
    }
    let mut remaining = names.clone();
    let mut parts: Vec<String> = Vec::new();
    for (constant, members) in AGGREGATES {
        if members.iter().all(|m| remaining.contains(m)) {
            parts.push((*constant).to_owned());
            remaining.retain(|n| !members.contains(n));
            break;
        }
    }
    for name in remaining {
        // An unrecognised name is emitted as a comment rather than a bare
        // identifier — `SpecSurface::f5-tmsh` is not valid Rust, and
        // rendering it silently produced a file that only failed at
        // `cargo build`.
        if let Some(release) = tcl_spectcl::catalogue::jim_and_later_release(name) {
            // No named constant: the row is open-ended at one Jim release.
            parts.push(format!(
                "core_in(tcl_dialect::model::Family::Jim, &[({release:?}, None)])"
            ));
            continue;
        }
        let Some(constant) = dialect_constant(name) else {
            parts.push(format!("/* unknown dialect {name:?} */"));
            continue;
        };
        parts.push(constant.to_owned());
    }
    match parts.len() {
        0 => "None".to_owned(),
        // A lone open-ended row is still a row list.
        1 if parts[0].starts_with("core_in(") => {
            format!("surface![SpecSurface::{}]", parts.remove(0))
        }
        1 => format!("SpecSurface::{}", parts.remove(0)),
        // Several named surfaces compose by listing their rows.
        _ => {
            let rows: Vec<String> = parts
                .into_iter()
                .map(|part| format!("SpecSurface::{part}"))
                .collect();
            format!("surface![{}]", rows.join(", "))
        }
    }
}

/// The surface constant for a canonical dialect name, or `None` when the
/// name is not one the registry knows.
fn dialect_constant(name: &str) -> Option<&'static str> {
    Some(match name {
        "tcl8.4" => "TCL84",
        "tcl8.5" => "TCL85",
        "tcl8.6" => "TCL86",
        "tcl9.0" => "TCL90",
        "tcl9.1" => "TCL91",
        "f5-irules" => "IRULES",
        "jim" => "JIM",
        "f5-iapps" => "IAPPS",
        "tk" => "TK",
        "expect" => "EXPECT",
        "bpf" => "BPF",
        "f5-tmsh" => "TMSH",
        "f5-bigip" => "BIGIP",
        "spectcl" => "SPECTCL",
        "sslictcl" => "SSLICTCL",
        _ => return None,
    })
}

fn arity_expr(value: &Value) -> String {
    let min = as_u64(&value["min"]);
    let max = value["max"].as_u64();
    let step = as_u64(&value["step"]);
    let also = value["also_exact"].as_u64();
    // `stepped` is an associated *constructor* taking all three bounds, not a
    // builder method — `Arity::at_least(1).stepped(2)` does not compile.
    let base = if step == 0 {
        match max {
            None => {
                if min == 0 {
                    "Arity::any()".to_owned()
                } else {
                    format!("Arity::at_least({min})")
                }
            }
            Some(max) if max == min => format!("Arity::exact({min})"),
            Some(max) => format!("Arity::new({min}, {max})"),
        }
    } else {
        format!(
            "Arity::stepped({min}, {}, {step})",
            max.map_or_else(|| "Arity::UNLIMITED".to_owned(), |m| m.to_string())
        )
    };
    let expression = match also {
        None => base,
        Some(n) => format!("{base}.with_also_exact({n})"),
    };
    if value["count"] == "positionals" {
        format!("{expression}.with_positionals()")
    } else {
        expression
    }
}

fn appended_arity_expr(value: &Value) -> String {
    match as_str(&value["kind"]) {
        "Exactly" => format!("AppendedArity::Exactly({})", as_u64(&value["n"])),
        "AtLeast" => format!("AppendedArity::AtLeast({})", as_u64(&value["n"])),
        _ => "AppendedArity::Unknown".to_owned(),
    }
}

fn role_map_expr(values: &[Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            format!(
                "({}, ArgRole::{})",
                as_u64(&entry["index"]),
                as_str(&entry["role"])
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

fn presentation_map_expr(values: &[Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            format!(
                "({}, ArgPresentation::{})",
                as_u64(&entry["index"]),
                as_str(&entry["presentation"])
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

fn prefix_map_expr(values: &[Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            format!(
                "({}, {})",
                as_u64(&entry["index"]),
                appended_arity_expr(&entry["arity"])
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

fn callback_taint_input_expr(value: &Value) -> String {
    match as_str(value) {
        "%P" => "CallbackTaintInput::TK_PROPOSED_VALUE".to_owned(),
        "%s" => "CallbackTaintInput::TK_CURRENT_VALUE".to_owned(),
        "%S" => "CallbackTaintInput::TK_EDIT_TEXT".to_owned(),
        "%A" => "CallbackTaintInput::TK_EVENT_CHAR".to_owned(),
        "%K" => "CallbackTaintInput::TK_EVENT_KEYSYM".to_owned(),
        spelling => format!("CallbackTaintInput::TkPercent({})", rust_string(spelling)),
    }
}

fn callback_taint_inputs_expr(values: &[Value]) -> String {
    let inputs: Vec<String> = values.iter().map(callback_taint_input_expr).collect();
    format!("&[{}]", inputs.join(", "))
}

fn callback_taint_map_expr(values: &[Value]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            format!(
                "({}, {})",
                as_u64(&entry["index"]),
                callback_taint_inputs_expr(as_array(&entry["inputs"]))
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

fn arg_type_map_expr(values: &[Value], indent: &str) -> String {
    if values.is_empty() {
        return "&[]".to_owned();
    }
    let inner = format!("{indent}    ");
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            let expected = entry["expected"]
                .as_str()
                .map_or_else(|| "None".to_owned(), |t| format!("Some(TclType::{t})"));
            let transparent: Vec<String> = as_array(&entry["transparent_from"])
                .iter()
                .map(|t| format!("TclType::{}", as_str(t)))
                .collect();
            format!(
                "{inner}({}, ArgTypeHint {{\n{inner}    expected: {expected},\n{inner}    shimmers: {},\n{inner}    transparent_from: &[{}],\n{inner}}}),",
                as_u64(&entry["index"]),
                as_bool(&entry["shimmers"]),
                transparent.join(", "),
            )
        })
        .collect();
    format!("&[\n{}\n{indent}]", items.join("\n"))
}

fn arg_value_expr(entry: &Value, indent: &str) -> String {
    let mut parts = vec![format!(
        "{indent}    value: {},",
        rust_string(as_str(&entry["value"]))
    )];
    let detail = as_str(&entry["detail"]);
    if !detail.is_empty() {
        parts.push(format!("{indent}    detail: {},", rust_string(detail)));
    }
    if let Some(min_tcl) = entry["min_tcl"].as_str() {
        parts.push(format!("{indent}    min_tcl: Some(TclVersion::{min_tcl}),"));
    }
    if let Some(lifecycle) = lifecycle_expr(entry) {
        parts.push(format!("{indent}    lifecycle: {lifecycle},"));
    }
    if let Some(code) = entry["code"].as_i64() {
        parts.push(format!("{indent}    code: Some({code}),"));
    }
    format!(
        "{indent}ArgValue {{\n{}\n{indent}    ..ArgValue::DEFAULT\n{indent}}},",
        parts.join("\n")
    )
}

fn arg_value_map_expr(values: &[Value], indent: &str) -> String {
    if values.is_empty() {
        return "&[]".to_owned();
    }
    let inner = format!("{indent}    ");
    let items: Vec<String> = values
        .iter()
        .map(|entry| {
            let vals: Vec<String> = as_array(&entry["values"])
                .iter()
                .map(|v| arg_value_expr(v, &format!("{inner}        ")))
                .collect();
            format!(
                "{inner}({}, &[\n{}\n{inner}    ]),",
                as_u64(&entry["index"]),
                vals.join("\n")
            )
        })
        .collect();
    format!("&[\n{}\n{indent}]", items.join("\n"))
}

fn integer_domain_expr(value: &Value) -> Option<String> {
    match as_str(&value["kind"]) {
        "Any" => Some("IntegerDomain::Any".to_owned()),
        "Range" => Some(format!(
            "IntegerDomain::Range({}, {})",
            value["lo"].as_i64().unwrap_or_default(),
            value["hi"].as_i64().unwrap_or_default()
        )),
        "Port" => Some("IntegerDomain::Port".to_owned()),
        _ => None,
    }
}

/// The Rust expression supplied for a `Hook` arity, if the author filled it in.
fn hook_expr(arity: &Value) -> Option<&str> {
    arity
        .get("hook")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|h| !h.is_empty())
}

/// Every option row still missing its arity hook, by option name.
fn unfilled_option_hooks(draft: &Value) -> Vec<&str> {
    as_array(draft.get("options").unwrap_or(&Value::Null))
        .iter()
        .filter(|opt| {
            let arity = &opt["value"]["arity"];
            as_str(&arity["kind"]) == "Hook" && hook_expr(arity).is_none()
        })
        .map(|opt| as_str(&opt["name"]))
        .collect()
}

/// Every option row whose lifecycle used a custom callback that seeding could
/// not recover and whose author has not yet supplied an expression.
fn unfilled_option_deprecation_hooks(draft: &Value) -> Vec<&str> {
    as_array(draft.get("options").unwrap_or(&Value::Null))
        .iter()
        .filter(|opt| {
            opt[crate::draft::OPTION_DEPRECATION_FIX_UNRECOVERABLE_KEY]
                .as_bool()
                .unwrap_or(false)
                && opt["deprecation_fix"]
                    .as_str()
                    .is_none_or(|expr| expr.trim().is_empty())
        })
        .map(|opt| as_str(&opt["name"]))
        .collect()
}

/// Render a `Lifecycle` constructor from a draft's four lifecycle keys, or
/// `None` when the draft declares no lifecycle release or fix at all (the
/// `..DEFAULT` field is then simply omitted).
fn lifecycle_expr(entry: &Value) -> Option<String> {
    let release = |key: &str| entry[key].as_str().filter(|v| !v.is_empty());
    let introduced = release("introduced_version");
    let deprecated = release("deprecated_version");
    let retired = release("retired_version");
    let deprecation_fix = entry["deprecation_fix"]
        .as_str()
        .map(str::trim)
        .filter(|expr| !expr.is_empty());
    if introduced.is_none()
        && deprecated.is_none()
        && retired.is_none()
        && deprecation_fix.is_none()
    {
        return None;
    }
    if let Some(deprecation_fix) = deprecation_fix {
        let release_expr = |value: Option<&str>| {
            value.map_or_else(
                || "None".to_owned(),
                |value| format!("Some({})", rust_string(value)),
            )
        };
        return Some(format!(
            "Lifecycle {{ introduced: {}, deprecated: {}, retired: {}, deprecation_fix: {deprecation_fix} }}",
            release_expr(introduced),
            release_expr(deprecated),
            release_expr(retired),
        ));
    }
    let mut expr = match introduced {
        Some(v) => format!("Lifecycle::introduced_in({})", rust_string(v)),
        None => match deprecated {
            Some(v) => format!("Lifecycle::deprecated_in({})", rust_string(v)),
            None => "Lifecycle::UNSPECIFIED".to_owned(),
        },
    };
    if introduced.is_some()
        && let Some(v) = deprecated
    {
        expr = format!("{expr}.deprecated_from({})", rust_string(v));
    }
    if let Some(v) = retired {
        expr = format!("{expr}.retired_from({})", rust_string(v));
    }
    Some(expr)
}

/// What a schema field whose key is one of the four `Lifecycle` editors
/// contributes to the emitted struct literal.
enum LifecycleLine {
    /// Not a release key — render the field the ordinary way.
    Other,
    /// The first release key: emit this text (empty when the draft declares no
    /// release at all, leaving the field to `..DEFAULT`).
    Emit(String),
    /// A later release key — the first already accounted for it.
    Skip,
}

/// Collapse the schema's lifecycle keys back into the registry's one
/// `lifecycle` field.
///
/// `CommandSpec` / `SubCommand` declare **one** `lifecycle` field; the form
/// edits it as independent releases and a quick-fix hook. Walking the schema and
/// emitting each key verbatim produced `introduced_version: Some("21.1.0"),` —
/// a spec file that does not compile (`struct SubCommand has no field named
/// introduced_version`), caught by rendering the registry back into itself.
/// The line lands at the position of the first release key, so field order is
/// unchanged.
fn lifecycle_line(key: &str, draft: &Value, indent: &str) -> LifecycleLine {
    let Some((first, rest)) = crate::coverage::LIFECYCLE_KEYS.split_first() else {
        return LifecycleLine::Other;
    };
    if *first == key {
        return LifecycleLine::Emit(
            lifecycle_expr(draft).map_or_default(|expr| format!("{indent}lifecycle: {expr},\n")),
        );
    }
    if rest.contains(&key) {
        return LifecycleLine::Skip;
    }
    LifecycleLine::Other
}

// Keep every OptionArg field in one renderer so adding registry metadata
// cannot silently miss one shorthand or generic literal path.
#[allow(clippy::too_many_lines)]
/// The Rust expression for a drafted [`tcl_registry::option_effect::EffectAxis`]
/// (`axis` and, for every axis but `case-sensitivity`, `value`).
fn effect_axis_expr(axis: &str, value: Option<&str>) -> Option<String> {
    match axis {
        "substitution" => {
            let kind = match value? {
                "backslashes" => "Backslashes",
                "commands" => "Commands",
                "variables" => "Variables",
                _ => return None,
            };
            Some(format!(
                "EffectAxis::Substitution(SubstitutionKind::{kind})"
            ))
        }
        "pattern-language" => {
            let kind = match value? {
                "glob" => "Glob",
                "regex" => "Regex",
                _ => return None,
            };
            Some(format!("EffectAxis::PatternLanguage(PatternType::{kind})"))
        }
        "case-sensitivity" => Some("EffectAxis::CaseSensitivity".to_owned()),
        "selection" => {
            let mode = match value? {
                "exact" => "Exact",
                "glob" => "Glob",
                "regexp" => "Regexp",
                "other" => "Other",
                _ => return None,
            };
            Some(format!("EffectAxis::Selection(CaseMatchMode::{mode})"))
        }
        _ => None,
    }
}

/// The Rust expression for a drafted `option` row's `effect` key, or `None`
/// for an unset or unreadable one.
fn option_effect_expr(entry: &Value) -> Option<String> {
    if entry.is_null() {
        return None;
    }
    let axis = || effect_axis_expr(as_str(&entry["axis"]), entry["value"].as_str());
    let kind = match as_str(&entry["kind"]) {
        "disables" => format!("OptionEffectKind::Disables({})", axis()?),
        "selects" => format!("OptionEffectKind::Selects({})", axis()?),
        "suppresses-role" => format!(
            "OptionEffectKind::SuppressesRole(ArgRole::{})",
            as_str(&entry["role"])
        ),
        "reserves-trailing-words" => format!(
            "OptionEffectKind::ReservesTrailingWords({})",
            as_u64(&entry["n"])
        ),
        "ends-options" => "OptionEffectKind::EndsOptions".to_owned(),
        _ => return None,
    };
    Some(format!(
        "OptionEffect {{ kind: {kind}, family: {} }}",
        rust_string(as_str(&entry["family"]))
    ))
}

/// The `value: OptionValue::Takes(OptionArg { … }),` line for an option
/// whose drafted `value` is set, or `None` for a bare flag.
fn option_value_expr(value: &Value, inner: &str) -> Option<String> {
    if !value.is_object() {
        return None;
    }
    let arg_indent = format!("{inner}    ");
    let mut arg_parts: Vec<String> = Vec::new();
    let arity = &value["arity"];
    let arity_expr = match as_str(&arity["kind"]) {
        "Fixed" => Some(format!("OptionArity::Fixed({})", as_u64(&arity["n"]))),
        "OptionalOneOf" => Some(format!(
            "OptionArity::OptionalOneOf(&[{}])",
            arity["values"].as_array().map_or_default(|values| values
                .iter()
                .map(|v| format!("{:?}", as_str(v)))
                .collect::<Vec<_>>()
                .join(", "))
        )),
        "OptionalNotOption" => Some("OptionArity::OptionalNotOption".to_owned()),
        // The hook is a function pointer, so seeding cannot recover it — but
        // the author can type it, and then it renders like any other arity.
        // Empty still falls through to the TODO below.
        "Hook" => hook_expr(arity).map(|h| format!("OptionArity::Hook({h})")),
        _ => Some("OptionArity::One".to_owned()),
    };
    match arity_expr {
        Some(expr) if expr != "OptionArity::One" => {
            arg_parts.push(format!("{arg_indent}    arity: {expr},"));
        }
        Some(_) => {}
        None => arg_parts.push(format!(
            "{arg_indent}    // TODO: this option consumed a computed number of words \
             (OptionArity::Hook); supply the hook.",
        )),
    }
    let role = as_str(&value["role"]);
    if !role.is_empty() && role != "Value" {
        arg_parts.push(format!("{arg_indent}    role: ArgRole::{role},"));
    }
    let also_role = value["also_role"].as_str();
    if let Some(also) = also_role {
        arg_parts.push(format!("{arg_indent}    also_role: Some(ArgRole::{also}),"));
    }
    let body_kind = as_str(&value["body_kind"]);
    if body_kind == "Structural" {
        arg_parts.push(format!("{arg_indent}    body_kind: BodyKind::Structural,"));
    }
    let script_timing = as_str(&value["script_timing"]);
    if !script_timing.is_empty() && script_timing != "SameInvocation" {
        arg_parts.push(format!(
            "{arg_indent}    script_timing: ScriptTiming::{script_timing},"
        ));
    }
    let callback_inputs = as_array(&value["callback_taint_inputs"]);
    if !callback_inputs.is_empty() {
        arg_parts.push(format!(
            "{arg_indent}    callback_taint_inputs: {},",
            callback_taint_inputs_expr(callback_inputs)
        ));
    }
    if as_str(&value["variable_scope"]) == "Global" {
        arg_parts.push(format!(
            "{arg_indent}    variable_scope: VariableScope::Global,"
        ));
    }
    let values = as_array(&value["values"]);
    if !values.is_empty() {
        let vals: Vec<String> = values
            .iter()
            .map(|v| arg_value_expr(v, &format!("{arg_indent}        ")))
            .collect();
        arg_parts.push(format!(
            "{arg_indent}    values: &[\n{}\n{arg_indent}    ],",
            vals.join("\n")
        ));
    }
    if as_bool(&value["closed"]) {
        arg_parts.push(format!("{arg_indent}    closed: true,"));
    }
    if let Some(domain) = integer_domain_expr(&value["integer"]) {
        arg_parts.push(format!("{arg_indent}    integer: Some({domain}),"));
    }
    let hint = as_str(&value["hint"]);
    if !hint.is_empty() {
        arg_parts.push(format!("{arg_indent}    hint: {},", rust_string(hint)));
    }
    let appended = appended_arity_expr(&value["appended_arity"]);
    if appended != "AppendedArity::Unknown" {
        arg_parts.push(format!("{arg_indent}    appended_arity: {appended},"));
    }
    if as_bool(&value["taints_var_write"]) && (role == "VarWrite" || also_role == Some("VarWrite"))
    {
        arg_parts.push(format!("{arg_indent}    taints_var_write: true,"));
    }
    Some(format!(
        "{inner}value: OptionValue::Takes(OptionArg {{\n{}\n{arg_indent}..OptionArg::DEFAULT\n{inner}}}),",
        arg_parts.join("\n")
    ))
}

fn option_expr(entry: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let mut parts = vec![format!(
        "{inner}name: {},",
        rust_string(as_str(&entry["name"]))
    )];

    parts.extend(option_value_expr(&entry["value"], &inner));

    let detail = as_str(&entry["detail"]);
    if !detail.is_empty() {
        parts.push(format!("{inner}detail: {},", rust_string(detail)));
    }
    if let Some(dialects) = entry["surface"].as_array() {
        parts.push(format!("{inner}surface: Some({}),", dialect_set(dialects)));
    }
    let aliases = as_array(&entry["aliases"]);
    if !aliases.is_empty() {
        parts.push(format!("{inner}aliases: {},", str_slice(aliases)));
    }
    if let Some(lifecycle) = lifecycle_expr(entry) {
        parts.push(format!("{inner}lifecycle: {lifecycle},"));
    }
    if let Some(min_abbrev) = entry["min_abbrev"].as_u64() {
        parts.push(format!("{inner}min_abbrev: Some({min_abbrev}),"));
    }
    if let Some(effect) = option_effect_expr(&entry["effect"]) {
        parts.push(format!("{inner}effect: Some({effect}),"));
    }
    format!(
        "{indent}OptionSpec {{\n{}\n{inner}..OptionSpec::DEFAULT\n{indent}}},",
        parts.join("\n")
    )
}

/// A row literal: only the fields that differ from the type's `DEFAULT`, then
/// the spread — the same shape every hand-written spec has used since the
/// registry-wide `..DEFAULT` conversion.
fn row_literal(name: &str, parts: &[String], indent: &str) -> String {
    format!(
        "{indent}{name} {{\n{}\n{indent}    ..{name}::DEFAULT\n{indent}}},",
        parts.join("\n")
    )
}

/// `Some(<dialect set>)` for a present dialect gate, or nothing for an absent
/// one — the field's own default.
fn dialects_line(entry: &Value, indent: &str) -> Option<String> {
    let dialects = entry["surface"].as_array()?;
    Some(format!(
        "{indent}    surface: Some({}),",
        dialect_set(dialects)
    ))
}

fn form_expr(entry: &Value, indent: &str) -> String {
    let mut parts = Vec::new();
    let kind = as_str(&entry["kind"]);
    if kind != "Default" {
        parts.push(format!("{indent}    kind: FormKind::{kind},"));
    }
    parts.push(format!(
        "{indent}    synopsis: {},",
        rust_string(as_str(&entry["synopsis"]))
    ));
    parts.extend(dialects_line(entry, indent));
    if let Some(lifecycle) = lifecycle_expr(entry) {
        parts.push(format!("{indent}    lifecycle: {lifecycle},"));
    }
    row_literal("FormSpec", &parts, indent)
}

/// The `Knowledge` literal a facts domain's rows and its draft knowledge
/// object (`{level, complete}`) state together (issue #1712).
///
/// `None` is the `Unknown` default, which the literal leaves to `DEFAULT`.
pub(crate) fn knowledge_expr(rows: Option<String>, knowledge: &Value) -> Option<String> {
    let path = "crate::knowledge::Knowledge";
    let level =
        knowledge["level"]
            .as_str()
            .unwrap_or(if rows.is_some() { "partial" } else { "unknown" });
    let rows = rows.unwrap_or_else(|| "&[]".to_owned());
    match level {
        "partial" => Some(format!("{path}::Partial({rows})")),
        "complete" => {
            let surfaces: Vec<String> = as_array(&knowledge["complete"])
                .iter()
                .filter_map(complete_surface_expr)
                .collect();
            Some(format!(
                "{path}::Complete({rows}, &[{}])",
                surfaces.join(", ")
            ))
        }
        _ => None,
    }
}

/// One `effects -complete` window as a `SpecSurface` literal.
fn complete_surface_expr(words: &Value) -> Option<String> {
    let words: Vec<&str> = as_array(words).iter().filter_map(Value::as_str).collect();
    let surface = "tcl_dialect::model::SpecSurface";
    match words.as_slice() {
        ["package", name] => Some(format!("{surface}::package({})", rust_string(name))),
        ["package", name, from, until] => Some(format!(
            "{surface}::package_in({}, &[({}, Some({}))])",
            rust_string(name),
            rust_string(from),
            rust_string(until)
        )),
        [from, until] => {
            let first = from.parse::<tcl_dialect::model::Release>().ok()?;
            let last = until.parse::<tcl_dialect::model::Release>().ok()?;
            let family = first.family();
            let end = family.next_release(last).map_or_else(
                || "None".to_owned(),
                |next| format!("Some({})", rust_string(next.as_str())),
            );
            Some(format!(
                "{surface}::core_in(tcl_dialect::model::Family::{family:?}, &[({}, {end})])",
                rust_string(first.as_str())
            ))
        }
        _ => None,
    }
}

fn side_effect_expr(entry: &Value, indent: &str) -> String {
    let mut parts = vec![format!(
        "{indent}    target: SideEffectTarget::{},",
        as_str(&entry["target"])
    )];
    for (key, field) in [("reads", "reads"), ("writes", "writes")] {
        if as_bool(&entry[key]) {
            parts.push(format!("{indent}    {field}: true,"));
        }
    }
    let side = as_str(&entry["connection_side"]);
    if side != "None" {
        parts.push(format!(
            "{indent}    connection_side: ConnectionSide::{side},"
        ));
    }
    parts.extend(dialects_line(entry, indent));
    if let Some(lifecycle) = lifecycle_expr(entry) {
        parts.push(format!("{indent}    lifecycle: {lifecycle},"));
    }
    row_literal("SideEffect", &parts, indent)
}

/// One `CommandForm` literal from an invocation-refinement draft row.
///
/// Only the fields the row actually declares are written; everything else
/// rides `..CommandForm::DEFAULT`, which is what makes an omitted overlay
/// inherit rather than blank the parent's fact.
fn command_form_expr(entry: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let mut parts = vec![format!(
        "{inner}name: {},",
        rust_string(as_str(&entry["name"]))
    )];
    let arity = arity_expr(&entry["arity"]);
    if arity != "Arity::any()" {
        parts.push(format!("{inner}arity: {arity},"));
    }
    if let Some(selector) = entry["selector"].as_object() {
        let words = str_slice(as_array(&selector["words"]));
        let constructor = if as_str(&selector["prefix_matching"]) == "Strict" {
            "exact"
        } else {
            "unique"
        };
        parts.push(format!(
            "{inner}literal_argument_prefix: Some(LiteralArgumentPrefix::{constructor}({words})),"
        ));
    }
    let roles = as_array(&entry["arg_roles"]);
    if !roles.is_empty() {
        parts.push(format!("{inner}arg_roles: {},", role_map_expr(roles)));
    }
    if let Some(receivers) = entry["variable_receivers"].as_array() {
        parts.push(format!(
            "{inner}variable_receivers: {},",
            variable_receivers_expr(receivers)
        ));
    }
    let options = as_array(&entry["options"]);
    if !options.is_empty() {
        let opt_indent = format!("{inner}    ");
        let rendered: Vec<String> = options
            .iter()
            .map(|option| option_expr(option, &opt_indent))
            .collect();
        parts.push(format!(
            "{inner}options: &[\n{}\n{inner}],",
            rendered.join("\n")
        ));
    }
    if let Some(relations) = entry["option_relations"].as_str() {
        parts.push(format!("{inner}option_relations: {relations},"));
    }
    if let Some(rows) = entry["surface"].as_array() {
        parts.push(format!("{inner}surface: Some({}),", dialect_set(rows)));
    }
    if let Some(traits) = entry["traits"].as_array() {
        parts.push(format!(
            "{inner}traits: Some({}),",
            flag_union("Traits", traits)
        ));
    }
    if let Some(mutator) = entry["mutator"].as_bool() {
        parts.push(format!("{inner}mutator: Some({mutator}),"));
    }
    if let Some(effects) = entry["side_effects"].as_array() {
        let rows = list_expr(effects, &inner, side_effect_expr);
        let knowledge = &entry["side_effects_knowledge"];
        // A form's empty row list replaces its parent's effects with none,
        // so its default level is partial rather than unknown.
        let expr = if knowledge.is_null() {
            knowledge_expr(Some(rows.unwrap_or_else(|| "&[]".to_owned())), knowledge)
        } else {
            knowledge_expr(rows, knowledge)
        };
        parts.push(format!(
            "{inner}side_effects: Some({}),",
            expr.unwrap_or_else(|| "crate::knowledge::Knowledge::Unknown".to_owned())
        ));
    }
    if let Some(expr) = entry["representation_effect"].as_str() {
        parts.push(format!("{inner}representation_effect: Some({expr}),"));
    }
    for (key, ty) in [
        ("lowering_hook", "LoweringHookId"),
        ("codegen_hook", "CodegenHookId"),
    ] {
        if let Some(id) = entry[key].as_str() {
            parts.push(format!("{inner}{key}: Some({ty}::{id}),"));
        }
    }
    row_literal("CommandForm", &parts, indent)
}

fn setter_constraint_expr(entry: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    format!(
        "{indent}SetterConstraint {{\n{inner}arg_index: {},\n{inner}required_prefix: {},\n{inner}code: DiagCode::{},\n{inner}message: {},\n{indent}}},",
        as_u64(&entry["arg_index"]),
        rust_string(as_str(&entry["required_prefix"])),
        as_str(&entry["code"]),
        rust_string(as_str(&entry["message"])),
    )
}

fn manufacturer_method_expr(entry: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let opt_index = |key: &str| {
        entry[key]
            .as_u64()
            .map_or_else(|| "None".to_owned(), |n| format!("Some({n})"))
    };
    format!(
        "{indent}crate::definer::ManufacturerMethod {{\n{inner}keyword: {},\n{inner}visibility: crate::definer::MemberVisibility::{},\n{inner}names_instance_at: {},\n{inner}definition_body_at: {},\n{inner}constructor_args_from: {},\n{indent}}},",
        rust_string(as_str(&entry["keyword"])),
        as_str(&entry["visibility"]),
        opt_index("names_instance_at"),
        opt_index("definition_body_at"),
        as_u64(&entry["constructor_args_from"]),
    )
}

fn documentation_example_expr(value: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let notes: Vec<String> = as_array(&value["notes"])
        .iter()
        .map(|note| {
            format!(
                "{inner}    crate::documentation::DocumentationAnnotation::new({}, {}, {}),",
                note["line"].as_u64().unwrap_or(0),
                rust_string(as_str(&note["needle"])),
                rust_string(as_str(&note["label"])),
            )
        })
        .collect();
    let code = rust_string(as_str(&value["code"]));
    let notes = format!("&[\n{}\n{inner}]", notes.join("\n"));
    let carrier = &value["carrier"];
    if carrier.is_null() {
        format!(
            "{indent}crate::documentation::DocumentationExample::new(\n{inner}{code},\n{inner}{notes},\n{indent}),"
        )
    } else {
        format!(
            "{indent}crate::documentation::DocumentationExample::with_carrier(\n{inner}{code},\n{inner}crate::documentation::DocumentationCarrier::new({}, {}),\n{inner}{notes},\n{indent}),",
            carrier["line"].as_u64().unwrap_or(0),
            rust_string(as_str(&carrier["needle"])),
        )
    }
}

fn hover_expr(value: &Value, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let synopsis = str_slice(as_array(&value["synopsis"]));
    format!(
        "Some(HoverSnippet {{\n{inner}summary: {},\n{inner}synopsis: {synopsis},\n{inner}snippet: {},\n{inner}source: {},\n{inner}examples: {},\n{inner}return_value: {},\n{indent}}})",
        rust_string(as_str(&value["summary"])),
        rust_string(as_str(&value["snippet"])),
        rust_string(as_str(&value["source"])),
        rust_string(as_str(&value["examples"])),
        rust_string(as_str(&value["return_value"])),
    )
}

fn sub_subcommand_expr(entry: &Value, indent: &str) -> String {
    let mut parts = vec![format!(
        "{indent}    name: {},",
        rust_string(as_str(&entry["name"]))
    )];
    for key in ["detail", "synopsis"] {
        let text = as_str(&entry[key]);
        if !text.is_empty() {
            parts.push(format!("{indent}    {key}: {},", rust_string(text)));
        }
    }
    if let Some(prefix) = entry["option_prefix_words"]
        .as_u64()
        .filter(|prefix| *prefix > 0)
    {
        parts.push(format!("{indent}    option_prefix_words: {prefix},"));
    }
    parts.extend(dialects_line(entry, indent));
    if let Some(lifecycle) = lifecycle_expr(entry) {
        parts.push(format!("{indent}    lifecycle: {lifecycle},"));
    }
    if entry
        .get("native_compilation")
        .is_some_and(|value| !value.is_null())
    {
        parts.push(format!(
            "{indent}    // Native worker compilation proof is excluded from this draft."
        ));
    }
    row_literal("SubSubCommand", &parts, indent)
}

/// Render a `&[...]` literal from `items`, one entry per line, or `None` when
/// the list is empty (the `..DEFAULT` fill already supplies an empty slice).
fn list_expr(
    items: &[Value],
    indent: &str,
    render_one: impl Fn(&Value, &str) -> String,
) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let inner = format!("{indent}    ");
    let rendered: Vec<String> = items.iter().map(|e| render_one(e, &inner)).collect();
    Some(format!("&[\n{}\n{indent}]", rendered.join("\n")))
}

/// Render one field's value expression, or `None` when it matches the
/// default and should be left to `..DEFAULT`.
fn field_expr(field: &FieldSchema, value: &Value, default: &Value, indent: &str) -> Option<String> {
    if value == default {
        return None;
    }
    if field.key == "variable_receivers" {
        return Some(variable_receivers_expr(value.as_array()?));
    }
    let expr = match field.kind {
        FieldKind::Bool => as_bool(value).to_string(),
        FieldKind::OptBool => format!("Some({})", value.as_bool()?),
        FieldKind::Count => as_u64(value).to_string(),
        FieldKind::OptCount | FieldKind::OptIndex => format!("Some({})", value.as_u64()?),
        FieldKind::Text | FieldKind::Prose => rust_string(as_str(value)),
        FieldKind::OptText => format!("Some({})", rust_string(value.as_str()?)),
        FieldKind::TextList => str_slice(as_array(value)),
        FieldKind::IndexList => index_slice(as_array(value)),
        FieldKind::OptIndexList => format!("Some({})", index_slice(value.as_array()?)),
        FieldKind::VariableReceivers => variable_receivers_expr(value.as_array()?),
        FieldKind::Enum {
            catalogue,
            optional,
        } => {
            let variant = value.as_str()?;
            let ty = enum_type_name(catalogue);
            let expr = tcl_spectcl::catalogue::rust_path(ty, variant);
            if optional {
                format!("Some({expr})")
            } else {
                expr
            }
        }
        FieldKind::FlagSet {
            catalogue,
            optional,
        } => {
            let ty = enum_type_name(catalogue);
            if optional {
                let items = value.as_array()?;
                if catalogue == "dialects" {
                    format!("Some({})", dialect_set(items))
                } else {
                    format!("Some({})", flag_union(ty, items))
                }
            } else {
                flag_union(ty, as_array(value))
            }
        }
        FieldKind::EnumList { catalogue } => enum_slice(enum_type_name(catalogue), as_array(value)),
        FieldKind::Arity => arity_expr(value),
        FieldKind::BodyInterpreter => body_interpreter_expr(value)?,
        FieldKind::RoleMap => role_map_expr(as_array(value)),
        FieldKind::PrefixMap => prefix_map_expr(as_array(value)),
        FieldKind::CallbackTaintMap => callback_taint_map_expr(as_array(value)),
        FieldKind::PresentationMap => presentation_map_expr(as_array(value)),
        FieldKind::ArgTypeMap => arg_type_map_expr(as_array(value), indent),
        FieldKind::ArgValueMap => arg_value_map_expr(as_array(value), indent),
        FieldKind::Options => "OPTIONS".to_owned(),
        FieldKind::Forms => "FORMS".to_owned(),
        FieldKind::Refinements => list_expr(as_array(value), indent, command_form_expr)?,
        FieldKind::SubCommands => "SUBCOMMANDS".to_owned(),
        FieldKind::SideEffects => list_expr(as_array(value), indent, side_effect_expr)?,
        FieldKind::Knowledge => return None,
        FieldKind::SetterConstraints => list_expr(as_array(value), indent, setter_constraint_expr)?,
        FieldKind::ManufacturerMethods => {
            list_expr(as_array(value), indent, manufacturer_method_expr)?
        }
        FieldKind::SubSubCommands => list_expr(as_array(value), indent, sub_subcommand_expr)?,
        // The descriptor is hoisted to a `static` beside the command, the way
        // every shipped class spec is written; the field just names it.
        FieldKind::ObjectClass => "Some(&OBJECT_CLASS)".to_owned(),
        FieldKind::TkGeometry => tk_geometry_expr(value)?,
        FieldKind::ClauseGrammar => clause_grammar_expr(value, indent)?,
        FieldKind::DefinitionBody => definition_body_expr(value, indent)?,
        FieldKind::SemanticOperation => semantic_operation_expr(value)?,
        FieldKind::RuntimeBacking => runtime_backing_expr(value)?,
        FieldKind::Hover => {
            if value.is_null() {
                return None;
            }
            hover_expr(value, indent)
        }
        FieldKind::DocumentationExamples => {
            list_expr(as_array(value), indent, documentation_example_expr)?
        }
        FieldKind::RustExpr { .. } => {
            let expr = value.as_str()?.trim();
            if expr.is_empty() {
                return None;
            }
            expr.to_owned()
        }
    };
    Some(expr)
}

/// The Rust type name behind a schema catalogue id.
fn enum_type_name(catalogue: &str) -> &'static str {
    match catalogue {
        "argRole" => "ArgRole",
        "tclType" => "TclType",
        "bodyKind" => "BodyKind",
        "scriptLookupScope" => "ScriptLookupScope",
        "irulesSourceFlow" => "tcl_registry::events::IrulesSourceFlowSpec",
        "storageType" => "StorageType",
        "byteArrayEffect" => "ByteArrayEffect",
        "commandTableEffect" => "CommandTableEffect",
        "patternType" => "PatternType",
        "sourcePathOperation" => "SourcePathOperation",
        "sourceConfigurationReference" => "SourceConfigurationReferenceLayout",
        "sourceIndexBounds" => "SourceIndexBounds",
        "channelConfiguration" => "ChannelConfigurationSpec",
        "formatType" => "FormatType",
        "formKind" => "FormKind",
        "definedSymbolKind" => "DefinedSymbolKind",
        "sideEffectTarget" => "SideEffectTarget",
        "connectionSide" => "ConnectionSide",
        "loweringHook" => "LoweringHookId",
        "codegenHook" => "CodegenHookId",
        "inlineCodegenHook" => "InlineCodegenHookId",
        "analyserHook" => "AnalyserHookId",
        "returnTypeHook" => "ReturnTypeHookId",
        "traits" => "Traits",
        "taintColour" => "TaintColour",
        "dialects" => "&'static [SpecSurface]",
        "defaultFormFirstWord" => "DefaultFormFirstWord",
        "sourceDeprecationAdvice" => "SourceDeprecationAdvice",
        "prefixMatching" => "PrefixMatching",
        "optionPlacement" => "OptionPlacement",
        _ => "Unknown",
    }
}

/// Render a `SubCommand` literal from a subcommand draft.
fn subcommand_literal(sub: &Value, default: &Draft, indent: &str) -> String {
    let inner = format!("{indent}    ");
    let mut lines: Vec<String> = Vec::new();
    for field in schema::SUBCOMMAND_FIELDS {
        match lifecycle_line(field.key, sub, &inner) {
            LifecycleLine::Emit(line) => {
                if !line.is_empty() {
                    lines.push(line.trim_end_matches('\n').to_owned());
                }
                continue;
            }
            LifecycleLine::Skip => continue,
            LifecycleLine::Other => {}
        }
        // Nested tables are inlined here rather than hoisted — a subcommand's
        // option list belongs with the subcommand that declares it.
        let value = sub.get(field.key).unwrap_or(&Value::Null);
        let default_value = default.get(field.key).unwrap_or(&Value::Null);
        let expr = match field.kind {
            FieldKind::Options => {
                let items = crate::render_rs::as_array(value);
                if items.is_empty() {
                    None
                } else {
                    let opt_indent = format!("{inner}    ");
                    let rendered: Vec<String> =
                        items.iter().map(|o| option_expr(o, &opt_indent)).collect();
                    Some(format!("&[\n{}\n{inner}]", rendered.join("\n")))
                }
            }
            FieldKind::SideEffects => knowledge_expr(
                field_expr(field, value, default_value, &inner),
                sub.get("side_effects_knowledge").unwrap_or(&Value::Null),
            ),
            _ => field_expr(field, value, default_value, &inner),
        };
        if let Some(expr) = expr {
            lines.push(format!("{inner}{}: {expr},", field.key));
        }
    }
    for note in unrenderable_notes(sub, &inner) {
        lines.push(note);
    }
    format!(
        "{indent}SubCommand {{\n{}\n{inner}..SubCommand::DEFAULT\n{indent}}},",
        lines.join("\n")
    )
}

/// `TODO` lines for the fields a seeded draft could not recover.
fn unrenderable_notes(draft: &Value, indent: &str) -> Vec<String> {
    as_array(draft.get(UNRENDERABLE_KEY).unwrap_or(&Value::Null))
        .iter()
        .filter_map(|key| {
            let key = key.as_str()?;
            // Option-arity hooks live in an option row, not a top-level field,
            // so they resolve against `options` and name the exact options
            // still outstanding. Filling every one clears the note.
            if key == crate::draft::OPTION_HOOK_KEY {
                let pending = unfilled_option_hooks(draft);
                if pending.is_empty() {
                    return None;
                }
                return Some(format!(
                    "{indent}// TODO: {} consumes a computed number of words \
                     (OptionArity::Hook); supply the hook under that option.",
                    pending
                        .iter()
                        .map(|n| format!("`{n}`"))
                        .collect::<Vec<_>>()
                        .join(", "),
                ));
            }
            if key == crate::draft::OPTION_DEPRECATION_FIX_HOOK_KEY {
                let pending = unfilled_option_deprecation_hooks(draft);
                if pending.is_empty() {
                    return None;
                }
                return Some(format!(
                    "{indent}// TODO: {} uses a contextual deprecation quick-fix callback; \
                     supply the `Some(DeprecationFixHook::Custom {{ resolver: ... }})` expression \
                     under that option.",
                    pending
                        .iter()
                        .map(|name| format!("`{name}`"))
                        .collect::<Vec<_>>()
                        .join(", "),
                ));
            }
            // Only note a field the author has not since filled in.
            let filled = draft
                .get(key)
                .and_then(Value::as_str)
                .is_some_and(|s| !s.trim().is_empty());
            (!filled).then(|| {
                format!(
                    "{indent}// TODO: `{key}` is set on the source command but its defining \
                     expression could not be recovered — supply it here."
                )
            })
        })
        .collect()
}

/// The set of extra imports the emitted literals need beyond the prelude.
fn extra_imports(draft: &Draft) -> Vec<&'static str> {
    let mut imports: Vec<&'static str> = Vec::new();
    let uses_inline_hook = |d: &Value| d.get("inline_codegen_hook").is_some_and(|v| !v.is_null());
    let mut needs_inline = uses_inline_hook(&Value::Object(draft.clone()));
    let mut needs_diag =
        !as_array(draft.get("setter_constraints").unwrap_or(&Value::Null)).is_empty();
    for sub in as_array(draft.get("subcommands").unwrap_or(&Value::Null)) {
        needs_inline = needs_inline || uses_inline_hook(sub);
        needs_diag = needs_diag
            || !as_array(sub.get("setter_constraints").unwrap_or(&Value::Null)).is_empty();
    }
    if needs_inline {
        imports.push("use crate::hooks::InlineCodegenHookId;");
    }
    if needs_diag {
        imports.push("use tcl_core_types::DiagCode;");
    }
    imports
}

/// A hoisted `const NAME: &[TYPE] = &[…];` table, or an empty string when the
/// draft declares no entries for it.
fn const_table(
    name: &str,
    ty: &str,
    items: &[Value],
    render_one: impl Fn(&Value) -> String,
) -> String {
    if items.is_empty() {
        return String::new();
    }
    let rendered: Vec<String> = items.iter().map(render_one).collect();
    format!(
        "const {name}: &[{ty}] = &[\n{}\n];\n\n",
        rendered.join("\n")
    )
}

/// The hoisted `static OBJECT_CLASS: ObjectClassSpec = …;` (and, when the
/// instance methods are not simply the command's own subcommands, the
/// `const OBJECT_CLASS_METHODS` table it points at), or an empty string when
/// the draft declares no class.
///
/// Eleven of the thirteen shipped class specs set `instance_methods:
/// SUBCOMMANDS` — a widget's instance methods *are* its subcommands — so that
/// case is written the way it is written by hand rather than duplicating the
/// table.
fn object_class_statics(draft: &Draft, default_sub: &Draft) -> String {
    let Some(class) = draft.get("object_class").filter(|v| !v.is_null()) else {
        return String::new();
    };
    let methods = as_array(&class["instance_methods"]);
    let subcommands = as_array(draft.get("subcommands").unwrap_or(&Value::Null));
    let (table, methods_expr) = if methods.is_empty() {
        (String::new(), "&[]".to_owned())
    } else if methods == subcommands {
        (String::new(), "SUBCOMMANDS".to_owned())
    } else {
        (
            const_table("OBJECT_CLASS_METHODS", "SubCommand", methods, |m| {
                subcommand_literal(m, default_sub, "    ")
            }),
            "OBJECT_CLASS_METHODS".to_owned(),
        )
    };
    // Older saved Studio drafts predate this field.  Preserve the registry's
    // conservative object-method default instead of emitting an invalid empty
    // enum variant when one is reopened and rendered.
    let method_prefix_matching = match as_str(&class["method_prefix_matching"]) {
        "Enabled" => "Enabled",
        _ => "Strict",
    };
    format!(
        "{table}static OBJECT_CLASS: ObjectClassSpec = ObjectClassSpec {{\n    class_name: \
         {},\n    instance_methods: {methods_expr},\n    superclasses: {},\n    \
         allow_unknown_methods: {},\n    method_prefix_matching: PrefixMatching::{},\n}};\n\n",
        rust_string(as_str(&class["class_name"])),
        str_slice(as_array(&class["superclasses"])),
        as_bool(&class["allow_unknown_methods"]),
        method_prefix_matching,
    )
}

/// The module doc line, from the draft's hover summary when it has one.
///
/// The summary is free text a user typed into a textarea, so it may contain
/// newlines. Every line has to carry its own `//!`: interpolating a multi-line
/// summary verbatim leaves the continuation lines as bare Rust and the
/// advertised drop-in file does not compile.
fn module_doc(name: &str, summary: &str) -> String {
    if summary.trim().is_empty() {
        return format!("//! `{name}`.\n");
    }
    let first = summary
        .split_once(". ")
        .map_or(summary, |(head, _)| head)
        .trim()
        .trim_end_matches('.');
    let mut lines = first.lines().map(str::trim).filter(|l| !l.is_empty());
    let Some(head) = lines.next() else {
        return format!("//! `{name}`.\n");
    };
    let mut out = format!("//! `{name}` — {}", lower_first(head));
    for line in lines {
        out.push_str("\n//! ");
        out.push_str(line);
    }
    out.push_str(".\n");
    out
}

/// How the generated `commands/<pack>/mod.rs` is meant to be applied.
///
/// The directory is the author's to name, and naming a pack the registry
/// already ships is the ordinary case — a contribution to `tcl` or `irules`
/// is a few commands added to hundreds. A whole-file render offered for that
/// directory is a drop-in that silently deletes the rest of the pack, so the
/// two cases are rendered differently and labelled differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleForm {
    /// The directory is new to the registry: the render is the whole file.
    Whole,
    /// The directory is a pack the registry already ships: the render is the
    /// lines to add to the `mod.rs` that is already there.
    Addition,
}

/// Render the `commands/<pack>/mod.rs` that declares each command's module
/// and collects their specs.
///
/// The one file in a pack contribution that is pure bookkeeping: a `mod` line
/// per command and a `vec![…]` naming each `spec()`. `command-registry.md`
/// tells a contributor to write both by hand, and the module stem has to match
/// the path [`suggested_path`] chose — two places to get wrong for no
/// judgement gained.
///
/// It carries `#![allow(non_snake_case)]` for the same reason every
/// hand-written pack does: [`suggested_path`] spells a command's `::` as the
/// registry's double underscore, so `HTTP::header` is `mod http__header` and
/// rustc's lint rejects the consecutive underscores.
///
/// The import is `crate::spec::CommandSpec`: the file is written *into*
/// `tcl-registry`, which does not alias itself, so naming the crate would not
/// resolve.
#[must_use]
pub fn render_pack_module(commands: &[(String, Draft)], pack: &str, form: ModuleForm) -> String {
    let stems: Vec<String> = commands.iter().map(|(name, _)| module_stem(name)).collect();
    let mut sorted: Vec<&String> = stems.iter().collect();
    sorted.sort_unstable();
    // Two commands whose stems collide own one module line: Rust has no
    // spelling for declaring the same module twice.  Which two they were is
    // not lost — `pack_export` reports the collision beside the files.
    sorted.dedup();

    if form == ModuleForm::Addition {
        return render_module_addition(&sorted, pack);
    }

    let mut out = String::from(COPYRIGHT_BANNER);
    let _ = write!(
        out,
        "\n\n//! `{pack}` command specifications.\n\n#![allow(non_snake_case)]\n\n"
    );
    for stem in &sorted {
        let _ = writeln!(out, "mod {stem};");
    }
    out.push_str("\nuse crate::spec::CommandSpec;\n");
    let collector = collector_fn(pack);
    let _ = write!(
        out,
        "\n/// Every command specification the `{pack}` pack declares.\n#[must_use]\npub fn {collector}() -> Vec<CommandSpec> {{\n    vec![\n"
    );
    for stem in &sorted {
        let _ = writeln!(out, "        {stem}::spec(),");
    }
    out.push_str("    ]\n}\n");
    out
}

/// The `mod.rs` rendered as an **addition** to a pack the registry already
/// ships: the `mod` lines and collector entries to put into the file that is
/// already there.
///
/// A whole-file render would be a drop-in that deletes every command the pack
/// already has, so this one is not a file at all — no copyright banner, no
/// collector body, and a banner comment that says which it is before anything
/// else on the page.
fn render_module_addition(stems: &[&String], pack: &str) -> String {
    let dir = format!("rust/tcl-registry/src/commands/{pack}/mod.rs");
    let collector = collector_fn(pack);
    let mut out = String::new();
    out.push_str("// ══════════════════════════════════════════════════════════════════\n");
    out.push_str("// ADDITIONS — NOT A DROP-IN FILE. DO NOT REPLACE mod.rs WITH THIS.\n");
    out.push_str("// ══════════════════════════════════════════════════════════════════\n");
    out.push_str("//\n");
    let _ = writeln!(out, "// `{pack}` is a pack tcl-registry already ships, and");
    let _ = writeln!(out, "// {dir}");
    out.push_str("// already declares every other command in it. Writing this file over\n");
    out.push_str("// that one would delete every last one of them. Merge the two lists\n");
    out.push_str("// below into the file instead, each in the order that file keeps.\n");
    if stems.iter().any(|stem| stem.contains("__")) {
        out.push_str("//\n");
        out.push_str("// A stem below spells a `::` as a double underscore, so the file needs\n");
        out.push_str("// `#![allow(non_snake_case)]` too if it does not carry one already.\n");
    }
    out.push_str("\n// Add beside the existing `mod` declarations:\n");
    for stem in stems {
        let _ = writeln!(out, "mod {stem};");
    }
    out.push('\n');
    let _ = writeln!(
        out,
        "// Add to the list `{collector}()` returns — a shipped pack may build"
    );
    out.push_str("// that from sub-builders or a `const` table, so the rows go wherever\n");
    out.push_str("// it keeps the rest of them:\n");
    for stem in stems {
        let _ = writeln!(out, "    {stem}::spec(),");
    }
    out
}

/// The name of the collector function a pack directory declares.
///
/// The directory is free text a user typed, and `commands/my-lib/` would give
/// `my-lib_command_specs`, which is not an identifier. Every non-alphanumeric
/// run becomes one underscore, exactly as a module stem does.
fn collector_fn(pack: &str) -> String {
    let mut ident = String::new();
    for ch in pack.chars() {
        if ch.is_alphanumeric() {
            ident.extend(ch.to_lowercase());
        } else if !ident.ends_with('_') {
            ident.push('_');
        }
    }
    let ident = ident.trim_matches('_');
    match ident.chars().next() {
        None => "pack_command_specs".to_owned(),
        Some(first) if first.is_ascii_digit() => format!("pack_{ident}_command_specs"),
        Some(_) => format!("{ident}_command_specs"),
    }
}

/// The module name [`suggested_path`] files a command under.
fn module_stem(name: &str) -> String {
    suggested_path(name, "x")
        .rsplit('/')
        .next()
        .and_then(|file| file.strip_suffix(".rs"))
        .unwrap_or("command")
        .to_owned()
}

/// Render `draft` as a complete `tcl-registry` command-spec source file.
#[must_use]
pub fn render(draft: &Draft) -> String {
    let default = draft::default_command_draft();
    let default_sub = draft::default_subcommand_draft();
    let name = draft.get("name").map_or("", |v| as_str(v));
    let summary = draft
        .get("hover")
        .and_then(|h| h.get("summary"))
        .and_then(Value::as_str)
        .unwrap_or("");

    let mut out = String::new();
    out.push_str(COPYRIGHT_BANNER);
    out.push_str("\n\n");
    out.push_str(&module_doc(name, summary));
    out.push('\n');
    for import in extra_imports(draft) {
        out.push_str(import);
        out.push('\n');
    }
    out.push_str("use crate::prelude::*;\n\n");

    out.push_str(&const_table(
        "FORMS",
        "FormSpec",
        as_array(draft.get("forms").unwrap_or(&Value::Null)),
        |f| form_expr(f, "    "),
    ));
    out.push_str(&const_table(
        "OPTIONS",
        "OptionSpec",
        as_array(draft.get("options").unwrap_or(&Value::Null)),
        |o| option_expr(o, "    "),
    ));
    out.push_str(&const_table(
        "SUBCOMMANDS",
        "SubCommand",
        as_array(draft.get("subcommands").unwrap_or(&Value::Null)),
        |s| subcommand_literal(s, &default_sub, "    "),
    ));
    out.push_str(&object_class_statics(draft, &default_sub));

    out.push_str("/// Command spec for `");
    out.push_str(name);
    out.push_str("`.\npub fn spec() -> CommandSpec {\n    CommandSpec {\n");
    let whole = Value::Object(draft.clone());
    for field in schema::COMMAND_FIELDS {
        match lifecycle_line(field.key, &whole, "        ") {
            LifecycleLine::Emit(line) => {
                out.push_str(&line);
                continue;
            }
            LifecycleLine::Skip => continue,
            LifecycleLine::Other => {}
        }
        let value = draft.get(field.key).unwrap_or(&Value::Null);
        let default_value = default.get(field.key).unwrap_or(&Value::Null);
        let expr = match field.kind {
            FieldKind::SideEffects => knowledge_expr(
                field_expr(field, value, default_value, "        "),
                draft.get("side_effects_knowledge").unwrap_or(&Value::Null),
            ),
            _ => field_expr(field, value, default_value, "        "),
        };
        if let Some(expr) = expr {
            out.push_str("        ");
            out.push_str(field.key);
            out.push_str(": ");
            out.push_str(&expr);
            out.push_str(",\n");
        }
    }
    for note in unrenderable_notes(&Value::Object(draft.clone()), "        ") {
        out.push_str(&note);
        out.push('\n');
    }
    out.push_str("        ..CommandSpec::DEFAULT\n    }\n}\n");
    out
}

/// Lower-case the first character, for splicing a summary into a doc line.
fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

/// The conventional file path for a command spec, relative to the repository
/// root — e.g. `rust/tcl-registry/src/commands/tcl/lappend_.rs`.
///
/// The stem follows the convention the registry's 1000-odd command files
/// already keep: a namespace `::` is a **double** underscore, every other run
/// of punctuation is a single one. `IP::ttl` is `ip__ttl.rs` and `ip_ttl` is
/// `ip_ttl.rs` — both are real iRules commands, and collapsing the `::` to one
/// underscore filed them at the same path, where the second silently replaced
/// the first. Names that collide with a Rust keyword get the registry's
/// trailing underscore (`lappend_.rs`).
#[must_use]
pub fn suggested_path(name: &str, pack: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
        "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final",
        "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
    ];
    // A fully-qualified name (`::tcl::dict::lappend`) would otherwise start with
    // the separator's underscores, and a run of other punctuation would double
    // up into the `::` spelling — neither reads like the module names already
    // in the registry.
    let mut stem = String::new();
    let mut chars = name.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_alphanumeric() {
            stem.extend(ch.to_lowercase());
        } else if ch == ':' && chars.peek() == Some(&':') {
            chars.next();
            if !stem.ends_with("__") {
                stem.push_str("__");
            }
        } else if !stem.ends_with('_') {
            stem.push('_');
        }
    }
    let mut stem = stem.trim_matches('_').to_owned();
    if stem.is_empty() {
        stem.push_str("command");
    }
    // The registry also appends `_` to names that shadow a Tcl-side module,
    // which is why `lappend` is `lappend_.rs`; keyword collisions are the
    // mechanical part we can decide here.
    if KEYWORDS.contains(&stem.as_str()) {
        stem.push('_');
    }
    format!("rust/tcl-registry/src/commands/{pack}/{stem}.rs")
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod authored_construction_hint_render_controls;
