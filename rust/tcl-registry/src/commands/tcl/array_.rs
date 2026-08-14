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

//! `array` — manipulate array variables.

use crate::hooks::{CodegenHookId, InlineCodegenHookId};
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "array option arrayName ?arg arg ...?",
    ..FormSpec::DEFAULT
}];

/// `array default`'s sub-verb (position 0 after the `default` word) — TIP
/// 508, landed as Tcl 9.0 (the TIP's target "8.7" became the 9.0 release).
/// Unlike `array names`' `mode` word (see [`NAMES_MODE_VALUES`]), this
/// position is unambiguous: `array default <verb> arrayName ?value?` always
/// has the verb immediately after `default`, so it is safe to close (see
/// `closed_value_args` on the `default` [`SubCommand`] below).
const DEFAULT_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "exists",
        detail: "Returns a boolean indicating whether a default value has been set for the array.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "get",
        detail: "Returns the current default value for the array.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "set",
        detail: "Sets the default value for the array to value.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "unset",
        detail: "Removes the default value for the array.",
        ..ArgValue::DEFAULT
    },
];

/// `array names arrayName ?mode? ?pattern?`'s `mode` word — one of `-exact`,
/// `-glob` (the default when `mode` is omitted), or `-regexp`, present since
/// Tcl 8.4. Completion/hover data only: `mode` and `pattern` share position 1
/// when only one trailing word is given (confirmed against tclsh 8.6.14 —
/// `array names arr -glob` with no further word reads `-glob` as `mode` with
/// an empty pattern, while `array names arr -exact` against an array holding
/// a literal `-exact` key with *no* other trailing word instead matches it as
/// a literal glob pattern), so this position cannot be `closed_value_args`
/// without risking a false-positive W127 on a legitimate pattern that happens
/// to spell one of these words.
const NAMES_MODE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "-exact",
        detail: "pattern is a literal element name, matched exactly.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "-glob",
        detail: "pattern is a glob-style pattern (the default matching rule).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "-regexp",
        detail: "pattern is matched as a regular expression.",
        ..ArgValue::DEFAULT
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "anymore",
        arity: Arity::exact(2),
        detail: "Returns 1 if there are any more elements left to be processed in an array search, 0 if all elements have already been returned.",
        synopsis: "array anymore arrayName searchId",
        return_type: Some(TclType::Boolean),
        arg_roles: &[(0, ArgRole::VarRead)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "default",
        // `default exists|get|unset arrayName` is 2 words after `default`;
        // only `default set arrayName value` reaches 3 (TIP 508: `set`
        // takes a `value`, the other three verbs take only `arrayName`).
        arity: Arity::new(2, 3),
        detail: "Manages the default value of the array.",
        synopsis: "array default subcommand arrayName args...",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        // `default set` (create-on-demand) and `default unset` mutate; `get`
        // /`exists` only read. The single flat entry covers all four verbs,
        // so this declares the union per the read/write field docs.
        mutator: true,
        dialects: Some(DialectSet::TCL90_PLUS),
        arg_values: &[(0, DEFAULT_VALUES)],
        closed_value_args: &[0],
        arg_values_accept_prefix: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "donesearch",
        arity: Arity::exact(2),
        detail: "Terminates an array search and destroys all the state associated with that search.",
        synopsis: "array donesearch arrayName searchId",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::ArrayExists)),
        arity: Arity::exact(1),
        detail: "Returns 1 if arrayName is an array variable, 0 if there is no variable by that name or if it is a scalar variable.",
        synopsis: "array exists arrayName",
        return_type: Some(TclType::Boolean),
        arg_roles: &[(0, ArgRole::VarRead)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "for",
        arity: Arity::exact(3),
        detail: "Iterates over array entries. The first argument is a two-element list of variable names for the key and value of each entry.",
        synopsis: "array for {keyVariable valueVariable} arrayName body",
        return_type: Some(TclType::String),
        // Index 0 is the `{keyVariable valueVariable}` binding list itself —
        // `LoopVarList`, mirroring `dict for`'s identically-shaped first
        // argument (`dict.rs`'s `for` subcommand) — not the array; the array
        // is index 1.
        arg_roles: &[
            (0, ArgRole::LoopVarList),
            (1, ArgRole::VarRead),
            (2, ArgRole::Body),
        ],
        lowering_hook: Some(crate::hooks::LoweringHookId::ArrayFor),
        loop_list_header: true,
        dialects: Some(DialectSet::TCL90_PLUS),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::new(1, 2),
        detail: "Returns a list containing pairs of elements.",
        synopsis: "array get arrayName ?pattern?",
        return_type: Some(TclType::List),
        arg_roles: &[(0, ArgRole::VarRead)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::ArrayNames)),
        arity: Arity::new(1, 3),
        detail: "Returns a list containing the names of all of the elements in the array that match pattern.",
        synopsis: "array names arrayName ?mode? ?pattern?",
        return_type: Some(TclType::List),
        arg_roles: &[(0, ArgRole::VarRead)],
        // Completion-only: position 1 is `mode` only when a `pattern` word
        // also follows; with a single trailing word it is `pattern` instead
        // (see the doc comment on `NAMES_MODE_VALUES`), so this is not
        // `closed_value_args`.
        arg_values: &[(1, NAMES_MODE_VALUES)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nextelement",
        arity: Arity::exact(2),
        detail: "Returns the name of the next element in arrayName, or an empty string if all elements have already been returned in this search.",
        synopsis: "array nextelement arrayName searchId",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::exact(2),
        detail: "Sets the values of one or more elements in arrayName.",
        synopsis: "array set arrayName list",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarWrite)],
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::ArraySize)),
        arity: Arity::exact(1),
        detail: "Returns a decimal string giving the number of elements in the array.",
        synopsis: "array size arrayName",
        return_type: Some(TclType::Int),
        arg_roles: &[(0, ArgRole::VarRead)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "startsearch",
        arity: Arity::exact(1),
        detail: "Initializes an element-by-element search through the array given by arrayName.",
        synopsis: "array startsearch arrayName",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "statistics",
        arity: Arity::exact(1),
        detail: "Returns statistics about the distribution of data within the hashtable that represents the array.",
        synopsis: "array statistics arrayName",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unset",
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::new(1, 2),
        detail: "Unsets all of the elements in the array that match pattern.",
        synopsis: "array unset arrayName ?pattern?",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarWrite)],
        mutator: true,
        // `Tcl_ArrayObjCmd` (tclVar.c, `ArrayUnsetCmd`) destroys matching
        // elements — or the whole array in the pattern-less form — via the
        // same `TclObjUnsetVar2` machinery as `unset`; a bare
        // `catch {array unset a …}` is the documented fire-and-forget
        // idiom the W302 suppression keys off. Never errors, even when
        // arrayName doesn't exist, isn't an array, or pattern matches
        // nothing (confirmed against tclsh 8.6.14).
        destructive: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `array`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "array",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        // The `unset` subform destroys elements or the whole array
        // (`ArrayUnsetCmd`, tclVar.c) — `FIRE_AND_FORGET_TEARDOWN` and the
        // `destructive` flag live on that subcommand.
        traits: Traits::NOT_PROC_FACTORY | Traits::BYTE_COMPILED | Traits::WHOLE_ARRAY_ARG,
        arity: Arity::at_least(1),
        // Deliberately *not* `assigns_variable_at: Some(1)`: that field's
        // consumer (`classify_variable_assignment`, tcl-compiler's
        // side_effects.rs) assumes a single fixed index is always the
        // written variable across every call shape, which does not hold
        // for this ensemble. Index 1 is the array name for most
        // subcommands, but not for `default` (index 1 is the `get`/`set`/
        // `exists`/`unset` verb word there; the array name is index 2 —
        // `array default get arrName`) or for `for` (index 1 is the
        // `{keyVar valVar}` binding list; the array name is index 2 —
        // `array for {k v} arrName body`). The same consumer also infers a
        // *write* from `args.len() > idx + 1` alone, which misclassifies a
        // plain read like `array get arr somePattern` or `array names arr
        // -glob pattern` as a write. Each subcommand instead declares its
        // own precise `side_effects` below, read by the same consumer's
        // subcommand-first hints path — the same fix already applied to
        // `unset` for an identical reason (see /).
        inferred_storage_type: Some(StorageType::Array),
        subcommands: SUBCOMMANDS,
        // Coarse fallback for any call the per-subcommand hints above don't
        // cover (e.g. an unresolvable subcommand word).
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Manipulate array variables.",
            synopsis: &["array option arrayName ?arg arg ...?"],
            snippet: "Dispatches on option to one of anymore, default (Tcl 9.0+), donesearch, exists, for (Tcl 9.0+), get, names, nextelement, set, size, startsearch, statistics, or unset. Unless a subcommand says otherwise, arrayName must already be an array variable; exists, get, names, size, and unset instead tolerate a missing or non-array arrayName, returning 0, an empty list/string, or false rather than raising an error. array set merges an even-sized key/value list into arrayName, creating it if it does not already exist. array unset removes the elements matching pattern, or the entire array when pattern is omitted; it never raises an error, even when arrayName does not exist. startsearch/anymore/nextelement/donesearch step through elements one at a time via an opaque search identifier; modifying the array while a search is active invalidates that search. array default (9.0+) configures a value returned in place of an error when reading a missing element, and array for (9.0+) iterates {keyVariable valueVariable} pairs directly, without the intermediate list array names or array get would build.",
            source: "Tcl array(n)",
            examples: "array set fruit {apple 3 banana 5 cherry 12}\nforeach name [lsort [array names fruit]] {\n    puts \"$name: $fruit($name)\"\n}\nif {[array exists fruit]} {\n    puts \"count: [array size fruit]\"\n}\narray unset fruit banana",
            return_value: "Depends on the subcommand: a boolean (exists, anymore, default exists), a decimal count (size), a list (get, names), an element name or empty string (nextelement), a search identifier (startsearch), free-form statistics text (statistics), the current default value (default get), or an empty string (set, unset, donesearch, default set/unset, for).",
        }),
        codegen_hook: Some(CodegenHookId::Array),
        inline_codegen_hook: Some(InlineCodegenHookId::Array),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
