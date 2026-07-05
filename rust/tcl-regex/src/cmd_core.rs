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

//! A [`tcl_cmd_core::regex::RegexEngine`] provider backed by this crate, so the
//! shared `regexp` / `regsub` / `lsearch -regexp` / `switch -regexp` command
//! plumbing can drive a faithful Tcl 9 ARE engine — replacing both the C-FFI
//! engine (`runtime/rust`) and the approximate `regex`-crate path (the bytecode
//! VM). Gated behind the `cmd-core` feature so the engine stays dependency-free
//! by default.

use crate::{Regex, defs};
use tcl_cmd_core::regex::{NO_MATCH, RegMatch, RegexEngine, RegexFlags};

/// The ARE engine as the shared plumbing's provider.
pub struct AreEngine;

impl RegexEngine for AreEngine {
    type Regex = Regex;

    fn compile(pattern: &[u8], flags: RegexFlags) -> Result<Regex, Vec<u8>> {
        let mut cflags = defs::REG_ADVANCED;
        if flags.nocase {
            cflags |= defs::REG_ICASE;
        }
        if flags.expanded {
            cflags |= defs::REG_EXPANDED;
        }
        if flags.linestop {
            cflags |= defs::REG_NLSTOP;
        }
        if flags.lineanchor {
            cflags |= defs::REG_NLANCH;
        }
        let text = core::str::from_utf8(pattern)
            .map_err(|_| b"invalid UTF-8 in regular expression".to_vec())?;
        let cps: Vec<u32> = text.chars().map(|c| c as u32).collect();
        Regex::compile(&cps, cflags).map_err(|e| e.message().as_bytes().to_vec())
    }

    fn nsub(re: &Regex) -> usize {
        re.nsub()
    }

    fn exec(re: &mut Regex, cps: &[i32], offset: usize, _notbol: bool) -> Option<Vec<RegMatch>> {
        // This engine is context-aware: anchors are resolved against absolute
        // positions in the whole subject, so the `notbol` hint is unnecessary
        // (the trait permits ignoring it).
        let subject: Vec<u32> = cps.iter().map(|&c| c as u32).collect();
        let groups = re.exec(&subject, offset, 0)?;
        Some(
            groups
                .iter()
                .map(|g| match g {
                    Some(s) => RegMatch {
                        so: s.start,
                        eo: s.end,
                    },
                    None => RegMatch {
                        so: NO_MATCH,
                        eo: NO_MATCH,
                    },
                })
                .collect(),
        )
    }
}
