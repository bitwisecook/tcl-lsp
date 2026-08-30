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

//! `update` — process pending events and idle callbacks.
//!
//! The SYNOPSIS (`update ?idletasks?`) and the entire DESCRIPTION/EXAMPLE
//! body text are byte-for-byte identical (word for word, once HTML markup
//! and whitespace are normalised) across every fetched manual page — Tcl
//! `update.n` 8.4, 8.5, 8.6, 9.0, and 9.1 — right down to shared, unchanged
//! wording for both the bare form and `idletasks`. A genuinely
//! version-invariant command: nothing about its documented behaviour
//! warrants a `surface:`/`min_tcl` gate anywhere in this spec. The two
//! cross-reference sections that *do* drift across versions are immaterial
//! to every field below — SEE ALSO reads "after, bgerror" on 8.4 and
//! "after, interp" from 8.5 on; KEYWORDS gains "asynchronous I/O" starting
//! at 8.6 — both are the manpage site's own indexing metadata, not
//! documented command behaviour.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "update ?idletasks?",
    ..FormSpec::DEFAULT
}];

/// The sole positional argument (index 0, only present in the one-word
/// form). Real `Tcl_UpdateObjCmd` resolves it with `Tcl_GetIndexFromObj`
/// under the default (non-`TCL_EXACT`) flags, so any unique abbreviation is
/// accepted: `update idle` and even `update i` both run exactly like
/// `update idletasks` (empirically confirmed on tclsh 8.6.14; `update foo`
/// alone fails, with `bad option "foo": must be idletasks`). None of the
/// five fetched manpages documents this abbreviation behaviour explicitly
/// (unlike e.g. `registry.n`, which spells out "Any unique abbreviation for
/// option is acceptable"), but nothing suggests it ever worked differently
/// either — a single "one legal keyword" argument resolved via
/// `Tcl_GetIndexFromObj` is a long-stable, version-independent C API
/// convention.
/// `CommandSpec::closed_value_args` has no prefix-matching mode (unlike
/// `SubCommand::arg_values_accept_prefix`), so marking this index there
/// would misreport a legitimate abbreviation as an invalid value — this is
/// completion/hover data only, the same reasoning `registry_.rs` applies to
/// `REGISTRY_OPTION_VALUES`.
const IDLETASKS_VALUES: &[ArgValue] = &[ArgValue {
    value: "idletasks",
    detail: "Service only idle callbacks — no new events or errors are processed — forcing operations that are normally deferred, such as display updates and window layout calculations, to run immediately.",
    ..ArgValue::DEFAULT
}];

/// Command spec for `update`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "update",
        // `ALL_TCL` (no iRules row) is deliberate, not an oversight: F5's TMM
        // interpreter does ban `update` — it is one of the K36322151
        // event-loop bans — and that ban is now encoded directly here, as an
        // explicit surface that omits an iRules row. `ALL_TCL` never
        // intersects the bare `IRULES` mask, so `update` falls out of iRules
        // by plain intersection, with no subtractive disable list; the
        // `irules_banned_commands_lack_the_irules_bit` contract test
        // (`tcl-registry/tests/dialect_profile.rs`) pins that every banned
        // name's spec lacks an iRules row. No other dialect profile (iApps,
        // tmsh, the EDA vendor shells, Expect, Tk, itcl) bans it or registers
        // a competing spec of its own (none of their command directories
        // define an `update`), and `ALL_TCL` intersects each of their
        // version|vendor masks, so it stays reachable — and, for Tk in
        // particular, essential — in every one of them.
        surface: Some(SpecSurface::ALL_TCL),
        // BYTE_COMPILED only — this codebase's "recognised core builtin"
        // convention, not literal bytecode emission (see traits.rs). Not
        // SAFE_INTERP_HIDDEN: Tcl's safe-interpreter hidden-command table
        // covers only cd/encoding/exec/exit/fconfigure/file/glob/load/open/
        // pwd/socket/source/unload — `update` touches no filesystem,
        // process, or network resource and is callable unmodified inside a
        // safe interpreter. Not FRAMELESS_RUNTIME either: the VM's
        // `cmd_update` (`tcl-vm/src/cmd_event.rs`) evaluates any due
        // `after`-scheduled scripts via `Vm::eval_source` — a genuine eval
        // fallback — matching `vwait`'s and `after`'s identical
        // BYTE_COMPILED-only trait baseline (`vwait.rs`, `after_.rs`).
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Process pending events and idle callbacks.",
            synopsis: &["update ?idletasks?"],
            snippet: "Enters the event loop repeatedly until every pending event — including idle callbacks — has been processed, bringing the application \"up to date\". With no argument, both new events (user input, timers, channel readiness) and idle callbacks are serviced; this is the standard idiom for keeping an application responsive during a long-running computation, since occasionally calling update lets any input queued in the meantime be handled on its next pass. update idletasks instead services only idle callbacks: no new events or errors are processed. Because most display updates and window layout recalculations are themselves scheduled as idle callbacks, update idletasks is the usual way to force pending visual changes onto the screen immediately rather than waiting for the script to finish — but an update that only happens in direct response to an event, such as one triggered by a window size change, will not occur under update idletasks alone.",
            source: "Tcl update(n)",
            examples: "# Force a pending display change onto the screen immediately\nset ::status \"Loading...\"\nupdate idletasks\n\n# Stay responsive to events during a long-running computation\nset done 0\nafter 5000 {set done 1}\nwhile {!$done} {\n    doWorkChunk\n    update\n}",
            return_value: "Always returns an empty string, whether or not idletasks is given.",
        }),
        forms: FORMS,
        arg_values: &[(0, IDLETASKS_VALUES)],
        ..CommandSpec::DEFAULT
    }
}
