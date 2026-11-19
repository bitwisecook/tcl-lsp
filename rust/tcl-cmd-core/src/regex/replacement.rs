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

//! Tcl and Jim `regsub` replacement templates.

use std::ops::Range;

/// One part of a replacement value after Tcl has formed the command's arguments.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplacementPart<'a, T> {
    /// Verbatim units to append, with the quoting backslash removed for `\\` and `\&`.
    Literal(&'a [T]),
    /// The whole match (0) or a numbered subexpression (1–9).
    Capture(u8),
}

/// Parse one replacement value into its source ranges and expansion parts.
/// Offsets use the input's units: bytes for counted strings or Tcl characters
/// for the native Unicode path. Non-ASCII units are always copied untouched.
pub fn replacement_parts<T: Copy + Into<u32>>(
    template: &[T],
) -> impl Iterator<Item = (Range<usize>, ReplacementPart<'_, T>)> {
    let mut pos = 0;
    std::iter::from_fn(move || {
        let start = pos;
        let unit = (*template.get(pos)?).into();
        pos += 1;
        let part = if unit == u32::from(b'&') {
            ReplacementPart::Capture(0)
        } else if unit == u32::from(b'\\') {
            match template
                .get(pos)
                .copied()
                .map(Into::into)
                .and_then(|u| u8::try_from(u).ok())
            {
                Some(next @ b'0'..=b'9') => {
                    pos += 1;
                    ReplacementPart::Capture(next - b'0')
                }
                Some(b'\\' | b'&') => {
                    pos += 1;
                    ReplacementPart::Literal(&template[start + 1..pos])
                }
                _ => ReplacementPart::Literal(&template[start..pos]),
            }
        } else {
            while pos < template.len() {
                let next = template[pos].into();
                if next == u32::from(b'\\') || next == u32::from(b'&') {
                    break;
                }
                pos += 1;
            }
            ReplacementPart::Literal(&template[start..pos])
        };
        Some((start..pos, part))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_spans_preserve_escaped_units_and_unrecognised_escapes() {
        // All five C Tcl releases and current Jim expand this template over (a) / a.
        let template = br"X\a&\1\&\\1\0\";
        let mut result = Vec::new();
        let mut captures = Vec::new();
        for (range, part) in replacement_parts(template) {
            match part {
                ReplacementPart::Capture(group) => {
                    captures.push((range, group));
                    result.push(b'a');
                }
                ReplacementPart::Literal(text) => result.extend_from_slice(text),
            }
        }
        assert_eq!(result, br"X\aaa&\1a\");
        assert_eq!(captures, [(3..4, 0), (4..6, 1), (11..13, 0)]);
    }
    #[test]
    fn native_template_offsets_are_in_the_original_units() {
        let template = [0xd800_u32, u32::from(b'\\'), u32::from(b'1'), 0xdc00];
        assert_eq!(
            replacement_parts(&template).collect::<Vec<_>>(),
            [
                (0..1, ReplacementPart::Literal(&template[..1])),
                (1..3, ReplacementPart::Capture(1)),
                (3..4, ReplacementPart::Literal(&template[3..])),
            ]
        );
    }
}
