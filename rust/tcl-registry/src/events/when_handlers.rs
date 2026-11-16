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

//! Finding the `when` handlers at the top level of an iRule source.

use super::EventRegistry;
use super::top_level_declarations::{BIGIP_EVENT_HANDLER_PRIORITY, IrulesTopLevelDeclaration};
use rustc_hash::FxHashSet;
use std::borrow::Cow;
use tcl_lexer::{LexerConfig, SourceSegmentationPurpose};

/// Parse top-level iRules event handlers through the shared syntax owner.
#[must_use]
pub fn top_level_when_handlers(source: &str) -> Vec<tcl_syntax::event_handler::EventHandler> {
    let resolver = WrittenHeadResolver;
    top_level_when_handlers_with_registry_and_head_resolver(
        source,
        crate::model::ingress::static_context_for("f5-irules").commands(),
        &resolver,
    )
}

/// Resolve a written command head at its absolute document offset.
///
/// The registry owns the top-level script boundary because command specs
/// declare which words are executable scripts. It cannot itself infer Tcl
/// command-table mutations, so a compiler or editor that has such facts
/// supplies them here.
/// An empty answer deliberately means a proven rebound: it cannot resolve a
/// registry spec or open a script surface.
pub trait CommandHeadResolver {
    /// Return the effective registry spelling for `written` at `offset`.
    fn resolve<'a>(&'a self, written: &str, offset: u32) -> Cow<'a, str>;
}

/// The lexical fallback for consumers without document identity facts.
pub(crate) struct WrittenHeadResolver;

impl CommandHeadResolver for WrittenHeadResolver {
    fn resolve<'a>(&'a self, written: &str, _offset: u32) -> Cow<'a, str> {
        Cow::Owned(written.to_owned())
    }
}

/// Whether an effective head opens the shared event-handler boundary grammar.
///
/// This is exclusively a registry trait on the effective, offset-resolved
/// command binding. In particular, a Tcl user proc spelled `when` is not an
/// iRules event handler merely because its written head happens to match.
fn is_event_handler_head(registry: &crate::CommandRegistry, resolved: &str) -> bool {
    registry
        .get(resolved)
        .is_some_and(|spec| spec.traits.contains(crate::Traits::IS_EVENT_HANDLER))
}

/// Parse top-level event handlers after resolving each command head.
///
/// Handler layout remains syntax-owned, while the registry decides whether the
/// effective command carries [`crate::Traits::IS_EVENT_HANDLER`].
#[must_use]
pub fn top_level_when_handlers_with_registry_and_head_resolver(
    source: &str,
    registry: &crate::CommandRegistry,
    resolver: &dyn CommandHeadResolver,
) -> Vec<tcl_syntax::event_handler::EventHandler> {
    let config = registry.profile().map_or_else(
        || LexerConfig::for_file_dialect("f5-irules"),
        |profile| LexerConfig::from_grammar(profile.grammar),
    );
    top_level_when_handlers_with_source_config_and_head_resolver(
        source,
        registry,
        config,
        SourceSegmentationPurpose::WholeInput,
        resolver,
    )
}

/// Parse handlers under the producer's complete source configuration and
/// explicitly selected local script purpose. The retained parent configuration
/// remains independent of this parser selection. Command identity comes only
/// from the supplied resolver; this boundary parser grants no event execution.
#[must_use]
pub fn top_level_when_handlers_with_source_config_and_head_resolver(
    source: &str,
    registry: &crate::CommandRegistry,
    parent_config: LexerConfig,
    purpose: SourceSegmentationPurpose,
    resolver: &dyn CommandHeadResolver,
) -> Vec<tcl_syntax::event_handler::EventHandler> {
    let config = purpose.config(parent_config);
    let events = EventRegistry::build();
    let mut handlers = tcl_syntax::event_handler::event_handlers_with_head_predicate(
        source,
        config,
        |written, offset| {
            let effective_head = resolver.resolve(written, offset);
            is_event_handler_head(registry, effective_head.as_ref())
        },
    );
    handlers.retain(|handler| {
        let args: Vec<&str> = handler.arguments.iter().map(String::as_str).collect();
        crate::events::IrulesDeclarationArguments::new(
            &args,
            &handler.argument_tokens,
            &handler.argument_single_tokens,
            &handler.argument_closed_braced_tokens,
        )
        .and_then(|arguments| registry.irules_event_declaration(arguments, &events))
        .is_some()
    });
    apply_inherited_priorities(source, config, registry, resolver, &mut handlers);
    handlers
}

fn apply_inherited_priorities(
    source: &str,
    config: LexerConfig,
    registry: &crate::CommandRegistry,
    resolver: &dyn CommandHeadResolver,
    handlers: &mut [tcl_syntax::event_handler::EventHandler],
) {
    use tcl_lexer::{SourceMap, word_span_at};

    let sm = SourceMap::new(source);
    let mut inherited = BIGIP_EVENT_HANDLER_PRIORITY.default_priority;
    let mut next_handler = 0usize;
    for command in tcl_syntax::event_handler::script_commands(source, config) {
        let Some([head]) = command.words.first().map(Vec::as_slice) else {
            continue;
        };
        let offset = word_span_at(source, head.span).start();
        while handlers
            .get(next_handler)
            .is_some_and(|handler| handler.span.start() < offset)
        {
            next_handler += 1;
        }
        if let Some(handler) = handlers
            .get_mut(next_handler)
            .filter(|handler| handler.span.start() == offset)
        {
            handler.effective_priority = handler
                .priority
                .and_then(|value| u16::try_from(value).ok())
                .unwrap_or(inherited);
            next_handler += 1;
            continue;
        }
        let effective = resolver.resolve(sm.token_text(*head), offset);
        let name = effective.as_ref();
        let Some(args) = command
            .words
            .iter()
            .skip(1)
            .map(|word| match word.as_slice() {
                [token] => Some(sm.token_text(*token)),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        if let Some(IrulesTopLevelDeclaration::Priority { value }) =
            registry.irules_top_level_effect(name, &args)
        {
            inherited = value;
        }
    }
}

/// Parse syntactically valid top-level event declarations without filtering
/// unknown event names. This is exclusively for unknown-event diagnostics;
/// executable inventories must use [`top_level_when_handlers_with_registry_and_head_resolver`].
#[must_use]
pub fn top_level_when_handler_candidates_with_registry_and_head_resolver(
    source: &str,
    registry: &crate::CommandRegistry,
    resolver: &dyn CommandHeadResolver,
) -> Vec<tcl_syntax::event_handler::EventHandler> {
    let config = registry.profile().map_or_else(
        || LexerConfig::for_file_dialect("f5-irules"),
        |profile| LexerConfig::from_grammar(profile.grammar),
    );
    tcl_syntax::event_handler::event_handlers_with_head_predicate(
        source,
        config,
        |written, offset| {
            let effective_head = resolver.resolve(written, offset);
            is_event_handler_head(registry, effective_head.as_ref())
        },
    )
    .into_iter()
    .filter(|handler| {
        let args: Vec<&str> = handler.arguments.iter().map(String::as_str).collect();
        crate::events::IrulesDeclarationArguments::new(
            &args,
            &handler.argument_tokens,
            &handler.argument_single_tokens,
            &handler.argument_closed_braced_tokens,
        )
        .and_then(|arguments| registry.irules_event_declaration_shape(arguments))
        .is_some()
    })
    .collect()
}

/// Scan distinct top-level event handlers through the shared Tcl syntax
/// owner. Names are normalised to upper case and returned in first-seen order;
/// the caller orders them canonically via [`EventRegistry::order_events`].
#[must_use]
pub fn scan_when_events(source: &str) -> Vec<String> {
    let mut seen = FxHashSet::default();
    let mut out = Vec::new();
    for handler in top_level_when_handlers(source) {
        if seen.insert(handler.event.clone()) {
            out.push(handler.event);
        }
    }
    out
}
