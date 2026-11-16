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

//! The top-level declarations of an iRule (`when`, `priority`, `timing`): their clauses, argument slots and handler priorities.

use tcl_lexer::{Token, TokenType};

/// How an iRules event-handler command treats an omitted priority clause.
///
/// BIG-IP gives `when` handlers a documented default priority of 500, so an
/// omitted clause is valid and does not itself justify a diagnostic. A
/// dialect can opt into an explicit-priority policy without teaching the
/// analyser a handler command name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventHandlerPriority {
    /// Keyword that introduces an explicit priority value.
    pub keyword: &'static str,
    /// Priority used by the runtime when the keyword is omitted.
    pub default_priority: u16,
    /// The smallest valid priority value (inclusive).
    pub min_priority: u16,
    /// The largest valid priority value (inclusive).
    pub max_priority: u16,
    /// Whether a lower number runs first (ascending execution order).
    pub lower_runs_first: bool,
    /// Whether an omitted priority is reportable in this dialect.
    pub warn_when_implicit: bool,
}

impl EventHandlerPriority {
    /// Whether `value` is a valid priority under this policy.
    #[must_use]
    pub fn accepts(&self, value: u16) -> bool {
        (self.min_priority..=self.max_priority).contains(&value)
    }
}

/// BIG-IP's `when` priority policy, live-measured
/// (`docs/design/f5/bigip-irule-parser-measurements.md` §6/§8): priority is
/// **0–1000 inclusive** (`1001` and `-1` are rejected at rule load, albeit
/// with the misleading `unexpected extra argument` wording), the default
/// is **500** — the midpoint — and **lower numbers run first** (attached
/// as `{low default high}`, execution order was priority 100, then the
/// implicit 500, then 900; an `HTTP::respond` in an earlier rule did not
/// stop later ones).
pub const BIGIP_EVENT_HANDLER_PRIORITY: EventHandlerPriority = EventHandlerPriority {
    keyword: "priority",
    default_priority: 500,
    min_priority: 0,
    max_priority: 1000,
    lower_runs_first: true,
    warn_when_implicit: false,
};

/// Lexical execution context of an iRules command invocation.
///
/// The analyser owns the structural classification; the registry owns which
/// command traits are legal in each context. Nested control-flow bodies inherit
/// their enclosing event or procedure context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrulesExecutionContext {
    /// File-level declaration surface.
    TopLevel,
    /// A `when EVENT` body, including its nested control flow.
    EventBody,
    /// A top-level `proc` body, including its nested control flow.
    ProcedureBody,
    /// A body reached from an already-invalid top-level executable command.
    InvalidNestedBody,
}

/// A fully valid declaration on iRules' declaration-only file surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrulesTopLevelDeclaration {
    /// A known event handler and its registry-owned body argument.
    Event {
        /// Canonical uppercase event name.
        event: String,
        /// Zero-based argument index of the handler body.
        body_index: usize,
        /// Explicit handler priority, when the declaration supplied one.
        priority: Option<u16>,
    },
    /// A procedure definition and its registry-owned name/body arguments.
    Procedure {
        /// Zero-based argument index of the procedure name.
        name_index: usize,
        /// Zero-based argument index of the procedure body.
        body_index: usize,
    },
    /// Change the inherited priority for subsequent event declarations.
    Priority {
        /// Valid BIG-IP priority (`0..=1000`).
        value: u16,
    },
    /// Toggle timing statistics for subsequent event declarations.
    Timing {
        /// Whether timing is enabled.
        enabled: bool,
    },
}

/// Source-structure facts for the arguments of an iRules declaration.
///
/// Declaration syntax is not determined by Tcl argument values alone: both
/// `when EVENT {body}` and `proc name {args} {body}` require their body to be
/// one braced source word.  The registry owns that requirement, while callers
/// supply the lexer facts that distinguish a braced word from a bare, quoted,
/// or compound one.
#[derive(Debug, Clone, Copy)]
pub struct IrulesDeclarationArguments<'a> {
    values: &'a [&'a str],
    tokens: &'a [Token],
    single_tokens: &'a [bool],
    closed_braced_tokens: &'a [bool],
}

impl<'a> IrulesDeclarationArguments<'a> {
    /// Construct a source-aware declaration argument view.
    ///
    /// Returns `None` unless every value has exactly one representative token
    /// and one single-token fact.
    #[must_use]
    pub fn new(
        values: &'a [&'a str],
        tokens: &'a [Token],
        single_tokens: &'a [bool],
        closed_braced_tokens: &'a [bool],
    ) -> Option<Self> {
        (values.len() == tokens.len()
            && values.len() == single_tokens.len()
            && values.len() == closed_braced_tokens.len())
        .then_some(Self {
            values,
            tokens,
            single_tokens,
            closed_braced_tokens,
        })
    }

    /// Decoded Tcl argument values, in source order.
    #[must_use]
    pub const fn values(self) -> &'a [&'a str] {
        self.values
    }

    /// Whether `index` is exactly one *closed* braced literal source word.
    #[must_use]
    pub fn is_braced_literal(self, index: usize) -> bool {
        self.single_tokens.get(index).copied().unwrap_or(false)
            && self
                .tokens
                .get(index)
                .is_some_and(|token| token.kind == TokenType::Str)
            && self
                .closed_braced_tokens
                .get(index)
                .copied()
                .unwrap_or(false)
    }
}

/// Determine which declaration argument words are closed braced literals.
///
/// A lexer [`TokenType::Str`] represents both `{body}` and recovery's
/// unterminated `{body`, so this source fact is deliberately separate from
/// token kind.  Callers retain the resulting parallel vector and hand it to
/// [`IrulesDeclarationArguments::new`], keeping declaration admission
/// identical across lowering, analysis, identity, and inventories.
#[must_use]
pub fn closed_braced_argument_words(
    source: &str,
    tokens: &[Token],
    single_tokens: &[bool],
) -> Option<Vec<bool>> {
    (tokens.len() == single_tokens.len()).then(|| {
        tokens
            .iter()
            .zip(single_tokens)
            .map(|(token, single)| {
                *single
                    && token.kind == TokenType::Str
                    && tcl_lexer::word_span_at(source, token.span)
                        .end()
                        .checked_sub(1)
                        .and_then(|close| source.as_bytes().get(close as usize))
                        == Some(&b'}')
            })
            .collect()
    })
}

/// The optional clauses of an iRules event declaration
/// (`when EVENT ?priority N? ?timing V? { body }`). Each clause is its keyword
/// plus one value word and appears at most once, in either order: the tmsh
/// synopsis writes `priority` first, the iRules `when` reference `[timing
/// on|off] [priority N]` (#2082).
pub const IRULES_EVENT_CLAUSES: [&str; 2] = [IRULES_PRIORITY_CLAUSE, IRULES_TIMING_CLAUSE];

/// The `priority N` clause keyword of an event declaration.
pub const IRULES_PRIORITY_CLAUSE: &str = "priority";

/// The `timing V` clause keyword of an event declaration.
pub const IRULES_TIMING_CLAUSE: &str = "timing";

/// Every spelling a `timing` value accepts, in an event declaration's clause
/// and in the file-level `timing` command alike.
pub const IRULES_TIMING_SPELLINGS: [&str; 4] = ["on", "off", "enable", "disable"];

/// What an incomplete iRules event declaration takes at one argument
/// position, read off the same layout
/// [`CommandRegistry::irules_event_declaration_shape`](crate::CommandRegistry::irules_event_declaration_shape)
/// validates. Advice for completion only: it grants no body entry, frame or
/// handler identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrulesDeclarationSlot {
    /// The event name.
    Event,
    /// A clause keyword not yet written (the spec's clause-keyword values,
    /// narrowed to the clauses that may still follow), or the body.
    Clause(Vec<&'static crate::ArgValue>),
    /// The value of `priority`: a number under
    /// [`BIGIP_EVENT_HANDLER_PRIORITY`]; there is no word list to offer.
    Priority,
    /// The value of `timing`: the spec's timing values.
    Timing(Vec<&'static crate::ArgValue>),
    /// Only the handler body may follow.
    Body,
}

/// Stateful effect of an iRules-only top-level declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrulesTopLevelEffect {
    /// `priority N` changes the inherited priority of following handlers.
    Priority,
    /// `timing on|off|enable|disable` changes timing collection.
    Timing,
}

/// Result of applying the registry-owned iRules placement contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrulesCommandPlacement {
    /// The invocation is permitted in this context.
    Allowed,
    /// The command is a declaration form and must be at file top level.
    RequiresTopLevel,
    /// The command is executable and must be inside an event or procedure.
    RequiresEventOrProcedure,
}
