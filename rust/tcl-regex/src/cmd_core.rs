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

//! ARE and bundled Jim engine providers for original-object command plumbing.
//! C compilation accepts exact native character units; Jim compilation retains
//! original `CString` bytes and its distinct integer program. The `cmd-core`
//! feature supplies these adapters.

use crate::{InfoFlag, Regex, defs};
use tcl_cmd_core::regex::{NO_MATCH, RegMatch, RegexEngine, RegexFlags};

/// The ARE engine as the shared plumbing's provider.
pub struct AreEngine;

/// Executable artifact tagged by its independently selected engine protocol.
pub enum CompiledRegex {
    C(Regex),
    Jim(crate::jim::Regex),
}

impl RegexEngine for AreEngine {
    type Regex = CompiledRegex;

    fn compile(pattern: &[u8], flags: RegexFlags) -> Result<CompiledRegex, Vec<u8>> {
        let text = core::str::from_utf8(pattern)
            .map_err(|_| b"invalid UTF-8 in regular expression".to_vec())?;
        let cps: Vec<u32> = text.chars().map(|c| c as u32).collect();
        Self::compile_units(&cps, flags).expect("ARE engine accepts native character units")
    }

    fn compile_units(pattern: &[u32], flags: RegexFlags) -> Option<Result<CompiledRegex, Vec<u8>>> {
        let mut cflags = defs::REG_ADVANCED;
        if flags.nosub {
            cflags |= defs::REG_NOSUB;
        }
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
        if flags.z_anchor {
            cflags |= defs::REG_ZANCHOR;
        }
        Some(
            Regex::compile(pattern, cflags)
                .map(CompiledRegex::C)
                .map_err(|e| e.message().as_bytes().to_vec()),
        )
    }

    fn compile_jim(pattern: &[u8], flags: RegexFlags) -> Option<Result<CompiledRegex, Vec<u8>>> {
        match crate::jim::Regex::compile(
            pattern,
            crate::jim::Flags::new()
                .with_nocase(flags.nocase)
                .with_lineanchor(flags.lineanchor)
                .with_linestop(flags.linestop)
                .with_expanded(flags.expanded),
        ) {
            Ok(program) => Some(Ok(CompiledRegex::Jim(program))),
            Err(crate::jim::Error::Syntax(code)) => Some(Err(crate::jim::Error::Syntax(code)
                .message()
                .as_bytes()
                .to_vec())),
            Err(crate::jim::Error::Unavailable) => None,
        }
    }
    fn exec_jim(
        re: &mut CompiledRegex,
        subject: &[u8],
        captures: usize,
        notbol: bool,
    ) -> Result<Option<Vec<RegMatch>>, &'static str> {
        let CompiledRegex::Jim(re) = re else {
            return Err("Jim regexp artifact engine mismatch");
        };
        re.execute(subject, captures, notbol)
            .map(|matched| {
                matched.map(|spans| {
                    spans
                        .into_iter()
                        .map(|span| RegMatch {
                            so: span.start.unwrap_or(NO_MATCH),
                            eo: span.end.unwrap_or(NO_MATCH),
                        })
                        .collect()
                })
            })
            .map_err(|_| "Jim regexp program execution unavailable")
    }
    fn nsub(re: &CompiledRegex) -> usize {
        match re {
            CompiledRegex::C(re) => re.nsub(),
            CompiledRegex::Jim(re) => re.nsub(),
        }
    }

    /// The `re_info` flag names, in `re_info` bit order — what `regexp -about`
    /// reports as the second element of its result.
    ///
    /// This engine records the bits as it compiles, which is the only place
    /// they exist: nothing downstream can recompute "which constructs did this
    /// pattern use" without being a second ARE parser, which is why the trait
    /// defaults to an empty list and the engine that knows overrides it.
    /// `InfoFlag::ALL` is `infonames[]`'s order from `tclRegexp.c`, so the
    /// rendered list matches tclsh element for element — `regexp -about
    /// {(?:a)}` is `0 REG_UNONPOSIX` on 8.4.20 through 9.1b0.
    fn info_names(re: &CompiledRegex) -> Vec<&'static str> {
        let CompiledRegex::C(re) = re else {
            return Vec::new();
        };
        re.info().flags().into_iter().map(InfoFlag::name).collect()
    }

    fn exec(
        re: &mut CompiledRegex,
        cps: &[i32],
        offset: usize,
        _notbol: bool,
    ) -> Option<Vec<RegMatch>> {
        // This engine is context-aware: anchors are resolved against absolute
        // positions in the whole subject, so the `notbol` hint is unnecessary
        // (the trait permits ignoring it).
        let CompiledRegex::C(re) = re else {
            return None;
        };
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
