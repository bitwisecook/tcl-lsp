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

//! Written component geometry under an independently selected authored namespace.

use tcl_lexer::{LexerConfig, Span};

/// Interpretation of the original authored namespace occurrence.
#[derive(Clone, Copy)]
pub enum AuthoredNamespaceOccurrencePurpose {
    /// A namespace operand retains its own trailing separator meaning.
    Namespace,
    /// A command, variable or class name retains an empty terminal component.
    Entity,
    /// A variable occurrence with its independently selected lexer configuration.
    Variable(LexerConfig),
}

/// Source correspondence of an independently selected authored namespace.
#[derive(Debug, PartialEq, Eq)]
pub enum AuthoredNamespaceComponent {
    /// The selected component is present in this exact original source extent.
    Written(Span),
    /// The selected component belongs to the resolved enclosing namespace.
    Inherited,
}

/// Locate one authored namespace component in an original written occurrence.
/// The caller independently owns the resolved and selected namespace reports,
/// complete current source and occurrence purpose. Constructed-key partitioning
/// is analytical here; failed or computed geometry remains unavailable.
/// These coordinates supply no Native name input, namespace identity or edit
/// permission. An inherited component has no editable span in this occurrence.
#[must_use]
pub fn authored_namespace_component_extent(
    source: &str,
    word: Span,
    resolved: &str,
    cell: &str,
    purpose: AuthoredNamespaceOccurrencePurpose,
) -> Option<AuthoredNamespaceComponent> {
    let start = usize::try_from(word.start()).ok()?;
    let raw = source.get(word.as_range())?;
    let (decoration, text, braced_scalar) =
        if let AuthoredNamespaceOccurrencePurpose::Variable(config) = purpose {
            if raw.starts_with("${") {
                // The actual Var span may end immediately before its closer.
                // The shared grammar owner selects the original header in the
                // complete source; only its contained name supplies geometry.
                let (name, _) =
                    super::split_braced_var_ref(source.get(start..)?, config.braced_var)?;
                if 2usize.checked_add(name.len())? > raw.len() {
                    return None;
                }
                (2usize, name, true)
            } else if let Some(rest) = raw.strip_prefix('$') {
                (1usize, rest, false)
            } else {
                (0usize, raw, false)
            }
        } else {
            (0usize, raw, false)
        };
    let text = if matches!(purpose, AuthoredNamespaceOccurrencePurpose::Variable(_)) {
        let root = super::split_element_ref_bytes(text.as_bytes())
            .map_or(text.as_bytes(), |(root, _)| root);
        text.get(..root.len())?
    } else {
        text
    };
    // Direct unbraced geometry cannot authenticate an escaped/computed
    // container. The shared braced header already owns its literal name.
    if !braced_scalar && text.contains(['$', '[', '\\']) {
        return None;
    }
    let cell_segments = super::key_segments(cell);
    let resolved_segments = super::key_segments(resolved);
    if cell_segments.is_empty() || !resolved_segments.starts_with(&cell_segments) {
        return None;
    }
    let extents = super::qualifier_component_extents(text.as_bytes()).collect::<Vec<_>>();
    let mut written_segments = extents
        .iter()
        .map(|extent| text.get(extent.clone()))
        .collect::<Option<Vec<_>>>()?;
    if !matches!(purpose, AuthoredNamespaceOccurrencePurpose::Namespace)
        && super::ends_with_separator(text.as_bytes())
    {
        written_segments.push("");
    }
    let unwritten = resolved_segments
        .len()
        .checked_sub(written_segments.len())?;
    if (text.starts_with("::") && unwritten != 0)
        || !resolved_segments[unwritten..]
            .iter()
            .map(String::as_str)
            .eq(written_segments.iter().copied())
    {
        return None;
    }
    let Some(index) = (cell_segments.len() - 1).checked_sub(unwritten) else {
        return Some(AuthoredNamespaceComponent::Inherited);
    };
    let extent = extents.get(index)?;
    let at = start.checked_add(decoration)?;
    Some(AuthoredNamespaceComponent::Written(Span::new(
        u32::try_from(at.checked_add(extent.start)?).ok()?,
        u32::try_from(at.checked_add(extent.end)?).ok()?,
    )))
}
