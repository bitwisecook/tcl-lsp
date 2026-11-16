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

//! Tokens for BIG-IP configuration and APL documents.

use super::emission::{Entry, encode_entries, push_object_token, push_span_entries};
use super::legend::{IAPPS_DIALECT, TokenKind};
use super::object_dispatch::WorkspaceTokenFacts;
use super::{SemanticTokens, collect_entries};
use tcl_lexer::LineIndex;
use tcl_registry::CommandRegistry;

/// Compute semantic tokens for an **APL** (iApp presentation) document.
///
/// APL is not Tcl — it is a declarative form-description grammar — so it does
/// not go through the Tcl segmenter at all.  Running the Tcl tokeniser over it
/// (which is what happens today for any document the server does not route
/// here) treats each braced block as one literal word and emits whole *lines*
/// as `String` tokens, actively mis-colouring the file.
///
/// The caller decides a document is APL with the server's `is_apl_source`
/// (language id `tcl-apl`, or a `*.apl` / `presentation` basename) — the
/// dialect string cannot be used for this, because `tcl-apl` and the Tcl
/// `tcl-iapp` *implementation* files both resolve to the `f5-iapps` dialect.
#[must_use]
pub fn apl_full(source: &str, registry: &CommandRegistry) -> SemanticTokens {
    encode_entries(&apl_entries(source, registry))
}

/// [`apl_full`] restricted to `range`, for viewport (`semanticTokens/range`)
/// requests.  Same half-open filter as the Tcl [`range`] path.
#[must_use]
pub fn apl_range(
    source: &str,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
) -> SemanticTokens {
    encode_entries(&clip_to_range(apl_entries(source, registry), range))
}

/// Compute semantic tokens for a **BIG-IP config** document (`bigip.conf`,
/// `.scf`).
///
/// Like APL (see [`apl_full`]), BIG-IP config text is not Tcl: it is a
/// brace-delimited declarative config.  The Tcl tokeniser reads each stanza
/// body as one literal braced word and emits whole *lines* as `String` tokens
/// — 272 of `samples/bigip/bigip.conf`'s 302 tokens are exactly that, which
/// mis-colours the file rather than merely under-colouring it.
///
/// The caller decides a document is BIG-IP config with the server's
/// `is_bigip_conf_name` / the `f5-bigip` dialect.
#[must_use]
pub fn bigip_conf_full(source: &str, registry: &CommandRegistry) -> SemanticTokens {
    encode_entries(&bigip_conf_entries(source, registry))
}

/// [`bigip_conf_full`] restricted to `range`.
#[must_use]
pub fn bigip_conf_range(
    source: &str,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
) -> SemanticTokens {
    encode_entries(&clip_to_range(bigip_conf_entries(source, registry), range))
}

/// Drop entries whose start falls outside `range` (half-open, per the LSP
/// `Range` semantics the Tcl [`range`] path uses).
fn clip_to_range(mut entries: Vec<Entry>, range: crate::definition::LspRange) -> Vec<Entry> {
    entries.retain(|(line, col, _, _, _)| {
        let pos = (*line, *col);
        let start = (range.start_line, range.start_character);
        let end = (range.end_line, range.end_character);
        pos >= start && pos < end
    });
    entries
}

/// Map the BIG-IP config lexer's tokens onto legend entries, and walk each
/// embedded iRule body as **Tcl**.
///
/// A `ltm rule /Common/x { … }` stanza's body is iRules code sitting inside a
/// config file.  Read as config it is nonsense — `when` / `if` / `switch`
/// become config *property keys* and `[HTTP::uri]` is not tokenised at all —
/// so the config lexer leaves those spans empty and they are re-walked here with
/// the iRules registry.  `registry` must therefore be the **iRules** one.
fn bigip_conf_entries(source: &str, registry: &CommandRegistry) -> Vec<Entry> {
    use tcl_bigip::conf_tokens::BigipTokenKind as B;

    let line_index = LineIndex::new(source);
    let mut entries: Vec<Entry> = Vec::new();

    // The Tcl regions are the bodies the shared rule parser recognises, ended
    // by the iRules brace grammar — not the config lexer's colour brackets,
    // which an escaped `\}` in the iRule would truncate.
    for rule in tcl_bigip::rule_extract::find_embedded_rules(source) {
        let (bstart, bend) = (rule.body_start_offset, rule.body_end_offset);
        let Some(body) = source.get(bstart..bend) else {
            continue;
        };
        // The body's tokens come back positioned relative to the body text, so
        // shift them onto the document: a token on the body's first line also
        // needs the column the body started at.
        let origin = line_index.position_at_utf16(u32::try_from(bstart).unwrap_or(0), source);
        for (line, col, len, kind, mods) in collect_entries(
            body,
            crate::profile_for_dialect("f5-irules"),
            registry,
            None,
            WorkspaceTokenFacts::default(),
        ) {
            let (line, col) = if line == 0 {
                (origin.line, origin.character.get() + col)
            } else {
                (origin.line + line, col)
            };
            entries.push((line, col, len, kind, mods));
        }
        // The same BIG-IP object-reference overlay a standalone `.irul` gets —
        // so `pool /Common/api_pool` inside a config's rule body reads as an
        // `object`, exactly as it would in its own file.  Spans come back
        // relative to the body, so shift them onto the document; the overlay
        // itself replaces the generic `string` the walk produced.
        for span in crate::irules_object_refs::object_ref_spans(body, registry) {
            let shifted = tcl_lexer::Span::new(
                span.start() + u32::try_from(bstart).unwrap_or(0),
                span.end() + u32::try_from(bstart).unwrap_or(0),
            );
            push_object_token(source, &line_index, shifted, &mut entries);
        }
    }
    for t in tcl_bigip::conf_tokens::tokenise_bigip_conf(source) {
        let kind = match t.kind {
            // BIG-IP-specific types.
            B::Partition => TokenKind::Partition,
            B::Pool => TokenKind::Pool,
            B::Monitor => TokenKind::Monitor,
            B::Profile => TokenKind::Profile,
            B::Vlan => TokenKind::Vlan,
            B::Interface => TokenKind::BigipInterface,
            B::IpAddress => TokenKind::IpAddress,
            B::Port => TokenKind::Port,
            B::RouteDomain => TokenKind::RouteDomain,
            B::Fqdn => TokenKind::Fqdn,
            B::Username => TokenKind::Username,
            B::Encrypted => TokenKind::Encrypted,
            B::Object => TokenKind::Object,
            // Shared primitives reuse the standard types.
            B::Comment => TokenKind::Comment,
            B::Keyword => TokenKind::Keyword,
            B::Property => TokenKind::OptionValue,
            B::Str => TokenKind::String,
            B::Escape => TokenKind::Escape,
            B::Number => TokenKind::Number,
        };
        let Some(text) = source.get(t.start as usize..t.end as usize) else {
            continue;
        };
        push_span_entries(
            source,
            &line_index,
            t.start as usize,
            text,
            kind,
            0,
            &mut entries,
        );
    }
    entries.sort_by_key(|(line, col, _, _, _)| (*line, *col));
    entries
}

/// Map the APL lexer's tokens onto legend entries, and walk each embedded Tcl
/// `[ … ]` region as Tcl.
///
/// APL embeds Tcl in bracket expressions, and the KCS feature doc is explicit
/// that they receive full Tcl highlighting.  The APL lexer leaves those spans
/// empty; they are re-walked here.
fn apl_entries(source: &str, registry: &CommandRegistry) -> Vec<Entry> {
    use tcl_bigip::apl::AplTokenKind as A;

    let line_index = LineIndex::new(source);
    let mut entries: Vec<Entry> = Vec::new();

    for (bstart, bend) in tcl_bigip::apl::embedded_tcl_regions(source) {
        let (bstart, bend) = (bstart as usize, bend as usize);
        let Some(body) = source.get(bstart..bend) else {
            continue;
        };
        let origin = line_index.position_at_utf16(u32::try_from(bstart).unwrap_or(0), source);
        for (line, col, len, kind, mods) in collect_entries(
            body,
            crate::profile_for_dialect(IAPPS_DIALECT),
            registry,
            None,
            WorkspaceTokenFacts::default(),
        ) {
            let (line, col) = if line == 0 {
                (origin.line, origin.character.get() + col)
            } else {
                (origin.line + line, col)
            };
            entries.push((line, col, len, kind, mods));
        }
    }
    for t in tcl_bigip::apl::tokenise_apl(source) {
        let kind = match t.kind {
            // APL-specific types.
            A::SectionKw => TokenKind::AplSection,
            A::FieldType => TokenKind::AplFieldType,
            A::Attribute => TokenKind::AplAttribute,
            A::SectionName => TokenKind::AplSectionName,
            A::FieldName => TokenKind::AplFieldName,
            A::Define => TokenKind::AplDefine,
            A::DefineName => TokenKind::AplDefineName,
            A::Directive => TokenKind::AplDirective,
            A::Optional => TokenKind::AplOptional,
            A::Validator => TokenKind::AplValidator,
            // Shared primitives reuse the standard types, so a theme that
            // styles Tcl strings/comments styles APL's the same way.
            A::Comment => TokenKind::Comment,
            A::Str => TokenKind::String,
            A::Number => TokenKind::Number,
            A::Variable => TokenKind::Variable,
            A::Operator => TokenKind::Operator,
            A::Escape => TokenKind::Escape,
        };
        let Some(text) = source.get(t.start as usize..t.end as usize) else {
            continue;
        };
        push_span_entries(
            source,
            &line_index,
            t.start as usize,
            text,
            kind,
            0,
            &mut entries,
        );
    }
    entries.sort_by_key(|(line, col, _, _, _)| (*line, *col));
    entries
}
