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

//! A mechanical scan of a C Tcl extension's source for the commands it
//! registers — the first of the three sources an extension is described from
//! (`docs/design/compiler/registry-consumer-contracts.md` § *C Tcl
//! extensions*).
//!
//! The scan is lexical. It reads no header and builds no syntax tree: it finds
//! the calls whose arguments carry a fact and says only what it can point at
//! by line.
//!
//! - **Commands** from `Tcl_CreateObjCommand`, `Tcl_CreateCommand` and
//!   `Tcl_NRCreateCommand` (and the `2` spelling of the first): the name when
//!   it is a string literal, or a macro that is one, and a row marked
//!   [`CName::Dynamic`] when it is anything else — a factory's computed name,
//!   a table's `cmds[i].name` — with the expression as written.
//! - **The package** from `Tcl_PkgProvide` and `Tcl_PkgProvideEx`.
//! - **Usage strings** from the `Tcl_WrongNumArgs` calls in the command
//!   procedure, and **subcommands** from the `Tcl_GetIndexFromObj` tables the
//!   procedure reads.
//! - **Evidence** of what the procedure's own body does: the evaluating,
//!   variable and command-table calls it makes. Only the procedure's own body
//!   is read, never its callees, so the absence of a call proves nothing and
//!   the scan never narrows the conservative default for a command; it
//!   reports what it saw.
//!
//! - **Entry points**: each function defined as `PREFIX_Init` (or
//!   `PREFIX_SafeInit`), the procedure `load` calls, so a directory holding
//!   several extensions can say which commands each one registers. Each call
//!   above records the function whose body holds it, and each function the
//!   names its body mentions, which is what ties a registration to the entry
//!   points that reach it ([`CScan::references`]).
//!
//! It is blind, by declaration, to methods registered through the `TclOO` C API
//! and to ensembles built in C: the calls that do either are reported as
//! [`CBlindSpot`]s so the commands behind them are not mistaken for absent.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

/// The calls that bind a command name to a C procedure.
const REGISTRARS: &[&str] = &[
    "Tcl_CreateObjCommand",
    "Tcl_CreateObjCommand2",
    "Tcl_CreateCommand",
    "Tcl_NRCreateCommand",
];

/// The calls that provide a package.
const PROVIDERS: &[&str] = &["Tcl_PkgProvide", "Tcl_PkgProvideEx"];

/// The calls that look a word up in an option table, whose third argument is
/// the table.
const INDEX_LOOKUPS: &[&str] = &["Tcl_GetIndexFromObj", "Tcl_GetIndexFromObjStruct"];

/// The usage-message call.
const WRONG_NUM_ARGS: &str = "Tcl_WrongNumArgs";

/// Calls that run a script or evaluate an expression.
const EVALUATING: &[&str] = &[
    "Tcl_Eval",
    "Tcl_EvalEx",
    "Tcl_EvalObjEx",
    "Tcl_EvalObjv",
    "Tcl_EvalFile",
    "Tcl_EvalTokens",
    "Tcl_EvalTokensStandard",
    "Tcl_GlobalEval",
    "Tcl_GlobalEvalObj",
    "Tcl_VarEval",
    "Tcl_VarEvalVA",
    "Tcl_RecordAndEval",
    "Tcl_RecordAndEvalObj",
    "Tcl_SubstObj",
    "Tcl_ExprString",
    "Tcl_ExprObj",
    "Tcl_ExprBoolean",
    "Tcl_ExprBooleanObj",
    "Tcl_ExprLong",
    "Tcl_ExprLongObj",
    "Tcl_ExprDouble",
    "Tcl_ExprDoubleObj",
    "Tcl_NREvalObj",
    "Tcl_NREvalObjv",
    "Tcl_NRCallObjProc",
];

/// Calls that read, write, link or trace a variable by name.
const VARIABLES: &[&str] = &[
    "Tcl_GetVar",
    "Tcl_GetVar2",
    "Tcl_GetVar2Ex",
    "Tcl_ObjGetVar2",
    "Tcl_SetVar",
    "Tcl_SetVar2",
    "Tcl_SetVar2Ex",
    "Tcl_ObjSetVar2",
    "Tcl_UnsetVar",
    "Tcl_UnsetVar2",
    "Tcl_UpVar",
    "Tcl_UpVar2",
    "Tcl_LinkVar",
    "Tcl_UnlinkVar",
    "Tcl_TraceVar",
    "Tcl_TraceVar2",
    "Tcl_UntraceVar",
    "Tcl_UntraceVar2",
];

/// Calls that change the command table.
const COMMAND_TABLE: &[&str] = &[
    "Tcl_CreateObjCommand",
    "Tcl_CreateObjCommand2",
    "Tcl_CreateCommand",
    "Tcl_NRCreateCommand",
    "Tcl_DeleteCommand",
    "Tcl_DeleteCommandFromToken",
    "Tcl_CreateAlias",
    "Tcl_CreateAliasObj",
    "Tcl_CreateEnsemble",
];

/// What the scan cannot read, by the call that does it.
const BLIND: &[(&str, &str)] = &[
    ("Tcl_NewMethod", "the TclOO C API"),
    ("Tcl_NewInstanceMethod", "the TclOO C API"),
    ("Tcl_NewObjectInstance", "the TclOO C API"),
    ("Tcl_ClassSetConstructor", "the TclOO C API"),
    ("Tcl_ClassSetDestructor", "the TclOO C API"),
    ("Tcl_CreateEnsemble", "a C-built ensemble"),
    ("Tcl_SetEnsembleMappingDict", "a C-built ensemble"),
    ("Tcl_SetEnsembleSubcommandList", "a C-built ensemble"),
    ("Tcl_SetEnsembleParameterList", "a C-built ensemble"),
    ("Tcl_SetEnsembleUnknownHandler", "a C-built ensemble"),
];

/// How a call spelled a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CName {
    /// A string literal, or a macro that is one: the name itself.
    Literal(String),
    /// Anything else, as written: the name is computed, and the scan cannot
    /// say what it is.
    Dynamic(String),
}

impl CName {
    /// The name, when it is a literal.
    #[must_use]
    pub fn literal(&self) -> Option<&str> {
        match self {
            Self::Literal(name) => Some(name),
            Self::Dynamic(_) => None,
        }
    }
}

/// One usage message a command procedure raises with `Tcl_WrongNumArgs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CUsage {
    /// How many leading words of the call the message repeats: 1 is the
    /// command name alone, so the message describes the command's own
    /// arguments; 2 includes a subcommand word.
    pub words: usize,
    /// The usage text, empty for a command that takes no argument.
    pub text: String,
    /// The line of the call.
    pub line: usize,
}

/// What a command procedure's own body calls, by the calls' names.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CEvidence {
    /// Calls that run a script or evaluate an expression.
    pub evaluates: BTreeSet<String>,
    /// Calls that touch a variable by name.
    pub variables: BTreeSet<String>,
    /// Calls that change the command table.
    pub command_table: BTreeSet<String>,
}

/// One command registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CDeclaredCommand {
    /// The name the registration spelled.
    pub name: CName,
    /// The registering call (`Tcl_CreateObjCommand` and its kin).
    pub api: String,
    /// The line of the registration.
    pub line: usize,
    /// The C function registered, when the argument names one defined in the
    /// same text.
    pub procedure: Option<String>,
    /// The function whose body makes the registration.
    pub function: Option<String>,
    /// The usage messages the procedure raises, in source order.
    pub usage: Vec<CUsage>,
    /// The subcommand names of the option tables the procedure reads, in
    /// table order.
    pub subcommands: Vec<String>,
    /// What the procedure's own body calls.
    pub evidence: CEvidence,
}

/// A package the source provides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CPackage {
    /// The package name.
    pub name: CName,
    /// The version.
    pub version: CName,
    /// The line of the call.
    pub line: usize,
    /// The function whose body makes the call.
    pub function: Option<String>,
}

/// A call the scan cannot read the effect of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CBlindSpot {
    /// The call.
    pub api: String,
    /// What it registers through, in words.
    pub through: &'static str,
    /// The line of the call.
    pub line: usize,
    /// The function whose body makes the call.
    pub function: Option<String>,
}

/// An extension's entry point: a function defined as `PREFIX_Init` or
/// `PREFIX_SafeInit`, which `load` calls for the library it loads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CEntryPoint {
    /// The prefix `load` names the extension by (`Pkga`).
    pub prefix: String,
    /// The function (`Pkga_Init`).
    pub function: String,
    /// The line it is defined at.
    pub line: usize,
}

/// What a scan of one source text found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CScan {
    /// Every registration, in source order.
    pub commands: Vec<CDeclaredCommand>,
    /// Every package provided, in source order.
    pub packages: Vec<CPackage>,
    /// The calls the scan is blind to.
    pub blind: Vec<CBlindSpot>,
    /// The entry points the text defines, in source order.
    pub entry_points: Vec<CEntryPoint>,
    /// Each function the text defines, with every name its body mentions: the
    /// functions it calls, and those it hands on by name (a command
    /// procedure it registers, a callback). An entry point reaches a function
    /// it names, and whatever that one names, across files.
    pub references: BTreeMap<String, BTreeSet<String>>,
}

/// Scan `text`, one C source file, for the commands it registers.
#[must_use]
pub fn scan_c_source(text: &str) -> CScan {
    let lexed = lex(text);
    let tokens = &lexed.tokens;
    let functions = functions(tokens);
    let tables = tables(tokens, &lexed.macros);
    let mut scan = CScan::default();
    let enclosing = |index: usize| {
        functions
            .iter()
            .find(|(_, body)| body.contains(&index))
            .map(|(name, _)| name.clone())
    };
    for index in 0..tokens.len() {
        let Tok::Ident(api) = &tokens[index].tok else {
            continue;
        };
        let line = tokens[index].line;
        if let Some((_, through)) = BLIND.iter().find(|(name, _)| name == api) {
            scan.blind.push(CBlindSpot {
                api: api.clone(),
                through,
                line,
                function: enclosing(index),
            });
        }
        let Some((args, _)) = call_arguments(tokens, index + 1) else {
            continue;
        };
        if REGISTRARS.contains(&api.as_str()) && args.len() >= 3 {
            let mut command = registration(api, line, &args, &lexed, &functions, &tables);
            command.function = enclosing(index);
            scan.commands.push(command);
        } else if PROVIDERS.contains(&api.as_str()) && args.len() >= 3 {
            scan.packages.push(CPackage {
                name: name_of(args[1], &lexed.macros),
                version: name_of(args[2], &lexed.macros),
                line,
                function: enclosing(index),
            });
        }
    }
    for (name, body) in &functions {
        let named = tokens[body.clone()]
            .iter()
            .filter_map(|token| match &token.tok {
                Tok::Ident(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        scan.references.insert(name.clone(), named);
        if let Some(prefix) = entry_prefix(name) {
            // The line of the name, which is the token before the parameter
            // list that precedes the body's opening brace.
            let line = tokens[..body.start]
                .iter()
                .rev()
                .find(|token| matches!(&token.tok, Tok::Ident(defined) if defined == name))
                .map_or(0, |token| token.line);
            scan.entry_points.push(CEntryPoint {
                prefix: prefix.to_owned(),
                function: name.clone(),
                line,
            });
        }
    }
    scan.entry_points.sort_by_key(|entry| entry.line);
    scan
}

/// The prefix of an entry point's name — `Pkga` of `Pkga_Init` or
/// `Pkga_SafeInit` — when `name` is one: a prefix that opens with a capital,
/// as the one `load` builds the procedure name from does.
fn entry_prefix(name: &str) -> Option<&str> {
    let prefix = name
        .strip_suffix("_SafeInit")
        .or_else(|| name.strip_suffix("_Init"))?;
    prefix
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
        .then_some(prefix)
}

/// The registration `api(interp, NAME, PROC, ...)` at `line`.
fn registration(
    api: &str,
    line: usize,
    args: &[&[Token]],
    lexed: &Lexed,
    functions: &BTreeMap<String, Range<usize>>,
    tables: &BTreeMap<String, Vec<String>>,
) -> CDeclaredCommand {
    let procedure = last_identifier(args[2]).filter(|name| functions.contains_key(name));
    let mut command = CDeclaredCommand {
        name: name_of(args[1], &lexed.macros),
        api: api.to_owned(),
        line,
        procedure: procedure.clone(),
        function: None,
        usage: Vec::new(),
        subcommands: Vec::new(),
        evidence: CEvidence::default(),
    };
    if let Some(range) = procedure.and_then(|name| functions.get(&name)) {
        read_procedure(&mut command, &lexed.tokens[range.clone()], lexed, tables);
    }
    command
}

/// Read what a command procedure's body states into `command`.
fn read_procedure(
    command: &mut CDeclaredCommand,
    body: &[Token],
    lexed: &Lexed,
    tables: &BTreeMap<String, Vec<String>>,
) {
    for index in 0..body.len() {
        let Tok::Ident(call) = &body[index].tok else {
            continue;
        };
        let call = call.as_str();
        let Some((args, _)) = call_arguments(body, index + 1) else {
            continue;
        };
        if EVALUATING.contains(&call) {
            command.evidence.evaluates.insert(call.to_owned());
        }
        if VARIABLES.contains(&call) {
            command.evidence.variables.insert(call.to_owned());
        }
        if COMMAND_TABLE.contains(&call) {
            command.evidence.command_table.insert(call.to_owned());
        }
        if call == WRONG_NUM_ARGS {
            command
                .usage
                .extend(usage_of(&args, body[index].line, lexed));
        }
        if INDEX_LOOKUPS.contains(&call) && args.len() >= 3 {
            let table = last_identifier_in(args[2], tables);
            for name in table
                .and_then(|name| tables.get(&name))
                .into_iter()
                .flatten()
            {
                if !command.subcommands.contains(name) {
                    command.subcommands.push(name.clone());
                }
            }
        }
    }
}

/// The usage a `Tcl_WrongNumArgs(interp, WORDS, objv, "usage")` call states,
/// when its word count and message are literals; `NULL` is the empty message.
fn usage_of(args: &[&[Token]], line: usize, lexed: &Lexed) -> Option<CUsage> {
    if args.len() < 4 {
        return None;
    }
    let words = match args[1] {
        [
            Token {
                tok: Tok::Num(count),
                ..
            },
        ] => count.parse().ok()?,
        _ => return None,
    };
    let text = match literal(args[3], &lexed.macros) {
        Some(text) => text,
        None if is_null(args[3]) => String::new(),
        None => return None,
    };
    Some(CUsage { words, text, line })
}

/// Whether `argument` is the null pointer or a zero.
fn is_null(argument: &[Token]) -> bool {
    matches!(
        argument,
        [Token { tok: Tok::Ident(name), .. }] if name == "NULL"
    ) || matches!(argument, [Token { tok: Tok::Num(zero), .. }] if zero == "0")
}

/// The name an argument spells: its literal, or the expression as written.
fn name_of(argument: &[Token], macros: &BTreeMap<String, String>) -> CName {
    literal(argument, macros)
        .map_or_else(|| CName::Dynamic(expression_text(argument)), CName::Literal)
}

/// The string an argument is: literals (and macros that are one) laid end to
/// end, with casts and a wrapping pair of parentheses taken off.
fn literal(argument: &[Token], macros: &BTreeMap<String, String>) -> Option<String> {
    let argument = strip_wrappers(argument);
    if argument.is_empty() {
        return None;
    }
    let mut text = String::new();
    for token in argument {
        match &token.tok {
            Tok::Str(piece) => text.push_str(piece),
            Tok::Ident(name) => text.push_str(macros.get(name)?),
            _ => return None,
        }
    }
    Some(text)
}

/// `argument` without leading casts (`(char *)`) and a pair of parentheses
/// that wraps all of it.
fn strip_wrappers(mut argument: &[Token]) -> &[Token] {
    loop {
        let Some(Token {
            tok: Tok::Other('('),
            ..
        }) = argument.first()
        else {
            return argument;
        };
        let Some(close) = matching(argument, 0) else {
            return argument;
        };
        let inside = &argument[1..close];
        let rest = &argument[close + 1..];
        let is_cast = !inside.is_empty()
            && inside
                .iter()
                .all(|token| matches!(&token.tok, Tok::Ident(_) | Tok::Other('*')));
        if rest.is_empty() {
            argument = inside;
        } else if is_cast {
            argument = rest;
        } else {
            return argument;
        }
    }
}

/// The last identifier in `argument`: `Proc` in `(Tcl_ObjCmdProc *) Proc` and
/// in `&Proc`.
fn last_identifier(argument: &[Token]) -> Option<String> {
    argument.iter().rev().find_map(|token| match &token.tok {
        Tok::Ident(name) => Some(name.clone()),
        _ => None,
    })
}

/// The last identifier in `argument` that names a table.
fn last_identifier_in(
    argument: &[Token],
    tables: &BTreeMap<String, Vec<String>>,
) -> Option<String> {
    argument.iter().rev().find_map(|token| match &token.tok {
        Tok::Ident(name) if tables.contains_key(name) => Some(name.clone()),
        _ => None,
    })
}

/// An expression's tokens as text, short enough to sit in a note.
fn expression_text(argument: &[Token]) -> String {
    let mut text = String::new();
    let mut previous_word = false;
    for token in argument {
        let (piece, word) = match &token.tok {
            Tok::Ident(name) => (name.clone(), true),
            Tok::Num(number) => (number.clone(), true),
            Tok::Str(string) => (format!("{string:?}"), true),
            Tok::Other(punctuation) => (punctuation.to_string(), false),
        };
        if previous_word && word {
            text.push(' ');
        }
        text.push_str(&piece);
        previous_word = word;
    }
    if text.chars().count() > 60 {
        text = text.chars().take(57).collect::<String>() + "...";
    }
    text
}

// ───────────────────────────── the lexer ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Ident(String),
    Num(String),
    Str(String),
    Other(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    tok: Tok,
    line: usize,
}

struct Lexed {
    tokens: Vec<Token>,
    /// Object-like macros whose body is a string literal.
    macros: BTreeMap<String, String>,
}

/// Lex `text` into identifiers, numbers, string literals and punctuation,
/// skipping comments, character literals and preprocessor lines, and noting
/// the macros that stand for a string.
fn lex(text: &str) -> Lexed {
    let chars: Vec<char> = text.chars().collect();
    let mut lexer = Lexer {
        chars: &chars,
        at: 0,
        line: 1,
        at_line_start: true,
        tokens: Vec::new(),
        macros: BTreeMap::new(),
    };
    while lexer.at < chars.len() {
        lexer.step();
    }
    Lexed {
        tokens: lexer.tokens,
        macros: lexer.macros,
    }
}

struct Lexer<'a> {
    chars: &'a [char],
    at: usize,
    line: usize,
    at_line_start: bool,
    tokens: Vec<Token>,
    macros: BTreeMap<String, String>,
}

impl Lexer<'_> {
    fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.at + offset).copied()
    }

    fn push(&mut self, tok: Tok) {
        self.tokens.push(Token {
            tok,
            line: self.line,
        });
        self.at_line_start = false;
    }

    fn step(&mut self) {
        let c = self.chars[self.at];
        match c {
            '\n' => {
                self.line += 1;
                self.at += 1;
                self.at_line_start = true;
            }
            c if c.is_whitespace() => self.at += 1,
            '/' if self.peek(1) == Some('/') => self.skip_line_comment(),
            '/' if self.peek(1) == Some('*') => self.skip_block_comment(),
            '#' if self.at_line_start => self.directive(),
            '"' => {
                let literal = self.string_literal();
                self.push(Tok::Str(literal));
            }
            '\'' => {
                self.skip_char_literal();
                self.at_line_start = false;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let word = self.take_while(|c| c.is_ascii_alphanumeric() || c == '_');
                self.push(Tok::Ident(word));
            }
            c if c.is_ascii_digit() => {
                let number = self.take_while(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_');
                self.push(Tok::Num(number));
            }
            c => {
                self.at += 1;
                self.push(Tok::Other(c));
            }
        }
    }

    fn take_while(&mut self, keep: impl Fn(char) -> bool) -> String {
        let start = self.at;
        while self.at < self.chars.len() && keep(self.chars[self.at]) {
            self.at += 1;
        }
        self.chars[start..self.at].iter().collect()
    }

    fn skip_line_comment(&mut self) {
        while self.at < self.chars.len() && self.chars[self.at] != '\n' {
            self.at += 1;
        }
    }

    fn skip_block_comment(&mut self) {
        self.at += 2;
        while self.at < self.chars.len() {
            if self.chars[self.at] == '*' && self.peek(1) == Some('/') {
                self.at += 2;
                return;
            }
            if self.chars[self.at] == '\n' {
                self.line += 1;
            }
            self.at += 1;
        }
    }

    /// A string literal's decoded text; `self.at` is on the opening quote.
    fn string_literal(&mut self) -> String {
        self.at += 1;
        let mut text = String::new();
        while self.at < self.chars.len() {
            let c = self.chars[self.at];
            self.at += 1;
            match c {
                '"' | '\n' => {
                    if c == '\n' {
                        self.line += 1;
                    }
                    break;
                }
                '\\' => text.push(self.escape()),
                c => text.push(c),
            }
        }
        text
    }

    /// The character an escape stands for; `self.at` is past the backslash.
    fn escape(&mut self) -> char {
        let Some(c) = self.peek(0) else {
            return '\\';
        };
        self.at += 1;
        match c {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            'a' => '\u{7}',
            '0'..='7' => {
                let mut value = c.to_digit(8).unwrap_or(0);
                for _ in 0..2 {
                    match self.peek(0).and_then(|d| d.to_digit(8)) {
                        Some(digit) => {
                            value = value * 8 + digit;
                            self.at += 1;
                        }
                        None => break,
                    }
                }
                char::from_u32(value).unwrap_or('\u{fffd}')
            }
            other => other,
        }
    }

    fn skip_char_literal(&mut self) {
        self.at += 1;
        while self.at < self.chars.len() {
            let c = self.chars[self.at];
            self.at += 1;
            match c {
                '\\' => self.at += 1,
                '\'' | '\n' => break,
                _ => {}
            }
        }
    }

    /// A preprocessor line: an object-like `#define NAME "text"` is a macro
    /// for a string; every other directive, with its continuation lines, is
    /// skipped.
    fn directive(&mut self) {
        let start = self.at;
        while self.at < self.chars.len() {
            if self.chars[self.at] == '\n' && self.chars[self.at.saturating_sub(1)] != '\\' {
                break;
            }
            if self.chars[self.at] == '\n' {
                self.line += 1;
            }
            self.at += 1;
        }
        let text: String = self.chars[start..self.at].iter().collect();
        if let Some((name, body)) = object_like_define(&text) {
            let inner = lex(&body.replace("\\\n", " "));
            if let Some(value) = string_of(&inner.tokens) {
                self.macros.insert(name.to_owned(), value);
            }
        }
    }
}

/// `NAME` and `BODY` of an object-like `#define NAME BODY`.
fn object_like_define(directive: &str) -> Option<(&str, &str)> {
    let rest = directive.trim_start_matches('#').trim_start();
    let rest = rest.strip_prefix("define")?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    let (name, body) = rest.split_at(end);
    if name.is_empty() || body.starts_with('(') {
        return None;
    }
    Some((name, body))
}

/// The text of tokens that are all string literals.
fn string_of(tokens: &[Token]) -> Option<String> {
    let mut text = String::new();
    if tokens.is_empty() {
        return None;
    }
    for token in tokens {
        match &token.tok {
            Tok::Str(piece) => text.push_str(piece),
            _ => return None,
        }
    }
    Some(text)
}

// ───────────────────────────── the parser ─────────────────────────────

/// The index of the token closing the bracket that opens at `open`.
fn matching(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token.tok {
            Tok::Other('(' | '[' | '{') => depth += 1,
            Tok::Other(')' | ']' | '}') => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// The arguments of the call whose `(` is at `open`, split at the top-level
/// commas, and the index of its `)`.
fn call_arguments(tokens: &[Token], open: usize) -> Option<(Vec<&[Token]>, usize)> {
    if !matches!(
        tokens.get(open),
        Some(Token {
            tok: Tok::Other('('),
            ..
        })
    ) {
        return None;
    }
    let close = matching(tokens, open)?;
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut start = open + 1;
    for index in open + 1..close {
        match tokens[index].tok {
            Tok::Other('(' | '[' | '{') => depth += 1,
            Tok::Other(')' | ']' | '}') => depth = depth.saturating_sub(1),
            Tok::Other(',') if depth == 0 => {
                args.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < close || !args.is_empty() {
        args.push(&tokens[start..close]);
    }
    Some((args, close))
}

/// The functions the text defines at file scope, by name, with the token range
/// of each body (the braces excluded).
fn functions(tokens: &[Token]) -> BTreeMap<String, Range<usize>> {
    let mut found = BTreeMap::new();
    let mut index = 0;
    while index < tokens.len() {
        match &tokens[index].tok {
            Tok::Other('{') => {
                // A brace that opens no function body: an initialiser, a
                // struct. Skip it whole.
                index = matching(tokens, index).map_or(tokens.len(), |close| close + 1);
            }
            Tok::Ident(name) => {
                if let Some((_, close)) = call_arguments(tokens, index + 1)
                    && matches!(
                        tokens.get(close + 1),
                        Some(Token {
                            tok: Tok::Other('{'),
                            ..
                        })
                    )
                    && let Some(end) = matching(tokens, close + 1)
                {
                    found.insert(name.clone(), close + 2..end);
                    index = end + 1;
                } else {
                    index += 1;
                }
            }
            _ => index += 1,
        }
    }
    found
}

/// The option tables the text defines at file scope — `NAME[] = { "a", "b",
/// NULL }`, or an array of structs whose first member is the name — by name,
/// with the names in table order.
fn tables(tokens: &[Token], macros: &BTreeMap<String, String>) -> BTreeMap<String, Vec<String>> {
    let mut found = BTreeMap::new();
    let mut depth = 0usize;
    for index in 0..tokens.len() {
        match &tokens[index].tok {
            Tok::Other('{') => depth += 1,
            Tok::Other('}') => depth = depth.saturating_sub(1),
            Tok::Ident(name) if depth == 0 => {
                if let Some(names) = table_initialiser(tokens, index + 1, macros) {
                    found.insert(name.clone(), names);
                }
            }
            _ => {}
        }
    }
    found
}

/// The names of the table whose `[` is at `open`, when `[ ] = {` follows.
fn table_initialiser(
    tokens: &[Token],
    open: usize,
    macros: &BTreeMap<String, String>,
) -> Option<Vec<String>> {
    let is = |at: usize, wanted: char| matches!(tokens.get(at), Some(Token { tok: Tok::Other(c), .. }) if *c == wanted);
    if !is(open, '[') {
        return None;
    }
    let close = matching(tokens, open)?;
    if !is(close + 1, '=') || !is(close + 2, '{') {
        return None;
    }
    let end = matching(tokens, close + 2)?;
    let mut names = Vec::new();
    let mut depth = 0usize;
    let mut element_start = close + 3;
    for index in close + 3..=end {
        let boundary = index == end || (depth == 0 && is(index, ','));
        if boundary {
            let element = &tokens[element_start..index];
            match first_name(element, macros) {
                Some(name) => names.push(name),
                None => break,
            }
            element_start = index + 1;
            continue;
        }
        match tokens[index].tok {
            Tok::Other('(' | '[' | '{') => depth += 1,
            Tok::Other(')' | ']' | '}') => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    (!names.is_empty()).then_some(names)
}

/// The name an initialiser element carries: its own literal, or the first
/// literal of the struct it braces. `NULL`, a zero and an empty element end
/// the table.
fn first_name(element: &[Token], macros: &BTreeMap<String, String>) -> Option<String> {
    let element = match element {
        [
            Token {
                tok: Tok::Other('{'),
                ..
            },
            inner @ ..,
        ] => {
            let end = inner
                .iter()
                .position(|token| matches!(token.tok, Tok::Other(',' | '}')))
                .unwrap_or(inner.len());
            &inner[..end]
        }
        other => other,
    };
    literal(element, macros).filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(scan: &CScan) -> Vec<Option<&str>> {
        scan.commands.iter().map(|c| c.name.literal()).collect()
    }

    /// The real test extension: its five commands and its package, each
    /// procedure's usage, the subcommands of the table `pkga_calc` reads and
    /// the one call that changes the command table. The negative: a
    /// registration whose name is computed is a row marked dynamic, never an
    /// invented name, and the rows around it are unchanged.
    /// The scan names each entry point with the line of its name, and each
    /// registration, package and unreadable call with the function that makes
    /// it; a function whose prefix does not open with a capital is not an entry
    /// point (negative), and what each function's body names is recorded.
    #[test]
    fn the_scan_names_entry_points_and_the_function_each_call_is_in() {
        let scan = scan_c_source(
            "int\nPkga_Init(Tcl_Interp *interp)\n{\n    Tcl_PkgProvide(interp, \"pkga\", \"1.0\");\n    \
             Tcl_CreateObjCommand(interp, \"x\", X, 0, 0);\n    Tcl_NewMethod(interp, c, n, 1, &t, 0);\n    \
             return Helper(interp);\n}\nint my_Init(Tcl_Interp *interp) { return 0; }\n\
             int Pkga_SafeInit(Tcl_Interp *interp) { return Pkga_Init(interp); }\n",
        );
        assert_eq!(
            scan.entry_points,
            [
                CEntryPoint {
                    prefix: "Pkga".to_owned(),
                    function: "Pkga_Init".to_owned(),
                    line: 2,
                },
                CEntryPoint {
                    prefix: "Pkga".to_owned(),
                    function: "Pkga_SafeInit".to_owned(),
                    line: 10,
                },
            ]
        );
        let init = Some("Pkga_Init".to_owned());
        assert_eq!(scan.commands[0].function, init);
        assert_eq!(scan.packages[0].function, init);
        assert_eq!(scan.blind[0].function, init);
        assert!(scan.references["Pkga_Init"].contains("Helper"));
        assert!(scan.references["Pkga_SafeInit"].contains("Pkga_Init"));
        assert!(scan.references.contains_key("my_Init"));
    }

    #[test]
    fn the_scan_finds_pkga_s_commands_and_provide() {
        let text = include_str!("../../../tcl-cshim/tests/c/pkga.c");
        let scan = scan_c_source(text);
        assert_eq!(
            names(&scan),
            [
                Some("pkga_eq"),
                Some("pkga_quote"),
                Some("pkga_calc"),
                Some("pkga_count"),
                Some("pkga_forget")
            ]
        );
        assert_eq!(scan.packages.len(), 1);
        assert_eq!(scan.packages[0].name, CName::Literal("pkga".to_owned()));
        assert_eq!(scan.packages[0].version, CName::Literal("1.0".to_owned()));
        assert!(scan.blind.is_empty());

        let usage = |command: &CDeclaredCommand| -> Vec<(usize, String)> {
            command
                .usage
                .iter()
                .map(|u| (u.words, u.text.clone()))
                .collect()
        };
        assert_eq!(
            usage(&scan.commands[0]),
            [(1, "string1 string2".to_owned())]
        );
        assert_eq!(usage(&scan.commands[1]), [(1, "value".to_owned())]);
        assert_eq!(usage(&scan.commands[3]), [(1, String::new())]);
        let calc = &scan.commands[2];
        assert_eq!(
            calc.subcommands,
            [
                "add", "sub", "range", "sum", "neg", "not", "fail", "join", "len", "dup"
            ]
        );
        assert_eq!(calc.procedure.as_deref(), Some("Pkga_CalcObjCmd"));
        assert!(
            usage(calc).contains(&(1, "subcommand ?arg ...?".to_owned())),
            "{:?}",
            usage(calc)
        );
        assert!(
            scan.commands[4]
                .evidence
                .command_table
                .contains("Tcl_DeleteCommand"),
            "pkga_forget deletes pkga_count"
        );
        assert_eq!(scan.commands[0].evidence, CEvidence::default());

        let with_factory = format!(
            "{text}\nstatic int Factory(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])\n\
             {{\n    Tcl_CreateObjCommand(interp, Tcl_GetString(objv[1]), Pkga_EqObjCmd, NULL, NULL);\n    return 0;\n}}\n"
        );
        let scan = scan_c_source(&with_factory);
        let dynamic: Vec<&CDeclaredCommand> = scan
            .commands
            .iter()
            .filter(|c| matches!(c.name, CName::Dynamic(_)))
            .collect();
        assert_eq!(dynamic.len(), 1, "one row marked dynamic");
        assert_eq!(
            dynamic[0].name,
            CName::Dynamic("Tcl_GetString(objv[1])".to_owned())
        );
        assert_eq!(dynamic[0].procedure.as_deref(), Some("Pkga_EqObjCmd"));
        assert_eq!(
            scan.commands.len(),
            6,
            "the five literal rows are still there"
        );
    }

    #[test]
    fn comments_strings_and_preprocessor_lines_hide_nothing_real_and_show_nothing_false() {
        let scan = scan_c_source(
            r#"
            /* Tcl_CreateObjCommand(interp, "in_comment", P, 0, 0); */
            // Tcl_CreateObjCommand(interp, "in_line_comment", P, 0, 0);
            #define NOT_A_CALL Tcl_CreateObjCommand(interp, "in_macro", P, 0, 0)
            static const char *text = "Tcl_CreateObjCommand(interp, \"in_string\", P, 0, 0)";
            int P(void *d, Tcl_Interp *i, int c, Tcl_Obj *const v[]) { return 0; }
            int Init(Tcl_Interp *interp) {
                Tcl_CreateObjCommand(interp, "real", P, NULL, NULL);
                return 0;
            }
            "#,
        );
        assert_eq!(names(&scan), [Some("real")]);
        assert_eq!(scan.commands[0].procedure.as_deref(), Some("P"));
    }

    #[test]
    fn a_string_literal_a_macro_and_adjacent_literals_are_names() {
        let scan = scan_c_source(
            r#"
            #define PREFIX "ext_"
            #define FULL "full_name"
            int Init(Tcl_Interp *interp) {
                Tcl_CreateObjCommand(interp, "plain", P, NULL, NULL);
                Tcl_CreateObjCommand(interp, FULL, P, NULL, NULL);
                Tcl_CreateObjCommand(interp, PREFIX "joined", P, NULL, NULL);
                Tcl_CreateObjCommand(interp, (char *) "cast", P, NULL, NULL);
                return 0;
            }
            "#,
        );
        assert_eq!(
            names(&scan),
            [
                Some("plain"),
                Some("full_name"),
                Some("ext_joined"),
                Some("cast")
            ]
        );
    }

    #[test]
    fn a_computed_name_is_a_dynamic_row_with_its_expression() {
        let scan = scan_c_source(
            r"
            int Factory(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
                Tcl_CreateObjCommand(interp, Tcl_GetString(objv[1]), P, NULL, NULL);
                Tcl_CreateObjCommand(interp, cmds[i].name, P, NULL, NULL);
                return 0;
            }
            ",
        );
        assert_eq!(scan.commands.len(), 2);
        assert_eq!(
            scan.commands[0].name,
            CName::Dynamic("Tcl_GetString(objv[1])".to_owned())
        );
        assert_eq!(
            scan.commands[1].name,
            CName::Dynamic("cmds[i].name".to_owned())
        );
        assert_eq!(scan.commands[0].line, 3);
    }

    #[test]
    fn the_string_api_and_the_nre_api_register_commands_too() {
        let scan = scan_c_source(
            r#"
            int Old(void *d, Tcl_Interp *i, int argc, const char *argv[]) { return 0; }
            int New(void *d, Tcl_Interp *i, int objc, Tcl_Obj *const objv[]) { return 0; }
            int Init(Tcl_Interp *interp) {
                Tcl_CreateCommand(interp, "old_style", Old, NULL, NULL);
                Tcl_NRCreateCommand(interp, "nre_style", New, NRProc, NULL, NULL);
                Tcl_CreateObjCommand2(interp, "wide", New, NULL, NULL);
                return 0;
            }
            "#,
        );
        let apis: Vec<&str> = scan.commands.iter().map(|c| c.api.as_str()).collect();
        assert_eq!(
            apis,
            [
                "Tcl_CreateCommand",
                "Tcl_NRCreateCommand",
                "Tcl_CreateObjCommand2"
            ]
        );
        assert_eq!(
            names(&scan),
            [Some("old_style"), Some("nre_style"), Some("wide")]
        );
    }

    #[test]
    fn a_package_is_provided_by_name_and_version() {
        let scan = scan_c_source(
            r#"
            int Init(Tcl_Interp *interp) {
                Tcl_PkgProvide(interp, "pkga", "1.0");
                Tcl_PkgProvideEx(interp, "pkgb", PACKAGE_VERSION, NULL);
                return 0;
            }
            "#,
        );
        assert_eq!(scan.packages.len(), 2);
        assert_eq!(scan.packages[0].name, CName::Literal("pkga".to_owned()));
        assert_eq!(scan.packages[0].version, CName::Literal("1.0".to_owned()));
        assert_eq!(scan.packages[1].name, CName::Literal("pkgb".to_owned()));
        assert_eq!(
            scan.packages[1].version,
            CName::Dynamic("PACKAGE_VERSION".to_owned()),
            "an unresolved macro is not a version"
        );
    }

    #[test]
    fn usage_messages_and_subcommand_tables_belong_to_the_registered_procedure() {
        let scan = scan_c_source(
            r#"
            static const char *const subs[] = { "add", "sub", NULL };
            static int Other(void *d, Tcl_Interp *i, int c, Tcl_Obj *const v[]) {
                Tcl_WrongNumArgs(i, 1, v, "not this one");
                return 0;
            }
            static int Calc(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
                int index;
                if (objc < 2) {
                    Tcl_WrongNumArgs(interp, 1, objv, "subcommand ?arg ...?");
                    return TCL_ERROR;
                }
                Tcl_GetIndexFromObj(interp, objv[1], subs, "subcommand", 0, &index);
                Tcl_WrongNumArgs(interp, 2, objv, "n m");
                Tcl_WrongNumArgs(interp, 1, objv, NULL);
                return TCL_OK;
            }
            int Init(Tcl_Interp *interp) {
                Tcl_CreateObjCommand(interp, "calc", Calc, NULL, NULL);
                return 0;
            }
            "#,
        );
        let calc = &scan.commands[0];
        assert_eq!(calc.subcommands, ["add", "sub"]);
        let usage: Vec<(usize, &str)> = calc
            .usage
            .iter()
            .map(|u| (u.words, u.text.as_str()))
            .collect();
        assert_eq!(usage, [(1, "subcommand ?arg ...?"), (2, "n m"), (1, "")]);
    }

    #[test]
    fn a_struct_table_names_its_subcommands_by_the_first_member() {
        let scan = scan_c_source(
            r#"
            static const struct { const char *name; int op; } ops[] = {
                {"open", 1}, {"close", 2}, {NULL, 0}
            };
            static int P(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
                Tcl_GetIndexFromObjStruct(interp, objv[1], ops, sizeof(ops[0]), "op", 0, &i);
                return 0;
            }
            int Init(Tcl_Interp *interp) {
                Tcl_CreateObjCommand(interp, "ext", P, NULL, NULL);
                return 0;
            }
            "#,
        );
        assert_eq!(scan.commands[0].subcommands, ["open", "close"]);
    }

    #[test]
    fn evidence_is_what_the_procedures_own_body_calls() {
        let scan = scan_c_source(
            r#"
            static int Runs(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
                Tcl_EvalObjEx(interp, objv[1], 0);
                Tcl_ObjSetVar2(interp, objv[2], NULL, objv[3], 0);
                Tcl_DeleteCommand(interp, "gone");
                return 0;
            }
            static int Quiet(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
                helper(interp);
                return 0;
            }
            int Init(Tcl_Interp *interp) {
                Tcl_CreateObjCommand(interp, "runs", Runs, NULL, NULL);
                Tcl_CreateObjCommand(interp, "quiet", Quiet, NULL, NULL);
                return 0;
            }
            "#,
        );
        let runs = &scan.commands[0].evidence;
        assert!(runs.evaluates.contains("Tcl_EvalObjEx"));
        assert!(runs.variables.contains("Tcl_ObjSetVar2"));
        assert!(runs.command_table.contains("Tcl_DeleteCommand"));
        assert_eq!(
            scan.commands[1].evidence,
            CEvidence::default(),
            "a callee's calls are not read, so nothing is claimed of it"
        );
    }

    #[test]
    fn the_oo_c_api_and_c_built_ensembles_are_reported_as_blind_spots() {
        let scan = scan_c_source(
            r#"
            int Init(Tcl_Interp *interp) {
                Tcl_Class cls = Tcl_NewMethod(interp, c, name, 1, &type, 0);
                Tcl_Command ens = Tcl_CreateEnsemble(interp, "ens", ns, 0);
                return 0;
            }
            "#,
        );
        let blind: Vec<(&str, &str)> = scan
            .blind
            .iter()
            .map(|b| (b.api.as_str(), b.through))
            .collect();
        assert_eq!(
            blind,
            [
                ("Tcl_NewMethod", "the TclOO C API"),
                ("Tcl_CreateEnsemble", "a C-built ensemble")
            ]
        );
    }

    #[test]
    fn lines_count_through_comments_continuations_and_strings() {
        let scan = scan_c_source(
            "/* a\nb */\n#define LONG \\\n  \"x\"\n\nint Init(Tcl_Interp *i) {\n  Tcl_CreateObjCommand(i, \"c\", P, 0, 0);\n}\n",
        );
        assert_eq!(scan.commands[0].line, 7);
    }

    #[test]
    fn a_parenthesised_name_is_read_through() {
        let scan = scan_c_source(
            r#"int Init(Tcl_Interp *i) { Tcl_CreateObjCommand(i, ("paren"), P, 0, 0); return 0; }"#,
        );
        assert_eq!(names(&scan), [Some("paren")]);
    }

    #[test]
    fn escapes_in_a_name_are_decoded() {
        let scan = scan_c_source(
            r#"int Init(Tcl_Interp *i) { Tcl_CreateObjCommand(i, "a\tb\101\x", P, 0, 0); return 0; }"#,
        );
        assert_eq!(names(&scan), [Some("a\tbAx")]);
    }

    #[test]
    fn a_character_literal_holding_a_quote_hides_nothing() {
        let scan = scan_c_source(
            "int Init(Tcl_Interp *i) { char q = '\"'; Tcl_CreateObjCommand(i, \"after_quote\", P, 0, 0); return 0; }",
        );
        assert_eq!(names(&scan), [Some("after_quote")]);
    }

    #[test]
    fn a_continued_define_is_one_macro() {
        let scan = scan_c_source(
            "#define LONG \\\n  \"x\"\nint Init(Tcl_Interp *i) {\n  Tcl_CreateObjCommand(i, LONG, P, 0, 0);\n}\n",
        );
        assert_eq!(names(&scan), [Some("x")]);
        assert_eq!(scan.commands[0].line, 4);
    }

    #[test]
    fn a_name_the_text_does_not_spell_out_is_dynamic_and_every_arm_of_a_conditional_is_read() {
        let scan = scan_c_source(
            r#"
            #include "names.h"
            #define NAMED(x) "named_" x
            int Init(Tcl_Interp *interp) {
            #ifdef WITH_EXTRA
                Tcl_CreateObjCommand(interp, "extra", P, NULL, NULL);
            #else
                Tcl_CreateObjCommand(interp, "plain", P, NULL, NULL);
            #endif
                Tcl_CreateObjCommand(interp, NAMED("a"), P, NULL, NULL);
                Tcl_CreateObjCommand(interp, FROM_A_HEADER, P, NULL, NULL);
                return 0;
            }
            "#,
        );
        assert_eq!(names(&scan), [Some("extra"), Some("plain"), None, None]);
        assert_eq!(
            scan.commands[2].name,
            CName::Dynamic("NAMED(\"a\")".to_owned())
        );
        assert_eq!(
            scan.commands[3].name,
            CName::Dynamic("FROM_A_HEADER".to_owned())
        );
    }

    #[test]
    fn commas_inside_a_nested_call_do_not_split_the_arguments() {
        let scan = scan_c_source(
            "int Init(Tcl_Interp *i) { Tcl_CreateObjCommand(i, pick(a, b), P, 0, 0); return 0; }",
        );
        assert_eq!(scan.commands.len(), 1);
        assert_eq!(
            scan.commands[0].name,
            CName::Dynamic("pick(a,b)".to_owned())
        );
    }

    #[test]
    fn a_procedure_the_text_does_not_define_is_not_read() {
        let scan = scan_c_source(
            "int Init(Tcl_Interp *i) { Tcl_CreateObjCommand(i, \"x\", Defined_Elsewhere, 0, 0); return 0; }",
        );
        assert_eq!(scan.commands[0].procedure, None);
        assert!(scan.commands[0].usage.is_empty());
    }

    #[test]
    fn two_lookups_of_one_table_name_each_subcommand_once() {
        let scan = scan_c_source(
            r#"
            static const char *const subs[] = { "add", "sub", NULL };
            static int P(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
                Tcl_GetIndexFromObj(interp, objv[1], subs, "subcommand", 0, &i);
                Tcl_GetIndexFromObj(interp, objv[2], subs, "subcommand", 0, &j);
                return 0;
            }
            int Init(Tcl_Interp *interp) {
                Tcl_CreateObjCommand(interp, "ext", P, NULL, NULL);
                return 0;
            }
            "#,
        );
        assert_eq!(scan.commands[0].subcommands, ["add", "sub"]);
    }

    #[test]
    fn a_table_ends_at_its_null() {
        let scan = scan_c_source(
            r#"
            static const char *const subs[] = { "add", NULL, "after" };
            static int P(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
                Tcl_GetIndexFromObj(interp, objv[1], subs, "subcommand", 0, &i);
                return 0;
            }
            int Init(Tcl_Interp *interp) {
                Tcl_CreateObjCommand(interp, "ext", P, NULL, NULL);
                return 0;
            }
            "#,
        );
        assert_eq!(scan.commands[0].subcommands, ["add"]);
    }
}
