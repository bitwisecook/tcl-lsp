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

//! `throw` — generate a machine-readable error.

use crate::prelude::*;

// `throw` is a Tcl 8.6 addition (TIP 90, alongside `try`): throw.html 404s
// with a genuine "URL Not Found" page — not a redirect quirk — on both the
// 8.4 and 8.5 doc trees (`tcl-lang.org/man/tcl{8.4,8.5}/TclCmd/throw.html`),
// while 8.6 (served under `.htm`), 9.0, and 9.1 all serve the full
// manpage. The 8.6/9.0/9.1 manpage bodies are byte-for-byte identical bar
// markup casing, doc-anchor ids, and the version banner — no textual delta
// anywhere in SYNOPSIS, DESCRIPTION, EXAMPLES, or SEE ALSO across those
// three versions: one form (`throw type message`), no options, unchanged
// since introduction.
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "throw type message",
    dialects: None,
}];

const COMPLETION_CODES: &[CompletionCode] = &[CompletionCode::Error];

/// Command spec for `throw`.
///
/// Synopsis, arity, and semantics are identical across Tcl 8.6, 9.0, and
/// 9.1 — `throw type message` has taken exactly one form since its
/// introduction, with no options ever added, removed, or changed in any of
/// the three manpages. `throw` does not exist in Tcl 8.4 or 8.5.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "throw",
        // `TCL86_PLUS`, via the mask-intersection rule
        // `CommandSpec::supports_dialect` / `ProfileQueries::is_available`,
        // already resolves availability correctly for every non-core
        // dialect with no extra gate needed: `f5-irules`'s profile mask is
        // the bare `IRULES` bit (an embedded Tcl 8.4.6 core), and
        // `f5-iapps`/`f5-tmsh`/the Quartus/Mentor/Xilinx EDA shells all
        // mask in `TCL85` (their documented Tcl base) — none of those five
        // intersect `TCL86_PLUS`, so `throw` is correctly unavailable in
        // all six (`tcl-dialect/src/profile.rs`). Expect and the
        // Cadence/Synopsys EDA shells mask in `TCL86`, and BPF masks in
        // `TCL90` — all four intersect `TCL86_PLUS`, so `throw` correctly
        // resolves there too (each embeds a real 8.6+ core with no disable
        // list of its own touching it — confirmed by grepping `irules/`,
        // `iapps/`, `expect/`, `eda_*/`, `tk/`, and `itcl/` for `"throw"`:
        // the only hits are unrelated prose — "an exception is thrown" —
        // inside other iRules commands' own hover text, no dialect
        // overrides or bans it). `f5-bigip`'s mask carries no Tcl-version
        // bit at all (a config-file surface, not a Tcl command surface),
        // so it is unaffected either way.
        dialects: Some(DialectSet::TCL86_PLUS),
        // `cmd_throw` (`tcl-vm/src/cmd_try.rs`) takes its two already-
        // substituted arguments, validates `type` as a non-empty Tcl list,
        // and builds its return-options dict directly via `options_dict` —
        // it never touches `vm` for evaluation (`let _ = vm;`) and has no
        // eval fallback of any kind, exactly like the structurally
        // identical `cmd_error`. Genuinely opens no call frame of its own,
        // so it belongs on the audited `FRAMELESS_RUNTIME` allow-list
        // alongside `error`/`return` (the
        // `frameless_runtime_covers_the_audited_allow_list` test in
        // `tcl-registry/src/registry.rs`, updated to match). Still
        // correctly excluded from `CommandRegistry::is_splice_safe` by its
        // own `TERMINATES_BLOCK` trait, the same way `error` is.
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::TERMINATES_BLOCK
            | Traits::CATCHABLE_THROW,
        arity: Arity::exact(2),
        completion: Some(CompletionDescriptor::exact(COMPLETION_CODES)),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Generate a machine-readable error",
            synopsis: &["throw type message"],
            snippet: "Unwinds the current evaluation with an error, with the same effect as `return -code error -errorcode $type $message`. type must be a syntactically valid, non-empty Tcl list of words describing the error in machine-readable form; it becomes the -errorcode return option (and, once trapped, the errorCode global) of the resulting exception. By convention the words in type go from most general to most specific, e.g. {ARITH DIVZERO {divide by zero}}. message is free-form text meant for display to a human and becomes the exception's result string (and, once trapped, the caught value). The stack unwinds until the error is trapped by an enclosing catch or try; an untrapped error that reaches the event loop is reported through bgerror, and one that reaches the top level of tclsh is printed to the console before causing a non-interactive exit (behaviour elsewhere depends on how Tcl is embedded). throw takes no options and has only this one form; it was added in Tcl 8.6 alongside try and does not exist in Tcl 8.4 or 8.5.",
            source: "Tcl throw(n)",
            examples: "# Raise an error identical to what expr produces for division by zero\nthrow {ARITH DIVZERO {divide by zero}} {divide by zero}\n\n# Catch it and inspect the machine-readable type alongside errorCode\nif {[catch {throw {MYAPP NOTFOUND} \"widget not found\"} msg]} {\n    puts \"error: $msg ($::errorCode)\"\n}",
            return_value: "Never returns to the caller: unconditionally completes with a TCL_ERROR whose result is message and whose -errorcode is type, catchable by an enclosing catch or try.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
