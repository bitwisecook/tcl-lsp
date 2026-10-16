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
//! entries with a `compileProc`) and `generic/tclCompCmds.c`, including the
//! word shapes each `compileProc` declines. A `compileProc` that fails with a
//! compile error (a wrong word count it reports itself) fails the whole unit,
//! so its command counts as compiled.

use tcl_dialect::TclVersion;
use tcl_runtime_api::ReleaseBinding;

/// What is known of an invocation's words after its command, as far as a
/// release's choice to compile it depends on them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct InvocationShape {
    /// Each word after the command, with its text where it is one literal
    /// word, when the words are known.
    pub words: Option<Box<[Option<Box<str>>]>>,
}

impl InvocationShape {
    /// The shape `release` reads of an invocation whose words after the
    /// command are `args`, the word at each index literal where `literal`
    /// says so. A release that revalidates running bytecode reads none.
    #[must_use]
    pub fn of<S: AsRef<str>>(
        release: TclVersion,
        args: &[S],
        literal: impl Fn(usize) -> bool,
    ) -> Self {
        if release.revalidates_running_bytecode() {
            return Self::default();
        }
        Self {
            words: Some(
                args.iter()
                    .enumerate()
                    .map(|(index, word)| literal(index).then(|| word.as_ref().into()))
                    .collect(),
            ),
        }
    }

    /// The shape of an invocation of `count` words after the command, none
    /// of them known to be literal.
    #[must_use]
    pub fn counted(count: usize) -> Self {
        Self {
            words: Some(vec![None; count].into()),
        }
    }

    /// The number of words after the command.
    #[must_use]
    pub fn arg_count(&self) -> Option<usize> {
        self.words.as_deref().map(<[_]>::len)
    }

    /// The text of the word after the command at `index`, when it is literal.
    #[must_use]
    pub fn word(&self, index: usize) -> Option<&str> {
        self.words.as_deref()?.get(index)?.as_deref()
    }
}

/// How `version` binds an invocation of `command` of `shape`. `None` for a
/// release that revalidates running bytecode, where the answer is not
/// observable and so not modelled. An unknown word that decides the answer
/// takes the binding that holds in fewer places.
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
    let in_procedure_if = |compiles: bool| {
        if compiles {
            ReleaseBinding::CompiledInProcedure
        } else {
            ReleaseBinding::Invoked
        }
    };
    Some(match command {
        "break" | "continue" | "expr" | "incr" | "llength" | "set" => ReleaseBinding::Compiled,
        // `TclCompileAppendCmd` compiles one value at most.
        "append" => compiled_if(count.is_some_and(|count| count <= 2)),
        // A wrong word count is a compile error that fails the whole unit,
        // so only a well-formed invocation's binding is observable.
        "if" | "for" | "while" => compiled_if(compiles_shape(version, command, shape, false)),
        "lindex" => compiled_if(count.is_some_and(|count| count >= 1)),
        "lset" => compiled_if(count.is_some_and(|count| count >= 2)),
        "regexp" => compiled_if(regexp_compiles(shape)),
        "string" => compiled_if(string_compiles(shape)),
        // The rest decline outside a procedure body, `TclCompileLappendCmd`
        // for more than one value and `TclCompileReturnCmd` for options or
        // inside a `catch`.
        "foreach" => in_procedure_if(compiles_shape(version, command, shape, true)),
        "lappend" => in_procedure_if(matches!(count, Some(0 | 2))),
        "list" => ReleaseBinding::CompiledInProcedure,
        "return" => {
            if count.is_some_and(|count| count <= 1) {
                ReleaseBinding::CompiledInProcedureOutsideCatch
            } else {
                ReleaseBinding::Invoked
            }
        }
        // `TclCompileCatchCmd` needs a local scalar for its result variable;
        // any other word count is its wrong-arity error, which counts as
        // compiled.
        "catch" => match count {
            Some(2) => in_procedure_if(shape.word(1).is_some_and(is_local_scalar)),
            Some(_) => ReleaseBinding::Compiled,
            None => ReleaseBinding::Invoked,
        },
        _ => ReleaseBinding::Invoked,
    })
}

/// Whether `version` compiles some invocation of `command`, so that what the
/// command table holds for it at a unit's entry can decide what the unit
/// runs.
#[must_use]
pub fn may_compile(version: TclVersion, command: &str) -> bool {
    !version.revalidates_running_bytecode()
        && matches!(
            command.strip_prefix("::").unwrap_or(command),
            "append"
                | "break"
                | "catch"
                | "continue"
                | "expr"
                | "for"
                | "foreach"
                | "if"
                | "incr"
                | "lappend"
                | "lindex"
                | "list"
                | "llength"
                | "lset"
                | "regexp"
                | "return"
                | "set"
                | "string"
                | "while"
        )
}

/// C's `TclIsLocalScalar`: a name with no namespace qualifier and no array
/// element.
fn is_local_scalar(name: &str) -> bool {
    let element = name.contains('(') && name.ends_with(')');
    !name.contains("::") && !element
}

/// [`compiles_control`] for an invocation of `shape`, a word not known to
/// be literal read as one that is not.
fn compiles_shape(
    version: TclVersion,
    command: &str,
    shape: &InvocationShape,
    in_procedure: bool,
) -> bool {
    let words: Vec<CompileWord<'_>> = (0..shape.arg_count().unwrap_or(0))
        .map(|index| {
            let word = shape.word(index);
            CompileWord {
                text: word.unwrap_or_default(),
                literal: word.is_some(),
            }
        })
        .collect();
    compiles_control(version, command, &words, in_procedure) == Some(true)
}

/// `TclCompileRegexpCmd`'s shape: literal `-nocase` or `--` options, then a
/// literal pattern and the string, the pattern a plain string once a leading
/// `^`, a trailing `$`, and a leading or trailing `.*` are set aside.
fn regexp_compiles(shape: &InvocationShape) -> bool {
    let Some(count) = shape.arg_count() else {
        return false;
    };
    if count < 2 {
        return false;
    }
    // Indexed as C indexes the command's words, the command at 0.
    let words = count + 1;
    let mut i = 1;
    while i < words - 2 {
        match shape.word(i - 1) {
            Some("--") => {
                i += 1;
                break;
            }
            Some(option) if option.len() > 1 && "-nocase".starts_with(option) => {}
            _ => return false,
        }
        i += 1;
    }
    if words - i != 2 {
        return false;
    }
    let Some(pattern) = shape.word(i - 1) else {
        return false;
    };
    if pattern.starts_with('-') {
        return false;
    }
    let bytes = pattern.as_bytes();
    let mut start = 0;
    let mut end = bytes.len();
    if end == 0 {
        return true;
    }
    if bytes[0] == b'^' {
        start = 1;
    }
    if bytes[end - 1] == b'$' && (end == 1 || bytes[end - 2] != b'\\') {
        end -= 1;
    }
    if end > start + 1 && bytes[start] == b'.' && bytes[start + 1] == b'*' {
        start += 2;
    }
    if end > start + 2
        && bytes[end - 3] != b'\\'
        && bytes[end - 2] == b'.'
        && bytes[end - 1] == b'*'
    {
        end -= 2;
    }
    start >= end
        || !bytes[start..end]
            .iter()
            .any(|byte| b"*+?{}()[].\\|^$".contains(byte))
}

/// `TclCompileStringCmd`'s shape: a literal subcommand, read as
/// `Tcl_GetIndexFromObj` reads it, of the ones it compiles with their word
/// counts.
fn string_compiles(shape: &InvocationShape) -> bool {
    const SUBCOMMANDS: [&str; 21] = [
        "bytelength",
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
        "repeat",
        "replace",
        "tolower",
        "toupper",
        "totitle",
        "trim",
        "trimleft",
        "trimright",
        "wordend",
        "wordstart",
    ];
    let (Some(count), Some(written)) = (shape.arg_count(), shape.word(0)) else {
        return false;
    };
    let subcommand = SUBCOMMANDS
        .iter()
        .find(|name| **name == written)
        .or_else(|| {
            let mut prefixed = SUBCOMMANDS
                .iter()
                .filter(|name| !written.is_empty() && name.starts_with(written));
            prefixed.next().filter(|_| prefixed.next().is_none())
        });
    match subcommand.copied() {
        Some("compare" | "equal" | "index") => count == 3,
        Some("length") => count == 2,
        Some("match") => {
            count == 3
                || (count == 4
                    && shape
                        .word(1)
                        .is_some_and(|option| option.len() > 1 && "-nocase".starts_with(option)))
        }
        _ => false,
    }
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

    fn shaped(command: &str, args: &[&str], literal: &[bool]) -> Option<ReleaseBinding> {
        let shape = InvocationShape::of(TclVersion::V8_4, args, |index| literal[index]);
        release_binding(TclVersion::V8_4, command, &shape)
    }

    fn bind(command: &str, args: &[&str]) -> Option<ReleaseBinding> {
        shaped(command, args, &vec![true; args.len()])
    }

    const COMPILED: Option<ReleaseBinding> = Some(ReleaseBinding::Compiled);
    const IN_PROCEDURE: Option<ReleaseBinding> = Some(ReleaseBinding::CompiledInProcedure);
    const INVOKED: Option<ReleaseBinding> = Some(ReleaseBinding::Invoked);

    #[test]
    fn only_a_release_keeping_running_bytecode_models_its_bindings() {
        let unknown = InvocationShape::default();
        assert_eq!(release_binding(TclVersion::V8_5, "set", &unknown), None);
        assert_eq!(release_binding(TclVersion::V8_4, "set", &unknown), COMPILED);
        assert_eq!(bind("switch", &[]), INVOKED);
        assert_eq!(
            InvocationShape::of(TclVersion::V9_0, &["x"], |_| true),
            InvocationShape::default(),
            "a release that revalidates reads no words"
        );
    }

    #[test]
    fn eight_four_compiles_some_commands_only_in_a_procedure() {
        assert_eq!(bind("catch", &["body"]), COMPILED);
        assert_eq!(bind("catch", &["body", "m"]), IN_PROCEDURE);
        assert_eq!(bind("catch", &["body", "::m"]), INVOKED);
        assert_eq!(bind("catch", &["body", "a(b)"]), INVOKED);
        assert_eq!(shaped("catch", &["body", "$v"], &[true, false]), INVOKED);
        assert_eq!(bind("::foreach", &["x", "l", "b"]), IN_PROCEDURE);
        assert_eq!(
            bind("foreach", &["{a b}", "l", "c d", "m", "b"]),
            IN_PROCEDURE
        );
        assert_eq!(bind("foreach", &["::x", "l", "b"]), INVOKED);
        assert_eq!(bind("lappend", &["L", "x"]), IN_PROCEDURE);
        assert_eq!(bind("lappend", &["L", "x", "y"]), INVOKED);
        let outside_catch = Some(ReleaseBinding::CompiledInProcedureOutsideCatch);
        assert_eq!(bind("return", &["x"]), outside_catch);
        assert_eq!(bind("return", &["-code", "error", "x"]), INVOKED);
        assert_eq!(
            release_binding(TclVersion::V8_4, "return", &InvocationShape::counted(1)),
            outside_catch
        );
    }

    #[test]
    fn eight_four_compiles_some_word_counts() {
        assert_eq!(bind("append", &["s", "x"]), COMPILED);
        assert_eq!(bind("append", &["s", "x", "y"]), INVOKED);
        assert_eq!(bind("lindex", &[]), INVOKED);
        assert_eq!(bind("lindex", &["l"]), COMPILED);
        assert_eq!(bind("lset", &["l", "x"]), COMPILED);
        assert_eq!(bind("lset", &["l"]), INVOKED);
    }

    #[test]
    fn eight_four_compiles_some_string_subcommands() {
        assert_eq!(bind("string", &["length", "x"]), COMPILED);
        assert_eq!(bind("string", &["len", "x"]), COMPILED, "a unique prefix");
        assert_eq!(bind("string", &["length", "x", "y"]), INVOKED);
        assert_eq!(bind("string", &["equal", "a", "b"]), COMPILED);
        assert_eq!(bind("string", &["equal", "-nocase", "a", "b"]), INVOKED);
        assert_eq!(bind("string", &["match", "-nocase", "a*", "b"]), COMPILED);
        assert_eq!(bind("string", &["toupper", "x"]), INVOKED);
        assert_eq!(bind("string", &["t", "x"]), INVOKED, "an ambiguous prefix");
        assert_eq!(shaped("string", &["length", "x"], &[false, true]), INVOKED);
    }

    #[test]
    fn eight_four_compiles_a_regexp_of_a_plain_pattern() {
        assert_eq!(bind("regexp", &["abc", "s"]), COMPILED);
        assert_eq!(bind("regexp", &["^ab.*", "s"]), COMPILED);
        assert_eq!(bind("regexp", &["-nocase", "--", "ab$", "s"]), COMPILED);
        assert_eq!(bind("regexp", &["a+", "s"]), INVOKED);
        assert_eq!(
            bind("regexp", &["a", "s", "m"]),
            INVOKED,
            "a match variable"
        );
        assert_eq!(bind("regexp", &["-all", "a", "s"]), INVOKED);
        assert_eq!(bind("regexp", &["-x", "s"]), INVOKED);
        assert_eq!(shaped("regexp", &["a", "s"], &[false, true]), INVOKED);
    }

    #[test]
    fn eight_four_compiles_control_commands_only_with_literal_scripts() {
        assert_eq!(shaped("if", &["1", "b"], &[true, true]), COMPILED);
        assert_eq!(shaped("if", &["$c", "b"], &[false, true]), INVOKED);
        assert_eq!(
            shaped("for", &["$i", "t", "n", "b"], &[false, true, true, true]),
            COMPILED
        );
        assert_eq!(shaped("while", &["t", "$b"], &[true, false]), INVOKED);
        assert_eq!(
            shaped("foreach", &["x", "$l", "b"], &[true, false, true]),
            IN_PROCEDURE
        );
        assert_eq!(
            shaped("foreach", &["x", "$l", "$b"], &[true, false, false]),
            INVOKED
        );
        assert_eq!(
            release_binding(TclVersion::V8_4, "if", &InvocationShape::default()),
            INVOKED
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
