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

//! The compiled locals of a procedure body, as the emulated release's
//! compiler makes them.
//!
//! A script the release compiles outside a procedure body — an `eval` or
//! `uplevel` script in a procedure's frame — creates no locals of its own:
//! `TclFindCompiledLocal` without a `procPtr` finds only the locals the
//! frame's procedure body made. Whether such a script compiles a `catch`
//! with a result variable turns on that table, so the runtime keeps each
//! procedure frame's body ([`ProcUnit`]) and reads its table lazily.
//!
//! The table is every name a compile procedure of the release makes a local
//! while it compiles the body: each variable a word reads (`$name`), in any
//! word of any command; the variable words of a command the release compiles
//! (`tcl_registry::release_compilation::has_compile_procedure`), by their
//! registry roles; and those of the literal scripts and expressions such a
//! command compiles with it. A compile procedure that declines a shape it
//! cannot compile makes the locals it reached before declining, which the
//! command's hook says.

use std::cell::OnceCell;
use std::collections::HashSet;

use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::TclVersion;
use tcl_lexer::LexerConfig;
use tcl_registry::hooks::{CodegenHookId, LoweringHookId};
use tcl_registry::release_compilation::compile_procedure;
use tcl_registry::{ArgRole, CommandRegistry};

use crate::parse::{self, WordBody, WordPart};

/// A procedure body a frame activates, and the compiled locals its compile
/// makes, read the first time a script compiled in the frame asks.
pub(crate) struct ProcUnit {
    body: Vec<u8>,
    params: Vec<Vec<u8>>,
    locals: OnceCell<HashSet<Vec<u8>>>,
}

/// What the release's compiler is consulted through.
pub(crate) struct Release<'r> {
    pub(crate) registry: &'r CommandRegistry,
    pub(crate) point: Option<SurfaceQuery<'static>>,
    pub(crate) version: TclVersion,
    pub(crate) config: LexerConfig,
}

impl ProcUnit {
    /// The body `body` with the parameters `params`, which are its first
    /// locals.
    pub(crate) fn new(body: &[u8], params: Vec<Vec<u8>>) -> Self {
        Self {
            body: body.to_vec(),
            params,
            locals: OnceCell::new(),
        }
    }

    /// Whether each of `names` is a compiled local of this body.
    pub(crate) fn has_locals(&self, names: &[Vec<u8>], release: &Release<'_>) -> bool {
        let locals = self.locals.get_or_init(|| {
            let mut scan = Scan {
                release,
                locals: self.params.iter().cloned().collect(),
            };
            scan.script(&self.body);
            scan.locals
        });
        names.iter().all(|name| locals.contains(name))
    }
}

struct Scan<'a, 'r> {
    release: &'a Release<'r>,
    locals: HashSet<Vec<u8>>,
}

/// How a compile procedure makes a local of a variable word.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LocalRule {
    /// `TclPushVarName`: a name with no namespace qualifier, an element
    /// naming its array.
    Name,
    /// `IndexTailVarIfKnown`: the tail of a qualified name; a name ending in
    /// `)` stops the compile.
    Tail,
    /// `TclLocalScalarFromToken`: a scalar with no qualifier; a qualified
    /// name stops the compile, and an element names its array and stops it.
    Scalar,
}

/// The elements of the list `text` where each is literal in it, as the
/// parser and `TclCompileSwitchCmd` take a list apart at compile time
/// (`TclFindElement`'s `literal`); `None` where one is not.
fn literal_elements(text: &[u8]) -> Option<Vec<Vec<u8>>> {
    let text = std::str::from_utf8(text).ok()?;
    let mut elements = Vec::new();
    let mut at = 0;
    while let Some(element) = tcl_syntax::list::find_element(text, at).ok()? {
        if !element.literal {
            return None;
        }
        elements.push(text.as_bytes()[element.value].to_vec());
        at = element.next;
    }
    Some(elements)
}

/// Whether `name` is qualified by a namespace.
fn is_qualified(name: &[u8]) -> bool {
    name.windows(2).any(|pair| pair == b"::")
}

impl Scan<'_, '_> {
    fn script(&mut self, src: &[u8]) {
        for command in parse::parse_script_with_config(src, self.release.config) {
            self.command(&command.words);
        }
    }

    fn command(&mut self, words: &[parse::Word<'_>]) {
        for word in words {
            if let WordBody::Parts(parts) = &word.body {
                self.parts(parts);
            }
        }
        // The parser expands a literal `{*}` of literal elements into literal
        // words; a compile procedure cannot take any other expansion.
        let mut texts: Vec<Option<Vec<u8>>> = Vec::new();
        let mut parts: Vec<Option<&[WordPart<'_>]>> = Vec::new();
        for word in words {
            match (&word.body, word.expand) {
                (WordBody::Literal(text), false) => {
                    texts.push(Some(text.to_vec()));
                    parts.push(None);
                }
                (WordBody::Parts(word_parts), false) => {
                    texts.push(None);
                    parts.push(Some(word_parts));
                }
                (WordBody::Literal(text), true) => match literal_elements(text) {
                    Some(elements) => {
                        parts.extend(elements.iter().map(|_| None));
                        texts.extend(elements.into_iter().map(Some));
                    }
                    None => return,
                },
                (WordBody::Parts(_), true) => return,
            }
        }
        let texts: Vec<Option<&str>> = texts
            .iter()
            .map(|text| {
                text.as_deref()
                    .and_then(|text| std::str::from_utf8(text).ok())
            })
            .collect();
        let Some((Some(name), args)) = texts.split_first() else {
            return;
        };
        let release = self.release;
        let Some(spec) = release.registry.get(name) else {
            return;
        };
        let spellings: Vec<&str> = args.iter().map(|arg| arg.unwrap_or("")).collect();
        let Some(call) = release
            .registry
            .resolve_call(name, &spellings, release.point)
        else {
            return;
        };
        let subcommand = if spec.subcommands.is_empty() {
            None
        } else {
            match (args.first(), call.sub) {
                (Some(Some(_)), Some(sub)) => Some(sub.name),
                _ => return,
            }
        };
        let Some(procedure) = compile_procedure(release.version, spec.name, subcommand) else {
            return;
        };
        let roles = |role| {
            release
                .registry
                .arg_indices_for_role(name, &spellings, role)
        };
        match call.lowering_hook {
            Some(LoweringHookId::Try) => return self.try_command(args),
            Some(LoweringHookId::Switch) => return self.switch_command(args),
            Some(LoweringHookId::Upvar) => return self.upvar_command(args),
            _ => {}
        }
        let rule = match (call.lowering_hook, call.codegen_hook) {
            _ if !procedure.binds_variables => None,
            (Some(LoweringHookId::Catch), _) | (_, Some(CodegenHookId::NamespaceUpvar)) => {
                Some(LocalRule::Scalar)
            }
            (Some(LoweringHookId::Global | LoweringHookId::Variable), _) => Some(LocalRule::Tail),
            // `TclCompileArrayUnsetCmd` compiles no pattern.
            (_, Some(CodegenHookId::ArrayUnset)) if args.len() != 2 => None,
            _ => Some(LocalRule::Name),
        };
        // A compile procedure that stops at a variable compiles nothing.
        if let Some(rule) = rule {
            let mut vars: Vec<usize> = roles(ArgRole::VarWrite);
            vars.extend(roles(ArgRole::VarRead));
            vars.sort_unstable();
            vars.dedup();
            for index in vars {
                let made = match (args.get(index), parts.get(index + 1)) {
                    (Some(Some(var)), _) => self.variable(var.as_bytes(), rule),
                    (_, Some(Some(word))) if rule == LocalRule::Name => {
                        self.element_array(word);
                        true
                    }
                    _ => false,
                };
                if !made {
                    return;
                }
            }
        }
        // A `dict` loop (its list followed by the dictionary and the body)
        // binds exactly a key and a value, any other loop at least one name
        // per list, and `dict update` one name per word.
        let lists = roles(ArgRole::LoopVarList);
        let bodies = roles(ArgRole::Body);
        let count: fn(usize) -> bool = match (call.lowering_hook, lists.as_slice()) {
            (Some(LoweringHookId::Dict), &[list]) if bodies == [list + 2] => |count| count == 2,
            _ => |count| count > 0,
        };
        for index in lists {
            let Some(Some(list)) = args.get(index) else {
                return;
            };
            if !self.loop_variables(list.as_bytes(), count) {
                return;
            }
        }
        for script in self.compiled_scripts(&call, args, &roles) {
            self.script(script.as_bytes());
        }
        for index in roles(ArgRole::Expr) {
            if let Some(Some(expr)) = args.get(index) {
                self.substitutions(
                    expr.as_bytes(),
                    tcl_registry::substitution::SubstitutionKinds::ALL,
                );
            }
        }
        if let (Some(kinds), Some(Some(text))) = (
            spec.substitutions_performed(
                tcl_registry::InvocationArguments::literals(&spellings),
                release.point,
            ),
            args.last(),
        ) {
            self.substitutions(text.as_bytes(), kinds);
        }
    }

    /// The literal scripts a compiled command compiles with it: each `Body`
    /// word up to the first one substituted, but those behind a test
    /// `TclCompileIfCmd` or `TclCompileWhileCmd` reads as a constant that
    /// never selects them, and none after one that always does.
    fn compiled_scripts<'w>(
        &self,
        call: &tcl_registry::ResolvedCall<'_>,
        args: &[Option<&'w str>],
        roles: &dyn Fn(ArgRole) -> Vec<usize>,
    ) -> Vec<&'w str> {
        let bodies = roles(ArgRole::Body);
        let constants = matches!(
            call.lowering_hook,
            Some(LoweringHookId::If | LoweringHookId::While)
        );
        let tests = if constants {
            roles(ArgRole::Expr)
        } else {
            Vec::new()
        };
        let numbers = self.release.version.number_syntax();
        let mut compiled = Vec::new();
        let mut previous = None;
        for body in bodies {
            let Some(Some(script)) = args.get(body) else {
                break;
            };
            let test = tests
                .iter()
                .rev()
                .find(|&&test| test < body && previous.is_none_or(|previous| test > previous))
                .and_then(|&test| args.get(test).copied().flatten())
                .map(|test| tcl_syntax::boolean::truthiness_with(test, numbers));
            previous = Some(body);
            match test {
                Some(Some(false)) => {}
                Some(Some(true)) => {
                    compiled.push(*script);
                    break;
                }
                _ => compiled.push(*script),
            }
        }
        compiled
    }

    /// `TclCompileTryCmd`: each handler's literal `on` or `trap` keyword, a
    /// known code or a non-empty `trap` list, at most two variables, each a
    /// local scalar made as it is read, and a literal script; a literal
    /// `finally` script. The scripts compile only where every clause does.
    fn try_command(&mut self, args: &[Option<&str>]) {
        let Some((Some(body), clauses)) = args.split_first() else {
            return;
        };
        let mut scripts = vec![*body];
        let mut rest = clauses;
        while let [Some(keyword), ..] = rest {
            match (*keyword, rest) {
                ("finally", [_, Some(script)]) => {
                    scripts.push(script);
                    rest = &[];
                }
                ("on" | "trap", [_, Some(selector), Some(vars), Some(script), tail @ ..]) => {
                    let known = *keyword == "on"
                        || parse::split_list(selector.as_bytes())
                            .is_ok_and(|codes| !codes.is_empty());
                    if !known || !self.loop_variables(vars.as_bytes(), |count| count <= 2) {
                        return;
                    }
                    if *script != "-" {
                        scripts.push(script);
                    }
                    rest = tail;
                }
                _ => return,
            }
        }
        if !rest.is_empty() {
            return;
        }
        for script in scripts {
            self.script(script.as_bytes());
        }
    }

    /// `TclCompileSwitchCmd`: a value and a literal pattern list, or a
    /// literal `--` ending the options before a value and literal
    /// pattern-body words, as many as the patterns; its bodies compile with
    /// it.
    fn switch_command(&mut self, args: &[Option<&str>]) {
        let operands = if args.len() == 2 {
            &args[1..]
        } else {
            let (mut mode, mut nocase) = (None, false);
            let mut at = 0;
            loop {
                let option = match args.get(at).copied().flatten() {
                    Some(option) if option.len() >= 2 => option,
                    _ => return,
                };
                if option == "--" {
                    break;
                }
                if "-nocase".starts_with(option) {
                    nocase = true;
                } else if let Some(flag) = ["-exact", "-glob", "-regexp"]
                    .into_iter()
                    .find(|flag| flag.starts_with(option))
                {
                    if mode.replace(flag).is_some() {
                        return;
                    }
                } else {
                    return;
                }
                at += 1;
            }
            // No opcode compares exactly without case.
            if nocase && mode.is_none_or(|mode| mode == "-exact") {
                return;
            }
            &args[at + 2..]
        };
        let words: Vec<Vec<u8>> = match operands {
            [Some(list)] => match literal_elements(list.as_bytes()) {
                Some(elements) => elements,
                None => return,
            },
            _ if operands.iter().all(Option::is_some) => operands
                .iter()
                .flatten()
                .map(|word| word.as_bytes().to_vec())
                .collect(),
            _ => return,
        };
        if words.is_empty() || words.len() % 2 != 0 || words.last().is_some_and(|body| body == b"-")
        {
            return;
        }
        for body in words.iter().skip(1).step_by(2) {
            if body != b"-" {
                self.script(body);
            }
        }
    }

    /// `TclCompileUpvarCmd`: a literal level word, or a first word that is
    /// none, then each other variable and local name, the local a scalar.
    fn upvar_command(&mut self, args: &[Option<&str>]) {
        let Some(Some(first)) = args.first() else {
            return;
        };
        let digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
        let level = digits(first.strip_prefix('#').unwrap_or(first));
        let pairs = &args[usize::from(level)..];
        if pairs.len() % 2 != 0 {
            return;
        }
        for pair in pairs.chunks(2) {
            let Some(local) = pair[1] else {
                return;
            };
            if !self.variable(local.as_bytes(), LocalRule::Scalar) {
                return;
            }
        }
    }

    /// The array a variable word with a literal name and a substituted
    /// element names (`TclPushVarName`): `r($i)`.
    fn element_array(&mut self, parts: &[WordPart<'_>]) {
        let (Some(WordPart::Text(head)), Some(WordPart::Text(tail))) =
            (parts.first(), parts.last())
        else {
            return;
        };
        let Some(open) = head.iter().position(|&b| b == b'(') else {
            return;
        };
        let name = &head[..open];
        if !name.is_empty() && tail.ends_with(b")") && !is_qualified(name) {
            self.locals.insert(name.to_vec());
        }
    }

    /// Make a local of the variable word `name` by `rule`; `false` where
    /// the compile procedure stops there.
    fn variable(&mut self, name: &[u8], rule: LocalRule) -> bool {
        match rule {
            LocalRule::Tail => {
                if name.last() == Some(&b')') {
                    return false;
                }
                let tail = name
                    .windows(2)
                    .rposition(|pair| pair == b"::")
                    .map_or(name, |at| &name[at + 2..]);
                if tail.is_empty() {
                    return false;
                }
                self.locals.insert(tail.to_vec());
                true
            }
            LocalRule::Name | LocalRule::Scalar => {
                if is_qualified(name) {
                    return rule == LocalRule::Name;
                }
                let (base, element) = crate::frame::split_array_ref(name);
                self.locals.insert(base.clone());
                rule == LocalRule::Name || element.is_none()
            }
        }
    }

    /// A loop's or handler's variable list of as many names as `count`
    /// accepts, each a local scalar made as it is read (`TclLocalScalar`);
    /// `false` where the compile procedure stops: another count, or a name
    /// that is no scalar.
    fn loop_variables(&mut self, list: &[u8], count: fn(usize) -> bool) -> bool {
        let Ok(names) = parse::split_list(list) else {
            return false;
        };
        if !count(names.len()) {
            return false;
        }
        names
            .iter()
            .all(|name| self.variable(name, LocalRule::Scalar))
    }

    /// The variables a literal word a compile procedure substitutes reads
    /// and its command substitutions, as `kinds` says it substitutes them.
    fn substitutions(&mut self, text: &[u8], kinds: tcl_registry::substitution::SubstitutionKinds) {
        if let WordBody::Parts(parts) = parse::scan_parts(
            text,
            kinds.variables,
            kinds.commands,
            false,
            self.release.config,
        ) {
            self.parts(&parts);
        }
    }

    fn parts(&mut self, parts: &[WordPart<'_>]) {
        for part in parts {
            match part {
                WordPart::Variable(var) => {
                    if !is_qualified(var.name) {
                        self.locals.insert(var.name.to_vec());
                    }
                    if let Some(index) = &var.index {
                        self.parts(index);
                    }
                }
                WordPart::Command(script) => self.script(script),
                WordPart::Text(_) | WordPart::ParseError(_) => {}
            }
        }
    }
}
