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

//! The scripts Tcl 8.4's compiler refuses (`TclCompileScript`).
//!
//! 8.4 compiles a script whole before any of it runs, and a compile procedure
//! that finds a command malformed (`catch {} a b`, `expr {1 +}`, `if 1`)
//! fails the compile with the command's error, as a parse error anywhere in
//! the script does. Nothing in the script runs; the error carries a `while
//! compiling` frame per script the failure is nested in, and a compile
//! procedure that compiles a script or an expression with its command adds
//! its own context line (`("if" then script line 2)`). From 8.5 a compile
//! procedure that cannot compile a command leaves it to run, and a parse
//! error is raised where it lies.
//!
//! Which compile procedure a command has is the registry's answer, through
//! the hook its specification carries; each runtime takes the script apart
//! with its own parser ([`Segmenter`]).

use tcl_dialect::model::SurfaceQuery;
use tcl_runtime_api::CompileFailure;

use crate::CommandRegistry;
use crate::clause_grammar::{ClauseGrammarSpec, ClauseRowShape};
use crate::hooks::{CodegenHookId, InlineCodegenHookId, LoweringHookId};

/// What a compile procedure made of its command.
enum Outcome {
    /// It compiled the command.
    Compiled,
    /// It left the command to run (`TCL_OUT_LINE_COMPILE`): its words are
    /// compiled as any command's are.
    OutLine,
}

/// The 8.4 compile procedures this module follows, by the hook the command's
/// specification carries.
#[derive(Clone, Copy)]
enum Procedure {
    Set,
    Incr,
    Append,
    Lappend,
    Llength,
    Break,
    Continue,
    Expr,
    If(IfKeywords),
    While,
    For,
    Foreach,
    Catch,
}

/// The keywords of `if`'s clause grammar: the one opening a further
/// condition, the noise word before a body, and the one before the final
/// body.
#[derive(Clone, Copy)]
struct IfKeywords {
    elseif: Option<&'static str>,
    then: Option<&'static str>,
    otherwise: Option<&'static str>,
}

impl IfKeywords {
    fn of(grammar: &ClauseGrammarSpec) -> Self {
        let slots = |shape: ClauseRowShape| match shape {
            ClauseRowShape::Repeated { slots } | ClauseRowShape::Once { slots } => slots,
            ClauseRowShape::Group { .. } => &[],
        };
        Self {
            elseif: grammar.rows.iter().find_map(|row| row.keyword),
            then: slots(grammar.head.shape).iter().find_map(|slot| slot.noise),
            otherwise: grammar.tail.and_then(|tail| tail.keyword),
        }
    }
}

/// One word of a command as the compiler reads it.
pub struct Word<'s> {
    /// The word as written, delimiters included.
    pub raw: &'s str,
    /// The text of a word with nothing to substitute
    /// (`TCL_TOKEN_SIMPLE_WORD`).
    pub simple: Option<String>,
    /// The scripts of its command substitutions, in order.
    pub scripts: Vec<&'s str>,
}

/// One command of a script as the compiler reads it.
pub struct Command<'s> {
    /// Where the command starts in its script.
    pub start: usize,
    /// Where C ends the text it quotes for the command: at its terminator,
    /// any whitespace before it kept, or at the end of the script.
    pub end: usize,
    /// Its words.
    pub words: Vec<Word<'s>>,
}

/// A script as `Tcl_ParseCommand` takes it apart: its commands before the
/// first it cannot parse, and that one's start and C's message.
pub struct Script<'s> {
    /// The commands it parses.
    pub commands: Vec<Command<'s>>,
    /// Where the first command it cannot parse starts, and why.
    pub fatal: Option<(usize, String)>,
}

/// How a runtime takes a script apart for [`compile_failure`]: its own
/// parser, which the compile follows command by command.
pub trait Segmenter {
    /// `source` taken apart.
    fn segment<'s>(&self, source: &'s str) -> Script<'s>;
}

/// What 8.4's compiler is asked about a script.
pub struct Compile<'a> {
    /// The registry whose hooks name each command's compile procedure.
    pub registry: &'a CommandRegistry,
    /// The surface point the registry is read at.
    pub point: Option<SurfaceQuery<'a>>,
    /// The parser the compile follows.
    pub segmenter: &'a dyn Segmenter,
    /// Whether the script is a procedure body: only there do `foreach`,
    /// `lappend` and a `catch` with a variable compile.
    pub in_procedure: bool,
    /// Whether a literal expression is parsed as stock 8.4 parses it: not
    /// where the profile adds operators of its own
    /// (`DialectProfile::f5_core_expr_grammar`).
    pub parses_expressions: bool,
}

/// Why 8.4's compiler refuses `source`, or `None` where it compiles.
#[must_use]
pub fn compile_failure(source: &str, compile: &Compile<'_>) -> Option<CompileFailure> {
    compile.script(source).err()
}

type Compiled = Result<(), CompileFailure>;

impl Compile<'_> {
    /// `TclCompileScript`: each command in turn, a failure logged with the
    /// command it arose in.
    fn script(&self, src: &str) -> Compiled {
        let parsed = self.segmenter.segment(src);
        for command in &parsed.commands {
            if let Err(mut failure) = self.command(&command.words) {
                log(&mut failure, src, command.start, command.end);
                return Err(failure);
            }
        }
        if let Some((start, message)) = parsed.fatal {
            let mut failure = failure(message);
            log(&mut failure, src, start, src.len());
            return Err(failure);
        }
        Ok(())
    }

    fn command(&self, words: &[Word<'_>]) -> Compiled {
        if let Some(procedure) = words
            .first()
            .and_then(|head| head.simple.as_deref())
            .and_then(|head| self.procedure(head, &words[1..]))
        {
            match self.compile_procedure(procedure, words)? {
                Outcome::Compiled => return Ok(()),
                Outcome::OutLine => {}
            }
        }
        self.words(words)
    }

    /// `TclCompileTokens` over every word: the command substitutions each
    /// holds are scripts compiled in turn.
    fn words(&self, words: &[Word<'_>]) -> Compiled {
        words.iter().try_for_each(|word| self.tokens(word))
    }

    fn tokens(&self, word: &Word<'_>) -> Compiled {
        word.scripts
            .iter()
            .try_for_each(|script| self.script(script))
    }

    /// `TclCompileCmdWord`: a simple word is a script compiled inline, any
    /// other compiled as a word.
    fn cmd_word(&self, word: &Word<'_>) -> Compiled {
        match &word.simple {
            Some(script) => self.script(script),
            None => self.tokens(word),
        }
    }

    /// `TclCompileExprWords` of one word: a simple word is an expression 8.4
    /// parses as it compiles it, its command substitutions scripts.
    fn expr_word(&self, word: &Word<'_>) -> Compiled {
        let Some(text) = &word.simple else {
            return self.tokens(word);
        };
        if !self.parses_expressions {
            return Ok(());
        }
        let scripts = tcl_syntax::expr::syntax_84::parse_tcl84(text).map_err(expression_failure)?;
        scripts.iter().try_for_each(|script| self.script(script))
    }

    /// The compile procedure 8.4 calls for `head`, from the hook the
    /// command's specification carries.
    fn procedure(&self, head: &str, args: &[Word<'_>]) -> Option<Procedure> {
        let name = head.strip_prefix("::").unwrap_or(head);
        let arg_texts: Vec<&str> = args
            .iter()
            .map(|word| word.simple.as_deref().unwrap_or(word.raw))
            .collect();
        let resolved = self.registry.resolve_call(name, &arg_texts, self.point)?;
        match (
            resolved.codegen_hook,
            resolved.inline_codegen_hook,
            resolved.lowering_hook,
        ) {
            (Some(CodegenHookId::Append), _, _) => Some(Procedure::Append),
            (Some(CodegenHookId::Lappend), _, _) => Some(Procedure::Lappend),
            (Some(CodegenHookId::Llength), _, _) => Some(Procedure::Llength),
            (_, Some(InlineCodegenHookId::Break), _) => Some(Procedure::Break),
            (_, Some(InlineCodegenHookId::Continue), _) => Some(Procedure::Continue),
            (_, _, Some(LoweringHookId::Set)) => Some(Procedure::Set),
            (_, _, Some(LoweringHookId::Incr)) => Some(Procedure::Incr),
            (_, _, Some(LoweringHookId::Expr)) => Some(Procedure::Expr),
            (_, _, Some(LoweringHookId::If)) => resolved
                .spec
                .clause_grammar
                .map(|grammar| Procedure::If(IfKeywords::of(grammar))),
            (_, _, Some(LoweringHookId::While)) => Some(Procedure::While),
            (_, _, Some(LoweringHookId::For)) => Some(Procedure::For),
            (_, _, Some(LoweringHookId::Foreach)) => Some(Procedure::Foreach),
            (_, _, Some(LoweringHookId::Catch)) => Some(Procedure::Catch),
            _ => None,
        }
    }

    fn compile_procedure(
        &self,
        procedure: Procedure,
        words: &[Word<'_>],
    ) -> Result<Outcome, CompileFailure> {
        let count = words.len();
        let compiled = |result: Compiled| result.map(|()| Outcome::Compiled);
        match procedure {
            Procedure::Set | Procedure::Incr => {
                if !(2..=3).contains(&count) {
                    return Err(wrong_args(match procedure {
                        Procedure::Set => "set varName ?newValue?",
                        _ => "incr varName ?increment?",
                    }));
                }
                compiled(self.words(&words[1..]))
            }
            Procedure::Append => match count {
                1 => Err(wrong_args("append varName ?value value ...?")),
                2 | 3 => compiled(self.words(&words[1..])),
                _ => Ok(Outcome::OutLine),
            },
            Procedure::Lappend => {
                if !self.in_procedure {
                    return Ok(Outcome::OutLine);
                }
                match count {
                    1 => Err(wrong_args("lappend varName ?value value ...?")),
                    3 => compiled(self.words(&words[1..])),
                    _ => Ok(Outcome::OutLine),
                }
            }
            Procedure::Llength => {
                if count != 2 {
                    return Err(wrong_args("llength list"));
                }
                compiled(self.words(&words[1..]))
            }
            Procedure::Break | Procedure::Continue => {
                if count != 1 {
                    return Err(wrong_args(match procedure {
                        Procedure::Break => "break",
                        _ => "continue",
                    }));
                }
                Ok(Outcome::Compiled)
            }
            Procedure::Expr => match &words[1..] {
                [] => Err(wrong_args("expr arg ?arg ...?")),
                [word] => compiled(self.expr_word(word)),
                rest => compiled(self.words(rest)),
            },
            Procedure::If(keywords) => self.compile_if(words, keywords),
            Procedure::While => self.compile_while(words),
            Procedure::For => self.compile_for(words),
            Procedure::Foreach => self.compile_foreach(words),
            Procedure::Catch => self.compile_catch(words),
        }
    }

    /// `TclCompileIfCmd`.
    fn compile_if(
        &self,
        words: &[Word<'_>],
        keywords: IfKeywords,
    ) -> Result<Outcome, CompileFailure> {
        if words.iter().any(|word| word.simple.is_none()) {
            return Ok(Outcome::OutLine);
        }
        let text = |index: usize| words[index].simple.as_deref().unwrap_or_default();
        let mut real_cond = true;
        let mut compile_scripts = true;
        let mut index = 0;
        while index < words.len() {
            let keyword = text(index);
            if index != 0 && Some(keyword) != keywords.elseif {
                break;
            }
            index += 1;
            if index >= words.len() {
                return Err(failure(format!(
                    "wrong # args: no expression after \"{}\" argument",
                    truncate(keyword, 50)
                )));
            }
            let condition = index;
            let mut constant = None;
            if real_cond {
                constant = tcl_syntax::boolean::truthiness_in_every_release(text(condition));
                match constant {
                    Some(value) => {
                        real_cond = false;
                        compile_scripts = value;
                    }
                    None => self
                        .expr_word(&words[condition])
                        .map_err(|failure| context(failure, "\n    (\"if\" test expression)"))?,
                }
            }
            index += 1;
            if index >= words.len() {
                return Err(failure(format!(
                    "wrong # args: no script following \"{}\" argument",
                    truncate(words[condition].raw, 50)
                )));
            }
            if Some(text(index)) == keywords.then {
                index += 1;
                if index >= words.len() {
                    return Err(failure(format!(
                        "wrong # args: no script following \"{}\" argument",
                        text(index - 1)
                    )));
                }
            }
            if compile_scripts {
                self.script(text(index)).map_err(|failure| {
                    let line = failure.line;
                    context(failure, &format!("\n    (\"if\" then script line {line})"))
                })?;
            }
            match constant {
                Some(true) => compile_scripts = false,
                Some(false) => {
                    real_cond = true;
                    compile_scripts = true;
                }
                None => {}
            }
            index += 1;
        }
        if index < words.len() {
            if Some(text(index)) == keywords.otherwise {
                index += 1;
                if index >= words.len() {
                    return Err(failure(format!(
                        "wrong # args: no script following \"{}\" argument",
                        text(index - 1)
                    )));
                }
            }
            if compile_scripts {
                self.script(text(index)).map_err(|failure| {
                    let line = failure.line;
                    context(failure, &format!("\n    (\"if\" else script line {line})"))
                })?;
            }
            if index + 1 < words.len() {
                return Err(failure(
                    "wrong # args: extra words after \"else\" clause in \"if\" command".to_owned(),
                ));
            }
        }
        Ok(Outcome::Compiled)
    }

    /// `TclCompileWhileCmd`: the body before the test, nothing at all for a
    /// constant false test.
    fn compile_while(&self, words: &[Word<'_>]) -> Result<Outcome, CompileFailure> {
        let [_, test, body] = words else {
            return Err(wrong_args("while test command"));
        };
        let (Some(test_text), Some(body_text)) = (&test.simple, &body.simple) else {
            return Ok(Outcome::OutLine);
        };
        let constant = tcl_syntax::boolean::truthiness_in_every_release(test_text);
        if constant == Some(false) {
            return Ok(Outcome::Compiled);
        }
        self.script(body_text).map_err(|failure| {
            let line = failure.line;
            context(failure, &format!("\n    (\"while\" body line {line})"))
        })?;
        if constant.is_none() {
            self.expr_word(test)
                .map_err(|failure| context(failure, "\n    (\"while\" test expression)"))?;
        }
        Ok(Outcome::Compiled)
    }

    /// `TclCompileForCmd`: start, body, next, then the test.
    fn compile_for(&self, words: &[Word<'_>]) -> Result<Outcome, CompileFailure> {
        let [_, start, test, next, body] = words else {
            return Err(wrong_args("for start test next command"));
        };
        if test.simple.is_none() || next.simple.is_none() || body.simple.is_none() {
            return Ok(Outcome::OutLine);
        }
        self.cmd_word(start)
            .map_err(|failure| context(failure, "\n    (\"for\" initial command)"))?;
        self.cmd_word(body).map_err(|failure| {
            let line = failure.line;
            context(failure, &format!("\n    (\"for\" body line {line})"))
        })?;
        self.cmd_word(next)
            .map_err(|failure| context(failure, "\n    (\"for\" loop-end command)"))?;
        self.expr_word(test)
            .map_err(|failure| context(failure, "\n    (\"for\" test expression)"))?;
        Ok(Outcome::Compiled)
    }

    /// `TclCompileForeachCmd`, only in a procedure body: the value lists,
    /// then the body.
    fn compile_foreach(&self, words: &[Word<'_>]) -> Result<Outcome, CompileFailure> {
        if !self.in_procedure {
            return Ok(Outcome::OutLine);
        }
        let count = words.len();
        if count < 4 || !count.is_multiple_of(2) {
            return Err(wrong_args(
                "foreach varList list ?varList list ...? command",
            ));
        }
        let body = &words[count - 1];
        let Some(body_text) = &body.simple else {
            return Ok(Outcome::OutLine);
        };
        let lists = &words[1..count - 1];
        for variables in lists.iter().step_by(2) {
            let Some(text) = &variables.simple else {
                return Ok(Outcome::OutLine);
            };
            let Ok(names) = tcl_syntax::list::split_list(text) else {
                return Ok(Outcome::OutLine);
            };
            if names.is_empty()
                || names
                    .iter()
                    .any(|name| name.contains("::") || name.ends_with(')'))
            {
                return Ok(Outcome::OutLine);
            }
        }
        lists
            .iter()
            .skip(1)
            .step_by(2)
            .try_for_each(|list| self.tokens(list))?;
        self.script(body_text).map_err(|failure| {
            let line = failure.line;
            context(failure, &format!("\n    (\"foreach\" body line {line})"))
        })?;
        Ok(Outcome::Compiled)
    }

    /// `TclCompileCatchCmd`: a script that fails to compile leaves the
    /// command to run, which raises the failure when it compiles the script.
    fn compile_catch(&self, words: &[Word<'_>]) -> Result<Outcome, CompileFailure> {
        let count = words.len();
        if !(2..=3).contains(&count) {
            return Err(wrong_args("catch command ?varName?"));
        }
        if count == 3 && !self.in_procedure {
            return Ok(Outcome::OutLine);
        }
        if let Some(variable) = words.get(2) {
            match &variable.simple {
                Some(name) if !name.contains("::") && !name.ends_with(')') => {}
                _ => return Ok(Outcome::OutLine),
            }
        }
        let compiled = self.cmd_word(&words[1]);
        Ok(if compiled.is_ok() {
            Outcome::Compiled
        } else {
            Outcome::OutLine
        })
    }
}

/// `LogCompilationInfo`: the failing command's line, and a `while
/// compiling` frame quoting at most 150 bytes of it.
fn log(failure: &mut CompileFailure, src: &str, start: usize, end: usize) {
    failure.line = 1 + u32::try_from(src.get(..start).unwrap_or(src).matches('\n').count())
        .unwrap_or(u32::MAX - 1);
    let text = src.get(start..end).unwrap_or_default();
    let (text, ellipsis) = if text.len() > 150 {
        let mut cut = 150;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        (&text[..cut], "...")
    } else {
        (text, "")
    };
    failure.error_info.push_str("\n    while compiling\n\"");
    failure.error_info.push_str(text);
    failure.error_info.push_str(ellipsis);
    failure.error_info.push('"');
}

fn failure(message: String) -> CompileFailure {
    CompileFailure {
        message,
        error_info: String::new(),
        line: 0,
        error_code: None,
    }
}

/// The failure 8.4's expression parser raises.
fn expression_failure(error: tcl_syntax::expr::syntax_84::Tcl84ExprError) -> CompileFailure {
    CompileFailure {
        error_code: error.error_code.map(str::to_owned),
        ..failure(error.message)
    }
}

fn wrong_args(usage: &str) -> CompileFailure {
    failure(format!("wrong # args: should be \"{usage}\""))
}

/// The context line a compile procedure adds to a nested failure.
fn context(mut failure: CompileFailure, line: &str) -> CompileFailure {
    failure.error_info.push_str(line);
    failure
}

/// At most `limit` bytes of `text`, as `%.*s` prints it.
fn truncate(text: &str, limit: usize) -> &str {
    let mut cut = text.len().min(limit);
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    &text[..cut]
}
