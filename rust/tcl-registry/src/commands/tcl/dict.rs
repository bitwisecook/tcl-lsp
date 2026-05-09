//! `dict` — dictionary operations.

use crate::hooks::CodegenHookId;
use crate::prelude::*;

/// Dynamic resolver: last arg is body for `dict update`/`dict with`.
///
/// Arg 0 (the dict variable) plays both `VarRead` and `VarWrite` roles —
/// the body sees the current keys mapped into local vars (read), and the
/// body's writes are reflected back into the dict on completion (write).
/// Mirrors Python's `frozenset({VAR_READ, VAR_WRITE})` after `8c95c2ee` /
/// `38d90003` (multi-role resolver shape). The Rust port emits this via
/// duplicate `(idx, role)` entries — `arg_indices_for_role` collects all
/// matches, so two rows with the same index produce the same observable
/// behaviour as Python's frozenset. The full `ArgRoleSet` type widening
/// (per SYNC1's spec) is deferred; the multi-role acceptance test passes
/// with the duplicate-entries form.
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

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "append",
        arity: Arity::at_least(2),
        detail: "Append to a value in a dictionary.",
        synopsis: "dict append dictionaryVariable key ?string ...?",
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
            },
        )],
        mutator: true,
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::any(),
        detail: "Create a new dictionary from key/value pairs.",
        synopsis: "dict create ?key value ...?",
        pure: true,
        return_type: Some(TclType::Dict),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::at_least(2),
        detail: "Test whether a key exists in a dictionary.",
        synopsis: "dict exists dictionaryValue key ?key ...?",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "filter",
        arity: Arity::at_least(2),
        detail: "Filter a dictionary.",
        synopsis: "dict filter dictionaryValue filterType ...",
        arg_role_resolver: Some(dict_filter_arg_roles),
        return_type: Some(TclType::Dict),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "for",
        arity: Arity::exact(3),
        detail: "Iterate over dictionary key/value pairs.",
        synopsis: "dict for {keyVar valueVar} dictionaryValue body",
        arg_roles: &[(2, ArgRole::Body)],
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
            },
        )],
        loop_list_header: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::at_least(1),
        detail: "Get a value from a dictionary.",
        synopsis: "dict get dictionaryValue ?key ...?",
        pure: true,
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
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
            },
        )],
        mutator: true,
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "keys",
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
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lappend",
        arity: Arity::at_least(2),
        detail: "Append list elements to a dictionary value.",
        synopsis: "dict lappend dictionaryVariable key ?value ...?",
        arg_roles: &[(0, ArgRole::VarWrite)],
        mutator: true,
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "map",
        arity: Arity::exact(3),
        detail: "Apply a transformation to each dictionary entry.",
        synopsis: "dict map {keyVar valueVar} dictionaryValue body",
        arg_roles: &[(2, ArgRole::Body)],
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
            },
        )],
        return_type: Some(TclType::Dict),
        loop_list_header: true,
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "merge",
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
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::at_least(1),
        detail: "Replace keys in a dictionary value.",
        synopsis: "dict replace dictionaryValue ?key value ...?",
        pure: true,
        return_type: Some(TclType::Dict),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::at_least(3),
        detail: "Set a value in a dictionary.",
        synopsis: "dict set dictionaryVariable key ?key ...? value",
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
            },
        )],
        mutator: true,
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
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
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unset",
        arity: Arity::at_least(2),
        detail: "Remove keys from a dictionary variable.",
        synopsis: "dict unset dictionaryVariable key ?key ...?",
        arg_roles: &[(0, ArgRole::VarWrite)],
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "update",
        arity: Arity::at_least(4),
        detail: "Map dictionary keys to variables, execute body, write back.",
        synopsis: "dict update dictionaryVariable key varName ?...? body",
        arg_role_resolver: Some(dict_last_arg_body),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
            },
        )],
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "values",
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
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "with",
        arity: Arity::at_least(2),
        detail: "Map all dictionary keys to variables, execute body, write back.",
        synopsis: "dict with dictionaryVariable ?key ...? body",
        arg_role_resolver: Some(dict_last_arg_body),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
            },
        )],
        mutator: true,
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `dict`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dict",
        traits: Traits::CSE_CANDIDATE | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        inferred_storage_type: Some(StorageType::Dict),
        hover: Some(HoverSnippet::brief(
            "Manipulate Tcl dictionaries.",
            &["dict subcommand ?arg ...?"],
            "Tcl dict(1)",
        )),
        codegen_hook: Some(CodegenHookId::Dict),
        lowering_hook: Some(crate::hooks::LoweringHookId::Dict),
        ..CommandSpec::DEFAULT
    }
}
