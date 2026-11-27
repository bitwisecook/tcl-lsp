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

//! The clock conversions whose reading differs between Tcl 8.4's glibc
//! `strftime` and 8.5's `clock.tcl`.

use super::{Civil, MONTHS, WEEKDAYS, hour12};
use tcl_dialect::ClockFormatGrammar;

/// Whether `grammar` reads the locale modifier `modifier` (`E` or `O`) before
/// `letter`; any other pair renders verbatim. Tcl 8.4 hands the format to
/// glibc `strftime`, which drops the modifier; 8.5 on reads only the pairs
/// `clock.tcl` lists (tclsh 8.4.20 and 8.5.19 to 9.1.0: `%Ez` is `+0000`
/// on 8.4 and `%Ez` from 8.5, `%OC` is `19` on 8.4 and `%OC` from 8.5).
pub(super) fn modifies(grammar: ClockFormatGrammar, modifier: char, letter: char) -> bool {
    match (grammar, modifier) {
        (ClockFormatGrammar::Strftime84, 'E') => {
            matches!(letter, 'C' | 'c' | 'x' | 'X' | 'y' | 'Y' | 'z')
        }
        (ClockFormatGrammar::Strftime84, _) => matches!(
            letter,
            'C' | 'd' | 'e' | 'H' | 'I' | 'k' | 'l' | 'm' | 'M' | 'S' | 'u' | 'w' | 'y' | 'z'
        ),
        (ClockFormatGrammar::Tcl85, 'E') => {
            matches!(letter, 'C' | 'E' | 'c' | 'x' | 'X' | 'y' | 'Y')
        }
        (ClockFormatGrammar::Tcl85, _) => matches!(
            letter,
            'd' | 'e' | 'H' | 'I' | 'k' | 'l' | 'm' | 'M' | 'S' | 'u' | 'w' | 'y'
        ),
    }
}

/// `clock.tcl`'s own reading of a modified conversion: `%EE` is the era, and
/// `%O` prints the field through the C locale's two-digit numerals (`%Ou` is
/// `03`). `None` for a pair that renders as its plain conversion.
pub(super) fn locale_conversion(c: &Civil, modifier: char, letter: char) -> Option<String> {
    if modifier == 'E' {
        return (letter == 'E').then(|| (if c.year > 0 { "C.E." } else { "B.C.E." }).to_owned());
    }
    let field = match letter {
        'd' | 'e' => i64::from(c.day),
        'H' | 'k' => c.hour,
        'I' | 'l' => hour12(c.hour),
        'm' => i64::from(c.month),
        'M' => c.min,
        'S' => c.sec,
        'u' => {
            if c.weekday == 0 {
                7
            } else {
                c.weekday
            }
        }
        'w' => c.weekday,
        _ => c.year.rem_euclid(100),
    };
    Some(format!("{field:02}"))
}

/// The composite conversions whose C-locale spelling differs between
/// `clock.tcl` (8.5 on) and 8.4's glibc `strftime`: `%D`/`%x` keep a
/// four-digit year and `%r` a lower-case `am`/`pm` only from 8.5.
pub(super) fn composite(c: &Civil, spec: char, tcl85: bool) -> String {
    match spec {
        'c' => format!(
            "{} {} {:2} {:02}:{:02}:{:02} {}",
            &WEEKDAYS[c.weekday as usize][..3],
            &MONTHS[(c.month - 1) as usize][..3],
            c.day,
            c.hour,
            c.min,
            c.sec,
            c.year
        ),
        'r' => format!(
            "{:02}:{:02}:{:02} {}",
            hour12(c.hour),
            c.min,
            c.sec,
            match (c.hour < 12, tcl85) {
                (true, true) => "am",
                (false, true) => "pm",
                (true, false) => "AM",
                (false, false) => "PM",
            }
        ),
        _ if tcl85 => format!("{:02}/{:02}/{}", c.month, c.day, c.year),
        _ => format!("{:02}/{:02}/{:02}", c.month, c.day, c.year.rem_euclid(100)),
    }
}
