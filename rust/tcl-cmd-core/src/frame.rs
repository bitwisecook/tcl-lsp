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

//! `uplevel`/`upvar` level words, resolved as C's `TclObjGetFrame` does.
//!
//! The engine supplies its own integer reads through [`ValueOps`]; this owns
//! only which reads C makes and what each outcome means.

use tcl_syntax::value::{ValueError, ValueOps};

use crate::error::CmdError;

/// The absolute frame level a `?level?` word selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameLevel {
    /// The word is a level and names this frame.
    Named(usize),
    /// No word, or one that is not a level: the frame one up.
    Default(usize),
}

/// A level word that names no frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadLevel {
    /// The word itself is a bad level (`bad level "<word>"`).
    Word,
    /// There is no frame one up from the global level (`bad level "1"`).
    NoCaller,
}

/// Resolve `word` against the `current` level as `TclObjGetFrame` does.
///
/// A relative level is a C `int` read, so `0x1`, ` 1 ` and `+1` count, and a
/// negative or past-`INT_MAX` one is bad; `#N` is absolute. An integer too
/// large for the `int` read is bad too, while any other word is no level at
/// all, and so [`FrameLevel::Default`].
///
/// # Errors
/// [`BadLevel`] when the word, or the default, names no frame.
pub fn frame_level<O: ValueOps>(
    ops: &mut O,
    word: Option<&O::Value>,
    current: usize,
) -> Result<FrameLevel, BadLevel> {
    let default = || {
        current
            .checked_sub(1)
            .map(FrameLevel::Default)
            .ok_or(BadLevel::NoCaller)
    };
    let Some(word) = word else {
        return default();
    };
    let named = match ops.as_c_int(word) {
        Ok(_) => match ops.as_int(word) {
            Ok(up) if (0..=i64::from(i32::MAX)).contains(&up) => usize::try_from(up)
                .ok()
                .and_then(|up| current.checked_sub(up)),
            _ => None,
        },
        Err(ValueError::IntegerOverflow) => None,
        Err(_) => {
            let spelling = ops.as_str(word);
            let Some(absolute) = spelling.strip_prefix('#') else {
                return default();
            };
            let absolute_value = ops.new_str(absolute);
            ops.pin_value(&absolute_value);
            let level = ops.as_c_int(&absolute_value).ok();
            ops.unpin_value(&absolute_value);
            level
                .filter(|&level| level == 0 || (level > 0 && !absolute.starts_with('-')))
                .and_then(|level| usize::try_from(level).ok())
                .filter(|&level| level <= current)
        }
    };
    named.map(FrameLevel::Named).ok_or(BadLevel::Word)
}

/// `bad level "<spec>"` with the `TCL LOOKUP LEVEL <spec>` code C sets.
#[must_use]
pub fn bad_level(spec: &str) -> CmdError {
    CmdError::with_error_code(
        format!("bad level \"{spec}\""),
        tcl_syntax::list::join_list(["TCL", "LOOKUP", "LEVEL", spec]),
    )
}
