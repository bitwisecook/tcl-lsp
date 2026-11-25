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

//! Checked application of replacements to original source byte ranges.

use std::fmt;
use std::ops::Range;

/// Replace a half-open byte range in the original source with UTF-8 text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEdit {
    /// Inclusive start and exclusive end in the unmodified source.
    pub span: Range<usize>,
    /// Text to insert at the range.
    pub text: String,
}

/// Original input indices of the edits that prevent the batch from applying.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEditError {
    /// The range is reversed, out of bounds or cuts a UTF-8 character.
    InvalidSpan {
        /// Index in the input batch.
        edit: usize,
    },
    /// Two ranges cover the same bytes, or an insertion is inside a replacement.
    Overlap {
        /// Input index of the edit that starts first after sorting.
        first: usize,
        /// Input index of the following edit.
        second: usize,
    },
}

impl fmt::Display for SourceEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidSpan { edit } => write!(f, "invalid source range for edit {edit}"),
            Self::Overlap { first, second } => {
                write!(f, "source edits {first} and {second} overlap")
            }
        }
    }
}

impl std::error::Error for SourceEditError {}

/// Apply one batch in `(start, end)` order. Insertions at the same point keep
/// their input order and come before replacements beginning at that point.
///
/// # Errors
/// Returns a [`SourceEditError`] for an invalid or overlapping range.
pub fn apply_edits(source: &str, edits: &[SourceEdit]) -> Result<String, SourceEditError> {
    let mut ordered: Vec<_> = edits.iter().enumerate().collect();
    ordered.sort_by_key(|(_, edit)| (edit.span.start, edit.span.end));
    let mut result = String::with_capacity(source.len());
    let mut cursor = 0;
    let mut previous = 0;
    for (index, edit) in ordered {
        if source.get(edit.span.clone()).is_none() {
            return Err(SourceEditError::InvalidSpan { edit: index });
        }
        if edit.span.start < cursor {
            return Err(SourceEditError::Overlap {
                first: previous,
                second: index,
            });
        }
        result.push_str(&source[cursor..edit.span.start]);
        result.push_str(&edit.text);
        cursor = edit.span.end;
        previous = index;
    }
    result.push_str(&source[cursor..]);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_insertions_and_replacements_in_the_original_source() {
        let edits = [
            SourceEdit {
                span: 0..2,
                text: "x".into(),
            },
            SourceEdit {
                span: 2..3,
                text: "y".into(),
            },
            SourceEdit {
                span: 0..0,
                text: "a".into(),
            },
            SourceEdit {
                span: 0..0,
                text: "b".into(),
            },
        ];
        assert_eq!(apply_edits("éz", &edits).unwrap(), "abxy");
    }

    #[test]
    fn rejects_invalid_utf8_ranges_and_overlaps() {
        let source = "aéb";
        for start in 0..=source.len() + 1 {
            for end in 0..=source.len() + 1 {
                let span = start..end;
                let valid = source.get(span.clone()).is_some();
                let edit = SourceEdit {
                    span,
                    text: "x".into(),
                };
                assert_eq!(apply_edits(source, &[edit]).is_ok(), valid);
            }
        }
        for span in [1..3, 1..4, 3..3] {
            let edits = [
                SourceEdit {
                    span: 0..4,
                    text: "x".into(),
                },
                SourceEdit {
                    span,
                    text: "y".into(),
                },
            ];
            assert_eq!(
                apply_edits(source, &edits),
                Err(SourceEditError::Overlap {
                    first: 0,
                    second: 1
                })
            );
        }
    }
}
