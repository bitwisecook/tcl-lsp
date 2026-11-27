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

//! The host command handlers that capture each declaration: rows, `speclib`, `command`, `subcommand`, `environment` and `dialect` blocks, `include`, `available?` and unknown commands.

use super::super::{
    FileBom, Log, Notice, Stmt, VocabularyClass, Word, available, check_vocabulary_version,
    parse_dialects, statements,
};
use super::static_drive::run_body;
use super::{
    BodyText, Drive, Node, PackNode, ScopeKind, SpeclibDecl, StagedBlock, StagedCommand, StagedSub,
    State, VerbatimBody, content_hash,
};
#[cfg(feature = "programmed-packs")]
use super::{Commands, EvaluatedCommandBody, interpreted};
#[cfg(feature = "programmed-packs")]
use crate::export::synth_word;
use std::cell::RefCell;
use std::rc::Rc;
#[cfg(feature = "programmed-packs")]
use tcl_dialect::model::surfaces_overlap;
#[cfg(feature = "programmed-packs")]
use tcl_spec_hooks::pack_eval::{PackEvalCtx, UnknownHandler, WordHandler};

/// Whether an argument's text can only have been written as a `{ … }` block
/// (or quoted equivalent): the evaluation loses the CST's per-word
/// braced-ness, so it is reconstructed from the one property that matters
/// to every consumer — a name is one whitespace-free word, a block is not.
fn blockish(text: &str) -> bool {
    tcl_source::spectcl::pack_value_is_block(text)
}

/// Rebuild a [`Stmt`] — the shape a segmented source statement has — from
/// a captured invocation.
pub(super) fn capture_stmt(word: &str, args: &[String], line: u32) -> Stmt {
    let mut words = Vec::with_capacity(args.len() + 1);
    words.push(Word {
        local_source: None,
        original_source: None,
        text: word.to_owned(),
        braced: blockish(word),
        line,
    });
    for arg in args {
        words.push(Word {
            local_source: None,
            original_source: None,
            text: arg.clone(),
            braced: blockish(arg),
            line,
        });
    }
    Stmt { words, line }
}

fn eval_notice(context: &str, line: u32, class: VocabularyClass, message: String) -> Notice {
    Notice {
        subject: None,
        newer_vocabulary: None,
        context: context.to_owned(),
        line,
        message,
        class,
    }
}

/// The handler for one plain row word.
#[cfg(feature = "programmed-packs")]
pub(super) fn row_handler(word: &'static str, state: &Rc<RefCell<State>>) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let mut st = state.borrow_mut();
        let line = st.absolute_line(ctx.line());
        if word == "default" {
            track_declared_support(&mut st, args, line);
        }
        let stmt = st.captured(word, args, line);
        st.push_node(Node::Row(stmt));
        Ok(None)
    })
}

/// Keep the running union of declared support for `available?` (E-R1).
pub(super) fn track_declared_support(state: &mut State, args: &[String], line: u32) {
    let mut scratch = Log::default();
    match args.first().map(String::as_str) {
        Some("dialects") => {
            if let Some(set) = parse_dialects(args.get(1).map_or("", |a| a), line, &mut scratch) {
                state.declared_support.surface = Some(set);
            }
        }
        Some("available") => {
            let availability = available::from_texts(&args[1..], line, &mut scratch);
            state.declared_support.unavailable |= availability.is_unavailable();
            if let Some(set) = availability.surface() {
                state.declared_support.surface = Some(set);
            }
        }
        _ => {}
    }
}

/// The `speclib` handler: header capture, extra-block reporting, and body
/// evaluation. A thin adapter over [`stage_speclib`], which the static
/// driver reaches with the file's own words instead of evaluated values.
#[cfg(feature = "programmed-packs")]
pub(super) fn speclib_handler(state: &Rc<RefCell<State>>, fast_path: bool) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let line = state.borrow().absolute_line(ctx.line());
        stage_speclib(interpreted(ctx, fast_path), &state, args, line)?;
        Ok(None)
    })
}

/// Stage one `speclib NAME VERSION { … }` declaration and run its body.
///
/// Shared by the host command and the static driver, so the file-level
/// shortcut and the interpreter cannot drift on what a `speclib` header
/// means. `args` are the statement's words after `speclib`, as values.
pub(super) fn stage_speclib(
    drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    args: &[String],
    line: u32,
) -> Result<(), String> {
    // Only a top-level `speclib` is the pack header; one inside any
    // evaluated body is an ordinary (unknown) row.
    let nested = {
        let st = state.borrow();
        !st.scopes.is_empty() || !st.base_lines.is_empty()
    };
    if nested {
        let mut st = state.borrow_mut();
        let stmt = st.captured("speclib", args, line);
        st.push_node(Node::Row(stmt));
        return Ok(());
    }
    if state.borrow().speclib.is_some() || state.borrow().refused_braced_name {
        let extra_name = args.first().cloned().unwrap_or_default();
        let commands = args.get(2).map_or(0, |body| {
            statements(body, 1, FileBom::Content)
                .iter()
                .filter(|s| s.word_text(0) == "command")
                .count()
        });
        state.borrow_mut().eval_notices.push(eval_notice(
            "pack",
            line,
            VocabularyClass::Presentation,
            format!(
                "a second `speclib` block (`{extra_name}`) in one file is not loaded; \
                 only the first pack in a file is read, so its {commands} command(s) \
                 are dropped — move it to its own `.tclspec`"
            ),
        ));
        return Ok(());
    }
    let name = args.first().cloned().unwrap_or_default();
    if blockish(&name) && !name.is_empty() {
        let mut st = state.borrow_mut();
        st.refused_braced_name = true;
        st.eval_notices.push(eval_notice(
            "pack",
            line,
            VocabularyClass::Presentation,
            "`speclib` needs a name and a vocabulary version before its body \
             block (`speclib NAME 1.1 { … }`); nothing loaded"
                .to_owned(),
        ));
        return Ok(());
    }
    let version = args.get(1).cloned().unwrap_or_default();
    let body = args.get(2).cloned();
    {
        let mut st = state.borrow_mut();
        st.speclib = Some(SpeclibDecl {
            name,
            version: version.clone(),
            line,
            had_body: body.is_some(),
        });
    }
    // An unsupported major fails the pack closed (§6.1): the body is
    // not even evaluated, matching "nothing is loaded". The replay
    // re-runs the check to produce the notice.
    let mut scratch = Log::default();
    if check_vocabulary_version(&version, line, &mut scratch) {
        return Ok(());
    }
    if let Some(body) = body {
        // Prefer the file's verbatim body so continuation collapse
        // cannot shift line attribution (see `VerbatimBody`).
        let verbatim = {
            let mut st = state.borrow_mut();
            st.verbatim
                .speclib
                .take_if(|candidate| candidate.stmt_line == line)
        };
        let block = BodyText::captured(verbatim, "speclib", args, body, line);
        state.borrow_mut().base_lines.push(block.base);
        let outcome = run_body(drive, state, &block);
        state.borrow_mut().base_lines.pop();
        outcome?;
    }
    Ok(())
}

#[cfg(feature = "programmed-packs")]
impl<'a> EvaluatedCommandBody<'a> {
    pub(super) fn select(arguments: &'a [String]) -> Option<Self> {
        match arguments {
            [_, body] => Some(Self {
                text: body,
                overrides: false,
            }),
            [_, flag, body] if flag == "-override" => Some(Self {
                text: body,
                overrides: true,
            }),
            _ => None,
        }
    }
}

/// The `command` handler: head capture plus body evaluation in a fresh
/// command scope.
#[cfg(feature = "programmed-packs")]
pub(super) fn command_handler(state: &Rc<RefCell<State>>, fast_path: bool) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let line = state.borrow().absolute_line(ctx.line());
        if !state.borrow().scopes.is_empty() {
            let mut st = state.borrow_mut();
            let stmt = st.captured("command", args, line);
            st.push_node(Node::Row(stmt));
            return Ok(None);
        }
        if state.borrow().commands == Commands::Skipped {
            return Ok(None);
        }
        let name = args.first().cloned().unwrap_or_default();
        let overrides = args.iter().any(|arg| arg == "-override");
        let candidate = state.borrow_mut().verbatim.take("command", &name, line);
        // The actual original brace operand supplies its selected body value,
        // including a one-word body. Evaluated fallback retains its old layout.
        let source_body = candidate.as_ref().and_then(|candidate| {
            let original = candidate.original.as_ref()?;
            let value = original.captured_text();
            candidate
                .original_for_capture("command", args, value)
                .map(|_| value.to_owned())
        });
        let evaluated = EvaluatedCommandBody::select(args);
        // Only the typed evaluated option slot can set this flag. Keep the
        // established authentic-original compatibility branch independent.
        let overrides = if source_body.is_some() {
            overrides
        } else {
            evaluated
                .as_ref()
                .is_some_and(|selected| selected.overrides)
        };
        let body = source_body.or_else(|| evaluated.map(|selected| selected.text.to_owned()));
        if body.is_none() && args.len() > 1 {
            state.borrow_mut().eval_notices.push(eval_notice(
                "command", line, VocabularyClass::Presentation,
                "evaluated command declaration requires NAME ?-override? BODY; no body selected from an unsupported argument layout".to_owned(),
            ));
        }
        let Some(body) = body else {
            state
                .borrow_mut()
                .pack_nodes
                .push(PackNode::Command(StagedCommand {
                    name,
                    overrides,
                    line,
                    body: None,
                }));
            return Ok(None);
        };
        let body = BodyText::captured(candidate, "command", args, body, line);
        stage_command(
            interpreted(ctx, fast_path),
            &state,
            name,
            overrides,
            line,
            &body,
        )?;
        Ok(None)
    })
}

impl BodyText {
    pub(super) fn static_word(word: &Word) -> Self {
        Self {
            original: word.original_source.clone(),
            text: word.text.clone(),
            base: word.line,
            verbatim: true,
        }
    }

    pub(super) fn captured(
        candidate: Option<VerbatimBody>,
        head: &str,
        args: &[String],
        value: String,
        line: u32,
    ) -> Self {
        if let Some(candidate) = candidate
            && let Some(original) = candidate.original_for_capture(head, args, &value)
        {
            return Self {
                original: Some(original),
                text: candidate.text,
                base: candidate.body_line,
                verbatim: true,
            };
        }
        Self {
            original: None,
            text: value,
            base: line,
            verbatim: false,
        }
    }
}

/// Evaluate one `command` body in a fresh command scope and stage the
/// declaration. Shared by the host command and the static driver.
pub(super) fn stage_command(
    drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    name: String,
    overrides: bool,
    line: u32,
    body: &BodyText,
) -> Result<(), String> {
    {
        let mut st = state.borrow_mut();
        st.scopes.push((ScopeKind::Command, Vec::new()));
        st.base_lines.push(body.base);
    }
    let outcome = run_body(drive, state, body);
    let nodes = {
        let mut st = state.borrow_mut();
        st.base_lines.pop();
        st.scopes.pop().map_or_default(|(_, nodes)| nodes)
    };
    outcome?;
    state
        .borrow_mut()
        .pack_nodes
        .push(PackNode::Command(StagedCommand {
            name,
            overrides,
            line,
            body: Some(nodes),
        }));
    Ok(())
}

/// The `subcommand` handler: valid only directly inside a `command` body,
/// where its own body evaluates in a subcommand scope; anywhere else it is
/// a plain row, which replays to the ordinary unknown-property notice.
#[cfg(feature = "programmed-packs")]
pub(super) fn subcommand_handler(state: &Rc<RefCell<State>>, fast_path: bool) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let line = state.borrow().absolute_line(ctx.line());
        let in_command = matches!(state.borrow().scopes.last(), Some((ScopeKind::Command, _)));
        if !in_command {
            let mut st = state.borrow_mut();
            let stmt = st.captured("subcommand", args, line);
            st.push_node(Node::Row(stmt));
            return Ok(None);
        }
        let name = args.first().cloned().unwrap_or_default();
        // The body is exactly the third word; a `subcommand` without one
        // is silently dropped.
        let Some(body) = args.get(1).cloned() else {
            return Ok(None);
        };
        let candidate = state.borrow_mut().verbatim.take("subcommand", &name, line);
        let body = BodyText::captured(candidate, "subcommand", args, body, line);
        stage_subcommand(interpreted(ctx, fast_path), &state, name, line, &body)?;
        Ok(None)
    })
}

/// Evaluate one `subcommand` body in a fresh subcommand scope and stage the
/// declaration into the owning command. Shared by the host command and the
/// static driver.
pub(super) fn stage_subcommand(
    drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    name: String,
    line: u32,
    body: &BodyText,
) -> Result<(), String> {
    {
        let mut st = state.borrow_mut();
        st.scopes.push((ScopeKind::Subcommand, Vec::new()));
        st.base_lines.push(body.base);
    }
    let outcome = run_body(drive, state, body);
    let nodes = {
        let mut st = state.borrow_mut();
        st.base_lines.pop();
        st.scopes.pop().map_or_default(|(_, nodes)| nodes)
    };
    outcome?;
    state.borrow_mut().push_node(Node::Sub(StagedSub {
        name,
        line,
        body: nodes,
    }));
    Ok(())
}

/// The `environment` / `dialect` handler: at pack scope the declaration's
/// body is **evaluated**, in a fresh block scope, exactly as a `command`
/// body is. Anywhere else it is a plain row, which replays to the ordinary
/// unknown-property notice.
#[cfg(feature = "programmed-packs")]
pub(super) fn block_handler(
    word: &'static str,
    state: &Rc<RefCell<State>>,
    fast_path: bool,
) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let line = state.borrow().absolute_line(ctx.line());
        if !state.borrow().scopes.is_empty() {
            let mut st = state.borrow_mut();
            let stmt = st.captured(word, args, line);
            st.push_node(Node::Row(stmt));
            return Ok(None);
        }
        let name = args.first().cloned().unwrap_or_default();
        let source = state.borrow_mut().source_stmt(word, args, line);
        let (head, body_at) = block_head(word, args, line, source.as_ref());
        let Some(body_at) = body_at else {
            state
                .borrow_mut()
                .pack_nodes
                .push(PackNode::Block(StagedBlock {
                    word,
                    head,
                    line,
                    body: None,
                }));
            return Ok(None);
        };
        let candidate = state.borrow_mut().verbatim.take(word, &name, line);
        let body = BodyText::captured(candidate, word, args, args[body_at].clone(), line);
        stage_block(interpreted(ctx, fast_path), &state, word, head, line, &body)?;
        Ok(None)
    })
}

/// Split an `environment` / `dialect` declaration into its head words and
/// the argument index of its body.
///
/// Evaluation hands over values, not words, so neither the body nor a
/// braced name survives in them: `{emit}` and `emit` are the same string,
/// and so are `{custom}` and `custom`. The file's own statement decides
/// both whenever it has one at this line, which is what keeps a braced name
/// rejected on this path as well. Failing that the layout decides —
/// `NAME ?-extend? BODY`, the body always last — never whitespace in the
/// value, which would lose a one-word body such as `{emit}`.
#[cfg(feature = "programmed-packs")]
fn block_head(
    word: &'static str,
    args: &[String],
    line: u32,
    source: Option<&Stmt>,
) -> (Vec<Word>, Option<usize>) {
    if let Some(stmt) = source {
        let body_at = stmt
            .words
            .iter()
            .enumerate()
            .skip(2)
            .find(|(_, source_word)| source_word.braced)
            .map(|(index, _)| index);
        let head = stmt
            .words
            .iter()
            .enumerate()
            .filter(|(index, _)| Some(*index) != body_at)
            .map(|(_, source_word)| source_word.clone())
            .collect();
        return (head, body_at.map(|index| index - 1));
    }
    let body_at = (args.len() > 1).then(|| args.len() - 1);
    let head = std::iter::once(synth_word(word, line))
        .chain(
            args.iter()
                .enumerate()
                .filter(|(index, _)| Some(*index) != body_at)
                .map(|(_, arg)| synth_word(arg, line)),
        )
        .collect();
    (head, body_at)
}

/// Evaluate one `environment` / `dialect` body in a fresh block scope and
/// stage the declaration. Shared by the host command and the static driver,
/// so the two evaluation paths cannot drift.
pub(super) fn stage_block(
    drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    word: &'static str,
    head: Vec<Word>,
    line: u32,
    body: &BodyText,
) -> Result<(), String> {
    {
        let mut st = state.borrow_mut();
        st.scopes.push((ScopeKind::Block, Vec::new()));
        st.base_lines.push(body.base);
    }
    let outcome = run_body(drive, state, body);
    let nodes = {
        let mut st = state.borrow_mut();
        st.base_lines.pop();
        st.scopes.pop().map_or_default(|(_, nodes)| nodes)
    };
    outcome?;
    state
        .borrow_mut()
        .pack_nodes
        .push(PackNode::Block(StagedBlock {
            word,
            head,
            line,
            body: Some(nodes),
        }));
    Ok(())
}

/// The `include` handler (2.0, Q6): valid only as a literal pack-scope
/// row; the resolved fragment evaluates in place with provenance
/// inherited, under the determinism contract's content-hash cycle key.
#[cfg(feature = "programmed-packs")]
pub(super) fn include_handler(state: &Rc<RefCell<State>>, fast_path: bool) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let line = state.borrow().absolute_line(ctx.line());
        let at_pack = state.borrow().scopes.is_empty();
        if !at_pack {
            // Inside a command/subcommand body `include` is not pack
            // vocabulary; captured as a row, it replays to the unknown
            // word path.
            let mut st = state.borrow_mut();
            let stmt = st.captured("include", args, line);
            st.push_node(Node::Row(stmt));
            return Ok(None);
        }
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        stage_include(interpreted(ctx, fast_path), &state, &words, line)?;
        Ok(None)
    })
}

fn capture_surface_include(state: &Rc<RefCell<State>>, words: &[&str], line: u32) -> bool {
    if !super::super::surface_roster::is_surface_row(words) {
        return false;
    }
    let mut st = state.borrow_mut();
    let owned: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
    let stmt = st.captured("include", &owned, line);
    st.push_node(Node::Row(stmt));
    true
}

/// Resolve and evaluate one `include` row. Shared by the host command and
/// the static driver, so the two evaluation paths cannot drift.
pub(super) fn stage_include(
    drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    words: &[&str],
    line: u32,
) -> Result<(), String> {
    // The vocabulary-consistency log's eval-side record: `include` is 2.0
    // vocabulary, and the replay turns this into the same per-site notice
    // the replay's readers log for a row they read themselves.
    state.borrow_mut().newer_word_sites.push((line, "include"));
    // The surface form (`include from … into …`, design Q6) composes a
    // command surface, not a file: there is nothing to resolve and nothing
    // to evaluate, so it is captured as an ordinary row and read by
    // `apply_pack_stmt` — the one parser both routes share.
    if capture_surface_include(state, words, line) {
        return Ok(());
    }
    let name = match super::super::include_name(words, line) {
        Ok(name) => name,
        Err(notice) => {
            state.borrow_mut().eval_notices.push(*notice);
            return Ok(());
        }
    };
    let resolved = {
        let st = state.borrow();
        st.include.clone().map(|context| context.resolve(&name))
    };
    let Some(resolved) = resolved else {
        let notice = eval_notice(
            "pack",
            line,
            VocabularyClass::Semantic,
            format!(
                "`include {name}` needs a pack search path and this load was given \
                 none; the row is dropped and its declarations are not loaded"
            ),
        );
        state.borrow_mut().eval_notices.push(notice);
        return Ok(());
    };
    let text = match resolved {
        Ok(text) => text,
        Err(error) => {
            let notice = eval_notice(
                "pack",
                line,
                VocabularyClass::Semantic,
                format!(
                    "`include {name}` did not resolve ({error}); the row is dropped \
                     and its declarations are not loaded"
                ),
            );
            state.borrow_mut().eval_notices.push(notice);
            return Ok(());
        }
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned();
    let hash = content_hash(&text);
    {
        let mut st = state.borrow_mut();
        if st.include_stack.contains(&hash) {
            let notice = eval_notice(
                "pack",
                line,
                VocabularyClass::Semantic,
                format!(
                    "`include {name}` closes an include cycle (its content is already \
                     on the inclusion path); the row is dropped"
                ),
            );
            st.eval_notices.push(notice);
            return Ok(());
        }
        if st.include_stack.len() >= super::super::INCLUDE_DEPTH_LIMIT {
            let notice = eval_notice(
                "pack",
                line,
                VocabularyClass::Semantic,
                format!(
                    "`include {name}` exceeds the include depth limit ({}); the row \
                     is dropped",
                    super::super::INCLUDE_DEPTH_LIMIT
                ),
            );
            st.eval_notices.push(notice);
            return Ok(());
        }
        st.include_stack.push(hash);
        st.included.push(hash);
        st.base_lines.push(1);
    }
    let was_in_include = {
        let mut st = state.borrow_mut();
        std::mem::replace(&mut st.in_include, true)
    };
    let body = BodyText {
        original: None,
        text,
        base: 1,
        verbatim: true,
    };
    let outcome = run_body(drive, state, &body);
    {
        let mut st = state.borrow_mut();
        st.in_include = was_in_include;
        st.base_lines.pop();
        st.include_stack.pop();
    }
    outcome
}

/// The `available?` query (E-R1): answers against the union of the pack's
/// declared support and downgrades the pack's cacheability class.
#[cfg(feature = "programmed-packs")]
pub(super) fn available_handler(state: &Rc<RefCell<State>>) -> WordHandler {
    let state = Rc::clone(state);
    Rc::new(move |ctx: &mut PackEvalCtx<'_>, args: &[String]| {
        let mut st = state.borrow_mut();
        let line = st.absolute_line(ctx.line());
        let mut scratch = Log::default();
        let requirement = available::from_texts(args, line, &mut scratch);
        if requirement.is_unavailable() || st.declared_support.unavailable {
            return Err("availability query has an explicit unavailable provider/window; no boolean answer or unrestricted default is inferred".to_owned());
        }
        // An unstated surface on either side names no provider to disagree
        // with, so the guard holds.
        let answer = match (requirement.surface(), st.declared_support.surface) {
            (Some(required), Some(declared)) => surfaces_overlap(required, declared),
            _ => true,
        };
        if !st.target_dependent {
            st.target_dependent = true;
        }
        st.eval_notices.push(eval_notice(
            "pack",
            line,
            VocabularyClass::Presentation,
            format!(
                "target-dependent registration: `available? {}` evaluates against the \
                 union of the pack's declared support, so the pack's snapshot depends \
                 on the analysis target and is excluded from caching (design E-R1); \
                 prefer an `-available` row on the registration itself",
                args.join(" ")
            ),
        ));
        Ok(Some(if answer { "1" } else { "0" }.to_owned()))
    })
}

/// The unresolved-dispatch handler: a sandbox denial is a hard determinism
/// error; anything else is unknown vocabulary, captured so the replay
/// classifies it under §6.1.
#[cfg(feature = "programmed-packs")]
pub(super) fn unknown_handler(state: &Rc<RefCell<State>>) -> UnknownHandler {
    let state = Rc::clone(state);
    Rc::new(
        move |ctx: &mut PackEvalCtx<'_>, name: &str, args: &[String]| {
            if let Some(axis) = super::super::denied_axis(name) {
                return Err(record_denial(
                    &state,
                    format!(
                        "pack evaluation is deterministic: `{name}` is denied by the \
                         pack sandbox (determinism axis: {axis}) and no partial \
                         registration is kept (design E §1.2)"
                    ),
                ));
            }
            let mut st = state.borrow_mut();
            let line = st.absolute_line(ctx.line());
            let stmt = st.captured(name, args, line);
            st.push_node(Node::Row(stmt));
            Ok(None)
        },
    )
}

/// Keep the first denial on the evaluation state and return its message as
/// the Tcl error the pack sees.
#[cfg(feature = "programmed-packs")]
fn record_denial(state: &Rc<RefCell<State>>, message: String) -> String {
    state
        .borrow_mut()
        .determinism_denial
        .get_or_insert_with(|| message.clone());
    message
}
