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
/// with `script_words` words directly, as `Tcl_EvalEx` does, rather than
/// compiling it: so each command of the script is looked up, and each script
/// it evaluates compiled, as it runs. 8.4 and 8.5 pass `TCL_EVAL_DIRECT` to
/// `Tcl_EvalObjEx` for `eval` (`Tcl_EvalObjCmd`), 8.4 for `uplevel`
/// (`Tcl_UplevelObjCmd`), and both for the script `uplevel` and `namespace
/// eval` join from several words; their `source` hands the file to
/// `Tcl_EvalEx` (`Tcl_FSEvalFile`).
#[must_use]
pub fn evaluates_script_directly(version: TclVersion, command: &str, script_words: usize) -> bool {
    let command = command.strip_prefix("::").unwrap_or(command);
    let joined = script_words > 1;
    match version {
        TclVersion::V8_4 => {
            matches!(command, "eval" | "uplevel" | "source")
                || (joined && command == "namespace eval")
        }
        TclVersion::V8_5 => {
            matches!(command, "eval" | "source")
                || (joined && matches!(command, "uplevel" | "namespace eval"))
        }
        _ => false,
    }
}

/// A word of an invocation as a compile procedure reads it: its text, and
/// whether it is one literal word (`TCL_TOKEN_SIMPLE_WORD`), its text then
/// its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompileWord<'w> {
    /// The word's text.
    pub text: &'w str,
    /// Whether the word is one literal.
    pub literal: bool,
}

/// Whether `version`'s compile procedure for the control command `command`
/// compiles an invocation whose words after the command are `words` where it
/// stands, in a procedure body or not, rather than leaving the command to
/// run; `None` for a command not modelled here.
///
/// Modelled from each release's `TclCompileIfCmd`, `TclCompileWhileCmd`,
/// `TclCompileForCmd`, `TclCompileForeachCmd` (and 8.6's `lmap`),
/// `TclCompileDictForCmd` (and 8.6's `dict map`) and
/// `TclCompileSwitchCmd`; 8.4 has no compile procedure for `switch` or
/// `dict`.
#[must_use]
pub fn compiles_control(
    version: TclVersion,
    command: &str,
    words: &[CompileWord<'_>],
    in_procedure: bool,
) -> Option<bool> {
    let command = command.strip_prefix("::").unwrap_or(command);
    let literal = |index: usize| words.get(index).is_some_and(|word| word.literal);
    Some(match command {
        "if" => words.len() >= 2 && words.iter().all(|word| word.literal),
        "while" => words.len() == 2 && literal(0) && literal(1),
        "for" => words.len() == 4 && literal(1) && literal(2) && literal(3),
        "foreach" => in_procedure && compiles_each_loop(words),
        "lmap" => version >= TclVersion::V8_6 && in_procedure && compiles_each_loop(words),
        "dict" => {
            let sub = words.first().filter(|word| word.literal)?;
            compiles_dict_each(version, sub.text, &words[1..], in_procedure)?
        }
        // The ensemble's implementation commands share its compile procedures.
        "tcl::dict::for" => compiles_dict_each(version, "for", words, in_procedure)?,
        "tcl::dict::map" => compiles_dict_each(version, "map", words, in_procedure)?,
        "switch" => compiled_switch_arms(version, words).is_some(),
        _ => return None,
    })
}

/// `dict for` and `dict map` (`TclCompileDictForCmd`, 8.6's
/// `CompileDictEachCmd`) in a procedure body: a literal pair of local scalar
/// names and a literal body. `None` for another subcommand.
fn compiles_dict_each(
    version: TclVersion,
    sub: &str,
    words: &[CompileWord<'_>],
    in_procedure: bool,
) -> Option<bool> {
    let since = match sub {
        "for" => TclVersion::V8_5,
        "map" => TclVersion::V8_6,
        _ => return None,
    };
    Some(
        version >= since
            && in_procedure
            && words.len() == 3
            && words[0].literal
            && words[2].literal
            && tcl_syntax::list::split_list(words[0].text).is_ok_and(|names| {
                names.len() == 2 && names.iter().all(|name| is_local_scalar(name))
            }),
    )
}

/// The loop variables of an each-loop `version` compiles in a procedure body
/// (see [`compiles_control`]): each variable list's names for `foreach` and
/// `lmap`, the key and value names for `dict for` and `dict map`.
#[must_use]
pub fn compiled_loop_variables(command: &str, words: &[CompileWord<'_>]) -> Vec<String> {
    let command = command.strip_prefix("::").unwrap_or(command);
    let lists: Vec<&str> = match command {
        "foreach" | "lmap" => words
            .split_last()
            .map(|(_, pairs)| pairs.iter().step_by(2).map(|word| word.text).collect())
            .unwrap_or_default(),
        "dict" => words.get(1).map(|word| vec![word.text]).unwrap_or_default(),
        "tcl::dict::for" | "tcl::dict::map" => words
            .first()
            .map(|word| vec![word.text])
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    lists
        .into_iter()
        .filter_map(|list| tcl_syntax::list::split_list(list).ok())
        .flatten()
        .map(|name| name.to_string())
        .collect()
}

/// Whether a name is one a compile procedure makes a local scalar of
/// (`LocalScalar`): no namespace qualifier and no array element.
fn is_local_scalar(name: &str) -> bool {
    !(name.contains("::") || name.ends_with(')') && name.contains('('))
}

/// `foreach` and `lmap` (`TclCompileForeachCmd`, 8.6's `CompileEachloopCmd`):
/// variable lists and values in pairs, a literal body, and each variable list
/// a literal list of local scalars.
fn compiles_each_loop(words: &[CompileWord<'_>]) -> bool {
    let Some((body, pairs)) = words.split_last() else {
        return false;
    };
    words.len() >= 3
        && words.len() % 2 == 1
        && body.literal
        && pairs.chunks(2).all(|pair| {
            pair[0].literal
                && tcl_syntax::list::split_list(pair[0].text).is_ok_and(|names| {
                    !names.is_empty() && names.iter().all(|name| is_local_scalar(name))
                })
        })
}

/// The scripts of the arms of a `switch` of `words` that `version` compiles
/// ([`compiled_switch_arms`]), each compiled with the command: every body but
/// a `-` fall-through.
#[must_use]
pub fn compiled_switch_scripts<'w>(
    version: TclVersion,
    words: &[CompileWord<'w>],
) -> Option<Vec<&'w str>> {
    let arms = compiled_switch_arms(version, words)?;
    Some(
        arms.into_iter()
            .skip(1)
            .step_by(2)
            .filter(|body| *body != "-")
            .collect(),
    )
}

/// The patterns and bodies, in turn, of a `switch` of `words` that
/// `version` compiles (`TclCompileSwitchCmd`, 8.5 on): the value and one
/// literal list of pattern and body pairs, or after literal options closed
/// by `--` (an `-exact`, `-glob` or `-regexp` mode at most once, `-nocase`
/// with a glob or, from 9.1, any mode; 9.1 adds `-integer`) the value and
/// either one such list or literal pattern and body words; the last body is
/// not `-`. `None` where the command runs.
#[must_use]
pub fn compiled_switch_arms<'w>(
    version: TclVersion,
    words: &[CompileWord<'w>],
) -> Option<Vec<&'w str>> {
    if version == TclVersion::V8_4 {
        return None;
    }
    let mut rest = words;
    if words.len() != 2 {
        let mut mode = None;
        let mut nocase = false;
        loop {
            if rest.len() < 3 {
                return None;
            }
            let word = rest[0];
            let option = |name: &str, shortest: usize| {
                word.literal && word.text.len() >= shortest && name.starts_with(word.text)
            };
            rest = &rest[1..];
            if word.literal && word.text == "--" {
                break;
            }
            let selected = if version >= TclVersion::V9_1 {
                [("-exact", 2), ("-glob", 2), ("-integer", 4), ("-regexp", 2)]
                    .iter()
                    .find(|&&(name, shortest)| option(name, shortest))
                    .map(|&(name, _)| name)
            } else {
                [("-exact", 2), ("-glob", 2), ("-regexp", 2)]
                    .iter()
                    .find(|&&(name, _)| word.text.len() <= name.len() && option(name, 2))
                    .map(|&(name, _)| name)
            };
            if let Some(name) = selected {
                if mode.replace(name).is_some() {
                    return None;
                }
            } else if option("-nocase", 2) {
                nocase = true;
            } else {
                return None;
            }
        }
        if nocase && mode.is_none_or(|mode| mode == "-exact") && version < TclVersion::V9_1 {
            return None;
        }
    }
    let arms = &rest[1..];
    let arm_words: Vec<&'w str> = if let [list] = arms {
        if !list.literal {
            return None;
        }
        let mut at = 0;
        let mut elements = Vec::new();
        loop {
            match tcl_syntax::list::find_element(list.text, at) {
                Ok(Some(element)) if element.literal => {
                    elements.push(&list.text[element.value.clone()]);
                    at = element.next;
                }
                Ok(None) => break,
                _ => return None,
            }
        }
        if elements.len() < 2 || elements.len() % 2 == 1 {
            return None;
        }
        elements
    } else {
        if arms.is_empty() || arms.len() % 2 == 1 || !arms.iter().all(|arm| arm.literal) {
            return None;
        }
        arms.iter().map(|arm| arm.text).collect()
    };
    arm_words
        .last()
        .is_some_and(|body| *body != "-")
        .then_some(arm_words)
}

/// The commands 8.6's compiler has a compile procedure for, and for an
/// ensemble the subcommands that have one: `builtInCmds[]`'s entries with a
/// `compileProc` (`generic/tclBasic.c`), the ensemble implementation maps of
/// `array` (`tclVar.c`), `dict` (`tclDictObj.c`), `info` (`tclCmdIL.c`),
/// `namespace` (`tclNamesp.c`), `string` (`tclCmdMZ.c`) and `clock`
/// (`tclClock.c`), and `TclOO`'s `next`, `nextto` and `self` (`tclOO.c`).
const COMPILED_8_6: &[(&str, Option<&[&str]>)] = &[
    ("append", None),
    ("array", Some(&["exists", "set", "unset"])),
    ("break", None),
    ("catch", None),
    (
        "clock",
        Some(&["clicks", "microseconds", "milliseconds", "seconds"]),
    ),
    ("concat", None),
    ("continue", None),
    (
        "dict",
        Some(&[
            "append", "create", "exists", "for", "get", "incr", "lappend", "map", "merge", "set",
            "unset", "update", "with",
        ]),
    ),
    ("error", None),
    ("expr", None),
    ("for", None),
    ("foreach", None),
    ("format", None),
    ("global", None),
    ("if", None),
    ("incr", None),
    ("info", Some(&["commands", "coroutine", "exists", "level"])),
    ("lappend", None),
    ("lassign", None),
    ("lindex", None),
    ("linsert", None),
    ("list", None),
    ("llength", None),
    ("lmap", None),
    ("lrange", None),
    ("lreplace", None),
    ("lset", None),
    (
        "namespace",
        Some(&[
            "code",
            "current",
            "origin",
            "qualifiers",
            "tail",
            "upvar",
            "which",
        ]),
    ),
    ("next", None),
    ("nextto", None),
    ("regexp", None),
    ("regsub", None),
    ("return", None),
    ("self", None),
    ("set", None),
    (
        "string",
        Some(&[
            "cat",
            "compare",
            "equal",
            "first",
            "index",
            "is",
            "last",
            "length",
            "map",
            "match",
            "range",
            "replace",
            "tolower",
            "toupper",
            "totitle",
            "trim",
            "trimleft",
            "trimright",
        ]),
    ),
    ("subst", None),
    ("switch", None),
    ("tailcall", None),
    ("throw", None),
    ("try", None),
    ("unset", None),
    ("upvar", None),
    ("variable", None),
    ("while", None),
    ("yield", None),
    ("yieldto", None),
];

/// The 8.6 compile procedures that compile no variable word: those of
/// `regexp` and `regsub`, which leave a command with a match or result
/// variable to run (`TclCompileRegexpCmd`, `TclCompileRegsubCmd`).
const BINDS_NO_VARIABLES_8_6: &[&str] = &["regexp", "regsub"];

/// What a release's compile procedure for a command does with its words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompileProcedure {
    /// Whether it compiles the command's variable words, making a procedure
    /// body's locals of them, rather than leaving a command with one to run.
    pub binds_variables: bool,
}

/// Whether [`compile_procedure`] models `version`'s compile procedures: 8.6
/// alone, the release whose compiled and directly evaluated commands read a
/// literal index apart, so the one where which commands compile is
/// observable.
#[must_use]
pub fn compile_procedures_modelled(version: TclVersion) -> bool {
    version == TclVersion::V8_6
}

/// `version`'s compile procedure for `command`, and for an ensemble for its
/// canonical `subcommand` (an ensemble whose subcommand is not known at
/// compile time has none); `None` where it has none, or its procedures are
/// not modelled ([`compile_procedures_modelled`]).
#[must_use]
pub fn compile_procedure(
    version: TclVersion,
    command: &str,
    subcommand: Option<&str>,
) -> Option<CompileProcedure> {
    if !compile_procedures_modelled(version) {
        return None;
    }
    let command = command.strip_prefix("::").unwrap_or(command);
    let (_, subcommands) = COMPILED_8_6.iter().find(|(name, _)| *name == command)?;
    if let Some(subcommands) = subcommands
        && !subcommand.is_some_and(|sub| subcommands.contains(&sub))
    {
        return None;
    }
    Some(CompileProcedure {
        binds_variables: !BINDS_NO_VARIABLES_8_6.contains(&command),
    })
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
    fn eight_six_compiles_its_builtins_and_ensemble_subcommands() {
        let compiles = |command, sub| compile_procedure(TclVersion::V8_6, command, sub).is_some();
        assert!(compiles("set", None));
        assert!(compiles("::catch", None));
        assert!(!compiles("gets", None));
        assert!(!compiles("eval", None));
        assert!(compiles("array", Some("set")));
        assert!(!compiles("array", Some("names")));
        assert!(!compiles("dict", None));
        assert!(compiles("dict", Some("update")));
        assert_eq!(
            compile_procedure(TclVersion::V8_6, "regsub", None),
            Some(CompileProcedure {
                binds_variables: false
            })
        );
        assert_eq!(compile_procedure(TclVersion::V8_5, "set", None), None);
    }

    #[test]
    fn eight_four_and_eight_five_evaluate_some_scripts_directly() {
        assert!(evaluates_script_directly(TclVersion::V8_4, "eval", 1));
        assert!(evaluates_script_directly(TclVersion::V8_4, "::uplevel", 1));
        assert!(evaluates_script_directly(TclVersion::V8_4, "source", 1));
        assert!(!evaluates_script_directly(
            TclVersion::V8_4,
            "namespace eval",
            1
        ));
        assert!(evaluates_script_directly(
            TclVersion::V8_4,
            "namespace eval",
            2
        ));
        assert!(!evaluates_script_directly(TclVersion::V8_4, "catch", 1));
        assert!(evaluates_script_directly(TclVersion::V8_5, "eval", 1));
        assert!(evaluates_script_directly(TclVersion::V8_5, "source", 1));
        assert!(!evaluates_script_directly(TclVersion::V8_5, "uplevel", 1));
        assert!(evaluates_script_directly(TclVersion::V8_5, "uplevel", 2));
        assert!(evaluates_script_directly(
            TclVersion::V8_5,
            "namespace eval",
            3
        ));
        assert!(!evaluates_script_directly(TclVersion::V8_6, "eval", 2));
        assert!(!evaluates_script_directly(TclVersion::V8_6, "source", 1));
    }
}
