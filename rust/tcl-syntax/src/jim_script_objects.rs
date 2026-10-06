// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original Jim Script objects built from the selected full token layout.

use crate::jim_script_layout::{
    JimScriptLayoutEntry, JimScriptLayoutUnavailable, prepare_jim_script_layout,
};
use crate::value::ValueError;
use std::ops::Range;
use tcl_lexer::{JimScriptLine, JimScriptTokenKind, JimScriptTokens};

/// One actual object constructor selected by the native Script layout.
#[derive(Debug, Clone, Copy)]
pub enum JimScriptObjectConstruction<'a> {
    /// Empty resident `ScriptLine`, distinct from a numeric object.
    Line {
        /// Original command argument count.
        argc: i32,
        /// Original parser line relative to the retained source baseline.
        line_delta: u32,
    },
    /// Pure native integer word count, including expansion's negative count.
    Word(i32),
    /// Original token with selected native escape processing already applied.
    Source {
        /// Native substitution or literal token purpose.
        kind: JimScriptTokenKind,
        /// Counted original token value after selected escape processing.
        bytes: &'a [u8],
        /// Original parser line relative to the retained source baseline.
        line_delta: u32,
    },
}

/// Purpose of one retained native Script token object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JimScriptObjectKind {
    /// Native command argument count and source line.
    Line {
        /// Original command argument count.
        argc: i32,
        /// Original parser line relative to the retained source baseline.
        line_delta: u32,
    },
    /// Native adjoining-token count.
    Word(i32),
    /// Original substituted or literal token purpose.
    Source(JimScriptTokenKind),
}

/// A token object retained once by its native Script backing.
pub struct JimScriptObject<V> {
    /// Native token purpose.
    pub kind: JimScriptObjectKind,
    /// Original object; inspection need not clone the handle.
    pub value: V,
}

/// Original objects for the whole native Script, including malformed tails.
pub struct JimScriptObjects<V> {
    tokens: Vec<JimScriptObject<V>>,
    /// Original first parser-token line delta.
    pub first_line_delta: u32,
    /// Mutable execution starts with the native parser's completeness line.
    pub completeness_line: JimScriptLine,
    /// Native missing marker; absent represents native space.
    pub missing: Option<u8>,
}

/// One original word, borrowing a range of the retained token backing.
#[derive(Debug)]
pub struct JimScriptWord {
    /// Original real token entries, excluding the WORD marker.
    pub tokens: Range<usize>,
    /// Expand the original resulting list object into argv.
    pub expand: bool,
}

/// One original command selected from actual LINE and WORD entries.
#[derive(Debug)]
pub struct JimScriptCommand {
    /// Native source line delta, selected before substitutions.
    pub line_delta: u32,
    /// Original words in evaluation order.
    pub words: Vec<JimScriptWord>,
    /// Next LINE entry, or the end of the actual backing.
    pub next: usize,
}

impl<V> JimScriptObjects<V> {
    /// Construct every original token once, before any command executes.
    ///
    /// # Errors
    /// Preserves missing roster geometry and backend object-access refusals.
    pub fn prepare(
        roster: &JimScriptTokens,
        mut construct: impl FnMut(JimScriptObjectConstruction<'_>) -> Result<V, ValueError>,
    ) -> Result<Self, ValueError> {
        let layout = prepare_jim_script_layout(roster).map_err(layout_unavailable)?;
        let mut tokens = Vec::with_capacity(layout.entries.len());
        for entry in layout.entries {
            let (kind, value) = match entry {
                JimScriptLayoutEntry::Line { argc, line_delta } => (
                    JimScriptObjectKind::Line { argc, line_delta },
                    construct(JimScriptObjectConstruction::Line { argc, line_delta })?,
                ),
                JimScriptLayoutEntry::Word(count) => (
                    JimScriptObjectKind::Word(count),
                    construct(JimScriptObjectConstruction::Word(count))?,
                ),
                JimScriptLayoutEntry::Token { roster_index } => {
                    let token = &roster.tokens[roster_index];
                    let original = roster.image.bytes().get(token.value.as_range()).ok_or(
                        ValueError::CommandProtocolUnavailable("Jim Script original token extent"),
                    )?;
                    let decoded = if token.kind == JimScriptTokenKind::Escaped {
                        crate::backslash::decode_bytes_in(original, tcl_dialect::EscapeSyntax::Jim)
                    } else {
                        std::borrow::Cow::Borrowed(original)
                    };
                    let value = construct(JimScriptObjectConstruction::Source {
                        kind: token.kind,
                        bytes: &decoded,
                        line_delta: token.line_delta,
                    })?;
                    (JimScriptObjectKind::Source(token.kind), value)
                }
            };
            tokens.push(JimScriptObject { kind, value });
        }
        Ok(Self {
            tokens,
            first_line_delta: layout.first_line_delta,
            completeness_line: layout.completeness_line,
            missing: layout.missing,
        })
    }

    /// Borrow original token objects without adding child ownership.
    #[must_use]
    pub fn tokens(&self) -> &[JimScriptObject<V>] {
        &self.tokens
    }

    /// Read the next native command without recreating its words or objects.
    ///
    /// # Errors
    /// Refuses an inconsistent native backing rather than guessing boundaries.
    pub fn command_at(&self, at: usize) -> Result<Option<JimScriptCommand>, ValueError> {
        let Some(first) = self.tokens.get(at) else {
            return Ok(None);
        };
        let JimScriptObjectKind::Line { argc, line_delta } = first.kind else {
            return Err(backing_unavailable());
        };
        let argc = usize::try_from(argc).map_err(|_| backing_unavailable())?;
        let mut words = Vec::with_capacity(argc);
        let mut at = at + 1;
        for _ in 0..argc {
            let (count, expand) = match self.tokens.get(at).map(|token| token.kind) {
                Some(JimScriptObjectKind::Word(count)) => {
                    at += 1;
                    (
                        usize::try_from(count.unsigned_abs()).map_err(|_| backing_unavailable())?,
                        count < 0,
                    )
                }
                Some(JimScriptObjectKind::Source(_)) => (1, false),
                _ => return Err(backing_unavailable()),
            };
            let end = at.checked_add(count).ok_or_else(backing_unavailable)?;
            let entries = self.tokens.get(at..end).ok_or_else(backing_unavailable)?;
            if entries
                .iter()
                .any(|token| !matches!(token.kind, JimScriptObjectKind::Source(_)))
            {
                return Err(backing_unavailable());
            }
            words.push(JimScriptWord {
                tokens: at..end,
                expand,
            });
            at = end;
        }
        Ok(Some(JimScriptCommand {
            line_delta,
            words,
            next: at,
        }))
    }
}

fn layout_unavailable(_: JimScriptLayoutUnavailable) -> ValueError {
    ValueError::CommandProtocolUnavailable("Jim Script native token layout")
}
fn backing_unavailable() -> ValueError {
    ValueError::CommandProtocolUnavailable("Jim Script original token backing")
}

/// Native completeness presentation before any command executes or result resets.
#[must_use]
pub fn missing_message(marker: Option<u8>) -> Option<&'static [u8]> {
    match marker {
        None | Some(b' ' | b'\\') => None,
        Some(b'[') => Some(b"unmatched \"[\""),
        Some(b'{') => Some(b"missing close-brace"),
        Some(b'}') => Some(b"extra characters after close-brace"),
        Some(_) => Some(b"missing quote"),
    }
}
