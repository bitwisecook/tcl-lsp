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

//! Class systems: class and object definitions, class factories and the classes a document manufactures under computed names.

use super::super::state::Analyser;
use super::super::types::ClassFactory;
use tcl_lexer::{Span, Token, TokenType};
use tcl_source::segment::SegmentedCommand;

mod class_definitions;
mod class_factories;
mod control_arms;
mod deferred_creations;
mod object_definitions;
mod parameterised_creations;
mod static_provenance;
mod static_reachability;
mod unknown_dispatch;
mod workspace_class_factory;

#[cfg(test)]
pub(super) use class_definitions::original_class_definition_body_in_analysis;
#[cfg(test)]
pub(super) use class_factories::{ProloguePiece, prologue_pieces, resolve_factory_member};
#[cfg(test)]
pub(super) use parameterised_creations::{
    ParameterisedCreation, join_parameterised_class_observations,
};
#[cfg(test)]
pub(super) use static_provenance::substitute_bound_words;

/// A recorded class that is itself a `TclOO` class factory, with the
/// definition-body grammar the bodies it manufactures obey.
///
/// See [`Analyser::user_metaclass_of_command`].
pub(super) struct UserMetaclass {
    /// The registry metaclass command at the root of the chain
    /// (`oo::class`), whose definition-body grammar governs the bodies this
    /// factory makes.
    root_command: String,
    /// Original relation naming policy and selected root candidate.
    root_policy: tcl_syntax::naming::NamePolicyProtocol,
    root_slot: tcl_core_types::ByteCommandSlot,
    /// The root registry metaclass's definition-body grammar.
    pub(super) grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
}

/// One definition-body member a class factory injects into every class it
/// manufactures, with each word resolved to a real source token.
pub(super) struct InjectedMember {
    /// Member keyword followed by its argument words.
    pub(super) texts: Vec<String>,
    /// Parallel tokens — each in the manufacturer's own body (a literal it
    /// always splices) or in the creation call (a `{*}$param` splice).
    pub(super) argv: Vec<Token>,
}

pub(super) struct ManufacturerLayout {
    /// Argument index of the new class's name.
    pub(super) name_arg: usize,
    /// Argument index of the new class's definition body.
    pub(super) body_arg: usize,
    /// Members the manufacturer always injects.
    pub(super) injected: Vec<InjectedMember>,
    /// `true` when the manufacturer override could not be read, so the
    /// class's superclass list is unknown — see
    /// [`super::super::types::ClassDef::inheritance_unknown`].
    pub(super) inheritance_unknown: bool,
}

/// Registry/user-factory facts needed after a class-manufacturing call has
/// been classified.
pub(super) struct ResolvedClassCreation {
    /// Published factory proof for a user-defined metaclass, absent for a
    /// registry definer.
    user_factory: Option<ClassFactory>,
    /// Descriptor selected by the shared catalogue owner for this written head.
    pub(super) registry_metaclass: Option<&'static tcl_registry::CommandSpec>,
    /// Name/body positions and injected definition members.
    layout: ManufacturerLayout,
}

/// How deep [`Analyser::collect_nested_statements`] descends into nested
/// script words before giving up.
///
/// A metaclass's unknown-dispatch member is a handful of lines in every real
/// corpus instance; the cap only bounds a pathological or generated body, and
/// stopping early can only *lose* evidence, which abstains.
const MAX_UNKNOWN_BODY_DEPTH: u32 = 8;

#[derive(Clone)]
pub(super) struct ControlArm {
    pub(super) controller: SegmentedCommand,
    pub(super) body_span: Span,
}

impl Analyser {
    /// The definition-body grammar attached to `cmd_name`'s spec, if it is a
    /// definer command.  Registry-sourced, so member recognition / argument
    /// structure in the body walkers never hardcodes a keyword list — a new
    /// definer is picked up the moment its spec carries a `definition_body`.
    pub(in super::super) fn definition_grammar(
        &self,
        cmd_name: &str,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        let context = self.analysis_context();
        context
            .context()
            .resolve_spec(context.commands(), cmd_name)?
            .definition_body
    }

    /// Registry-driven recursive script-block collection for broad structural
    /// scans that do not claim control-flow reachability.
    fn collect_script_blocks(
        &self,
        text: &str,
        base: u32,
        depth: u32,
        out: &mut Vec<Vec<SegmentedCommand>>,
    ) {
        if depth >= MAX_UNKNOWN_BODY_DEPTH {
            return;
        }
        let Some(context) = self.retained_static_source_context() else {
            return;
        };
        let registry = context.commands().as_ref();
        let block = tcl_source::segment::segment_commands_with_offset_and_config(
            text,
            base,
            self.file_lexer_config().nested(),
        );
        for seg in &block {
            let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            for idx in tcl_registry::ArgRole::ALL
                .iter()
                .filter(|role| role.carries_script())
                .flat_map(|&role| registry.arg_indices_for_role(seg.name(), &args, role))
            {
                let (Some(word), Some(tok)) = (seg.args().get(idx), seg.argv.get(idx + 1)) else {
                    continue;
                };
                // Only a braced word is a script whose text is exactly what
                // runs; a substituted or quoted one is composed at run time
                // and re-segmenting its written form would read code Tcl
                // never evaluates.
                if tok.kind != TokenType::Str {
                    continue;
                }
                let inner_base = tok
                    .span
                    .start()
                    .saturating_add(u32::from(tok.content_offset));
                self.collect_script_blocks(word, inner_base, depth + 1, out);
            }
        }
        out.push(block);
    }

    /// The actual source input held by this walk, including availability.
    /// Transient walk caches and reconstructed environment labels cannot
    /// replace its original source, full configuration or realm.
    pub(super) fn retained_static_source_context(
        &self,
    ) -> Option<std::sync::Arc<tcl_registry::model::ContextRegistry>> {
        let input = self.result.resolved_input.as_ref()?;
        let config = self.file_lexer_config();
        if self
            .resolved_input
            .as_ref()
            .is_some_and(|current| current != input)
            || self.profile.cache_key() != input.analyser_profile().cache_key()
            || self.unit_profile.unwrap_or(self.profile).cache_key()
                != input.unit_profile().cache_key()
            || input.lexer_config() != config
            || !self.result.matches_original_source_image(
                &tcl_lexer::SourceImage::document(&self.source),
                config,
            )
            || !self.head_identities.matches_resolved_analysis_input(input)
            || self.retained_source_advice_purpose().is_none()
        {
            return None;
        }
        Some(input.context_registry())
    }

    /// Direct statements only (no nested body flattening), used by the
    /// static provenance evaluator so a non-selected branch cannot leak a
    /// binding into the outer path.
    pub(super) fn direct_statements_in_span(&self, span: Span) -> Option<Vec<SegmentedCommand>> {
        let start = span.start() as usize;
        let end = span.end() as usize;
        let raw = self.source.get(start..end)?;
        let (body, base) = if matches!(raw, "}" | "{}") {
            // A closed empty braced word has the lexer's degenerate span on
            // its closing delimiter. It contains no script to execute.
            ("", start)
        } else {
            raw.strip_prefix(['{', '"'])
                .map_or((raw, start), |inner| (inner, start.saturating_add(1)))
        };
        Some(
            tcl_source::segment::segment_commands_with_offset_and_config(
                body,
                u32::try_from(base).unwrap_or(0),
                self.file_lexer_config().nested(),
            ),
        )
    }

    /// The statements of the script at `span`, plus those of every nested
    /// script word, re-segmented from the document's own bytes.
    ///
    /// `span` may be a body **word**'s span (whose start sits on the opening
    /// delimiter) or a plain script range; the delimiter is stripped when
    /// present, matching [`Self::manufacturer_next_call`].
    fn statements_in_span(&self, span: Span) -> Option<Vec<SegmentedCommand>> {
        let start = span.start() as usize;
        let end = span.end() as usize;
        if start >= end || end > self.source.len() {
            return None;
        }
        let raw = self.source.get(start..end)?;
        let (body, base) = raw
            .strip_prefix(['{', '"'])
            .map_or((raw, start), |inner| (inner, start.saturating_add(1)));
        let mut blocks = Vec::new();
        self.collect_script_blocks(body, u32::try_from(base).unwrap_or(0), 0, &mut blocks);
        Some(blocks.into_iter().flatten().collect())
    }
}
