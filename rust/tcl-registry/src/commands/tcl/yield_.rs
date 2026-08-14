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

//! `yield` — suspend a coroutine and produce a value to its resumer.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    // Suspending the running coroutine — saving its continuation and
    // handing control back to whichever call resumes it — is yield's own,
    // always-true effect on the interpreter's execution state, independent
    // of whatever value is produced. Matches the sibling control-transfer
    // commands in this directory (`return`, `break`, `continue`, `throw`,
    // `exit`, `tailcall`), all of which declare `writes: true` here too.
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "yield ?value?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `yield`.
///
/// `yield` is a Tcl 8.6 addition (TIP 328, coroutines) documented on the
/// shared `coroutine.n` manpage alongside `coroutine` and `yieldto` — there
/// has never been a standalone `yield.n` page (`yield.html`/`.htm` 404s on
/// every version's doc tree, 8.4 through 9.1, confirmed by fetching all
/// five). The shared `coroutine.n` page itself 404s on both the 8.4 and 8.5
/// doc trees (a genuine "URL Not Found", not a redirect quirk) — coroutines
/// do not exist before 8.6. The 8.6.18, 9.0.4, and 9.1b0 manpages document
/// `yield`'s SYNOPSIS (`yield ?value?`) and DESCRIPTION identically, word
/// for word (confirmed by diffing the fetched pages); the only changes
/// anywhere on the shared page across those three versions are 9.0 adding
/// the unrelated `coroinject`/`coroprobe` siblings (their own registry
/// specs, not present in 8.6) and a one-word typo fix between 9.0 and 9.1
/// in the coroprobe paragraph ("any any" -> "and any"). No option, arity,
/// default, or error condition of `yield` itself has changed since its
/// introduction.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yield",
        // `TCL86_PLUS` alone already resolves availability correctly
        // everywhere, via the mask-intersection rule
        // (`CommandSpec::supports_dialect` / `ProfileQueries::is_available`):
        // iRules/iApps/tmsh/the Xilinx/Quartus/Mentor EDA shells all mask in
        // an 8.4 or 8.5 Tcl-version bit only (`tcl-dialect/src/profile.rs`),
        // none of which intersects `TCL86_PLUS`, so `yield` is correctly
        // unavailable in all six (their embedded cores predate coroutines).
        // Expect and the Cadence/Synopsys EDA shells mask in `TCL86`, and
        // BPF masks in `TCL90` — all four intersect `TCL86_PLUS`, so `yield`
        // correctly resolves there too (each embeds a real 8.6+/9.0+ core
        // with no override of its own: grepping `irules/`, `iapps/`,
        // `expect/`, `eda_*/`, `tk/`, and `itcl/` for "yield" finds no
        // dialect-specific form, ban, or extra option anywhere). `f5-bigip`'s
        // mask carries no Tcl-version bit at all (a config-file surface, not
        // a Tcl command surface).
        dialects: Some(DialectSet::TCL86_PLUS),
        traits: Traits::BYTE_COMPILED | Traits::LANGUAGE_KEYWORD | Traits::TRANSFERS_CONTROL,
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Suspend the current coroutine and produce a value to whichever call resumes it.",
            synopsis: &["yield ?value?"],
            snippet: "Suspends the innermost coroutine currently running and produces value (the empty string if omitted) as the result of whichever call is currently resuming it — the enclosing coroutine command on the coroutine's first suspension, or the coroutine's own command name (name ?value...?) on every suspension after that. Execution resumes, and this yield call itself returns, only once that command name is invoked again; the single value passed to it (again the empty string if none is given) becomes yield's own result at the point it had suspended, and the coroutine continues running from there. yield may only run inside a coroutine's own execution context — directly, or from a procedure the coroutine calls — calling it anywhere else (the top level, or an ordinary call not currently running inside a coroutine) is an error. Because yield accepts only a single resumption value, a coroutine that needs to resume with more than one argument at once uses yieldto instead. Unchanged from its introduction in Tcl 8.6 through 9.0 and 9.1; no yield command exists before Tcl 8.6.",
            source: "Tcl coroutine(n)",
            examples: "coroutine accumulator apply {{} {\n    set total 0\n    while 1 {\n        incr total [yield $total]\n    }\n}}\nputs [accumulator 5]   ;# -> 5\nputs [accumulator 10]  ;# -> 15\nrename accumulator {}\n\n# yield with no value produces the empty string\nproc ping {} {\n    while 1 {\n        yield\n    }\n}\ncoroutine pinger ping\nputs [string length [pinger]]  ;# -> 0\nrename pinger {}",
            return_value: "The value passed by whichever call resumes the coroutine after this yield suspends it (the coroutine's own command invoked as name ?value...?), or the empty string if that call supplies none.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
