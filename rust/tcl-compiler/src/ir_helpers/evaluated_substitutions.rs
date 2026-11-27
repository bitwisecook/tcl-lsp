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

//! The commands the `[…]` substitutions of one statement or expression run: the inventory, its entry points and the walk that recovers them in evaluation order.

use super::command_words::{CommandWord, tokenise_command_words};
use super::substitution_surfaces::{
    collect_expr_command_surface_refs, evaluated_command_substitution_surfaces, unbraced_words,
};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use tcl_lexer::{LexerConfig, SourceMap, TokenType};
use tcl_registry::{ArgRole, CommandRegistry, InvocationWord, InvocationWords, Traits};
use tcl_source::limits::MAX_BRACKET_TEXT_DEPTH;

/// Where one recovered command sits in [`EvaluatedCommandSubstitutions`]'
/// inventories, in the order the walk met it.
#[derive(Clone, Copy)]
pub(super) enum CommandSubstitutionIndex {
    /// An index into [`EvaluatedCommandSubstitutions::commands`].
    Ordinary(usize),
    /// An index into [`EvaluatedCommandSubstitutions::in_frame_commands`].
    InFrame(usize),
}

/// Registry/dialect-shaped command invocations recovered from every evaluated
/// `[...]` surface owned by one statement.
#[derive(Default)]
pub(crate) struct EvaluatedCommandSubstitutions {
    /// Commands in evaluation order within each recovered bracket script.
    pub commands: Vec<Vec<CommandWord>>,
    /// Commands reached only by descending into a brace-quoted word the
    /// callee evaluates **in this frame**: an expression — the `[incr x]` of
    /// `puts [expr {$x + [incr x]}]` — or a protected script, the body of a
    /// `catch` — the `incr x` of `[catch {incr x}]`.
    ///
    /// Kept apart from [`Self::commands`] rather than merged into it because
    /// the two answer different questions. A consumer asking *what cells does
    /// this statement write* wants both: the write is real either way (#2141).
    /// A consumer asking *which procedures does this body call* — the
    /// global-write summary's call graph — reads `commands` alone, because a
    /// recursive callee's summary is opaque and folding that opacity in here
    /// would put a frame barrier on `return [expr {[fib $n] + 1}]` that the
    /// unbraced `return [fib $n]` earns for a reason unrelated to this word.
    /// Closing that second gap means fixing the recursion summary first; the
    /// two are separate, and this split says which is which.
    pub in_frame_commands: Vec<Vec<CommandWord>>,
    /// The commands of [`Self::in_frame_commands`] reached inside a protected
    /// script — the body of a `catch` or a `try` — with the substitutions and
    /// expression words in it. Each runs only where every command before it
    /// in the script completed normally, and the error that stops the rest
    /// is absorbed, not raised: a write among them may not happen, and the
    /// place keeps what it held where the script stopped first.
    pub protected_commands: Vec<Vec<CommandWord>>,
    /// The brace-quoted words themselves — the text of each `{$x + [incr x]}`
    /// or `{incr x; puts $y}` a recovered command evaluates in this frame,
    /// which is where the variable reads of that word sit: no command word of
    /// [`Self::in_frame_commands`] holds them.
    pub in_frame_texts: Vec<String>,
    /// The names a loop the substitutions run binds per iteration (`a` of
    /// `[foreach a {x} {}]`). A loop over nothing binds none, so each is a
    /// possible write the statement also reads.
    pub may_bound_names: Vec<String>,
    /// Indices into both inventories in the order the shared walk evaluates
    /// them. Keeping the classifications separate must not move an earlier
    /// expression's nested invocation after a later sibling substitution.
    pub(super) evaluation_order: Vec<CommandSubstitutionIndex>,
    /// A malformed fragment, a recursion-limit hit, or a protected script that
    /// is run-time data (`[catch $script]`) prevented complete recovery.
    pub opaque: bool,
}

impl EvaluatedCommandSubstitutions {
    /// Record `words`, reached as `reach` says, after every command the walk
    /// met before it.
    pub(super) fn record(&mut self, reach: Reach, words: Vec<CommandWord>) {
        if reach == Reach::Statement {
            self.evaluation_order
                .push(CommandSubstitutionIndex::Ordinary(self.commands.len()));
            self.commands.push(words);
            return;
        }
        if reach == Reach::Protected {
            self.protected_commands.push(words.clone());
        }
        self.evaluation_order
            .push(CommandSubstitutionIndex::InFrame(
                self.in_frame_commands.len(),
            ));
        self.in_frame_commands.push(words);
    }

    /// Every command the statement may run, whether or not it took an in-frame
    /// word to reach, in Tcl evaluation order. Variable-effect and
    /// binding-transition consumers use this view; call-graph consumers may
    /// still select the ordinary inventory without changing its classification.
    pub(crate) fn all_commands(&self) -> impl Iterator<Item = &Vec<CommandWord>> {
        self.evaluation_order.iter().map(|index| match *index {
            CommandSubstitutionIndex::Ordinary(index) => &self.commands[index],
            CommandSubstitutionIndex::InFrame(index) => &self.in_frame_commands[index],
        })
    }

    /// [`Self::all_commands`] without the ordinary commands `keep` rejects;
    /// in-frame commands are always kept.
    pub(crate) fn all_commands_keeping<'s>(
        &'s self,
        keep: impl Fn(&[CommandWord]) -> bool + 's,
    ) -> impl Iterator<Item = &'s Vec<CommandWord>> {
        self.evaluation_order
            .iter()
            .filter_map(move |index| match *index {
                CommandSubstitutionIndex::Ordinary(index) => {
                    Some(&self.commands[index]).filter(|words| keep(words))
                }
                CommandSubstitutionIndex::InFrame(index) => Some(&self.in_frame_commands[index]),
            })
    }
}

/// Recover the commands executed by `[...]` substitutions in `stmt` using the
/// exact lexer grammar selected for its registry profile.
///
/// This is effect analysis, so failure widens through [`Self::opaque`] rather
/// than being interpreted as an empty command list.  Dynamic/substituted heads
/// remain represented by [`CommandWord::substituted`]; consumers decide which
/// state domain that unknown identity invalidates.
#[must_use]
pub(crate) fn evaluated_command_substitutions(
    stmt: &Statement,
    registry: &CommandRegistry,
) -> EvaluatedCommandSubstitutions {
    evaluated_command_substitutions_with_heads(stmt, registry, None)
}

/// [`evaluated_command_substitutions`] with a caller-supplied head resolver.
///
/// A caller holding the module's command bindings can answer *which command a
/// recovered head actually reaches*, which the registry alone cannot: after
/// `interp alias {} e {} expr`, `[e {$x + [incr x]}]` runs `expr` on a word
/// the raw spelling `e` has no role for, and the `[incr x]` in it was missed.
/// Callers without bindings pass `None` and get the raw spelling, as before.
#[must_use]
pub(crate) fn evaluated_command_substitutions_with_heads(
    stmt: &Statement,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
) -> EvaluatedCommandSubstitutions {
    evaluated_command_substitutions_with_replay(stmt, registry, heads, None)
}

/// Recover invocations while replaying each completed command before the next
/// head's expression roles are queried. Both callbacks share the caller's
/// binding state, so earlier alias transitions affect later descent.
#[must_use]
pub(crate) fn evaluated_command_substitutions_with_replay(
    stmt: &Statement,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    observe: Option<EmbeddedCommandObserver<'_>>,
) -> EvaluatedCommandSubstitutions {
    let surfaces = evaluated_command_substitution_surfaces(stmt, registry);
    command_substitutions_in_surfaces_with_replay(
        &surfaces.texts,
        surfaces.opaque,
        surfaces.conditional,
        registry,
        heads,
        observe,
    )
}

/// Recover source substitutions with the actual availability and lexer input.
/// Missing or foreign metadata is opaque; no entered script/frame is inferred.
#[must_use]
pub(crate) fn evaluated_command_substitutions_with_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: LexerConfig,
) -> EvaluatedCommandSubstitutions {
    evaluated_command_substitutions_with_heads_and_metadata_context(
        stmt, registry, None, metadata, config,
    )
}

/// The same source inventory with a genuine module target/prefix resolver.
#[must_use]
pub(crate) fn evaluated_command_substitutions_with_heads_and_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: LexerConfig,
) -> EvaluatedCommandSubstitutions {
    evaluated_command_substitutions_with_replay_and_metadata_context(
        stmt, registry, heads, None, metadata, config,
    )
}

/// Replay source substitutions under retained availability and lexical policy.
#[must_use]
pub(crate) fn evaluated_command_substitutions_with_replay_and_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    observe: Option<EmbeddedCommandObserver<'_>>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: LexerConfig,
) -> EvaluatedCommandSubstitutions {
    let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
        return EvaluatedCommandSubstitutions {
            opaque: true,
            ..EvaluatedCommandSubstitutions::default()
        };
    };
    let mut surfaces = evaluated_command_substitution_surfaces(stmt, registry);
    if let Statement::Call { args, tokens, .. } | Statement::Barrier { args, tokens, .. } = stmt {
        // Every unbraced original argv word substitutes before dispatch,
        // including a structural command's dynamic body operand.
        surfaces.texts = unbraced_words(args, tokens.as_ref())
            .filter(|arg| arg.contains('['))
            .collect();
    }
    recover_substitution_surfaces(
        &surfaces.texts,
        surfaces.opaque,
        SubstitutionWalkContext {
            config,
            registry,
            heads,
            observe,
            conditional: surfaces.conditional,
            metadata: Some(metadata),
        },
    )
}

/// The command a recovered substitution head reaches, and the words an alias
/// chain prepends ahead of the ones written at the call site.
pub(crate) struct ResolvedEmbeddedHead {
    /// The spelling whose registry descriptor answers for this call.
    pub command: String,
    /// Arguments the alias chain supplies before the written ones, which
    /// shift every written word's index in the callee's own argv.
    pub prepended: Vec<String>,
}

/// Resolves a recovered substitution's head. `None` for a spelling with no
/// single statically known registry-backed target — the raw spelling is then
/// used, exactly as when no resolver is supplied.
pub(crate) type EmbeddedHeadResolver<'a> = &'a dyn Fn(&str) -> Option<ResolvedEmbeddedHead>;

/// Observes a completed command and whether expression control may skip it.
pub(crate) type EmbeddedCommandObserver<'a> = &'a dyn Fn(&[CommandWord], bool);

#[derive(Clone, Copy)]
struct SubstitutionWalkContext<'a> {
    pub(super) config: LexerConfig,
    pub(super) registry: &'a CommandRegistry,
    pub(super) metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    heads: Option<EmbeddedHeadResolver<'a>>,
    observe: Option<EmbeddedCommandObserver<'a>>,
    conditional: bool,
}

/// Where a recovered command runs, relative to the statement whose
/// substitutions the walk recovers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reach {
    /// A substitution the statement runs itself.
    Statement,
    /// A word a recovered command evaluates in this frame: an expression.
    InFrame,
    /// A protected script, or a word or substitution inside one.
    Protected,
}

fn walk_text(
    text: &str,
    context: &SubstitutionWalkContext<'_>,
    depth: u32,
    reach: Reach,
    out: &mut EvaluatedCommandSubstitutions,
) {
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        out.opaque = true;
        return;
    }
    let source_map = SourceMap::new(text);
    let Ok(tokens) = tcl_lexer::Lexer::with_config(
        text,
        tcl_lexer::SourceSegmentationPurpose::NestedScript.config(context.config),
    )
    .tokenise_all() else {
        out.opaque = true;
        return;
    };
    for token in tokens.iter().filter(|token| token.kind == TokenType::Cmd) {
        let inner = source_map.token_text(*token);
        let recovered = tokenise_command_words(inner, context.config);
        let protected = reach == Reach::Protected;
        for words in recovered {
            // Tcl substitutes command words from left to right before it
            // invokes the enclosing command. A nested `[rename …]` therefore
            // changes the binding that resolves its outer command; recording
            // the outer command first reverses that observable order.
            for word in words.iter().filter(|word| !word.braced_literal) {
                walk_text(&word.text, context, depth + 1, reach, out);
            }
            walk_in_frame_words(&words, context, (depth, protected), out);
            if let Some(observe) = context.observe {
                observe(&words, context.conditional);
            }
            out.record(reach, words);
        }
    }
}

/// The argument indices `lookup` evaluates **as an expression in this frame**,
/// as indices into its own argv-minus-head.
///
/// The single owner of that question, shared by the two walks that ask it: the
/// variable-effect walk ([`walk_in_frame_words`]) and the call-site walk
/// ([`crate::word_subst::lifted_calls_with_registry`]). The two disagreeing
/// about which commands a statement runs is what let `return [expr {[fact …]}]`
/// hide a recursive call from the caller-evidence scan (#2118).
///
/// Two shapes reach the same expression, and both come from the registry
/// rather than from the spelling `expr`:
///
/// * a single [`tcl_registry::ArgRole::Expr`] word — `[expr {$x + 1}]`;
/// * every word of a command that
///   [concatenates its arguments into one expression](tcl_registry::Traits::EXPR_CONCATENATES_ARGS).
///   `expr 1 + {[incr x]}` joins its words into `1 + [incr x]` and then
///   parses *that*, so the braced word's `[…]` runs: tclsh 8.6.18 and
///   9.0.4 both print `3` then `2` for
///   `set x 1; puts [expr 1 + {[incr x]}]; puts $x`.
///
/// Only `Expr` is named, never `Body`: a body word runs in this frame too but
/// may bind names of its own, which a flat command list cannot represent.
///
/// Read through the document's command *surface*, not the bare catalogue: a
/// `# tcl-lsp: stub myexpr {value:expr}` declares an expression word exactly
/// as a shipped command does, and lowering honours that role, so `myexpr
/// {[id 9]}` runs the call it holds. A declaration only ever *adds* a role
/// position, so this can widen the answer and never narrow it.
pub(crate) fn in_frame_expression_arg_indices(
    lookup: &str,
    args: &[&str],
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
) -> Vec<usize> {
    let mut descend: Vec<usize> =
        surface.arg_indices_for_role(lookup, args, tcl_registry::ArgRole::Expr);
    if surface
        .commands()
        .get_for_surface(lookup, surface.commands().own_surface_query())
        .is_some_and(|spec| {
            spec.traits
                .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
        })
    {
        descend.extend(0..args.len());
    }
    descend.sort_unstable();
    descend.dedup();
    descend
}

/// Parse one in-frame expression text under the walk's source dialect, or
/// `None` when the source metadata names no dialect to parse it under.
fn substitution_expression(
    text: &str,
    context: &SubstitutionWalkContext<'_>,
) -> Option<tcl_syntax::expr::ExprNode> {
    let Some(metadata) = context.metadata else {
        return Some(crate::expr_parser::parse_expr(
            text,
            context.registry.profile().map(|profile| profile.name),
        ));
    };
    let dialect = source_substitution_dialect(metadata)?;
    let mut parser = dialect.expression_parse_context(
        metadata
            .source_analysis_input()
            .map(crate::analyser::ResolvedAnalysisInput::unit_profile),
    );
    parser.lexer_grammar = context.config.grammar_over(parser.lexer_grammar);
    Some(crate::expr_parser::parse_expr_with_syntax_context(
        text, &parser,
    ))
}

/// Descend the brace-quoted words a recovered command evaluates **in this
/// frame**: an expression, and a protected script.
///
/// The word lexer is right to stop at `{…}` — the *command* parser
/// substitutes nothing there — but `expr` re-parses that text as an
/// expression, and a `[…]` in it is a substitution the statement really
/// runs. `puts [expr {$x + [incr x]}]` writes `x` exactly as
/// `puts [incr x]` does, and without this descent the write was invisible
/// and O102 forwarded a stale literal across it (#2141).
///
/// Which words are an expression is [`in_frame_expression_arg_indices`]' to
/// answer; this walks the brace-quoted ones among them.
///
/// A script the call's clause plan places as protected
/// ([`tcl_registry::ClauseTiming::Protected`]: the body of `catch` and of
/// `try`) runs once in this frame whatever it does, and its completion is
/// observed rather than propagated, so what it writes and reads is the
/// statement's: `set c [catch {incr x}]` writes `x`, and an `if` that reads
/// it was folded on the value `x` held before (#2231). Its commands are
/// descended, and their own protected scripts and expression words with
/// them, where the script is known text: brace-quoted, or a quoted or bare
/// word that substitutes nothing (`[catch "incr x"]`). A script that is
/// run-time data (`[catch $script]`) is unreadable, and the walk is marked
/// [`EvaluatedCommandSubstitutions::opaque`]. Every other body is left: it
/// may run zero or many times, in another frame, or bind names of its own,
/// which a flat command list cannot represent. What the walk reaches inside
/// a protected script, or inside a word of a command `in_protected` says
/// is in one, is also [`EvaluatedCommandSubstitutions::protected_commands`].
fn walk_in_frame_words(
    words: &[CommandWord],
    context: &SubstitutionWalkContext<'_>,
    (depth, in_protected): (u32, bool),
    out: &mut EvaluatedCommandSubstitutions,
) {
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        out.opaque = true;
        return;
    }
    let Some(head) = words.first().and_then(CommandWord::literal) else {
        return;
    };
    let resolved = context.heads.and_then(|resolve| resolve(head));
    if context.metadata.is_some() && context.heads.is_some() && resolved.is_none() {
        out.opaque = true;
        return;
    }
    let (lookup, prepended) = resolved.as_ref().map_or((head, [].as_slice()), |target| {
        (target.command.as_str(), target.prepended.as_slice())
    });
    // The callee's own argv: whatever an alias chain prepends, then the
    // words written here. Every role index the registry answers with is
    // an index into *that*, so a written word sits at `index - shift`.
    let shift = prepended.len();
    let mut args: Vec<&str> = prepended.iter().map(String::as_str).collect();
    args.extend(
        words
            .iter()
            .skip(1)
            .map(|word| word.literal().unwrap_or("")),
    );

    // The variable-effect walk is reached from consumers that hold only a
    // catalogue, so it asks the same owner with no declarations attached.
    let Some((indices, concatenated)) =
        source_expression_indices(words, lookup, prepended, &args, context, out)
    else {
        return;
    };
    let expression_texts = if concatenated {
        vec![args.join(" ")]
    } else {
        indices
            .iter()
            .filter_map(|&index| args.get(index).map(|arg| (*arg).to_owned()))
            .collect()
    };
    let mut conditional = context.conditional
        || (concatenated && words.iter().skip(1).any(|word| word.literal().is_none()));
    let mut opaque = false;
    for text in &expression_texts {
        let Some(expr) = substitution_expression(text, context) else {
            opaque = true;
            conditional = true;
            continue;
        };
        collect_expr_command_surface_refs(&expr, &mut Vec::new(), &mut opaque, &mut conditional, 0);
    }
    out.opaque |= opaque;
    let expression_context = SubstitutionWalkContext {
        conditional,
        ..*context
    };
    for index in indices {
        // A prepended word is a value the alias already holds, not source
        // this call substitutes.
        let Some(source_index) = index.checked_sub(shift) else {
            continue;
        };
        // An unbraced expression word already substituted at the command
        // level, so the enclosing walk has seen its `[…]` and descending
        // again would double-count the effect.
        let Some(word) = words
            .get(source_index + 1)
            .filter(|word| word.braced_literal)
        else {
            continue;
        };
        out.in_frame_texts.push(word.text.clone());
        let reach = if in_protected {
            Reach::Protected
        } else {
            Reach::InFrame
        };
        walk_text(&word.text, &expression_context, depth + 1, reach, out);
    }

    walk_clause_scripts(
        words,
        (lookup, &args, shift),
        context,
        depth,
        out,
        |timing| timing == tcl_registry::ClauseTiming::Protected,
    );
    // A loop's body and fixtures run zero or more times in this frame, and
    // its header binds its variables on each iteration: what they write the
    // statement may write, as a protected script's commands may (#2323).
    walk_clause_scripts(
        words,
        (lookup, &args, shift),
        context,
        depth,
        out,
        |timing| {
            matches!(
                timing,
                tcl_registry::ClauseTiming::PerIteration
                    | tcl_registry::ClauseTiming::LoopFixture(_)
            )
        },
    );
    record_loop_binders(words, (lookup, &args, shift), context, out);
}

/// Record the names the call's loop-variable lists bind
/// ([`EvaluatedCommandSubstitutions::may_bound_names`]); a list that is
/// run-time data may bind any name.
fn record_loop_binders(
    words: &[CommandWord],
    (lookup, args, shift): (&str, &[&str], usize),
    context: &SubstitutionWalkContext<'_>,
    out: &mut EvaluatedCommandSubstitutions,
) {
    for index in
        context
            .registry
            .arg_indices_for_role(lookup, args, tcl_registry::ArgRole::LoopVarList)
    {
        let Some(source_index) = index.checked_sub(shift) else {
            continue;
        };
        let Some(word) = words.get(source_index + 1) else {
            continue;
        };
        let names = if word.braced_literal {
            Some(word.text.as_str())
        } else {
            word.literal().filter(|_| !word.raw.contains('\\'))
        }
        .and_then(|list| {
            tcl_syntax::word_rules::WordValueRules::from_config(&context.config)
                .split_list(list)
                .ok()
        });
        let Some(names) = names else {
            out.opaque = true;
            continue;
        };
        for name in names {
            if !name.is_empty() && !out.may_bound_names.iter().any(|known| *known == name) {
                out.may_bound_names.push(name.into_owned());
            }
        }
    }
}

/// Descend the scripts the call `words` runs at a clause timing `admit`
/// accepts — a protected script ([`tcl_registry::ClauseTiming::Protected`]:
/// the body of a `catch` or a `try`), or a loop's body — where each is known
/// text ([`walk_in_frame_words`] says which). Their commands may not run, so
/// they are recorded as protected. `callee` is the command the call reaches,
/// its argv after any words an alias prepends, and how many those are.
fn walk_clause_scripts(
    words: &[CommandWord],
    (lookup, args, shift): (&str, &[&str], usize),
    context: &SubstitutionWalkContext<'_>,
    depth: u32,
    out: &mut EvaluatedCommandSubstitutions,
    admit: impl Fn(tcl_registry::ClauseTiming) -> bool,
) {
    let protected: Vec<usize> = context
        .registry
        .clause_plan(lookup, args)
        .map_or_default(|plan| {
            plan.clauses
                .iter()
                .filter(|clause| admit(clause.timing))
                .flat_map(|clause| clause.operands(tcl_registry::ArgRole::Body))
                .collect()
        });
    for index in protected {
        let Some(source_index) = index.checked_sub(shift) else {
            continue;
        };
        let Some(word) = words.get(source_index + 1) else {
            continue;
        };
        // The script is the text of a brace-quoted word, or of a quoted or
        // bare word that substitutes nothing and has no escape to read. A word
        // that substitutes, or is expanded, is run-time data: the script may
        // write any name, so the walk is incomplete.
        let script = if word.braced_literal {
            Some(word.text.as_str())
        } else {
            word.literal().filter(|_| !word.raw.contains('\\'))
        };
        let Some(script) = script else {
            out.opaque = true;
            continue;
        };
        out.in_frame_texts.push(script.to_owned());
        // The script's commands run in order, each after the substitutions
        // of its own words, and any of them may be skipped: an error before
        // it stops the script.
        let script_context = SubstitutionWalkContext {
            conditional: true,
            ..*context
        };
        for inner in tokenise_command_words(script, context.config) {
            for word in inner.iter().filter(|word| !word.braced_literal) {
                walk_text(
                    &word.text,
                    &script_context,
                    depth + 1,
                    Reach::Protected,
                    out,
                );
            }
            walk_in_frame_words(&inner, &script_context, (depth + 1, true), out);
            if let Some(observe) = script_context.observe {
                observe(&inner, script_context.conditional);
            }
            out.record(Reach::Protected, inner);
        }
    }
}

fn source_substitution_dialect(
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
) -> Option<tcl_native_model::InvocationDialect> {
    metadata
        .source_analysis_input()
        .map(|input| tcl_native_model::InvocationDialect::of_profile(input.unit_profile()))
        .or_else(|| {
            metadata
                .context()
                .environment
                .point()
                .map(tcl_native_model::InvocationDialect::of_point)
        })
}

fn source_expression_indices(
    words: &[CommandWord],
    lookup: &str,
    prepended: &[String],
    args: &[&str],
    context: &SubstitutionWalkContext<'_>,
    out: &mut EvaluatedCommandSubstitutions,
) -> Option<(Vec<usize>, bool)> {
    let selected = if let Some(metadata) = context.metadata {
        let mut effective: Vec<_> = prepended
            .iter()
            .map(|word| InvocationWord::Literal(word))
            .collect();
        effective.extend(words.iter().skip(1).map(CommandWord::invocation_word));
        let mut invocation_words =
            InvocationWords::structured(InvocationWord::Literal(lookup), &effective);
        if let Some(dialect) = source_substitution_dialect(metadata) {
            invocation_words = invocation_words.with_dialect(dialect);
        }
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.registry,
                Some(metadata.context()),
                invocation_words,
                metadata.context().authoring_query().realm,
            );
        let Some(invocation) = resolution.resolved() else {
            out.opaque = true;
            return None;
        };
        let (roles, complete) = invocation.argument_roles();
        out.opaque |= !complete;
        let concatenated = invocation
            .semantics
            .traits
            .contains(Traits::EXPR_CONCATENATES_ARGS);
        let mut indices: Vec<_> = roles
            .into_iter()
            .filter_map(|(index, role)| {
                (role == ArgRole::Expr)
                    .then_some(invocation.semantics.argument_offset + usize::from(index))
            })
            .collect();
        if concatenated {
            indices.extend(0..args.len());
        }
        indices.sort_unstable();
        indices.dedup();
        (indices, concatenated)
    } else {
        let surface = tcl_registry::model::DocumentCommandSurface::new(context.registry, None);
        (
            in_frame_expression_arg_indices(lookup, args, &surface),
            surface
                .commands()
                .get(lookup)
                .is_some_and(|spec| spec.traits.contains(Traits::EXPR_CONCATENATES_ARGS)),
        )
    };
    Some(selected)
}

/// Recover commands from an already-selected collection of evaluated Tcl
/// source surfaces, resolving each recovered head through `heads` when the
/// caller can supply one.
#[must_use]
pub(crate) fn command_substitutions_in_surfaces_with_replay(
    surfaces: &[&str],
    initially_opaque: bool,
    conditional: bool,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    observe: Option<EmbeddedCommandObserver<'_>>,
) -> EvaluatedCommandSubstitutions {
    let config = registry
        .profile()
        .map_or_else(LexerConfig::default, |profile| {
            LexerConfig::from_grammar(profile.grammar)
        });
    let mut out = EvaluatedCommandSubstitutions {
        opaque: initially_opaque,
        ..EvaluatedCommandSubstitutions::default()
    };
    let context = SubstitutionWalkContext {
        config,
        registry,
        metadata: None,
        heads,
        observe,
        conditional,
    };
    for text in surfaces {
        walk_text(text, &context, 0, Reach::Statement, &mut out);
    }
    out
}

fn recover_substitution_surfaces(
    surfaces: &[&str],
    initially_opaque: bool,
    context: SubstitutionWalkContext<'_>,
) -> EvaluatedCommandSubstitutions {
    let mut out = EvaluatedCommandSubstitutions {
        opaque: initially_opaque,
        ..EvaluatedCommandSubstitutions::default()
    };
    for text in surfaces {
        walk_text(text, &context, 0, Reach::Statement, &mut out);
    }
    out
}

/// Recover commands from command substitutions nested in one expression.
#[must_use]
pub(crate) fn expression_command_substitutions(
    expr: &ExprNode,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
) -> EvaluatedCommandSubstitutions {
    expression_command_substitutions_with_replay(expr, registry, heads, None)
}

#[must_use]
pub(crate) fn expression_command_substitutions_with_replay(
    expr: &ExprNode,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    observe: Option<EmbeddedCommandObserver<'_>>,
) -> EvaluatedCommandSubstitutions {
    let mut texts = Vec::new();
    let mut opaque = false;
    let mut conditional = false;
    collect_expr_command_surface_refs(expr, &mut texts, &mut opaque, &mut conditional, 0);
    command_substitutions_in_surfaces_with_replay(
        &texts,
        opaque,
        conditional,
        registry,
        heads,
        observe,
    )
}

/// Conditional expression command inventory under the supplied metadata and
/// exact syntax policy. Missing/foreign input is opaque; no native replay or
/// callback/frame entry is established by this readonly inventory.
#[must_use]
pub(crate) fn expression_command_substitutions_with_replay_and_metadata_context(
    expr: &ExprNode,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    observe: Option<EmbeddedCommandObserver<'_>>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: LexerConfig,
) -> EvaluatedCommandSubstitutions {
    let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
        return EvaluatedCommandSubstitutions {
            opaque: true,
            ..Default::default()
        };
    };
    if metadata
        .source_analysis_input()
        .is_some_and(|input| input.lexer_config().normalized() != config.normalized())
    {
        return EvaluatedCommandSubstitutions {
            opaque: true,
            ..Default::default()
        };
    }
    let mut texts = Vec::new();
    let mut opaque = false;
    let mut conditional = false;
    collect_expr_command_surface_refs(expr, &mut texts, &mut opaque, &mut conditional, 0);
    recover_substitution_surfaces(
        &texts,
        opaque,
        SubstitutionWalkContext {
            config,
            registry,
            metadata: Some(metadata),
            heads,
            observe,
            conditional,
        },
    )
}
