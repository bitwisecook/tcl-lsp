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

//! Definition bodies as data: members and their effects, semantic operations, clause grammars and object classes.

use super::field_values::{dialects, role_map, str_list};
use super::rust_expressions::handle_binding_expr;
use super::subcommand_seed::subcommand_body;
use super::{UNRENDERABLE_KEY, Unrecovered};
use crate::catalogue;
use serde_json::{Value, json};
use tcl_registry::clause_grammar::{ClauseGrammarSpec, ClauseRow, ClauseRowShape, ClauseSlot};
use tcl_registry::definer::{
    BuiltinObjectMethod, DeclaredMemberVisibility, DefinitionBodyGrammar, ManufacturerMethod,
    MemberBodyCommand, MemberEffect, MemberReceiver, MemberSpec, OptionalMemberArgument,
};
use tcl_registry::semantic_operation::SemanticOperationId;
use tcl_registry::spec::ObjectClassSpec;

pub(super) fn manufacturer_method(method: &ManufacturerMethod) -> Value {
    json!({
        "keyword": method.keyword,
        "visibility": catalogue::variant_name(&method.visibility),
        "names_instance_at": method.names_instance_at,
        "definition_body_at": method.definition_body_at,
        "constructor_args_from": method.constructor_args_from,
    })
}

/// Seed the `definition_body` value: `null`, the name of the shipped grammar
/// whose data this is (`tcloo`, `snit`, … —
/// [`tcl_spectcl::SHIPPED_DEFINITION_BODIES`], the names a pack may write), or
/// the whole grammar.
///
/// The shipped grammar is recognised by its data rather than its address: a
/// `const` has no single address (every `&TCLOO_GRAMMAR` is its own promoted
/// allocation), and a pack that spells a shipped grammar out inline — the
/// snit port — describes exactly the grammar the name does.
pub(crate) fn definition_body(grammar: Option<&'static DefinitionBodyGrammar>) -> Value {
    let Some(grammar) = grammar else {
        return Value::Null;
    };
    let block = definition_body_block(grammar);
    tcl_spectcl::SHIPPED_DEFINITION_BODIES
        .iter()
        .find(|(_, shipped)| definition_body_block(shipped) == block)
        .map_or(block, |(name, _)| json!(name))
}

/// A grammar as data, field for field — the inline `definition_body` value.
/// The bare-word construction hint is a function pointer, so the draft records
/// only whether one is set.
#[must_use]
pub fn definition_body_block(grammar: &DefinitionBodyGrammar) -> Value {
    let DefinitionBodyGrammar {
        family,
        members,
        implicit_vars,
        implicit_receiver_vars,
        member_body_namespace_path,
        builtin_type_methods,
        builtin_object_methods,
        builtin_terminating_methods,
        member_body_commands,
        bare_word_construction,
        bare_word_construction_hint,
        bare_word_construction_source_hint,
        dynamic_method_dispatch,
        manufacturers,
        unknown_dispatch_method,
        property_accessor_methods,
    } = grammar;
    json!({
        "family": catalogue::variant_name(family),
        "members": Value::Array(members.iter().map(member_spec).collect()),
        "implicit_vars": str_list(implicit_vars),
        "implicit_receiver_vars": str_list(implicit_receiver_vars),
        "member_body_namespace_path": str_list(member_body_namespace_path),
        "builtin_type_methods": str_list(builtin_type_methods),
        "builtin_object_methods": Value::Array(
            builtin_object_methods.iter().map(builtin_object_method).collect()
        ),
        "builtin_terminating_methods": str_list(builtin_terminating_methods),
        "member_body_commands": Value::Array(
            member_body_commands.iter().map(member_body_command).collect()
        ),
        "bare_word_construction": bare_word_construction,
        "bare_word_construction_hint": bare_word_construction_hint.is_some(),
        "bare_word_construction_source_hint": bare_word_construction_source_hint.map(|hint| json!({ "values": str_list(hint.values), "prefixes": str_list(hint.prefixes) })),
        "dynamic_method_dispatch": dynamic_method_dispatch,
        "manufacturers": Value::Array(manufacturers.iter().map(manufacturer_method).collect()),
        "unknown_dispatch_method": unknown_dispatch_method,
        "property_accessor_methods": str_list(property_accessor_methods),
    })
}

/// One member row: its keyword and layout, what it declares, and what it does
/// to the members it names.
fn member_spec(member: &MemberSpec) -> Value {
    json!({
        "keyword": member.keyword,
        "arg_roles": role_map(member.arg_roles),
        "optional_argument": member.optional_argument.map_or(Value::Null, optional_member_argument),
        "all_args_var": member.all_args_var,
        "all_args_ref": member.all_args_ref.map(|kind| catalogue::variant_name(&kind)),
        "kind": catalogue::variant_name(&member.kind),
        "wrapper_block_body": member.wrapper_block_body,
        "surface": dialects(member.surface),
        "retraction": member.retraction.map(|retraction| catalogue::variant_name(&retraction)),
        "slot": member.slot.map_or(Value::Null, |slot| json!({
            "default_op": catalogue::variant_name(&slot.default_op),
            "dedup": slot.dedup,
        })),
        "visibility_effect": member
            .visibility_effect
            .map(|visibility| catalogue::variant_name(&visibility)),
        "effect": member_effect(member.effect),
        "wrapper_shift": member.wrapper_shift.map_or(Value::Null, |shift| json!({
            "receiver": shift.receiver.map(MemberReceiver::spelling),
            "visibility": shift.visibility.map(DeclaredMemberVisibility::as_str),
        })),
    })
}

/// A member's optional word: its fixed position and each accepted spelling.
fn optional_member_argument(optional: OptionalMemberArgument) -> Value {
    json!({
        "position": optional.position,
        "values": Value::Array(optional.values.iter().map(|value| json!({
            "value": value.value,
            "role": catalogue::variant_name(&value.role),
            "surface": dialects(value.surface),
            "declared_visibility": value
                .declared_visibility
                .map(|visibility| catalogue::variant_name(&visibility)),
        })).collect()),
    })
}

/// A member effect in the `.tclspec` vocabulary: its kind and the payload that
/// kind carries (`{kind: callable, receiver: instance, role: method,
/// name_slot: 0, …}`).
pub(crate) fn member_effect(effect: MemberEffect) -> Value {
    let kind = effect.kind_spelling();
    match effect {
        MemberEffect::Callable {
            receiver,
            role,
            name_slot,
            params_slot,
            body_slot,
        } => json!({
            "kind": kind,
            "receiver": receiver.spelling(),
            "role": role.spelling(),
            "name_slot": name_slot,
            "params_slot": params_slot,
            "body_slot": body_slot,
        }),
        MemberEffect::Forward {
            name_slot,
            prefix_slot,
        } => json!({ "kind": kind, "name_slot": name_slot, "prefix_slot": prefix_slot }),
        MemberEffect::StateDeclaration { scope } => {
            json!({ "kind": kind, "scope": scope.spelling() })
        }
        MemberEffect::Relation { slot } => json!({ "kind": kind, "slot": slot.spelling() }),
        MemberEffect::InitScript { body_slot, timing } => {
            json!({ "kind": kind, "body_slot": body_slot, "timing": timing.spelling() })
        }
        MemberEffect::Visibility | MemberEffect::Retraction | MemberEffect::Configuration => {
            json!({ "kind": kind })
        }
    }
}

fn builtin_object_method(method: &BuiltinObjectMethod) -> Value {
    json!({
        "name": method.name,
        "visibility": catalogue::variant_name(&method.visibility),
        "receiver": catalogue::variant_name(&method.receiver),
        "operation": method.operation.as_ref().map(catalogue::variant_name),
        "detail": method.detail,
    })
}

fn member_body_command(command: &MemberBodyCommand) -> Value {
    json!({
        "name": command.name,
        "detail": command.detail,
        "binds_handle": command.binds_handle.as_ref().map(handle_binding_expr),
    })
}

/// Seed a `semantic_operation` value: `null`, or `{kind, detail}` in the
/// operation's own spellings ([`SemanticOperationId::kind_str`] /
/// [`SemanticOperationId::detail_str`]), a closed vocabulary.
pub(crate) fn semantic_operation(operation: Option<SemanticOperationId>) -> Value {
    operation.map_or(
        Value::Null,
        |operation| json!({ "kind": operation.kind_str(), "detail": operation.detail_str() }),
    )
}

/// Seed the `clause_grammar` value from a live [`ClauseGrammarSpec`].
///
/// The descriptor is plain data all the way down — rows of slots, each a role
/// with a few flags — so the draft holds the grammar itself, spelt in the
/// `.tclspec` vocabulary (`-timing per-iteration`, `-pattern
/// completion-code`), never an [`Unrecovered`] note. A row's surface is the
/// same dialect-name list every other surface key holds.
pub(crate) fn clause_grammar(grammar: Option<&'static ClauseGrammarSpec>) -> Value {
    let Some(grammar) = grammar else {
        return Value::Null;
    };
    json!({
        "head": clause_row(&grammar.head),
        "rows": Value::Array(grammar.rows.iter().map(clause_row).collect()),
        "tail": grammar.tail.as_ref().map_or(Value::Null, clause_row),
        "fallthrough_body": grammar.fallthrough_body,
        "default_clause": grammar.default_clause.map_or(Value::Null, |default| json!({
            "row": default.row,
            "final_only": default.final_only,
        })),
        "selection": grammar.selection.spelling(),
        "surface": dialects(grammar.surface),
    })
}

/// One clause row: its keyword, its shape (the slots, or the layout a group
/// cites), its timing and its surface.
pub(crate) fn clause_row(row: &ClauseRow) -> Value {
    let (shape, layout) = match row.shape {
        ClauseRowShape::Repeated { .. } => ("repeated", None),
        ClauseRowShape::Once { .. } => ("once", None),
        ClauseRowShape::Group { layout } => ("group", Some(layout)),
    };
    json!({
        "keyword": row.keyword,
        "keyword_required": row.keyword_required,
        "shape": shape,
        "slots": Value::Array(row.slots().iter().map(clause_slot).collect()),
        "layout": layout,
        "timing": row.timing.spelling(),
        "surface": dialects(row.surface),
    })
}

/// One clause slot: its role and the flags that qualify it.
pub(crate) fn clause_slot(slot: &ClauseSlot) -> Value {
    json!({
        "role": catalogue::variant_name(&slot.role),
        "noise": slot.noise,
        "handler": slot.handler.map(tcl_registry::clause_grammar::handler_spelling),
        "conditional_binding": slot.conditional_binding,
        "optional": slot.optional,
    })
}

/// Seed the `object_class` value from a live [`ObjectClassSpec`].
///
/// The five fields are plain data — a class name, a method table that *is*
/// `&[SubCommand]`, superclass names, and two policies — so the draft holds the
/// descriptor itself rather than an [`Unrecovered`] note. Each instance method
/// is drafted with the same [`subcommand_body`] the command's own subcommands
/// use, which is what lets the `SpecTcl` renderer write `method NAME { … }`
/// rows in the `subcommand` body grammar the DSL reuses for them.
pub(super) fn object_class(class: Option<&'static ObjectClassSpec>) -> Value {
    let Some(class) = class else {
        return Value::Null;
    };
    let methods: Vec<Value> = class
        .instance_methods
        .iter()
        .map(|method| {
            let mut method_lost = Unrecovered::default();
            let mut body = subcommand_body(method, &mut method_lost);
            body.insert(UNRENDERABLE_KEY.to_owned(), method_lost.into_value());
            Value::Object(body)
        })
        .collect();
    json!({
        "class_name": class.class_name,
        "instance_methods": Value::Array(methods),
        "superclasses": str_list(class.superclasses),
        "allow_unknown_methods": class.allow_unknown_methods,
        "method_prefix_matching": catalogue::variant_name(&class.method_prefix_matching),
    })
}
