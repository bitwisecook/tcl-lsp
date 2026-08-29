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

//! `tailcall` — replace the current procedure with another command.

use crate::hooks::CodegenHookId;
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

// `tailcall` is a Tcl 8.6 addition (TIP 285, alongside the NRE engine):
// tailcall.html 404s with a genuine "URL Not Found" page — not a redirect
// quirk — on both the 8.4 and 8.5 doc trees, while 8.6 (served under
// .htm), 9.0, and 9.1 all serve the full manpage. Confirmed independently
// against source: `tailcall` and `TclNRTailcallObjCmd` are entirely absent
// from Tcl 8.4.20's and 8.5.19's `generic/tclBasic.c`. The 8.6/9.0/9.1
// manpage bodies are byte-for-byte identical bar markup casing, doc-anchor
// ids, and the version banner — no textual delta anywhere in SYNOPSIS,
// DESCRIPTION, EXAMPLES, or SEE ALSO across those three versions — and
// `TclNRTailcallObjCmd` itself carries the same logic in all three source
// trees (9.1's `Tcl_Size`/`TCL_INDEX_NONE` ABI migration and one added
// `Tcl_IncrRefCount` are internal-only, not behavioural). The one
// version-specific wrinkle found anywhere: Tcl 9.1's `tclBasic.c` command
// table tags `tailcall` (also `list`, `lappend`, `yieldto`) with the new
// `CMD_COMPILES_EXPANDED` flag, absent from both 8.6 and 9.0 — it lets the
// bytecode compiler keep emitting inline `tailcall` bytecode even when an
// argument uses `{*}` expansion, rather than falling back to a generic
// invoke. That is a compiler-internal instruction-selection detail with no
// user-visible effect (script behaviour, arity, errors, and the return
// value are identical either way), so it is not reflected as a spec field
// here.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "tailcall command ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `tailcall`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tailcall",
        // `TCL86_PLUS` also, via the mask-intersection rule
        // `CommandSpec::supports_dialect` / `ProfileQueries::is_available`
        // apply, already resolves availability correctly for every
        // non-core dialect with no extra gate needed: `f5-irules`'s
        // profile mask is the bare `IRULES` bit (an embedded Tcl 8.4.6
        // core — `tcl-dialect/src/profile.rs`), and `f5-iapps`/`f5-tmsh`/
        // the Quartus/Mentor/Xilinx EDA shells all mask in `TCL85` (their
        // documented Tcl base) — none of those five intersect
        // `TCL86_PLUS`, so `tailcall` is correctly unavailable in all six.
        // Expect and the Cadence/Synopsys EDA shells mask in `TCL86`, and
        // BPF masks in `TCL90` — all four intersect `TCL86_PLUS`, so
        // `tailcall` correctly resolves there too (each embeds a real
        // 8.6+ core with no disable list of its own touching it —
        // confirmed by grepping `irules/`, `iapps/`, `expect/`, `eda_*/`,
        // `tk/`, and `itcl/` for `"tailcall"`: no dialect overrides or
        // bans it). `f5-bigip`'s mask carries no Tcl-version bit at all
        // (a config-file surface, not a Tcl command surface), so it is
        // unaffected either way.
        surface: Some(SpecSurface::TCL86_PLUS),
        // `tailcall command ?arg ...?` is variadic, so an ordinary call
        // like `tailcall dispatch key {status ok}` (exactly four words,
        // the last brace-quoted) matches the `HEAD NAME ARGS BODY`
        // four-token shape the proc-factory signature scanner looks for —
        // it needs `NOT_PROC_FACTORY` to keep that scanner from mistaking
        // a plain tailcall for a `proc`-defining wrapper call, same
        // reasoning as `lindex`/`lrange`/`ledit` in this directory.
        traits: Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::NOT_PROC_FACTORY
            | Traits::REPLACES_FRAME
            | Traits::TRANSFERS_CONTROL,
        // `objc < 1` in `TclNRTailcallObjCmd` can never be true in
        // practice — `objc` always counts the `tailcall` word itself — so
        // a bare `tailcall` (zero further words) is always legal,
        // alongside any larger word count: real arity is 0..∞ in 8.6,
        // 9.0, and 9.1 alike (confirmed identical in all three source
        // trees, not just one).
        arity: Arity::any(),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            // Scheduling (or clearing) the frame's deferred replacement
            // call and forcing the current evaluation to complete
            // TCL_RETURN is `tailcall`'s own, always-true effect,
            // independent of whatever `command` itself later does once
            // invoked — matches the sibling control-transfer commands in
            // this directory (`return`, `break`, `continue`, `throw`,
            // `exit`), all of which declare `writes: true` here too.
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Replace the currently executing procedure, lambda, or method with a tail call to another command.",
            synopsis: &["tailcall command ?arg ...?"],
            snippet: "Ends the current procedure, lambda application, or TclOO method invocation by replacing it with command — a genuine tail call: the current call frame is popped and reused rather than kept on the stack, so tail-recursive code can recurse far beyond Tcl's normal nested-evaluation limit without accumulating call frames. command is resolved in the tailcalling code's own current namespace, not the namespace of whoever called it — the one difference from the otherwise-equivalent `return [uplevel 1 [list command ?arg ...?]]`. arg ... become command's own arguments verbatim, and command's eventual result (or error) becomes what callers of the tailcalling procedure see. tailcall may only be called directly from a proc, lambda, or method — anywhere else it raises \"tailcall can only be called from a proc, lambda or method\" — and it may not be invoked from within an uplevel into a procedure or inside a catch inside a procedure or lambda: both intercept the control transfer tailcall relies on, so wrapping either around a tailcall call does not behave the same as writing it directly in the calling body. A bare tailcall with no command argument clears any tailcall already scheduled earlier in the same call and otherwise ends the call like a plain return; calling tailcall command again later in the same call replaces rather than stacks with an earlier scheduled one. Added in Tcl 8.6 — no tailcall command exists in Tcl 8.4 or 8.5.",
            source: "Tcl tailcall(n)",
            examples: "proc factorial {n {accum 1}} {\n    if {$n < 2} {\n        return $accum\n    }\n    tailcall factorial [expr {$n - 1}] [expr {$accum * $n}]\n}\n\n# Delegate to a differently-shaped implementation without growing the\n# call stack, however many times this dispatch chains.\nproc process {data} {\n    if {[llength $data] > 1000} {\n        tailcall processChunked $data\n    }\n    tailcall processSimple $data\n}",
            return_value: "The result of evaluating command ?arg ...? once the current procedure/lambda/method call actually ends — from the caller's point of view, calling something that tailcalls is indistinguishable from it directly returning whatever command itself returns. A bare tailcall carries no result of its own; the call ends with whatever result was already current, exactly like a bare return.",
        }),
        codegen_hook: Some(CodegenHookId::Tailcall),
        // `tailcall command ?arg …?` — arg 0 is the command to become, invoked
        // with the trailing args appended (a variable count).  Declaring it a
        // command prefix makes references / go-to-definition / rename see the
        // proc named in a `tailcall` the same as a direct call.  `Unknown`
        // (not `AtLeast(0)`) because those trailing words are ordinary
        // sibling arguments of the `tailcall` call itself, not extras the
        // runtime materialises the way `lsort -command`'s two compared
        // elements are — the callback-head extraction only bakes further
        // words sharing the *same* braced literal as index 0, so an
        // unbraced trailing word (`tailcall foo $a $b`) would otherwise
        // silently under-count the callee's real arity.
        command_prefixes: &[(0, AppendedArity::Unknown)],
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
