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

//! Lowering a procedure body in isolation for the body cache, and which bodies are eligible.

use super::{Lowerer, lower_with};
use crate::command_binding::SourceCommandBindings;
use crate::ir::{Module, Script};
use tcl_registry::CommandRegistry;
use tcl_registry::events::IrulesExecutionContext;

/// Lower a single `proc` body in isolation at `proc_depth == 1` (a fresh
/// [`Lowerer`] with an empty const-map frame), returning the **offset-0** body
/// [`Script`].  Replicates the body-lowering setup `lower_proc` performs for a
/// static literal body (`proc_depth += 1`; push an empty const-map;
/// `lower_body(text, 0, namespace)`).
///
/// For a body free of cross-item context (no nested `proc` / `namespace
/// import`/`export` / command alias / const-map materialisation), this is
/// byte-identical to the body the whole-file lowering produces for that
/// procedure, normalised to offset 0 — the seam the per-procedure
/// lowering memo keys on the offset-0 body text and feeds back through
/// [`Lowerer::with_body_cache`].
#[must_use]
pub fn lower_proc_body_isolated(
    body_text: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Script {
    let mut lowerer = Lowerer::for_standalone_source(registry, config, dialect);
    lowerer.start_module();
    lowerer.source.clone_from(&body_text.to_owned());
    let frame = tcl_names::variable::VariableExecutionFrame::Procedure {
        namespace: namespace.to_owned(),
        identity: "::body-cache".to_owned(),
    };
    lowerer.source_bindings = Some(
        SourceCommandBindings::analyse_in_frame_with_options(
            body_text,
            &frame,
            config,
            registry,
            lowerer.module.source_entry.options(),
        )
        .into(),
    );
    lowerer.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
        lowerer.lower_body(body_text, 0, namespace)
    })
}

/// Legacy text-shape predicate retained for callers that inspect source shape.
///
/// This does not prove cache eligibility. The lowering ingress uses
/// [`SourceCommandBindings::can_reuse_isolated_body`] and restores exact
/// positioned proof carriers through
/// [`SourceCommandBindings::restore_script_bindings`] after a cache hit.
/// A command's spelling cannot establish the implementation it executes.
#[must_use]
pub fn body_cache_eligible(body: &str) -> bool {
    // Command keywords whose body-level use carries a cross-item effect the
    // isolated lowering drops. `proc` covers a nested procedure definition.
    // Matched as a word followed by Tcl inter-word whitespace (space, tab, CR,
    // newline, or a `\`-continuation), so a tab-separated `interp\talias` trips
    // it too.
    const WORD_DISQUALIFIERS: &[&str] = &[
        "namespace",
        "interp",
        "rename",
        "method",
        "when",
        "apply",
        "alias",
        "proc",
    ];
    // Substring markers (the token itself is the marker — no trailing arg).
    const SUBSTR_DISQUALIFIERS: &[&str] = &["oo::", "::oo", "itcl"];
    !SUBSTR_DISQUALIFIERS.iter().any(|kw| body.contains(kw))
        && !WORD_DISQUALIFIERS
            .iter()
            .any(|kw| contains_word_followed_by_ws(body, kw))
}

/// Legacy lexical indication that source contains alias-related words.
///
/// This is a presentation/compatibility predicate, not command-table evidence
/// or a cache gate. Use the shared positioned binding owner for those questions.
#[must_use]
pub fn source_may_alias_commands(source: &str) -> bool {
    contains_word_followed_by_ws(source, "interp") || contains_word_followed_by_ws(source, "rename")
}

/// True when `word` occurs in `source` followed by Tcl inter-word whitespace —
/// i.e. used as a command with an argument. Ignores the character *before* the
/// word (a preceding non-boundary only over-disqualifies, which is safe).
fn contains_word_followed_by_ws(source: &str, word: &str) -> bool {
    source.match_indices(word).any(|(i, _)| {
        source
            .as_bytes()
            .get(i + word.len())
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'\\'))
    })
}

/// Like [`lower_to_ir_with_dialect`] but with a memoised per-procedure body-lowering
/// callback: a top-level `proc`'s static body is lowered
/// through `body_cache` `(offset-0 body text, namespace) -> offset-0 Script` and
/// rebased, so an unchanged proc's body IR is reused across edits.  The caller must
/// only install a cache for **context-free** files (see [`Lowerer::body_cache`]);
/// byte-identity is guarded by the corpus differential gates.
///
/// The cached bodies must be produced by [`lower_proc_body_isolated`] under the
/// **same** `dialect`, since the dialect selects the expression grammar the
/// body's conditions are parsed with.
#[must_use]
pub fn lower_to_ir_with_body_cache(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    body_cache: &dyn Fn(&str, &str) -> Script,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config)
            .with_dialect(dialect)
            .with_body_cache(body_cache),
        source,
    )
}
