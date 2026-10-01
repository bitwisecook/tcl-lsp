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

//! A reference body run as a declared implementation
//! (`docs/design/compiler/value-evaluation.md` § *Three routes, declared on
//! the spec*, the implementation route).
//!
//! A command a pack backs with a Tcl body says what runs when it is called, and
//! when that body is value-position Tcl the bounded host can run it too: the
//! answer to `vendor::double 21` is the body's answer, computed at analysis time
//! under the target release, never because anyone guessed. What may reach the
//! host is the registry's call, and it is a **whitelist**: a command the scan
//! does not list is outside the sandbox however harmless it looks, so a body
//! that reaches for the frame (`upvar`, `uplevel`, `global`, `variable`), a
//! channel, a process, the world (`clock`, `info`, `file`) or a command nobody
//! has heard of derives nothing. The list is the hook host's own
//! ([`SANDBOX_WORDS`] against `tcl_spec_hooks::SANDBOX_COMMANDS`, held equal by
//! a test in `tcl-spectcl`), and the host enforces it again at run time; the scan
//! decides only that a derivation is worth installing, and declines whatever it
//! cannot follow — a command computed from a variable, a script argument it
//! cannot read, a callback — because a body the scan admits has to be one it read
//! to the end.
//!
//! A body is a function of its parameters. Every variable it names is a plain
//! local — no namespace-qualified name, no name it computes — and the expression
//! functions it calls are the ones [`MATH_FUNCTIONS`] lists, so `rand` and
//! `srand`, which read the interpreter's random state, and a function nobody
//! here has read are refused as a command off the list is.
//!
//! The derivation takes exactly one `proc` that defines the command, whose
//! parameters are all required, and whose body is a script of whitelisted
//! commands in which `return` appears only as the last statement. That body
//! becomes the hook body of a declared implementation: the evaluate family
//! answers by `fold`, so the script runs as a command substitution and its value
//! is folded, with a final `return V` standing as `set` of `V`.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock, PoisonError};

use tcl_lexer::script::{CommandSpan, WordKind, group_commands};
use tcl_lexer::{Lexer, LexerConfig, SourceMap, Token, TokenType};
use tcl_syntax::formal_params::parse_formal_parameters;
use tcl_syntax::list::split_list;

use super::const_ops::Needs;
use super::declared::{
    DeclaredEvaluation, DeclaredImplementation, DeclaredSemantics, DeclaredStructure,
};
use super::route::{
    CompletionSupport, ContextDependency, DeclaredInput, EvaluatorCapability, Exactness, HostKind,
    ImplementationBudget, ImplementationIdentity,
};

/// Every command a reference body may call: the closed whitelist the hook host
/// runs a body against, besides its family's emitter verbs.
pub const SANDBOX_WORDS: &[&str] = &[
    "set", "expr", "if", "while", "for", "foreach", "switch", "return", "break", "continue",
    "incr", "lappend", "lassign", "list", "lindex", "llength", "lrange", "lreplace", "lsearch",
    "lsort", "join", "split", "string", "format", "scan", "regexp", "regsub", "dict", "binary",
];

/// How far down one body the scan reads before it declines.
const MAX_DEPTH: u32 = 32;

/// The variable a final `return V` is rewritten to set.
const RESULT: &str = "__spec_result";

/// The expression functions a body may call. `rand` and `srand` read the
/// interpreter's random state, and a function the scan does not list is a
/// command (`tcl::mathfunc::NAME`) nobody here has read.
const MATH_FUNCTIONS: &[&str] = &[
    "abs", "acos", "asin", "atan", "atan2", "bool", "ceil", "cos", "cosh", "double", "entier",
    "exp", "floor", "fmod", "hypot", "int", "isqrt", "log", "log10", "max", "min", "pow", "round",
    "sin", "sinh", "sqrt", "tan", "tanh", "wide",
];

/// What a command that is not on the whitelist reaches for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// The caller's frame or the namespace's variables.
    Frame,
    /// A channel.
    Channel,
    /// A process or the loading of code.
    Process,
    /// The world: the clock, the interpreter, the file system.
    World,
    /// Something the scan does not know, which is outside the sandbox all the
    /// same.
    Unlisted,
}

impl Reach {
    fn of(word: &str) -> Self {
        match word {
            "upvar" | "uplevel" | "global" | "variable" | "namespace" | "unset" | "array"
            | "trace" | "proc" | "rename" => Self::Frame,
            "open" | "close" | "puts" | "gets" | "read" | "seek" | "tell" | "flush" | "eof"
            | "fconfigure" | "fcopy" | "fileevent" | "socket" | "chan" => Self::Channel,
            "exec" | "source" | "load" | "package" | "eval" | "subst" | "interp" | "after"
            | "vwait" | "update" => Self::Process,
            "clock" | "info" | "file" | "glob" | "pwd" | "cd" | "env" | "encoding" | "time" => {
                Self::World
            }
            _ => Self::Unlisted,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Frame => "the caller's frame",
            Self::Channel => "a channel",
            Self::Process => "a process or other code",
            Self::World => "the world",
            Self::Unlisted => "a command the sandbox does not list",
        }
    }
}

/// Why a reference body is not run as a declared implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inexpressible {
    /// The text is not exactly one `proc` with literal, braced parameters and
    /// body, defining the command it backs.
    NotOneProc,
    /// A parameter is not a plain required one: it has a default, or the list
    /// ends in `args`.
    Parameters(String),
    /// A command outside the whitelist.
    Command {
        /// The command word.
        word: String,
        /// What it reaches for.
        reach: Reach,
    },
    /// A command whose name is computed, expanded or substituted.
    ComputedCommand,
    /// A script argument the scan cannot read to the end: one that is computed,
    /// or a callback a command runs.
    Script(String),
    /// A variable that is not a plain local: a namespace-qualified name, or one
    /// the body computes.
    Variable(String),
    /// An expression function the scan does not list, `rand` among them.
    Function(String),
    /// The body uses the name the rewritten `return` sets.
    Reserved,
    /// A `return` that is not the last statement of the body.
    EarlyReturn,
    /// A final `return` with options or more than one argument.
    ReturnOptions,
    /// The text does not lex, or is nested past what the scan reads.
    Unreadable,
}

impl fmt::Display for Inexpressible {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOneProc => {
                f.write_str("the text is not exactly one `proc` that defines the command")
            }
            Self::Parameters(name) => write!(f, "parameter `{name}` is not a plain required one"),
            Self::Command { word, reach } => {
                write!(
                    f,
                    "`{word}` is outside the sandbox: it reaches {}",
                    reach.label()
                )
            }
            Self::ComputedCommand => f.write_str("a command name is computed"),
            Self::Script(why) => write!(f, "a script argument cannot be read to the end: {why}"),
            Self::Variable(name) => {
                write!(f, "variable `{name}` is not a plain local of the procedure")
            }
            Self::Function(name) => {
                write!(f, "expression function `{name}` is outside the sandbox")
            }
            Self::Reserved => write!(f, "the body uses `{RESULT}`, which the derivation reserves"),
            Self::EarlyReturn => f.write_str("`return` is not the last statement of the body"),
            Self::ReturnOptions => f.write_str("the last `return` takes options or several words"),
            Self::Unreadable => f.write_str("the body does not lex, or nests too deeply"),
        }
    }
}

/// A reference body as the declared implementation the registry derives from
/// it.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceImplementation {
    /// The declaration to install at the command's scope, its implementation
    /// not yet bound to a hook slot and its identity not yet naming a pack —
    /// both of which the host plan fills in.
    pub declared: &'static DeclaredSemantics,
}

/// What the hook host is given for a derived implementation: the parameters and
/// the body text of the hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceHook {
    /// The procedure's parameters, which bind the declared inputs in order.
    pub params: Vec<String>,
    /// The hook body: `fold` of the script's value.
    pub body: String,
}

/// The implementation and the hook body of the `proc` in `definition` as the
/// body of `command`, or why the sandbox cannot run it.
///
/// # Errors
///
/// The reason the body is not expressible in the sandbox.
pub fn derive(
    command: &str,
    definition: &str,
) -> Result<(ReferenceImplementation, ReferenceHook), Inexpressible> {
    let hook = hook_of(command, definition)?;
    Ok((
        ReferenceImplementation {
            declared: declared_for(command, &hook),
        },
        hook,
    ))
}

/// The hook of `definition`, without installing anything: the whole scan.
///
/// # Errors
///
/// The reason the body is not expressible in the sandbox.
pub fn hook_of(command: &str, definition: &str) -> Result<ReferenceHook, Inexpressible> {
    let parts = definition_parts(command, definition).ok_or(Inexpressible::NotOneProc)?;
    let params = parameters(&parts.params)?;
    let script = scan_body(&parts.body)?;
    Ok(ReferenceHook {
        params,
        body: format!("fold [\n{script}\n]"),
    })
}

/// The three words after `proc`.
struct Definition {
    params: String,
    body: String,
}

/// `definition` as `proc NAME {params} {body}` defining `command`.
fn definition_parts(command: &str, definition: &str) -> Option<Definition> {
    let source = Lexed::new(definition).ok()?;
    let [only] = source.commands.as_slice() else {
        return None;
    };
    let [keyword, name, params, body] = only.words.as_slice() else {
        return None;
    };
    let literal = |word: &tcl_lexer::script::WordSpan| source.literal(word);
    if literal(keyword)? != "proc" {
        return None;
    }
    let defined = literal(name)?;
    if defined.trim_start_matches("::") != command.trim_start_matches("::") {
        return None;
    }
    if params.kind != WordKind::Braced || body.kind != WordKind::Braced {
        return None;
    }
    Some(Definition {
        params: literal(params)?,
        body: literal(body)?,
    })
}

/// The names of the plain required parameters in `formals`.
fn parameters(formals: &str) -> Result<Vec<String>, Inexpressible> {
    let parsed = parse_formal_parameters(formals).map_err(|_| Inexpressible::NotOneProc)?;
    let last = parsed.len().saturating_sub(1);
    let mut names = Vec::with_capacity(parsed.len());
    for (index, parameter) in parsed.iter().enumerate() {
        if parameter.default.is_some() || (index == last && parameter.name == "args") {
            return Err(Inexpressible::Parameters(parameter.name.clone()));
        }
        names.push(parameter.name.clone());
    }
    Ok(names)
}

/// A lexed script: its tokens and its commands.
struct Lexed<'a> {
    source: &'a str,
    map: SourceMap<'a>,
    tokens: Vec<Token>,
    commands: Vec<CommandSpan>,
}

impl<'a> Lexed<'a> {
    fn new(source: &'a str) -> Result<Self, Inexpressible> {
        // dialect-drift-ok: a reference body is the library's own Tcl, not a
        // document's, and the hook host evaluates it as plain Tcl; the scan
        // reads it the way the host will.
        let config = LexerConfig::default();
        let tokens = Lexer::with_config(source, config)
            .tokenise_all()
            .map_err(|_| Inexpressible::Unreadable)?;
        let commands = group_commands(&tokens, source, config);
        Ok(Self {
            source,
            map: SourceMap::new(source),
            tokens,
            commands,
        })
    }

    /// The source text of `word`.
    fn source_of(&self, word: &tcl_lexer::script::WordSpan) -> &'a str {
        let start = usize::try_from(word.span.start()).unwrap_or(0);
        let end = usize::try_from(word.span.end()).unwrap_or(0);
        self.source.get(start..end).unwrap_or_default()
    }

    /// The fragments of `word`: each token's kind and inner text.
    fn fragments(&self, word: &tcl_lexer::script::WordSpan) -> Vec<(TokenType, &'a str)> {
        self.tokens[word.tokens.clone()]
            .iter()
            .map(|token| (token.kind, self.map.token_text(*token)))
            .collect()
    }

    /// The value of `word` when it has no substitution, no escape and no
    /// expansion: a braced word's content, or a bare or quoted word's text.
    fn literal(&self, word: &tcl_lexer::script::WordSpan) -> Option<String> {
        if word.expand {
            return None;
        }
        let mut text = String::new();
        for (kind, fragment) in self.fragments(word) {
            match kind {
                TokenType::Str => text.push_str(fragment),
                TokenType::Esc if !fragment.contains('\\') => text.push_str(fragment),
                _ => return None,
            }
        }
        Some(text)
    }
}

/// Read `script` — the body of the proc — and return it with a final `return`
/// rewritten, or why it is not expressible.
fn scan_body(script: &str) -> Result<String, Inexpressible> {
    if script.contains(RESULT) {
        return Err(Inexpressible::Reserved);
    }
    let lexed = Lexed::new(script)?;
    let last = lexed.commands.len().checked_sub(1);
    let mut rewritten = script.to_owned();
    for (index, command) in lexed.commands.iter().enumerate() {
        let tail = Some(index) == last;
        check_command(&lexed, command, 0, tail)?;
        if tail && head_word(&lexed, command).as_deref() == Some("return") {
            rewritten = rewrite_return(&lexed, command, script)?;
        }
    }
    Ok(rewritten)
}

/// `script` with its final `return` as a `set` of the result variable.
fn rewrite_return(
    lexed: &Lexed<'_>,
    command: &CommandSpan,
    script: &str,
) -> Result<String, Inexpressible> {
    let first = &command.words[0];
    let start = usize::try_from(first.span.start()).map_err(|_| Inexpressible::Unreadable)?;
    let end = usize::try_from(first.span.end()).map_err(|_| Inexpressible::Unreadable)?;
    match command.words.len() {
        1 => {
            let whole =
                usize::try_from(command.span.end()).map_err(|_| Inexpressible::Unreadable)?;
            Ok(format!(
                "{}set {RESULT} {{}}{}",
                &lexed.source[..start],
                &script[whole.max(end)..]
            ))
        }
        2 if !command.words[1].expand => Ok(format!(
            "{}set {RESULT}{}",
            &script[..start],
            &script[end..]
        )),
        _ => Err(Inexpressible::ReturnOptions),
    }
}

/// The literal command word of `command`.
fn head_word(lexed: &Lexed<'_>, command: &CommandSpan) -> Option<String> {
    lexed.literal(command.words.first()?)
}

/// One command: its head on the whitelist, every substitution in it read, and
/// every script and expression it carries followed.
fn check_command(
    lexed: &Lexed<'_>,
    command: &CommandSpan,
    depth: u32,
    allow_return: bool,
) -> Result<(), Inexpressible> {
    if depth > MAX_DEPTH {
        return Err(Inexpressible::Unreadable);
    }
    let Some(name) = head_word(lexed, command) else {
        return Err(Inexpressible::ComputedCommand);
    };
    if !SANDBOX_WORDS.contains(&name.as_str()) {
        let reach = Reach::of(&name);
        return Err(Inexpressible::Command { word: name, reach });
    }
    if name == "return" && !allow_return {
        return Err(Inexpressible::EarlyReturn);
    }
    for word in &command.words {
        for (kind, text) in lexed.fragments(word) {
            match kind {
                TokenType::Cmd => scan_script(text, depth + 1)?,
                TokenType::Var if text.contains("::") => {
                    return Err(Inexpressible::Variable(text.to_owned()));
                }
                _ => {}
            }
        }
    }
    let args = &command.words[1..];
    for index in variable_words(&name, args, lexed) {
        if let Some(word) = args.get(index)
            && !stays_local(lexed, word)
        {
            return Err(Inexpressible::Variable(lexed.source_of(word).to_owned()));
        }
    }
    match name.as_str() {
        "if" => if_words(lexed, args, depth),
        "while" => {
            expression(lexed, args.first(), depth)?;
            script_word(lexed, args.get(1), depth)
        }
        "for" => {
            script_word(lexed, args.first(), depth)?;
            expression(lexed, args.get(1), depth)?;
            script_word(lexed, args.get(2), depth)?;
            script_word(lexed, args.get(3), depth)
        }
        "foreach" => script_word(lexed, args.last(), depth),
        "switch" => switch_words(lexed, args, depth),
        "expr" => args
            .iter()
            .try_for_each(|word| expression(lexed, Some(word), depth)),
        "dict" => match args.first().and_then(|word| lexed.literal(word)) {
            Some(word) if dict_runs_a_script(&word) => Err(Inexpressible::Script(
                "a `dict` subcommand that runs a body".to_owned(),
            )),
            _ => Ok(()),
        },
        "lsort" | "lsearch" | "regsub" => {
            if takes_a_callback(&name, args, lexed) {
                Err(Inexpressible::Script(format!(
                    "`{name} -command` runs a callback"
                )))
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

/// The `dict` subcommands that run a script of the caller's.
const DICT_SCRIPT_SUBCOMMANDS: &[&str] = &["for", "map", "with", "update", "filter"];

/// Whether `word` is, as the ensemble reads it, a `dict` subcommand that runs a
/// script: the exact name, or any abbreviation that resolves to one or could
/// (`dict fo` is `for`, and `dict f` is ambiguous between a script's and a
/// value's, which no release is read to settle). Every subcommand of every
/// release is a candidate, so an abbreviation one release makes unique and a later
/// one ambiguous is held to the stricter answer.
fn dict_runs_a_script(word: &str) -> bool {
    use crate::abbrev::KeywordMatch;
    let Some(spec) = crate::cache::default_registry().get("dict") else {
        return true;
    };
    match spec.resolve_subcommand_word(word, None, None, None) {
        KeywordMatch::Unique(name) => DICT_SCRIPT_SUBCOMMANDS.contains(&name),
        KeywordMatch::Ambiguous(names) => names
            .iter()
            .any(|name| DICT_SCRIPT_SUBCOMMANDS.contains(name)),
        KeywordMatch::Unknown => false,
    }
}

/// Whether the switches in front of a call's operands include `-command`, as the
/// command's own option table reads them: an abbreviation counts exactly when the
/// real command accepts one, and a switch's value is not a switch.
fn takes_a_callback(
    command: &str,
    args: &[tcl_lexer::script::WordSpan],
    lexed: &Lexed<'_>,
) -> bool {
    let Some(spec) = crate::cache::default_registry().get(command) else {
        return true;
    };
    let words: Vec<String> = args
        .iter()
        .map(|word| lexed.literal(word).unwrap_or_default())
        .collect();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    spec.leading_switch_names(&words).contains(&"-command")
}

/// Read a nested script.
fn scan_script(script: &str, depth: u32) -> Result<(), Inexpressible> {
    let lexed = Lexed::new(script)?;
    for command in &lexed.commands {
        check_command(&lexed, command, depth, false)?;
    }
    Ok(())
}

/// A word that is a script: braced, so its text is its body.
fn script_word(
    lexed: &Lexed<'_>,
    word: Option<&tcl_lexer::script::WordSpan>,
    depth: u32,
) -> Result<(), Inexpressible> {
    let Some(word) = word else {
        return Ok(());
    };
    match (word.kind, lexed.literal(word)) {
        (WordKind::Braced, Some(text)) => scan_script(&text, depth + 1),
        _ => Err(Inexpressible::Script("a body that is computed".to_owned())),
    }
}

/// A word that is an expression. A braced one is read as text here; a bare or
/// quoted one has had its substitutions read as tokens already, and its literal
/// fragments are read as text.
fn expression(
    lexed: &Lexed<'_>,
    word: Option<&tcl_lexer::script::WordSpan>,
    depth: u32,
) -> Result<(), Inexpressible> {
    let Some(word) = word else {
        return Ok(());
    };
    if word.kind == WordKind::Braced {
        return match lexed.literal(word) {
            Some(text) => expression_text(&text, depth),
            None => Ok(()),
        };
    }
    for (kind, fragment) in lexed.fragments(word) {
        if matches!(kind, TokenType::Esc | TokenType::Str) {
            expression_text(fragment, depth)?;
        }
    }
    Ok(())
}

/// The text of an expression: each command substitution in it is a script to
/// read, each variable is a plain local, and each function is one the scan lists.
/// A string in quotes is read for its variables and nothing else, and one in
/// braces is not read.
fn expression_text(text: &str, depth: u32) -> Result<(), Inexpressible> {
    for inner in bracketed(text) {
        scan_script(inner, depth + 1)?;
    }
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut quoted = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => {
                quoted = !quoted;
                index += 1;
            }
            b'{' if !quoted => index = skip_braced(bytes, index),
            b'$' => {
                let (name, next) = variable_name(text, index + 1);
                if name.contains("::") {
                    return Err(Inexpressible::Variable(name.to_owned()));
                }
                index = next;
            }
            byte if !quoted && (byte.is_ascii_alphabetic() || byte == b'_') => {
                let start = index;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                let word = &text[start..index];
                if text[index..].trim_start().starts_with('(') && !MATH_FUNCTIONS.contains(&word) {
                    return Err(Inexpressible::Function(word.to_owned()));
                }
            }
            _ => index += 1,
        }
    }
    Ok(())
}

/// The index just past the brace group that opens at `start`.
fn skip_braced(bytes: &[u8], start: usize) -> usize {
    let mut level = 0_usize;
    let mut index = start;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 1,
            b'{' => level += 1,
            b'}' => {
                level -= 1;
                if level == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
        index += 1;
    }
    bytes.len()
}

/// The variable a `$` at `start - 1` names, and where it ends: `${…}` to its
/// closing brace, otherwise the run of word characters and `:`.
fn variable_name(text: &str, start: usize) -> (&str, usize) {
    let rest = &text[start.min(text.len())..];
    if let Some(inner) = rest.strip_prefix('{') {
        let close = inner.find('}').unwrap_or(inner.len());
        return (&inner[..close], start + 1 + close + 1);
    }
    let length = rest
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
        .count();
    (&rest[..length], start + length)
}

/// The indices, among `args`, of the words `name` takes as variable names.
///
/// What every one of them must be is a plain local ([`stays_local`]): a name
/// written outside the procedure's frame, or one the body computes, is state
/// the sandbox does not share with the code it stands for.
fn variable_words(
    name: &str,
    args: &[tcl_lexer::script::WordSpan],
    lexed: &Lexed<'_>,
) -> Vec<usize> {
    let literal = |index: usize| args.get(index).and_then(|word| lexed.literal(word));
    let count = args.len();
    match name {
        "set" | "incr" | "lappend" => vec![0],
        "lassign" => (1..count).collect(),
        "foreach" => (0..count.saturating_sub(1)).step_by(2).collect(),
        "scan" => (2..count).collect(),
        "regsub" => count.checked_sub(1).into_iter().collect(),
        "regexp" => {
            let mut index = 0;
            while let Some(switch) = literal(index).filter(|word| word.starts_with('-')) {
                index += 1;
                match switch.as_str() {
                    "--" => break,
                    "-start" => index += 1,
                    _ => {}
                }
            }
            (index + 2..count).collect()
        }
        "dict" => match literal(0).as_deref() {
            Some("set" | "unset" | "append" | "incr" | "lappend") => vec![1],
            _ => Vec::new(),
        },
        "binary" if literal(0).as_deref() == Some("scan") => (3..count).collect(),
        "switch" => (0..count)
            .filter(|&index| {
                index > 0
                    && matches!(
                        literal(index - 1).as_deref(),
                        Some("-matchvar" | "-indexvar")
                    )
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether `word` names a variable of the procedure's own frame: it begins with
/// literal text, not a substitution, and that text holds no namespace
/// separator.
fn stays_local(lexed: &Lexed<'_>, word: &tcl_lexer::script::WordSpan) -> bool {
    if word.expand {
        return false;
    }
    match lexed.fragments(word).first() {
        Some(&(kind, first)) => {
            matches!(kind, TokenType::Esc | TokenType::Str) && !first.contains("::")
        }
        None => true,
    }
}

/// The text of each outermost `[…]` in `expression`, brackets balanced and
/// backslash-escaped ones skipped.
fn bracketed(expression: &str) -> Vec<&str> {
    let bytes = expression.as_bytes();
    let mut found = Vec::new();
    let mut level = 0_usize;
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 1,
            b'[' => {
                if level == 0 {
                    start = index + 1;
                }
                level += 1;
            }
            b']' if level > 0 => {
                level -= 1;
                if level == 0 {
                    found.push(&expression[start..index]);
                }
            }
            _ => {}
        }
        index += 1;
    }
    found
}

/// `if cond ?then? body ?elseif cond ?then? body …? ?else body?`.
fn if_words(
    lexed: &Lexed<'_>,
    args: &[tcl_lexer::script::WordSpan],
    depth: u32,
) -> Result<(), Inexpressible> {
    let keyword = |index: usize| args.get(index).and_then(|word| lexed.literal(word));
    let mut index = 0;
    loop {
        expression(lexed, args.get(index), depth)?;
        index += 1;
        if keyword(index).as_deref() == Some("then") {
            index += 1;
        }
        script_word(lexed, args.get(index), depth)?;
        index += 1;
        match keyword(index).as_deref() {
            Some("elseif") => index += 1,
            Some("else") => {
                return script_word(lexed, args.get(index + 1), depth);
            }
            Some(_) => return script_word(lexed, args.get(index), depth),
            None => return Ok(()),
        }
    }
}

/// `switch ?options? string {pattern body …}` or `switch ?options? string
/// pattern body …`.
fn switch_words(
    lexed: &Lexed<'_>,
    args: &[tcl_lexer::script::WordSpan],
    depth: u32,
) -> Result<(), Inexpressible> {
    let mut index = 0;
    while let Some(option) = args.get(index).and_then(|word| lexed.literal(word)) {
        index += 1;
        if option == "--" {
            break;
        }
        if !option.starts_with('-') {
            index -= 1;
            break;
        }
    }
    // The string being matched.
    index += 1;
    let rest = args.get(index..).unwrap_or_default();
    if let [list] = rest {
        let Some(text) = lexed.literal(list) else {
            return Err(Inexpressible::Script(
                "a switch body that is computed".to_owned(),
            ));
        };
        let Ok(elements) = split_list(&text) else {
            return Err(Inexpressible::Unreadable);
        };
        for body in elements.iter().skip(1).step_by(2) {
            if body.as_ref() != "-" {
                scan_script(body, depth + 1)?;
            }
        }
        return Ok(());
    }
    for body in rest.iter().skip(1).step_by(2) {
        if lexed.literal(body).as_deref() != Some("-") {
            script_word(lexed, Some(body), depth)?;
        }
    }
    Ok(())
}

/// The declaration of a hook for `command`, leaked once per command and hook.
fn declared_for(command: &str, hook: &ReferenceHook) -> &'static DeclaredSemantics {
    static MEMO: OnceLock<Mutex<HashMap<(String, String), &'static DeclaredSemantics>>> =
        OnceLock::new();
    let key = (
        command.to_owned(),
        format!("{}\u{0}{}", hook.params.join(" "), hook.body),
    );
    let mut memo = MEMO
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(done) = memo.get(&key) {
        return done;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hook.params.hash(&mut hasher);
    hook.body.hash(&mut hasher);
    let inputs: Vec<DeclaredInput> = (0..hook.params.len())
        .map(|index| DeclaredInput::Operand {
            index,
            exactness: Exactness::Exact,
        })
        .collect();
    let identity = ImplementationIdentity {
        pack: "",
        id: Box::leak(format!("{}.reference", command.trim_start_matches("::")).into_boxed_str()),
        content_hash: hasher.finish(),
    };
    let capability = EvaluatorCapability {
        identity,
        host: HostKind::BoundedTcl,
        target: Needs::NONE,
        inputs: Box::leak(inputs.into_boxed_slice()),
        depends: Box::leak(
            vec![
                ContextDependency::TclProfile,
                ContextDependency::ImplementationIdentity,
            ]
            .into_boxed_slice(),
        ),
        budget: ImplementationBudget::default(),
        completion: CompletionSupport::NormalOnly,
    };
    let declared: &'static DeclaredSemantics = Box::leak(Box::new(DeclaredSemantics {
        scope: Box::leak(command.to_owned().into_boxed_str()),
        structure: DeclaredStructure::default(),
        evaluation: DeclaredEvaluation::Implementation(DeclaredImplementation {
            capability,
            slot: None,
        }),
        option_declines: &[],
    }));
    memo.insert(key, declared);
    derived_identities()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert((identity.id, identity.content_hash));
    declared
}

/// The `(id, content hash)` of every implementation this module has derived.
fn derived_identities() -> &'static Mutex<HashSet<(&'static str, u64)>> {
    static DERIVED: OnceLock<Mutex<HashSet<(&'static str, u64)>>> = OnceLock::new();
    DERIVED.get_or_init(Mutex::default)
}

/// Whether `declared` is an implementation this module derived from a reference
/// body, and not one a pack wrote: what a draft of the command need not carry,
/// because the body it came from does.
#[must_use]
pub fn is_derived(declared: &DeclaredSemantics) -> bool {
    declared.implementation().is_some_and(|implementation| {
        let identity = implementation.capability.identity;
        derived_identities()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&(identity.id, identity.content_hash))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn derived(definition: &str) -> Result<ReferenceHook, Inexpressible> {
        hook_of("vendor::f", definition)
    }

    #[test]
    fn a_pure_value_body_becomes_a_fold_of_its_value() {
        let hook = derived("proc vendor::f {x} {expr {$x * 2}}").expect("expressible");
        assert_eq!(hook.params, vec!["x".to_owned()]);
        assert_eq!(hook.body, "fold [\nexpr {$x * 2}\n]");

        // A final `return V` stands as `set` of V; one without a value as the
        // empty string.
        let hook =
            derived("proc vendor::f {a b} {\n    set s [string cat $a $b]\n    return $s\n}")
                .expect("expressible");
        assert_eq!(hook.params, vec!["a".to_owned(), "b".to_owned()]);
        assert!(hook.body.contains("set __spec_result $s"), "{}", hook.body);
        assert!(!hook.body.contains("return"), "{}", hook.body);
        let empty = derived("proc vendor::f {} {return}").expect("expressible");
        assert!(
            empty.body.contains("set __spec_result {}"),
            "{}",
            empty.body
        );
    }

    #[test]
    fn control_flow_and_substitutions_are_followed() {
        for body in [
            "if {$x > 1} {set y a} elseif {$x < 0} {set y b} else {set y c}; set y",
            "set n 0; foreach i $x {incr n}; set n",
            "switch -- $x {a {set y 1} b {set y 2} default {set y 3}}; set y",
            "set y [expr {[llength $x] + 1}]",
            "for {set i 0} {$i < 3} {incr i} {lappend l $i}; set l",
            "while {$x > 0} {incr x -1}; set x",
            "if {$x > 1} then {set y a} else {set y b}; set y",
            "switch -- $x {a - b {set y 1} default {set y 2}}; set y",
            "switch -- $x a - b {set y 1} default {set y 2}; set y",
        ] {
            assert!(
                derived(&format!("proc vendor::f {{x}} {{{body}}}")).is_ok(),
                "{body}: {:?}",
                derived(&format!("proc vendor::f {{x}} {{{body}}}"))
            );
        }
    }

    /// The scan is a whitelist: a command it does not list is refused whether it
    /// is a known escape, a command it has no opinion on, or one nobody has
    /// written, and each is refused for the reach it names.
    #[test]
    fn a_command_the_scan_does_not_list_derives_nothing() {
        let refused =
            |body: &str| derived(&format!("proc vendor::f {{x}} {{{body}}}")).expect_err(body);
        for (body, word, reach) in [
            ("upvar 1 x y", "upvar", Reach::Frame),
            ("uplevel 1 {set y 1}", "uplevel", Reach::Frame),
            ("global g; set g", "global", Reach::Frame),
            ("variable v; set v", "variable", Reach::Frame),
            ("puts hello", "puts", Reach::Channel),
            ("gets stdin", "gets", Reach::Channel),
            ("exec ls", "exec", Reach::Process),
            ("source /etc/passwd", "source", Reach::Process),
            ("clock seconds", "clock", Reach::World),
            ("info exists x", "info", Reach::World),
            ("file exists $x", "file", Reach::World),
            ("frobnicate $x", "frobnicate", Reach::Unlisted),
            ("string cat a; catch {set y 1}", "catch", Reach::Unlisted),
        ] {
            assert_eq!(
                refused(body),
                Inexpressible::Command {
                    word: word.to_owned(),
                    reach
                },
                "{body}"
            );
        }
        // Reached through a substitution, a condition, or a nested body.
        for body in [
            "set y [clock seconds]",
            "expr {[clock seconds] + 1}",
            "if {$x} {upvar 1 x y}",
            "if {$x} {set y 1} else {puts $x}",
            "if {$x} {set y 1} elseif {[clock seconds]} {set y 2}",
            "foreach i $x {puts $i}",
            "while {$x} {exec ls}",
            "while {[clock seconds]} {set y 1}",
            "for {puts a} {1} {incr i} {set y 1}",
            "for {set i 0} {[clock seconds]} {incr i} {set y 1}",
            "for {set i 0} {1} {puts a} {set y 1}",
            "for {set i 0} {1} {incr i} {puts a}",
            "switch $x {a {exec ls}}",
            "switch $x a {exec ls}",
            "set y \"[info level]\"",
        ] {
            assert!(
                matches!(refused(body), Inexpressible::Command { .. }),
                "{body}: {:?}",
                refused(body)
            );
        }
    }

    /// A subcommand and a switch are read as the command reads them, so an
    /// abbreviation that runs a callback is refused as the exact spelling is, and
    /// one that cannot is not refused for looking like it.
    #[test]
    fn a_callback_is_found_under_any_abbreviation_the_command_accepts() {
        let body = |inner: &str| format!("proc vendor::f {{x}} {{{inner}}}");
        for callback in [
            "dict for {k v} $x {set y $v}",
            "dict fo {k v} $x {set y $v}",
            "dict map {k v} $x {set v}",
            "dict ma {k v} $x {set v}",
            "dict wi x {set y 1}",
            "dict w x {set y 1}",
            "dict update x k v {set y 1}",
            "dict upd x k v {set y 1}",
            "dict fi $x key *",
            // Ambiguous between a script's and a value's: no release is read to settle it.
            "dict f $x key *",
            "dict m {k v} $x {set v}",
            "lsort -command cmp $x",
            "lsort -comm cmp $x",
            "lsort -c cmp $x",
            "lsort -nocase -comm cmp $x",
            "lsort -index 1 -command cmp $x",
        ] {
            assert!(
                matches!(derived(&body(callback)), Err(Inexpressible::Script(_))),
                "{callback}: {:?}",
                derived(&body(callback))
            );
        }
        for plain in [
            "dict get $x k",
            "dict ge $x k",
            "dict mer $x $x",
            "dict v $x",
            "dict k $x",
            "lsort -nocase $x",
            "lsort -d $x",
            "lsort -integer $x",
            // After the switches a word is data, whatever it looks like.
            "lsearch $x -command",
            "lsort -decreasing -index 0 $x",
        ] {
            assert!(
                derived(&body(plain)).is_ok(),
                "{plain}: {:?}",
                derived(&body(plain))
            );
        }
    }

    #[test]
    fn what_the_scan_cannot_read_to_the_end_derives_nothing() {
        let refused =
            |body: &str| derived(&format!("proc vendor::f {{x}} {{{body}}}")).expect_err(body);
        assert_eq!(refused("$x a b"), Inexpressible::ComputedCommand);
        assert_eq!(refused("[set x set] y 1"), Inexpressible::ComputedCommand);
        assert_eq!(refused("{*}$x"), Inexpressible::ComputedCommand);
        assert!(matches!(refused("if {$x} $x"), Inexpressible::Script(_)));
        assert!(matches!(
            refused("dict for {k v} $x {set y $v}"),
            Inexpressible::Script(_)
        ));
        assert!(matches!(
            refused("lsort -command cmp $x"),
            Inexpressible::Script(_)
        ));
        assert_eq!(
            refused("if {$x} {return 1}; return 2"),
            Inexpressible::EarlyReturn
        );
        assert_eq!(refused("return 1; set y 2"), Inexpressible::EarlyReturn);
        assert_eq!(
            refused("return -code error oops"),
            Inexpressible::ReturnOptions
        );
    }

    /// The scan is a whitelist and never a blacklist: of every command the shipped
    /// registry knows, the ones the list does not name are each refused as a
    /// command off the list, whatever they do — so a command the registry gains
    /// later is outside the sandbox until someone lists it — and the ones it
    /// names are not.
    #[test]
    fn every_command_the_registry_knows_is_refused_unless_the_whitelist_lists_it() {
        let registry = crate::CommandRegistry::build_default();
        let mut refused = 0;
        for name in registry.command_names() {
            if SANDBOX_WORDS.contains(&name)
                || name
                    .chars()
                    .any(|c| c.is_whitespace() || "{}[]\"$;\\#".contains(c))
            {
                continue;
            }
            let outcome = derived(&format!("proc vendor::f {{x}} {{{name} $x}}"));
            assert!(
                matches!(&outcome, Err(Inexpressible::Command { word, .. }) if word == name),
                "{name}: {outcome:?}"
            );
            refused += 1;
        }
        assert!(refused > 100, "the registry's own commands: {refused}");
        for word in SANDBOX_WORDS {
            let outcome = derived(&format!("proc vendor::f {{x}} {{{word}}}"));
            assert!(
                !matches!(outcome, Err(Inexpressible::Command { .. })),
                "{word}: {outcome:?}"
            );
        }
    }

    /// A body is a function of its parameters. A variable outside the
    /// procedure's frame, a name the body computes, a random number and an
    /// expression function nobody listed are each state the sandbox does not
    /// share with the code it stands for.
    #[test]
    fn a_body_that_reaches_outside_its_own_frame_through_a_name_derives_nothing() {
        let refused =
            |body: &str| derived(&format!("proc vendor::f {{x}} {{{body}}}")).expect_err(body);
        for body in [
            "set ::g 1",
            "set y $::g",
            "set y $ns::g",
            "set y ${::g}",
            "expr {$::g + 1}",
            "expr {${::g} + 1}",
            "expr $::g",
            "lappend ns::l 1",
            "foreach {a ::b} $x {set a}",
            "regexp -start 1 {a} $x ::m",
            "regsub a $x b ::out",
            "dict set ::d k v",
            "scan $x %d ::n",
            "binary scan $x a ::v",
            "switch -matchvar ::m -regexp -- $x {a {set y 1}}",
            "set $x 1",
            "incr [set x] 1",
            "lassign $x $x",
        ] {
            assert!(
                matches!(refused(body), Inexpressible::Variable(_)),
                "{body}: {:?}",
                refused(body)
            );
        }
        assert_eq!(
            refused("expr {rand()}"),
            Inexpressible::Function("rand".to_owned())
        );
        assert_eq!(
            refused("expr {srand(1) + 1}"),
            Inexpressible::Function("srand".to_owned())
        );
        assert_eq!(
            refused("expr {frob($x)}"),
            Inexpressible::Function("frob".to_owned())
        );
        assert_eq!(
            refused("expr frob($x)"),
            Inexpressible::Function("frob".to_owned())
        );
        assert_eq!(
            refused("if {env($x)} {set y 1}"),
            Inexpressible::Function("env".to_owned())
        );
        assert_eq!(refused("set __spec_result 1"), Inexpressible::Reserved);

        // What is a function of the parameters alone still derives.
        for body in [
            "set counts($x) 1; set counts($x)",
            "expr {max($x, 3) + abs(-1) + int(2.5)}",
            "expr {sqrt($x) > 2 && $x ne \"a(b\" && $x ne {c(d}}",
            "set y [expr {$x(1) + 1}]",
            "foreach {a b} $x {lappend out $b $a}; set out",
            "regexp -nocase -- {(a+)} $x whole sub; set sub",
            "regexp -start 1 -- {(a+)} $x whole sub; set sub",
            "string map {:: /} $x",
        ] {
            assert!(
                derived(&format!("proc vendor::f {{x}} {{{body}}}")).is_ok(),
                "{body}: {:?}",
                derived(&format!("proc vendor::f {{x}} {{{body}}}"))
            );
        }
    }

    #[test]
    fn a_body_nested_past_the_limit_derives_nothing() {
        let nested = |levels: usize| {
            format!(
                "proc vendor::f {{x}} {{{}set y 1{}}}",
                "if {1} {".repeat(levels),
                "}".repeat(levels)
            )
        };
        assert!(hook_of("vendor::f", &nested(20)).is_ok());
        assert_eq!(
            hook_of("vendor::f", &nested(40)),
            Err(Inexpressible::Unreadable)
        );
    }

    #[test]
    fn only_one_proc_with_plain_parameters_defines_the_command() {
        for definition in [
            "proc other::f {x} {set x}",
            "proc vendor::f {x} {set x}\nset y 1",
            "set body {set x}\nproc vendor::f {x} $body",
            "proc vendor::f {x} \"set x\"",
            "foreach vendor::f {x} {set x}",
            "expr {1 + 1}",
        ] {
            assert_eq!(
                derived(definition),
                Err(Inexpressible::NotOneProc),
                "{definition}"
            );
        }
        assert_eq!(
            derived("proc vendor::f {x {y 1}} {set x}"),
            Err(Inexpressible::Parameters("y".to_owned()))
        );
        assert_eq!(
            derived("proc vendor::f {x args} {set x}"),
            Err(Inexpressible::Parameters("args".to_owned()))
        );
        assert!(hook_of("::vendor::f", "proc vendor::f {x} {set x}").is_ok());
    }

    #[test]
    fn a_derived_implementation_is_told_from_one_a_pack_wrote() {
        let (derived, _) = derive("vendor::f", "proc vendor::f {x} {set x}").expect("expressible");
        assert!(is_derived(derived.declared));

        // The host plan binds a slot and names a pack, and it is still the same
        // derivation.
        let slot = crate::pack_hooks::allocate(
            crate::pack_hooks::HookFamily::Evaluate,
            &crate::pack_hooks::HookInputs::declared([crate::pack_hooks::HookInput::Words]),
        )
        .expect("an evaluate slot");
        let bound = derived.declared.bound("vendor", slot);
        assert!(is_derived(&bound));

        // A pack's own implementation of the same id and another body is not,
        // and nor is a declaration with no implementation.
        let mut written = *derived.declared;
        if let DeclaredEvaluation::Implementation(implementation) = &mut written.evaluation {
            implementation.capability.identity.content_hash ^= 1;
        }
        assert!(!is_derived(&written));
        written.evaluation = DeclaredEvaluation::Route(super::super::route::EvalRoute::None {
            reason: super::super::decline::NoRouteReason::Declared,
        });
        assert!(!is_derived(&written));
    }

    #[test]
    fn the_declaration_names_one_input_per_parameter_and_the_text_it_runs() {
        let (implementation, hook) =
            derive("vendor::f", "proc vendor::f {a b} {string cat $a $b}").expect("expressible");
        let declared = implementation.declared;
        assert_eq!(declared.scope, "vendor::f");
        let DeclaredEvaluation::Implementation(declaration) = declared.evaluation else {
            panic!("an implementation");
        };
        assert_eq!(declaration.slot, None);
        assert_eq!(declaration.capability.identity.pack, "");
        assert_eq!(declaration.capability.identity.id, "vendor::f.reference");
        assert_eq!(
            declaration.capability.inputs,
            &[
                DeclaredInput::Operand {
                    index: 0,
                    exactness: Exactness::Exact
                },
                DeclaredInput::Operand {
                    index: 1,
                    exactness: Exactness::Exact
                },
            ]
        );
        assert_eq!(
            declaration.capability.depends,
            &[
                ContextDependency::TclProfile,
                ContextDependency::ImplementationIdentity
            ]
        );
        // The same text is the same declaration, leaked once; another text is
        // another implementation.
        let (again, _) =
            derive("vendor::f", "proc vendor::f {a b} {string cat $a $b}").expect("expressible");
        assert!(std::ptr::eq(again.declared, declared));
        let (other, other_hook) =
            derive("vendor::f", "proc vendor::f {a b} {string cat $b $a}").expect("expressible");
        let DeclaredEvaluation::Implementation(moved) = other.declared.evaluation else {
            panic!("an implementation");
        };
        assert_ne!(
            moved.capability.identity.content_hash,
            declaration.capability.identity.content_hash
        );
        assert_ne!(other_hook, hook);
    }
}
