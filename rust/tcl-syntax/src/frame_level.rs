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

//! Selected grammar of a materialised Tcl frame-level word. Command arity and
//! the choice between optional probing and a required level belong to the
//! command descriptor. Frame lookup and object conversion effects belong to
//! the interpreter.

use crate::number::{Number, ParseFlags, parse_whole_with};
use tcl_dialect::{FrameLevelPresence, NumberSyntax};

/// Numeric frame distance or absolute depth from a supplied literal word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralLevel {
    /// Number of frames back from the current activation.
    Relative(u32),
    /// Depth counted from the global frame.
    Absolute(u32),
}

fn integer(word: &str, numbers: NumberSyntax) -> Option<Number> {
    parse_whole_with(
        word,
        ParseFlags {
            integer_only: true,
            ..ParseFlags::for_syntax(numbers)
        },
    )
}

/// Read the numeric value of a supplied level under its own numeric policy.
/// Availability of this depth in the current call stack is a separate query.
#[must_use]
pub fn parse_supplied_level(word: &str, numbers: NumberSyntax) -> Option<LiteralLevel> {
    let (absolute, text) = match word.strip_prefix('#') {
        Some(rest) => (true, rest),
        None => (false, word),
    };
    let Number::Int(value) = integer(text, numbers)? else {
        return None;
    };
    let depth = u32::try_from(value).ok()?;
    Some(if absolute {
        LiteralLevel::Absolute(depth)
    } else {
        LiteralLevel::Relative(depth)
    })
}

/// Whether a materialised first operand is claimed as a level by the selected
/// leading-word probe. A claimed operand can still raise at conversion or lookup.
/// An argc-based policy needs the caller's cardinality. Raw NUL needs the native
/// object conversion schedule and cannot be projected from the bytes alone.
#[must_use]
pub fn leading_word_is_level(
    word: &[u8],
    presence: FrameLevelPresence,
    numbers: NumberSyntax,
) -> Option<bool> {
    if presence == FrameLevelPresence::ArgumentParity || word.contains(&0) {
        return None;
    }
    if word.first() == Some(&b'#') {
        return Some(true);
    }
    let digit = word.first().is_some_and(u8::is_ascii_digit);
    let integer = || integer(std::str::from_utf8(word).ok()?, numbers);
    Some(match presence {
        FrameLevelPresence::DigitOrHash => digit,
        FrameLevelPresence::DigitOrNonNegativeInteger => {
            digit
                || match integer() {
                    Some(Number::Int(value)) => value >= 0 || i32::try_from(value).is_err(),
                    Some(Number::Big { .. }) => true,
                    _ => false,
                }
        }
        FrameLevelPresence::IntegerOrHash => {
            matches!(integer(), Some(Number::Int(_) | Number::Big { .. }))
        }
        FrameLevelPresence::ArgumentParity => unreachable!("cardinality policy handled above"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;

    #[test]
    fn probing_a_command_and_reading_a_supplied_level_use_separate_rules() {
        for version in TclVersion::ALL {
            let numbers = version.number_syntax();
            assert_eq!(
                parse_supplied_level("#0x0", numbers),
                Some(LiteralLevel::Absolute(0))
            );
            assert_eq!(
                parse_supplied_level("+1", numbers),
                Some(LiteralLevel::Relative(1))
            );
            assert_eq!(
                leading_word_is_level(b"+1", version.uplevel_level_presence(), numbers),
                Some(version >= TclVersion::V8_6)
            );
            assert_eq!(
                leading_word_is_level(b"#oops", version.uplevel_level_presence(), numbers),
                Some(true)
            );
            assert_eq!(parse_supplied_level("#oops", numbers), None);
        }
        assert_eq!(
            leading_word_is_level(
                b"1",
                FrameLevelPresence::ArgumentParity,
                NumberSyntax::Tcl85
            ),
            None
        );
        assert_eq!(
            leading_word_is_level(b"1\0", FrameLevelPresence::DigitOrHash, NumberSyntax::Tcl84),
            None
        );
    }
}
