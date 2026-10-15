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

    /// Whether every word after the command at `indices` is literal; `false`
    /// when the words are not known.
    fn literal_at(&self, mut indices: impl Iterator<Item = usize>) -> bool {
        self.words.is_some() && indices.all(|index| self.word(index).is_some())
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
        // `TclCompileIfCmd` compiles only words free of substitution, and
        // `TclCompileForCmd` and `TclCompileWhileCmd` only a literal test,
        // step and body.
        "if" => compiled_if(shape.literal_at(0..count.unwrap_or(0))),
        "for" => compiled_if(count == Some(4) && shape.literal_at(1..4)),
        "while" => compiled_if(count == Some(2) && shape.literal_at(0..2)),
        "lindex" => compiled_if(count.is_some_and(|count| count >= 1)),
        "lset" => compiled_if(count.is_some_and(|count| count >= 2)),
        "regexp" => compiled_if(regexp_compiles(shape)),
        "string" => compiled_if(string_compiles(shape)),
        // The rest decline outside a procedure body, `TclCompileForeachCmd`
        // too for a variable list or body that is not literal or a variable
        // that is not a local scalar, `TclCompileLappendCmd` for more than one
        // value and `TclCompileReturnCmd` for options or inside a `catch`.
        "foreach" => in_procedure_if(foreach_compiles(shape)),
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

/// `TclCompileForeachCmd`'s shape: variable lists and value lists in pairs,
/// then a literal body, each variable list a literal list of local scalars.
fn foreach_compiles(shape: &InvocationShape) -> bool {
    let Some(count) = shape.arg_count() else {
        return false;
    };
    if count < 3 || count % 2 == 0 || shape.word(count - 1).is_none() {
        return false;
    }
    (0..count - 1).step_by(2).all(|index| {
        shape.word(index).is_some_and(|list| {
            tcl_syntax::list::split_list(list).is_ok_and(|names| {
                !names.is_empty() && names.iter().all(|name| is_local_scalar(name))
            })
        })
    })
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
    fn eight_four_evaluates_eval_and_uplevel_scripts_directly() {
        assert!(evaluates_script_directly(TclVersion::V8_4, "eval"));
        assert!(evaluates_script_directly(TclVersion::V8_4, "::uplevel"));
        assert!(!evaluates_script_directly(TclVersion::V8_4, "catch"));
        assert!(!evaluates_script_directly(TclVersion::V8_5, "eval"));
    }
}
