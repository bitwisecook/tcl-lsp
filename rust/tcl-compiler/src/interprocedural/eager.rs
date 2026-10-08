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

//! When the module's load may first run each of its procedures, and where
//! each one's `proc` statement surely runs — the one fact the instance
//! lifecycle proof ([`super::global_instance_classes`]) and the completion
//! proof (D330, D348) read.
//!
//! The load is the top-level script in source order with the bodies it runs
//! where it runs them: a `namespace eval` or `apply` body, and a method body,
//! taken at its own place, which no call to it can precede. A statement may
//! run procedures of the module by name — its head, the commands its words
//! substitute, and a script or command prefix its command runs before it
//! returns (a registry [`ScriptTiming::SameInvocation`] position) — and a
//! procedure may run from the first statement that names it, or from the
//! first run of a procedure that names it. A callback a command stores runs
//! after the load. A statement may also run a procedure no name shows: a
//! computed head, a computed script run at once, a word the walk cannot
//! read, or a sourced file; from the first such statement, or the first run
//! of a procedure whose body holds one, any procedure may run.

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;
use tcl_registry::{ArgRole, CommandRegistry, ScriptTiming, Traits};

use crate::depth_guard::MAX_BRACKET_TEXT_DEPTH;
use crate::ir::{CommandTokens, Module, Script, Statement, WordExpr, WordPart};
use crate::ir_helpers::CommandWord;

/// Where the load may first run each procedure of the module.
#[derive(Debug, Clone, Default)]
pub(crate) struct EagerInvocations {
    /// Each procedure the load may run by name, with the position of the
    /// first statement that may run it.
    named: HashMap<String, u32>,
    /// The first position from which the load may run a procedure no name
    /// shows.
    unnamed_from: Option<u32>,
}

impl EagerInvocations {
    /// The fact for `ir_module`, its commands read from `registry`.
    #[must_use]
    pub(crate) fn of(ir_module: &Module, registry: &CommandRegistry) -> Self {
        let known: HashSet<String> = ir_module.procedures.keys().cloned().collect();
        let units: Vec<Span> = ir_module
            .body_units
            .values()
            .map(|unit| unit.span)
            .collect();
        let walk = Walk {
            registry,
            config: crate::dynamic_names::lexer_config_for(registry),
            known: &known,
            units: &units,
        };
        let owner = |span: Span| -> Option<&str> {
            ir_module
                .procedures
                .iter()
                .filter(|(_, procedure)| encloses(procedure.span, span))
                .min_by_key(|(_, procedure)| procedure.span.end() - procedure.span.start())
                .map(|(qname, _)| qname.as_str())
        };

        let mut load = Self::default();
        let mut at = |stmt: &Statement, caller: &str| {
            let mut reach = Reach::default();
            walk.statement(stmt, caller, &mut reach);
            load.note(stmt.span().start(), reach);
        };
        crate::ir::for_each_statement(&ir_module.top_level, &mut |stmt| at(stmt, "::top"));
        for (qname, unit) in &ir_module.body_units {
            if owner(unit.span).is_none() {
                crate::ir::for_each_statement(&unit.body, &mut |stmt| at(stmt, qname));
            }
        }
        for method in ir_module.methods.values() {
            crate::ir::for_each_statement(&method.body, &mut |stmt| at(stmt, "::top"));
        }

        // What each procedure's own runs reach: its body and the bodies
        // lowering registered inside it.
        let mut bodies: HashMap<&str, Reach> = HashMap::new();
        for (qname, procedure) in &ir_module.procedures {
            let reach = bodies.entry(qname.as_str()).or_default();
            crate::ir::for_each_statement(&procedure.body, &mut |stmt| {
                walk.statement(stmt, qname, reach);
            });
        }
        for (qname, unit) in &ir_module.body_units {
            if let Some(owner) = owner(unit.span) {
                let reach = bodies.entry(owner).or_default();
                crate::ir::for_each_statement(&unit.body, &mut |stmt| {
                    walk.statement(stmt, qname, reach);
                });
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for (caller, reach) in &bodies {
                let Some(position) = load.named.get(*caller).copied() else {
                    continue;
                };
                for callee in &reach.named {
                    if load.named.get(callee).is_none_or(|old| position < *old) {
                        load.named.insert(callee.clone(), position);
                        changed = true;
                    }
                }
            }
        }
        for (caller, reach) in &bodies {
            if reach.unnamed
                && let Some(position) = load.named.get(*caller).copied()
            {
                load.unnamed_from =
                    Some(load.unnamed_from.map_or(position, |old| old.min(position)));
            }
        }
        load
    }

    /// Record what the statement at `position` may run.
    fn note(&mut self, position: u32, reach: Reach) {
        for target in reach.named {
            self.named
                .entry(target)
                .and_modify(|old| *old = (*old).min(position))
                .or_insert(position);
        }
        if reach.unnamed {
            self.unnamed_from = Some(self.unnamed_from.map_or(position, |old| old.min(position)));
        }
    }

    /// The first position at which the load may run `qname` by name, or
    /// `None` where only a callback runs it — the instance lifecycle proof's
    /// reading.
    #[must_use]
    pub(crate) fn named(&self, qname: &str) -> Option<u32> {
        self.named.get(qname).copied()
    }

    /// The first position at which the load may run `qname` at all: by name,
    /// or from the first statement that may run a procedure no name shows.
    /// `None` where only a callback runs it, after the load.
    #[must_use]
    pub(crate) fn may_run_from(&self, qname: &str) -> Option<u32> {
        match (self.named(qname), self.unnamed_from) {
            (Some(named), Some(unnamed)) => Some(named.min(unnamed)),
            (named, unnamed) => named.or(unnamed),
        }
    }
}

/// The definition reach of the module's procedures (D348): where each one's
/// `proc` statement surely runs, and where the load may first run each.
pub(crate) struct DefinitionReach {
    invocations: EagerInvocations,
    points: HashMap<String, u32>,
}

impl DefinitionReach {
    /// The reach for `ir_module`, its commands read from `registry`.
    #[must_use]
    pub(crate) fn of(ir_module: &Module, registry: &CommandRegistry) -> Self {
        Self {
            invocations: EagerInvocations::of(ir_module, registry),
            points: definition_points(ir_module),
        }
    }

    /// Where the load may first run each procedure.
    #[must_use]
    pub(crate) const fn invocations(&self) -> &EagerInvocations {
        &self.invocations
    }

    /// Whether `procedure`'s `proc` statement surely ran before the load may
    /// first run `runner`; a runner only a callback runs takes every
    /// definition the load surely runs.
    #[must_use]
    pub(crate) fn defined(&self, procedure: &str, runner: &str) -> bool {
        self.points.get(procedure).is_some_and(|&at| {
            self.invocations
                .may_run_from(runner)
                .is_none_or(|from| at < from)
        })
    }

    /// State each summary's definition reach: where its definition runs, and
    /// which of its callees are defined before it may first run.
    pub(crate) fn state(&self, procedures: &mut HashMap<String, super::ProcSummary>) {
        for (qname, summary) in procedures {
            summary.defined_at = self.points.get(qname).copied();
            summary.defined_callees = summary
                .direct_calls
                .iter()
                .filter(|callee| self.defined(callee, qname))
                .cloned()
                .collect();
        }
    }
}

/// Where each procedure's `proc` statement surely runs at load: a direct
/// statement of the top level, or of a `namespace eval` body that is one,
/// before any `return` that may end the script or body around it.
#[must_use]
pub(crate) fn definition_points(ir_module: &Module) -> HashMap<String, u32> {
    let by_span: HashMap<Span, &str> = ir_module
        .procedures
        .iter()
        .map(|(qname, procedure)| (procedure.span, qname.as_str()))
        .collect();
    let mut points = HashMap::new();
    let top = direct_definitions(&ir_module.top_level, &by_span, &mut points);
    for statement in &ir_module.top_level.statements {
        let span = statement.span();
        if top.is_some_and(|end| span.start() >= end) {
            break;
        }
        if !matches!(statement, Statement::Barrier { .. }) {
            continue;
        }
        // The body a `namespace eval` runs is the widest unit the statement
        // holds; an `apply` lambda's is no `namespace eval` of the top level.
        let body = ir_module
            .body_units
            .iter()
            .filter(|(_, unit)| encloses(span, unit.span))
            .max_by_key(|(_, unit)| unit.span.end() - unit.span.start());
        if let Some((qname, unit)) = body
            && !ir_module.lambda_body_units.contains(qname)
        {
            direct_definitions(&unit.body, &by_span, &mut points);
        }
    }
    points
}

/// Record each procedure `script` defines by a direct statement before its
/// first `return`, and answer that `return`'s position.
fn direct_definitions(
    script: &Script,
    by_span: &HashMap<Span, &str>,
    points: &mut HashMap<String, u32>,
) -> Option<u32> {
    let mut end: Option<u32> = None;
    crate::ir::for_each_statement(script, &mut |statement| {
        if matches!(statement, Statement::Return { .. }) {
            let position = statement.span().start();
            end = Some(end.map_or(position, |old| old.min(position)));
        }
    });
    for statement in &script.statements {
        let span = statement.span();
        if end.is_some_and(|end| span.start() >= end) {
            break;
        }
        if let Some(qname) = by_span.get(&span) {
            points.insert((*qname).to_owned(), span.start());
        }
    }
    end
}

/// Whether `outer` holds `inner`.
fn encloses(outer: Span, inner: Span) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end() && outer != inner
}

/// What a statement may run: the procedures it names, and whether it may run
/// one no name shows.
#[derive(Debug, Default)]
struct Reach {
    named: HashSet<String>,
    unnamed: bool,
}

/// The walk over what a statement runs before it completes.
struct Walk<'a> {
    registry: &'a CommandRegistry,
    config: tcl_lexer::LexerConfig,
    known: &'a HashSet<String>,
    /// The bodies lowering registered as units of their own, whose
    /// statements the fact visits in place of the words that hold them.
    units: &'a [Span],
}

/// One command a statement runs, as the walk reads it.
struct Invocation<'w> {
    /// The head as written, which names a procedure of the module.
    head: &'w str,
    /// The command the head resolves to, which the registry describes.
    command: &'w str,
    args: &'w [String],
    /// Whether each argument is literal text.
    literal: &'w [bool],
    /// Whether an argument expands, so no position can be read.
    expands: bool,
    /// Whether a unit of its own holds the code its words carry.
    held: bool,
}

impl Walk<'_> {
    /// What `stmt`, run in `caller`, may run: its own command and the
    /// commands its words substitute. The statements nested in it are
    /// visited apart.
    fn statement(&self, stmt: &Statement, caller: &str, reach: &mut Reach) {
        let substitutions = crate::ir_helpers::evaluated_command_substitutions(stmt, self.registry);
        reach.unnamed |= substitutions.opaque;
        for words in substitutions.all_commands() {
            self.recovered(words, caller, reach);
        }
        let (Statement::Call {
            command,
            canonical_command,
            args,
            tokens,
            span,
            ..
        }
        | Statement::Barrier {
            command,
            canonical_command,
            args,
            tokens,
            span,
            ..
        }) = stmt
        else {
            return;
        };
        let words = tokens.as_ref().map(|tokens| tokens.word_exprs.as_slice());
        if words
            .and_then(<[WordExpr]>::first)
            .is_some_and(|head| !is_literal(head))
        {
            reach.unnamed = true;
            return;
        }
        let literal: Vec<bool> = words.map_or_else(
            || vec![false; args.len()],
            |words| words.iter().skip(1).map(is_literal).collect(),
        );
        let held = matches!(stmt, Statement::Barrier { .. })
            && self.units.iter().any(|unit| encloses(*span, *unit));
        self.invocation(
            &Invocation {
                head: command,
                command: canonical_command.as_deref().unwrap_or(command),
                args,
                literal: &literal,
                expands: words.is_some_and(|words| {
                    words
                        .iter()
                        .any(|word| matches!(word, WordExpr::Expand { .. }))
                }),
                held,
            },
            caller,
            0,
            reach,
        );
    }

    /// A command a substitution runs.
    fn recovered(&self, words: &[CommandWord], caller: &str, reach: &mut Reach) {
        let Some((head, args)) = words.split_first() else {
            return;
        };
        if head.substituted || head.expanded {
            reach.unnamed = true;
            return;
        }
        let texts: Vec<String> = args.iter().map(|word| word.text.clone()).collect();
        let literal: Vec<bool> = args.iter().map(|word| !word.substituted).collect();
        self.invocation(
            &Invocation {
                head: &head.text,
                command: &head.text,
                args: &texts,
                literal: &literal,
                expands: args.iter().any(|word| word.expanded),
                held: false,
            },
            caller,
            0,
            reach,
        );
    }

    /// What one command runs before it returns: the procedure it names, the
    /// procedure an invoker names, a sourced file, and each script or command
    /// prefix it runs at once.
    fn invocation(&self, call: &Invocation<'_>, caller: &str, depth: u32, reach: &mut Reach) {
        if let Some(target) = super::resolve_internal_call(call.head, caller, self.known) {
            reach.named.insert(target);
        }
        let args: Vec<&str> = call.args.iter().map(String::as_str).collect();
        let traits = self.registry.invocation_traits(call.command, &args, None);
        if traits.contains(Traits::SOURCES_FILE) {
            reach.unnamed = true;
        }
        if traits.contains(Traits::INVOKES_USER_PROC)
            && let Some(target) = call
                .args
                .first()
                .and_then(|name| super::resolve_internal_call(name, caller, self.known))
        {
            reach.named.insert(target);
        }
        if call.held
            || traits.intersects(Traits::DEFINES_PROCEDURE.union(Traits::BODY_RUNS_IN_OWN_FRAME))
        {
            return;
        }
        for role in [
            ArgRole::Body,
            ArgRole::CommandPrefix,
            ArgRole::LambdaLiteral,
        ] {
            for index in self
                .registry
                .arg_indices_for_role(call.command, &args, role)
            {
                if call.expands {
                    reach.unnamed = true;
                    return;
                }
                if self
                    .registry
                    .script_timing(call.command, &args, index, None)
                    != Some(ScriptTiming::SameInvocation)
                {
                    continue;
                }
                match call.args.get(index) {
                    Some(text)
                        if role != ArgRole::LambdaLiteral
                            && call.literal.get(index).copied().unwrap_or(false) =>
                    {
                        self.script(text, caller, depth + 1, reach);
                    }
                    _ => reach.unnamed = true,
                }
            }
        }
    }

    /// What a script run at once runs, command by command.
    fn script(&self, text: &str, caller: &str, depth: u32, reach: &mut Reach) {
        if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            reach.unnamed = true;
            return;
        }
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(text, 0, self.config);
        let map = tcl_lexer::SourceMap::new(text);
        for command in &commands {
            if command.is_partial {
                reach.unnamed = true;
                continue;
            }
            let tokens = CommandTokens::from_segmented(&map, self.config, command);
            self.words(&tokens.word_exprs, caller, depth, reach);
        }
    }

    /// One command of a script: the commands its words substitute, then the
    /// command itself.
    fn words(&self, words: &[WordExpr], caller: &str, depth: u32, reach: &mut Reach) {
        for word in words {
            self.substitutions(word, caller, depth, reach);
        }
        let Some((head, args)) = words.split_first() else {
            return;
        };
        let (WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. }) = head else {
            reach.unnamed = true;
            return;
        };
        let texts: Vec<String> = args.iter().map(WordExpr::legacy_text).collect();
        let literal: Vec<bool> = args.iter().map(is_literal).collect();
        self.invocation(
            &Invocation {
                head: text,
                command: text,
                args: &texts,
                literal: &literal,
                expands: args
                    .iter()
                    .any(|word| matches!(word, WordExpr::Expand { .. })),
                held: false,
            },
            caller,
            depth,
            reach,
        );
    }

    /// The commands `word` substitutes.
    fn substitutions(&self, word: &WordExpr, caller: &str, depth: u32, reach: &mut Reach) {
        match word {
            WordExpr::CommandSubstitution { spelling, .. } => {
                self.bracket(spelling, caller, depth, reach);
            }
            WordExpr::Template { parts, .. } => {
                for part in parts {
                    match part {
                        WordPart::CommandSubstitution { spelling, .. } => {
                            self.bracket(spelling, caller, depth, reach);
                        }
                        WordPart::Opaque { .. } => reach.unnamed = true,
                        WordPart::Text { .. } | WordPart::Variable { .. } => {}
                    }
                }
            }
            WordExpr::Expand { word, .. } => self.substitutions(word, caller, depth, reach),
            WordExpr::Opaque { .. } => reach.unnamed = true,
            WordExpr::Literal { .. }
            | WordExpr::BracedLiteral { .. }
            | WordExpr::Variable { .. } => {}
        }
    }

    /// The script a `[…]` substitution runs.
    fn bracket(&self, spelling: &str, caller: &str, depth: u32, reach: &mut Reach) {
        match spelling
            .strip_prefix('[')
            .and_then(|inner| inner.strip_suffix(']'))
        {
            Some(inner) => self.script(inner, caller, depth + 1, reach),
            None => reach.unnamed = true,
        }
    }
}

/// Whether `word` is literal text.
const fn is_literal(word: &WordExpr) -> bool {
    matches!(
        word,
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
    )
}
