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

//! What a literal argument holds: `binary` template gates (W200, W202), a subnet mask (W121) and non-ASCII text (W108).

use super::super::helpers::{find_dotted_quads, source_slice};
use crate::analyser::state::Analyser;
use rustc_hash::FxHashSet;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SpecProvider;

impl Analyser {
    /// W200/W202: version gates on a literal `binary` template.
    ///
    /// Site selection uses the original source schema's shared
    /// `FormatType::Binary` projection. Effective alias ordinals retain their
    /// own written operand anchors; a captured prefix supplies no call-site
    /// span. Unknown template values and rebound heads supply no template.
    ///
    /// The field grammar itself comes from the binary owner
    /// (`tcl_cmd_core::binary::specifiers`), parsed with the suffix
    /// admitted so the gate — not the parse — decides; sites are
    /// buffered and settled post-walk against the effective Tcl version
    /// (§6 argument-DSL rung).
    ///
    /// W200 is the `u` suffix (TIP 275, Tcl 8.5). W202 is a field
    /// letter that does not exist on the target at all (`t n m r R q
    /// Q`, also 8.5); its floor comes from
    /// `tcl_cmd_core::binary::specifier_min_version`. They are separate
    /// codes because the fixes differ: a suffix can be dropped, an
    /// absent letter needs a different field.
    ///
    /// One site per code per template: every field shares the template
    /// token's span, so several gated fields give one squiggle each,
    /// not one per field.
    pub(in crate::analyser) fn emit_binary_field_version_gates(
        &mut self,
        templates: &[crate::analyser::commands::OriginalFormatTemplate],
    ) {
        for template in templates {
            if template.format.kind != tcl_registry::patterns::FormatType::Binary {
                continue;
            }
            self.push_binary_field_gates(&template.bytes, template.span);
        }
    }

    /// Buffer the W200/W202 gate sites for one literal template.
    fn push_binary_field_gates(&mut self, fmt: &[u8], span: tcl_lexer::Span) {
        let fields = tcl_cmd_core::binary::specifiers(fmt, true);
        if fields.iter().any(|f| f.modifier == Some(b'u')) {
            self.dsl_gate_sites
                .push(super::super::version_gate::DslGateSite {
                    span,
                    code: DiagCode::W200,
                    what: "unsigned modifier 'u' on binary format specifier".to_string(),
                    min: tcl_dialect::TclVersion::V8_5,
                });
        }

        // Gated field letters, deduped and reported in template order so
        // the message is stable regardless of how often each appears.
        let mut gated: Vec<(u8, tcl_dialect::TclVersion)> = Vec::new();
        for f in &fields {
            if let Some(min) = tcl_cmd_core::binary::specifier_min_version(f.letter)
                && !gated.iter().any(|(l, _)| *l == f.letter)
            {
                gated.push((f.letter, min));
            }
        }
        if let Some(min) = gated.iter().map(|(_, m)| *m).max() {
            let letters = gated
                .iter()
                .map(|(l, _)| format!("'{}'", char::from(*l)))
                .collect::<Vec<_>>()
                .join(", ");
            let noun = if gated.len() == 1 {
                "binary field specifier"
            } else {
                "binary field specifiers"
            };
            self.dsl_gate_sites
                .push(super::super::version_gate::DslGateSite {
                    span,
                    code: DiagCode::W202,
                    what: format!("{noun} {letters}"),
                    min,
                });
        }
    }

    /// W121: a dotted-quad literal that looks like a subnet mask
    /// but has non-contiguous bits (`255.255.255.1`, `255.0.255.0`) is
    /// almost certainly a mistake.
    pub(in crate::analyser) fn emit_w121_invalid_subnet_mask(
        &mut self,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let script_spans: FxHashSet<_> = original
            .and_then(|call| {
                let scripts = call.with_schema(|schema| {
                    let mut scripts = schema
                        .authored_source_script_arguments()
                        .unwrap_or_default();
                    let plan = match schema.clause_walk() {
                        Some(Ok(plan)) => Some(plan),
                        Some(Err(abstention)) => Some(abstention.inert),
                        None => None,
                    };
                    if let Some(plan) = plan {
                        scripts.extend(plan.roles.iter().filter_map(|&(index, role)| {
                            (role == tcl_registry::ArgRole::Body).then_some(index)
                        }));
                    }
                    scripts
                })?;
                let (_, tokens, _) = call.source_arguments()?;
                Some(
                    scripts
                        .into_iter()
                        .filter_map(|index| tokens.get(index).map(|tok| tok.span))
                        .collect(),
                )
            })
            .unwrap_or_default();
        let mut seen: FxHashSet<u32> = FxHashSet::default();
        for (tok, text) in arg_tokens.iter().zip(args.iter()) {
            if script_spans.contains(&tok.span) || seen.contains(&tok.span.start()) {
                continue;
            }
            for quad in find_dotted_quads(text, 3) {
                let octets: Vec<u32> = quad
                    .octets
                    .iter()
                    .map(|o| o.parse::<u32>().unwrap_or(999))
                    .collect();
                if octets.iter().any(|&o| o > 255) {
                    continue;
                }
                let (a, b, c, d) = (octets[0], octets[1], octets[2], octets[3]);
                if !looks_like_subnet_mask(a, b, c, d) {
                    continue;
                }
                if is_valid_subnet_mask(a, b, c, d) {
                    continue;
                }
                seen.insert(tok.span.start());
                let quad = format!("{a}.{b}.{c}.{d}");
                let mut message = format!(
                    "'{quad}' looks like a subnet mask but has non-contiguous bits. A valid \
                     mask must be contiguous leading 1-bits followed by 0-bits."
                );
                if let Some(s) = nearest_valid_mask(a, b, c, d)
                    && s != quad
                {
                    use std::fmt::Write as _;
                    let _ = write!(message, " Did you mean '{s}'?");
                }
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W121,
                        tok.span,
                        message,
                        super::super::types::Severity::Warning,
                    ));
            }
        }
    }

    /// W108: a non-ASCII character in an argument token —
    /// either a Unicode confusable (visually resembling ASCII) or a
    /// known copy-paste artifact (smart quote, NBSP, em-dash, …).
    /// `args` / `arg_tokens` exclude the command name (the command word
    /// is not scanned).  Operates in the default **confusables** mode
    /// (→ **strict** for F5 iRules / iApps); the `common` mode — which
    /// needs Unicode general-category data Rust std lacks — is not
    /// implemented.  One diagnostic per offending character, with an
    /// ASCII-replacement fix when one is known.
    pub(in crate::analyser) fn emit_w108_non_ascii(&mut self, arg_tokens: &[tcl_lexer::Token]) {
        use super::super::confusables_table::{auto_fix_for, confusable_to_ascii};
        use super::super::state::NonAsciiMode;

        // Resolve the effective mode: an explicit `tclLsp.style.nonAscii`
        // setting, or the per-dialect default (strict for ASCII-only F5
        // dialects, confusables otherwise).
        let mode = match self.non_ascii_mode {
            NonAsciiMode::Default => {
                if self.profile.is_irules()
                    || self.profile.vendor_surface == Some(SpecProvider::Package("iapps"))
                {
                    NonAsciiMode::Strict
                } else {
                    NonAsciiMode::Confusables
                }
            }
            explicit => explicit,
        };
        if mode == NonAsciiMode::Off {
            return;
        }

        let mut seen: FxHashSet<u32> = FxHashSet::default();
        for tok in arg_tokens {
            if seen.contains(&tok.span.start()) {
                continue;
            }
            let Some(slice) = source_slice(&self.source, tok.span) else {
                continue;
            };
            // Skip a multi-line braced body — its inner commands are
            // checked when the body is recursed into.
            if tok.kind == tcl_lexer::TokenType::Esc && slice.contains('\n') {
                continue;
            }
            // Comment regions of a braced script argument (any brace depth).
            // Comments are prose: outside strict mode only the invisible /
            // direction-altering trojan-source set can mislead review there,
            // so an em-dash or smart quote in a comment is not a finding.
            // Strict mode (ASCII-only F5 platforms) keeps flagging comment
            // bytes too — the platform constraint applies to the whole file.
            let comments = if tok.kind == tcl_lexer::TokenType::Str && mode != NonAsciiMode::Strict
            {
                // The token slice starts at the opening `{` (and stops short
                // of the closer), so lex the *content* past `content_offset`
                // and shift the regions back into slice coordinates.
                let coff = usize::from(tok.content_offset);
                slice.get(coff..).map_or_else(Vec::new, |content| {
                    comment_regions_recursive(content, self.file_lexer_config())
                        .into_iter()
                        .map(|r| r.start + coff..r.end + coff)
                        .collect()
                })
            } else {
                Vec::new()
            };
            let mut flagged_here = false;
            for (rel, ch) in slice.char_indices() {
                if is_standard_ascii(ch) {
                    continue;
                }
                // Bidi formatting controls belong to W305, which scans the
                // whole file — a bidi override reorders the text *around*
                // itself, so a per-argument scan is the wrong shape — and
                // reports at error severity rather than as a style warning.
                // Skipping them here is what stops one character producing two
                // codes; see `confusables_table::BIDI_CONTROLS` for the exact
                // set, and for why directional *marks* and zero-width
                // characters deliberately stay with W108.
                if super::super::super::confusables_table::is_bidi_control(ch) {
                    continue;
                }
                if comments.iter().any(|r| r.contains(&rel)) && !is_review_hazard_unicode(ch) {
                    continue;
                }
                let fix = auto_fix_for(ch).or_else(|| confusable_to_ascii(ch));
                let is_confusable = fix.is_some();
                // Mode-dependent filtering (strict flags everything):
                //  * confusables — confusables / auto-fix artifacts, plus
                //    the invisible / direction-altering trojan-source set
                //    (a bidi override is a review hazard wherever it sits,
                //    table membership or not);
                //  * common — those plus any non-benign character
                //    (control / format / separator / unassigned / …).
                match mode {
                    NonAsciiMode::Confusables
                        if !is_confusable && !is_review_hazard_unicode(ch) =>
                    {
                        continue;
                    }
                    NonAsciiMode::Common if !is_confusable && is_benign_unicode(ch) => continue,
                    _ => {}
                }
                flagged_here = true;
                let start = tok.span.start() + u32::try_from(rel).unwrap_or(0);
                let end = start + u32::try_from(ch.len_utf8()).unwrap_or(1);
                let span = tcl_lexer::Span::new(start, end);
                let fixes = fix.map_or_default(|repl| {
                    vec![super::super::types::CodeFix {
                        span,
                        new_text: repl.to_string(),
                        description: "Replace with ASCII equivalent".to_string(),
                        // W108: substituting the ASCII look-alike changes the character's
                        // value. In an identifier that is the repair; inside a string literal
                        // it rewrites the program's data.
                        safety: crate::irules_checks::FixSafety::BehaviourHardening,
                    }]
                });
                self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
    DiagCode::W108,
    span,
    format!(
                        "Non-ASCII character U+{:04X} '{ch}' \u{2014} outside the standard ASCII \
                         printable/whitespace set",
                        ch as u32
                    ),
    super::super::types::Severity::Warning,
).with_fixes(fixes));
            }
            if flagged_here {
                seen.insert(tok.span.start());
            }
        }
    }
}

/// True when `ch` is a standard ASCII character W108 leaves alone: tab
/// / LF / CR, or printable ASCII `0x20`-`0x7e`.
fn is_standard_ascii(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\r' | ' '..='~')
}

/// W108 "common" mode benign-Unicode test. A character is
/// *intentional* (not flagged) when its Unicode general category is a
/// Letter, Number, Mark, Symbol, or Punctuation (any script). Control,
/// True when `ch` is invisible or direction-altering — the trojan-source
/// set that can make reviewed text lie about the code it sits next to:
/// bidi embedding / override / isolate controls and other format
/// characters (`Cf`), non-ASCII control characters (`Cc`), and the Unicode
/// line / paragraph separators (`Zl` / `Zp`).  These stay flagged inside
/// comments (where ordinary non-ASCII prose is fine) and bypass the
/// confusables-table gate in code.
pub(in super::super) fn is_review_hazard_unicode(ch: char) -> bool {
    use unicode_general_category::{GeneralCategory as G, get_general_category};
    matches!(
        get_general_category(ch),
        G::Format | G::Control | G::LineSeparator | G::ParagraphSeparator
    )
}

/// Byte ranges of `slice` (a braced script argument's content) covered by
/// comments, at any brace depth: the comment tokens of the slice's own
/// lex, plus — recursively — those of every nested braced word, offset
/// into this slice's coordinates.  A slice that fails to lex contributes
/// no regions (conservative: everything stays scanned).  A braced plain
/// string whose content merely *looks* like a comment under-flags — the
/// acceptable cost of not knowing which braced words are scripts.
pub(super) fn comment_regions_recursive(
    slice: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<std::ops::Range<usize>> {
    fn collect(
        slice: &str,
        base: usize,
        config: tcl_lexer::LexerConfig,
        out: &mut Vec<std::ops::Range<usize>>,
    ) {
        if !slice.contains('#') {
            return;
        }
        let Ok(tokens) = tcl_lexer::Lexer::with_config(slice, config.nested()).tokenise_all()
        else {
            return;
        };
        for tok in &tokens {
            let start = tok.span.start() as usize;
            let end = tok.span.end() as usize;
            match tok.kind {
                tcl_lexer::TokenType::Comment => out.push(base + start..base + end),
                tcl_lexer::TokenType::Str => {
                    // Recurse on the brace *content* — the token span covers
                    // the opener, which would unbalance a nested lex.
                    let coff = usize::from(tok.content_offset);
                    if let Some(inner) = slice.get(start + coff..end) {
                        collect(inner, base + start + coff, config, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    collect(slice, 0, config, &mut out);
    out
}

/// format, separator, surrogate, private-use, and unassigned characters
/// are *not* benign (they almost always indicate encoding/copy-paste
/// issues) and are flagged.
pub(in super::super) fn is_benign_unicode(ch: char) -> bool {
    use unicode_general_category::{GeneralCategory as G, get_general_category};
    matches!(
        get_general_category(ch),
        // Letters (L*)
        G::UppercaseLetter
            | G::LowercaseLetter
            | G::TitlecaseLetter
            | G::ModifierLetter
            | G::OtherLetter
            // Numbers (N*)
            | G::DecimalNumber
            | G::LetterNumber
            | G::OtherNumber
            // Marks (M*)
            | G::NonspacingMark
            | G::SpacingMark
            | G::EnclosingMark
            // Symbols (S*)
            | G::MathSymbol
            | G::CurrencySymbol
            | G::ModifierSymbol
            | G::OtherSymbol
            // Punctuation (P*)
            | G::ConnectorPunctuation
            | G::DashPunctuation
            | G::OpenPunctuation
            | G::ClosePunctuation
            | G::InitialPunctuation
            | G::FinalPunctuation
            | G::OtherPunctuation
    )
}

/// Mask-octet values that can appear in a contiguous subnet mask.
const VALID_MASK_OCTETS: &[u32] = &[0, 128, 192, 224, 240, 248, 252, 254, 255];

/// True when the four octets form a valid contiguous subnet mask
/// (all-1s then all-0s).
pub(in super::super) fn is_valid_subnet_mask(a: u32, b: u32, c: u32, d: u32) -> bool {
    let val = (a << 24) | (b << 16) | (c << 8) | d;
    if val == 0 {
        return true;
    }
    let inverted = val ^ 0xFFFF_FFFF;
    (inverted & inverted.wrapping_add(1)) == 0
}

/// Heuristic: the dotted-quad plausibly *intends* to be a mask.
pub(in super::super) fn looks_like_subnet_mask(a: u32, b: u32, c: u32, d: u32) -> bool {
    if a == 255 && !(b == 255 && c == 255 && d == 255) {
        return true;
    }
    a >= 128 && [a, b, c, d].iter().all(|o| VALID_MASK_OCTETS.contains(o))
}

/// Suggest the nearest valid contiguous mask, or `None`.
pub(in super::super) fn nearest_valid_mask(a: u32, b: u32, c: u32, d: u32) -> Option<String> {
    let val = (a << 24) | (b << 16) | (c << 8) | d;
    let mut leading = 0u32;
    for bit in (0..32).rev() {
        if val & (1 << bit) != 0 {
            leading += 1;
        } else {
            break;
        }
    }
    if leading == 0 || leading == 32 {
        return None;
    }
    let candidate = 0xFFFF_FFFFu32 << (32 - leading);
    Some(format!(
        "{}.{}.{}.{}",
        (candidate >> 24) & 0xFF,
        (candidate >> 16) & 0xFF,
        (candidate >> 8) & 0xFF,
        candidate & 0xFF
    ))
}
