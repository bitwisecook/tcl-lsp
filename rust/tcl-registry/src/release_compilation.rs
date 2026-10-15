// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! What a release's bytecode compiler does with an invocation, where it is
//! observable: a release that does not revalidate running bytecode
//! ([`TclVersion::revalidates_running_bytecode`]) keeps the commands a unit
//! compiled until the unit is next entered, so which commands it compiles
//! inline decides what a running unit sees after a command-table change.
//!
//! Modelled for Tcl 8.4.20 from `generic/tclBasic.c`'s `builtInCmds[]` (the
//! entries with a `compileProc`) and `generic/tclCompCmds.c`. A `compileProc`
//! that declines one particular word shape (a non-literal body, a `regexp`
//! pattern it cannot compile) is not modelled; the command counts as compiled.

use tcl_dialect::TclVersion;
use tcl_runtime_api::ReleaseBinding;

/// What is known of an invocation's words after its command, as far as a
/// release's choice to compile it depends on them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct InvocationShape {
    /// Whether each word after the command is one literal word, when the
    /// words are known.
    pub literal: Option<Box<[bool]>>,
    /// The first word after the command, when it is literal.
    pub subcommand: Option<Box<str>>,
}

impl InvocationShape {
    /// The shape of an invocation whose words after the command are `args`,
    /// the word at each index literal where `literal` says so.
    #[must_use]
    pub fn of<S: AsRef<str>>(args: &[S], literal: impl Fn(usize) -> bool) -> Self {
        let literal: Box<[bool]> = (0..args.len()).map(literal).collect();
        Self {
            subcommand: args
                .first()
                .filter(|_| literal.first().copied().unwrap_or(false))
                .map(|word| word.as_ref().into()),
            literal: Some(literal),
        }
    }

    /// The number of words after the command.
    #[must_use]
    pub fn arg_count(&self) -> Option<usize> {
        self.literal.as_deref().map(<[bool]>::len)
    }

    /// Whether every word after the command at `indices` is literal; `false`
    /// when the words are not known.
    fn literal_at(&self, mut indices: impl Iterator<Item = usize>) -> bool {
        self.literal.as_deref().is_some_and(|literal| {
            indices.all(|index| literal.get(index).copied().unwrap_or(false))
        })
    }
}

/// How `version` binds an invocation of `command` of `shape`. `None` for a
/// release that revalidates running bytecode, where the answer is not
/// observable and so not modelled. An unknown word that decides the answer
/// takes the binding that holds in fewer places, except a `catch` result
/// variable, which is assumed present.
#[must_use]
pub fn release_binding(
    version: TclVersion,
    command: &str,
    shape: &InvocationShape,
) -> Option<ReleaseBinding> {
    if version.revalidates_running_bytecode() {
        return None;
    }
    let command = command.strip_prefix("::").unwrap_or(command);
    let count = shape.arg_count();
    let compiled_if = |compiles: bool| {
        if compiles {
            ReleaseBinding::Compiled
        } else {
            ReleaseBinding::Invoked
        }
    };
    Some(match command {
        "append" | "break" | "continue" | "expr" | "incr" | "llength" | "regexp" | "set" => {
            ReleaseBinding::Compiled
        }
        // `TclCompileIfCmd` compiles only words free of substitution, and
        // `TclCompileForCmd` and `TclCompileWhileCmd` only a literal test,
        // step and body.
        "if" => compiled_if(shape.literal_at(0..count.unwrap_or(0))),
        "for" => compiled_if(count == Some(4) && shape.literal_at(1..4)),
        "while" => compiled_if(count == Some(2) && shape.literal_at(0..2)),
        "lindex" => compiled_if(count.is_some_and(|count| count >= 1)),
        "lset" => compiled_if(count.is_some_and(|count| count >= 2)),
        // `TclCompileForeachCmd` declines outside a procedure body and for a
        // variable list or body that is not literal.
        "foreach" => match count {
            Some(count) if count >= 3 && count % 2 == 1 => {
                let words = (0..count - 1).step_by(2).chain(std::iter::once(count - 1));
                if shape.literal_at(words) {
                    ReleaseBinding::CompiledInProcedure
                } else {
                    ReleaseBinding::Invoked
                }
            }
            _ => ReleaseBinding::Invoked,
        },
        // `TclCompileCatchCmd` needs a local for its result variable, and
        // `TclCompileLappendCmd`, `TclCompileListCmd` and
        // `TclCompileReturnCmd` decline outside a procedure body.
        "catch" if count == Some(1) => ReleaseBinding::Compiled,
        "catch" | "lappend" | "list" | "return" => ReleaseBinding::CompiledInProcedure,
        // `TclCompileStringCmd` compiles these subcommands and invokes the rest.
        "string" => match shape.subcommand.as_deref() {
            Some("compare" | "equal" | "index" | "length" | "match") => ReleaseBinding::Compiled,
            _ => ReleaseBinding::Invoked,
        },
        _ => ReleaseBinding::Invoked,
    })
}

/// Whether `version` evaluates the script an invocation of `command` runs
/// directly, as `Tcl_EvalEx` does, rather than compiling it: 8.4's `eval` and
/// `uplevel` pass `TCL_EVAL_DIRECT` to `Tcl_EvalObjEx` (`Tcl_EvalObjCmd`,
/// `Tcl_UplevelObjCmd`), so each command of the script is looked up as it
/// runs.
#[must_use]
pub fn evaluates_script_directly(version: TclVersion, command: &str) -> bool {
    let command = command.strip_prefix("::").unwrap_or(command);
    version == TclVersion::V8_4 && matches!(command, "eval" | "uplevel")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bind(command: &str, args: &[&str]) -> Option<ReleaseBinding> {
        release_binding(
            TclVersion::V8_4,
            command,
            &InvocationShape::of(args, |_| true),
        )
    }

    #[test]
    fn only_a_release_keeping_running_bytecode_models_its_bindings() {
        let unknown = InvocationShape::default();
        assert_eq!(release_binding(TclVersion::V8_5, "set", &unknown), None);
        assert_eq!(
            release_binding(TclVersion::V8_4, "set", &unknown),
            Some(ReleaseBinding::Compiled)
        );
        assert_eq!(bind("switch", &[]), Some(ReleaseBinding::Invoked));
    }

    #[test]
    fn eight_four_compiles_some_commands_only_in_a_procedure() {
        assert_eq!(bind("catch", &["body"]), Some(ReleaseBinding::Compiled));
        assert_eq!(
            bind("catch", &["body", "m"]),
            Some(ReleaseBinding::CompiledInProcedure)
        );
        assert_eq!(
            release_binding(TclVersion::V8_4, "catch", &InvocationShape::default()),
            Some(ReleaseBinding::CompiledInProcedure)
        );
        assert_eq!(
            bind("::foreach", &["x", "l", "b"]),
            Some(ReleaseBinding::CompiledInProcedure)
        );
    }

    #[test]
    fn eight_four_compiles_some_string_subcommands() {
        assert_eq!(
            bind("string", &["length", "x"]),
            Some(ReleaseBinding::Compiled)
        );
        assert_eq!(
            bind("string", &["toupper", "x"]),
            Some(ReleaseBinding::Invoked)
        );
        let substituted = InvocationShape::of(&["length", "x"], |index| index != 0);
        assert_eq!(
            release_binding(TclVersion::V8_4, "string", &substituted),
            Some(ReleaseBinding::Invoked)
        );
    }

    #[test]
    fn eight_four_compiles_control_commands_only_with_literal_scripts() {
        let shaped = |command: &str, args: &[&str], literal: &[bool]| {
            release_binding(
                TclVersion::V8_4,
                command,
                &InvocationShape::of(args, |index| literal[index]),
            )
        };
        assert_eq!(
            shaped("if", &["1", "b"], &[true, true]),
            Some(ReleaseBinding::Compiled)
        );
        assert_eq!(
            shaped("if", &["$c", "b"], &[false, true]),
            Some(ReleaseBinding::Invoked)
        );
        assert_eq!(
            shaped("for", &["$i", "t", "n", "b"], &[false, true, true, true]),
            Some(ReleaseBinding::Compiled)
        );
        assert_eq!(
            shaped("while", &["t", "$b"], &[true, false]),
            Some(ReleaseBinding::Invoked)
        );
        assert_eq!(
            shaped("foreach", &["x", "$l", "b"], &[true, false, true]),
            Some(ReleaseBinding::CompiledInProcedure)
        );
        assert_eq!(
            shaped("foreach", &["x", "$l", "$b"], &[true, false, false]),
            Some(ReleaseBinding::Invoked)
        );
        assert_eq!(
            release_binding(TclVersion::V8_4, "if", &InvocationShape::default()),
            Some(ReleaseBinding::Invoked)
        );
    }

    #[test]
    fn eight_four_evaluates_eval_and_uplevel_scripts_directly() {
        assert!(evaluates_script_directly(TclVersion::V8_4, "eval"));
        assert!(evaluates_script_directly(TclVersion::V8_4, "::uplevel"));
        assert!(!evaluates_script_directly(TclVersion::V8_4, "catch"));
        assert!(!evaluates_script_directly(TclVersion::V8_5, "eval"));
    }
}
