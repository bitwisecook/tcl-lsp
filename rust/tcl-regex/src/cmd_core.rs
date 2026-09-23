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

use crate::{ExecLimits, ExecOutcome, ExecStop, InfoFlag, Regex, defs};
use tcl_cmd_core::regex::{
    EngineIdentity, MatchLimits, PrecisionDecline, RegMatch, RegexEngine, RegexFlags,
    RegexpPrecision,
};

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
    fn info_names(re: &Regex) -> Vec<&'static str> {
        re.info().flags().into_iter().map(InfoFlag::name).collect()
    }

    fn exec(re: &mut Regex, cps: &[i32], offset: usize, notbol: bool) -> RegexpPrecision<RegMatch> {
        Self::exec_within(re, cps, offset, notbol, MatchLimits::default())
    }

    /// The engine's three-way answer onto the plumbing's: the spans as
    /// [`RegMatch`]es, a completed no-match, and a stopped search as the
    /// decline it is. The work the search spent is added to the caller's
    /// counter, and the search runs under what that counter leaves of the
    /// caller's budget.
    fn exec_within(
        re: &mut Regex,
        cps: &[i32],
        offset: usize,
        _notbol: bool,
        limits: MatchLimits<'_>,
    ) -> RegexpPrecision<RegMatch> {
        // This engine is context-aware: anchors are resolved against absolute
        // positions in the whole subject, so the `notbol` hint is unnecessary
        // (the trait permits ignoring it).
        let subject: Vec<u32> = cps.iter().map(|&c| c as u32).collect();
        let already = limits.spent.map_or(0, std::cell::Cell::get);
        let engine_limits = ExecLimits {
            fuel: limits
                .fuel
                .unwrap_or(crate::MATCH_FUEL)
                .saturating_sub(already),
            cancel: limits.cancel,
        };
        let (outcome, spent) = re.exec_metered(&subject, offset, 0, &engine_limits);
        if let Some(counter) = limits.spent {
            counter.set(already.saturating_add(spent));
        }
        match outcome {
            ExecOutcome::Matched(groups) => {
                let span = |s: &crate::Span| RegMatch {
                    so: s.start,
                    eo: s.end,
                };
                let whole = groups
                    .first()
                    .copied()
                    .flatten()
                    .map_or(RegMatch { so: 0, eo: 0 }, |s| span(&s));
                RegexpPrecision::Exact {
                    whole,
                    groups: groups
                        .iter()
                        .skip(1)
                        .map(|g| g.as_ref().map(span))
                        .collect(),
                    captures_exact: true,
                }
            }
            ExecOutcome::NoMatch => RegexpPrecision::NoMatch,
            ExecOutcome::Stopped(stop) => RegexpPrecision::Declined(match stop {
                ExecStop::Fuel { spent } => PrecisionDecline::FuelExhausted { spent },
                ExecStop::Depth { limit } => PrecisionDecline::DepthExhausted { limit },
                ExecStop::Cancelled => PrecisionDecline::Cancelled,
            }),
        }
    }

    /// Bumped with any change to what a pattern compiles to or matches.
    const IDENTITY: EngineIdentity = EngineIdentity {
        name: "tcl-regex ARE",
        revision: 2,
    };

    fn retained_bytes(re: &Regex) -> usize {
        re.retained_bytes()
    }
}
