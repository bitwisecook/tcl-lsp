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

//! `dict` — dictionary operations.

use crate::hooks::{CodegenHookId, InlineCodegenHookId};
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dict option arg ?arg ...?",
    dialects: None,
}];

/// Dynamic resolver: last arg is body for `dict update`/`dict with`.
///
/// Arg 0 (the dict variable) plays both `VarRead` and `VarWrite` roles —
/// the body sees the current keys mapped into local vars (read), and the
/// body's writes are reflected back into the dict on completion (write).
/// This is the multi-role resolver shape: arg 0 carries both `VAR_READ`
/// and `VAR_WRITE`. It is emitted via duplicate `(idx, role)` entries —
/// `arg_indices_for_role` collects all matches, so two rows with the same
/// index produce the same observable behaviour as a single combined role.
/// The full `ArgRoleSet` type widening is deferred; the multi-role
/// acceptance test passes with the duplicate-entries form.
fn dict_last_arg_body(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let mut roles = Vec::new();
    if args.len() >= 2 {
        roles.push((0, ArgRole::VarWrite));
        roles.push((0, ArgRole::VarRead));
        if let Ok(last) = u8::try_from(args.len() - 1) {
            roles.push((last, ArgRole::Body));
        }
    }
    roles
}

/// Dynamic resolver: body only when filter type is "script".
fn dict_filter_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 3 && args[1] == "script" {
        u8::try_from(args.len() - 1)
            .map(|last| vec![(last, ArgRole::Body)])
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// `dict filter dictionaryValue filterType ...`'s `filterType` word (index 1
/// after the `filter` subcommand): a closed 3-member set (`Tcl_GetIndexFromObj`
/// over `{"key", "script", "value"}` in `DictFilterCmd`, tclDictObj.c, flags
/// `0` — no `TCL_EXACT`, so any unique prefix is also accepted, e.g. `dict
/// filter $d k foo*`), unchanged across 8.5–9.1.
///
/// The `key`/`value` rules' *trailing* glob-pattern arity is not, though: the
/// 8.5 manpage synopsis is `dict filter dictionaryValue key globPattern`
/// (exactly one pattern, no `?...?`), while 8.6 through 9.1 document `dict
/// filter dictionaryValue key ?globPattern ...?` (zero or more, matching any
/// of them) — confirmed by diffing the fetched 8.5 and 8.6 manpages. That
/// nuance can't be expressed as an `ArgValue.min_tcl` (the *value* `key` is
/// legal in every dict-supporting release; only what may follow it changed),
/// so it lives in prose in each entry's `detail` instead.
const FILTER_TYPE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "key",
        detail: "Keeps key/value pairs whose key matches any of the given glob patterns (string match rules). Tcl 8.5 accepts exactly one globPattern; Tcl 8.6 and later accept zero or more, matching any of them.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "script",
        detail: "Keeps key/value pairs for which {keyVariable valueVariable} script evaluates to a true value. A TCL_BREAK result stops filtering immediately; TCL_CONTINUE excludes just that pair.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "value",
        detail: "Keeps key/value pairs whose value matches any of the given glob patterns (string match rules). Tcl 8.5 accepts exactly one globPattern; Tcl 8.6 and later accept zero or more, matching any of them.",
        ..ArgValue::DEFAULT
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "append",
        arity: Arity::at_least(2),
        detail: "Append to a value in a dictionary.",
        synopsis: "dict append dictionaryVariable key ?string ...?",
        // "The updated dictionary value is returned" — present in the 8.6
        // through 9.1 manpages; the 8.5 manpage omits the closing sentence,
        // but `DictAppendCmd` (tclDictObj.c) returns the same dict in both
        // 8.5 and 8.6, so this is a documentation gap in 8.5, not a real
        // behavioural difference.
        return_type: Some(TclType::Dict),
        var_elements_effect: Some(VarElementsEffect::ExtendsDictValuesByName { values_from: 2 }),
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        mutator: true,
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        const_fold: Some(crate::const_fold::fold_dict_create),
        // `?key value ...?` — an even count, 0 or more (confirmed against
        // tclsh 8.6.14: `dict create a` fails "wrong # args").
        arity: Arity::stepped(0, Arity::UNLIMITED, 2),
        detail: "Create a new dictionary from key/value pairs.",
        synopsis: "dict create ?key value ...?",
        pure: true,
        return_type: Some(TclType::Dict),
        return_elements: Some(ReturnElements::DictOfPairs { from: 0 }),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        const_fold: Some(crate::const_fold::fold_dict_exists),
        arity: Arity::at_least(2),
        detail: "Test whether a key exists in a dictionary.",
        synopsis: "dict exists dictionaryValue key ?key ...?",
        pure: true,
        return_type: Some(TclType::Boolean),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "filter",
        arity: Arity::at_least(2),
        detail: "Filter a dictionary.",
        synopsis: "dict filter dictionaryValue filterType ...",
        arg_role_resolver: Some(dict_filter_arg_roles),
        return_type: Some(TclType::Dict),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        arg_values: &[(1, FILTER_TYPE_VALUES)],
        // A single keyword word, not a list — exact whole-word matching
        // (W127) is safe here, unlike `open`'s POSIX access-flag list.
        closed_value_args: &[1],
        // `Tcl_GetIndexFromObj` accepts any unique prefix (see
        // `FILTER_TYPE_VALUES`'s doc comment).
        arg_values_accept_prefix: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "for",
        arity: Arity::exact(3),
        detail: "Iterate over dictionary key/value pairs.",
        synopsis: "dict for {keyVar valueVar} dictionaryValue body",
        // "The result of the command is an empty string" — true in every
        // version dict exists in (8.5–9.1); unlike `update`/`with`, the
        // result is not body's own result.
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::LoopVarList), (2, ArgRole::Body)],
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        loop_list_header: true,
        cfg_rewrite_name: Some("::tcl::dict::for"),
        analyser_hook: Some(crate::hooks::AnalyserHookId::DictFor),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        const_fold: Some(crate::const_fold::fold_dict_get),
        inline_codegen_hook: Some(InlineCodegenHookId::DictGet),
        arity: Arity::at_least(1),
        detail: "Get a value from a dictionary.",
        synopsis: "dict get dictionaryValue ?key ...?",
        pure: true,
        return_type: Some(TclType::String),
        // Single-key form only — the compiler applies the element fact to
        // the two-arg call shape (`dict get $d $k`).
        return_elements: Some(ReturnElements::ElementOf { container_arg: 0 }),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "incr",
        arity: Arity::new(2, 3),
        detail: "Increment a value in a dictionary.",
        synopsis: "dict incr dictionaryVariable key ?increment?",
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        mutator: true,
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        return_type: Some(TclType::Dict),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "keys",
        const_fold: Some(crate::const_fold::fold_dict_keys),
        arity: Arity::new(1, 2),
        detail: "Return the keys of a dictionary.",
        synopsis: "dict keys dictionaryValue ?globPattern?",
        pure: true,
        return_type: Some(TclType::List),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lappend",
        arity: Arity::at_least(2),
        detail: "Append list elements to a dictionary value.",
        synopsis: "dict lappend dictionaryVariable key ?value ...?",
        // "The updated dictionary value is returned" (DictLappendCmd,
        // tclDictObj.c) — true in every version dict exists in.
        return_type: Some(TclType::Dict),
        var_elements_effect: Some(VarElementsEffect::ListifiesDictValue),
        arg_roles: &[(0, ArgRole::VarWrite)],
        mutator: true,
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "map",
        arity: Arity::exact(3),
        detail: "Apply a transformation to each dictionary entry.",
        synopsis: "dict map {keyVar valueVar} dictionaryValue body",
        arg_roles: &[(0, ArgRole::LoopVarList), (2, ArgRole::Body)],
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        return_type: Some(TclType::Dict),
        loop_list_header: true,
        dialects: Some(DialectSet::TCL86_PLUS),
        cfg_rewrite_name: Some("::tcl::dict::map"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "merge",
        const_fold: Some(crate::const_fold::fold_dict_merge),
        arity: Arity::any(),
        detail: "Merge one or more dictionaries.",
        synopsis: "dict merge ?dictionaryValue ...?",
        pure: true,
        return_type: Some(TclType::Dict),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::at_least(1),
        detail: "Remove keys from a dictionary value.",
        synopsis: "dict remove dictionaryValue ?key ...?",
        pure: true,
        return_type: Some(TclType::Dict),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        // `dictionaryValue ?key value ...?` — an odd count from 1
        // (confirmed against tclsh 8.6.14: `dict replace $d a` fails
        // "wrong # args").
        arity: Arity::stepped(1, Arity::UNLIMITED, 2),
        detail: "Replace keys in a dictionary value.",
        synopsis: "dict replace dictionaryValue ?key value ...?",
        pure: true,
        return_type: Some(TclType::Dict),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::at_least(3),
        detail: "Set a value in a dictionary.",
        synopsis: "dict set dictionaryVariable key ?key ...? value",
        var_elements_effect: Some(VarElementsEffect::SetsDictValue),
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        mutator: true,
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        return_type: Some(TclType::Dict),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        const_fold: Some(crate::const_fold::fold_dict_size),
        arity: Arity::exact(1),
        detail: "Return the number of key/value pairs.",
        synopsis: "dict size dictionaryValue",
        pure: true,
        return_type: Some(TclType::Int),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unset",
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::at_least(2),
        detail: "Remove keys from a dictionary variable.",
        synopsis: "dict unset dictionaryVariable key ?key ...?",
        // `Tcl_DictObjCmd` (tclDictObj.c, `DictUnsetCmd`) removes the key —
        // erroring on a missing *nested* path — so `catch {dict unset d …}`
        // is the documented fire-and-forget idiom the W302 suppression keys
        // off. (The variable itself is written back, created when absent —
        // the `VarWrite` role below — so this is a key removal, not a
        // variable destroy.)
        destructive: true,
        // "The updated dictionary value is returned" (DictUnsetCmd).
        return_type: Some(TclType::Dict),
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        mutator: true,
        // `DictUnsetCmd` calls `Tcl_NewDictObj()` when the variable reads as
        // unset rather than erroring (`dictPtr == NULL` branch, tclDictObj.c
        // — identical in the 8.5, 8.6, 9.0, and 9.1 sources), the same
        // auto-vivify behaviour as `append`/`incr`/`lappend`/`set` above.
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "update",
        dialects: None,
        // `dictionaryVariable key varName ?key varName ...? body` — an
        // even count from 4 (1 dict var + 2n key/varName pairs, n >= 1,
        // + 1 body — confirmed against tclsh 8.6.14: `dict update d k v
        // extra body` (5 args) fails "wrong # args").
        arity: Arity::stepped(4, Arity::UNLIMITED, 2),
        // Like `dict with`: the dictionary variable is read on entry and
        // written back after the body — a scope alias (reads-own-def).
        creates_scope_alias: true,
        detail: "Map dictionary keys to variables, execute body, write back.",
        synopsis: "dict update dictionaryVariable key varName ?...? body",
        arg_role_resolver: Some(dict_last_arg_body),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        mutator: true,
        analyser_hook: Some(crate::hooks::AnalyserHookId::DictUpdate),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "values",
        const_fold: Some(crate::const_fold::fold_dict_values),
        arity: Arity::new(1, 2),
        detail: "Return the values of a dictionary.",
        synopsis: "dict values dictionaryValue ?globPattern?",
        pure: true,
        return_type: Some(TclType::List),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "with",
        arity: Arity::at_least(2),
        detail: "Map all dictionary keys to variables, execute body, write back.",
        synopsis: "dict with dictionaryVariable ?key ...? body",
        // The dictionary variable is read on entry and written back when the
        // body ends — a scope alias whose def is its own use (the SSA walk
        // marks it reads-own-def so a `dict with $param {}` reference keeps
        // the parameter used).
        creates_scope_alias: true,
        arg_role_resolver: Some(dict_last_arg_body),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        mutator: true,
        analyser_hook: Some(crate::hooks::AnalyserHookId::DictWith),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::exact(1),
        detail: "This returns information (intended for display to people) about the given dictionary though the format of this data is dependent on the implementation of the dictionary.",
        synopsis: "dict info dictionaryValue",
        pure: true,
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "getd",
        arity: Arity::at_least(3),
        // `dict getdef`/`getwithdefault` (and their unambiguous prefix `getd`)
        // were added by TIP 342 in Tcl 9.0 — absent in the 8.6.x series.
        dialects: Some(DialectSet::TCL90_PLUS),
        detail: "Synonym for ``dict getdef`` — returns the value that the key path maps to in the dictionary value, or the default if the key is absent.",
        synopsis: "dict getd dictionaryValue ?key ...? key default",
        pure: true,
        return_type: Some(TclType::String),
        // Same dict-argument shape as plain `dict get`'s `SubCommand`: the
        // key-path lookup forces a dict intrep on `dictionaryValue` exactly
        // the same way, default value or not — `shimmers: false` here was
        // an inconsistency, not a documented difference from `dict get`.
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "getdef",
        arity: Arity::at_least(3),
        // Added by TIP 342 in Tcl 9.0.
        dialects: Some(DialectSet::TCL90_PLUS),
        detail: "Returns the value that the key path maps to in the dictionary value, or the default if the key is absent.",
        synopsis: "dict getdef dictionaryValue ?key ...? key default",
        pure: true,
        return_type: Some(TclType::String),
        // Same dict-argument shape as plain `dict get`'s `SubCommand`: the
        // key-path lookup forces a dict intrep on `dictionaryValue` exactly
        // the same way, default value or not — `shimmers: false` here was
        // an inconsistency, not a documented difference from `dict get`.
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "getwithdefault",
        arity: Arity::at_least(3),
        // Added by TIP 342 in Tcl 9.0 (the long-form spelling of `getdef`).
        dialects: Some(DialectSet::TCL90_PLUS),
        detail: "Returns the value that the key path maps to in the dictionary value, or the default if the key is absent. Alias for dict getdef.",
        synopsis: "dict getwithdefault dictionaryValue ?key ...? key default",
        pure: true,
        return_type: Some(TclType::String),
        // Same dict-argument shape as plain `dict get`'s `SubCommand`: the
        // key-path lookup forces a dict intrep on `dictionaryValue` exactly
        // the same way, default value or not — `shimmers: false` here was
        // an inconsistency, not a documented difference from `dict get`.
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `dict`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dict",
        // The `unset` subform removes keys (`DictUnsetCmd`, tclDictObj.c) —
        // `FIRE_AND_FORGET_TEARDOWN` and the `destructive` flag live on
        // that subcommand.
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CSE_CANDIDATE
            | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        inferred_storage_type: Some(StorageType::Dict),
        hover: Some(HoverSnippet {
            summary: "Query and manipulate dictionary values — ordered key/value mappings — via a suite of subcommands.",
            synopsis: &["dict option arg ?arg ...?"],
            snippet: "dict was added in Tcl 8.5 (TIP 111); it does not exist in 8.4. append, create, exists, filter, for, get, incr, info, keys, lappend, merge, remove, replace, set, size, unset, update, values, and with are available from 8.5. map was added in 8.6 (TIP 405). getdef and getwithdefault (TIP 342, aliases of each other) were added in 9.0 and return a caller-supplied default instead of raising an error when the key path is absent. From Tcl 9.0, when the dictionaryVariable argument to append, incr, lappend, set, unset, update, or with names a missing element of an array that has a default value configured (see array default), that default is used as the starting dictionary value for the operation rather than an error being raised.\n\nA dictionary's string representation is any list with an even number of elements, each consecutive pair being one key/value mapping. When such a string has a repeated key, only the last value for that key is kept. Operations that derive a new dictionary from an old one preserve the existing key order, appending newly-added keys at the end and excising removed ones.",
            source: "Tcl dict(n)",
            examples: "set d [dict create name Alice age 30]\ndict set d age 31\ndict get $d age\ndict incr d age\ndict for {key value} $d {\n    puts \"$key -> $value\"\n}\ndict with d {\n    puts \"$name is $age\"\n}",
            return_value: "Depends on option: append, create, filter, incr, lappend, map, merge, remove, replace, set, and unset return a new or updated dictionary value; get, getdef, getwithdefault, and getd return the plain value found at the given key path (or the caller-supplied default, for getdef/getwithdefault/getd, if absent); exists returns a boolean, size an integer, keys/values a list, info an implementation-defined string, and for/update/with return an empty string, the result of body, and the result of body respectively.",
        }),
        codegen_hook: Some(CodegenHookId::Dict),
        lowering_hook: Some(crate::hooks::LoweringHookId::Dict),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
