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

//! `yieldto` — suspend a coroutine, ceding execution to another command.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

// `yieldto` is a Tcl 8.6 addition (TIP 328, alongside `coroutine`/`yield`):
// coroutine.html 404s with a genuine "URL Not Found" page — not a redirect
// quirk — on both the 8.4 and 8.5 doc trees (neither has any coroutine.n
// page at all), while 8.6 (served under .htm), 9.0, and 9.1 all serve the
// shared `coroutine`/`yield`/`yieldto` manpage. Its `yieldto command
// ?arg...?` synopsis and DESCRIPTION prose are byte-for-byte identical
// across all three: the only textual deltas anywhere on the whole page
// across those versions are in the *other* commands it shares the page
// with — 9.0 added `coroinject`/`coroprobe` (absent from 8.6) and swapped
// the page's own recommended "multi-argument yield" idiom from `yieldto
// return -level 0 $value` (8.6) to `tailcall yieldto string cat $value`
// (9.0/9.1) — neither edit touches yieldto's own documented syntax or
// behaviour (the 8.6 idiom still works unchanged on 9.0/9.1; the page just
// stopped recommending it). Confirmed independently against source:
// `generic/tclBasic.c`'s command table carries an unchanged
// `TclNRYieldToObjCmd`/`TclCompileYieldToCmd` pairing in the 8.6.16, 9.0.4,
// and 9.1b0 trees alike — so this is a genuinely bytecode-compiled core
// builtin, not just a recognised one — and `TclNRYieldToObjCmd`'s body
// (the `objc < 2` arity check, the "yieldto can only be called in a
// coroutine" / "yieldto called in deleted namespace" error paths, and the
// tailcall-relay mechanism itself — "This is essentially code from
// TclNRTailcallObjCmd", per the source's own comment) is identical across
// all three versions bar internal C-API churn (`int`/`ClientData` becoming
// `Tcl_Size`/`TCL_UNUSED(void *)`/`TCL_INDEX_NONE` in 9.x, and one added
// `corPtr->yieldPtr` bookkeeping line in 9.0+ with no user-visible effect).
// The one version-specific wrinkle found anywhere: Tcl 9.1's command table
// additionally tags `yieldto` (also `tailcall`, `list`, `lappend`) with the
// new `CMD_COMPILES_EXPANDED` flag, absent from both 8.6 and 9.0 — a
// compiler-internal instruction-selection detail (inline bytecode stays
// valid across a `{*}`-expanded argument) with no user-visible effect on
// script behaviour, arity, errors, or the return value, so — exactly as
// `tailcall_.rs` treats the identical flag on `tailcall` — it is not
// reflected as a spec field here. `yieldto`'s `CmdInfo` row also carries
// `CMD_IS_SAFE` in all three trees, so it is not hidden inside a safe
// interpreter.
//
// No sibling dialect directory (irules/iapps/tmsh/expect/tk/itcl/eda_*)
// registers its own `yieldto` spec or override, so availability outside
// plain Tcl follows the bare version gate below: iRules (embedded Tcl
// 8.4.6), iApps/tmsh/Quartus/Mentor/Xilinx (Tcl 8.5) all predate 8.6 and so
// never reach it; Expect and the Cadence/Synopsys EDA shells embed a real
// 8.6+ core with no disable list of their own touching it, so they resolve
// it like any other Tcl 8.6+ command. The `bpf` dialect's profile mask
// nominally intersects `TCL86_PLUS` too (it tracks Tcl 9.0), but
// `bpf-tcl-ir`'s own front-end (`is_concurrency_command`,
// `bpf-tcl-ir/src/lower.rs`) separately and deliberately rejects
// `coroutine`/`yield`/`yieldto`/`coroinject`/`coroprobe` with a dedicated
// `OutOfSubset` diagnostic — coroutine suspension has no meaning for an
// eBPF program, which is a single bounded run to a verdict — so no
// additional dialect exclusion belongs on this spec.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "yieldto command ?arg ...?",
    ..FormSpec::DEFAULT
}];

// `yieldto`'s own effect — parking the coroutine and scheduling `command`
// to run via the same caller-environment relay `tailcall` itself uses
// ("This is essentially code from TclNRTailcallObjCmd", per
// `TclNRYieldToObjCmd`'s own source comment) — is InterpState-changing and
// always-true, independent of whatever `command` itself later does once
// invoked. Matches `tailcall`'s own `SIDE_EFFECTS` entry and reasoning
// exactly, rather than adding a second `Unknown`-target entry for
// `command`'s unknowable effects the way `coroinject`/`coroprobe` do —
// `command` is a runtime-resolved reference tracked separately via
// `command_prefixes` below, not text this spec re-evaluates itself.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// Command spec for `yieldto`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yieldto",
        surface: Some(SpecSurface::TCL86_PLUS),
        // Deliberately no `TAINT_SINK`: `command` is looked up and invoked
        // by name with already-substituted argument words — ordinary
        // dynamic command dispatch, the same mechanism a plain `$cmd arg`
        // call uses — never re-parsed as Tcl source text the way
        // `eval`/`uplevel`/`apply`'s bodies are. `yieldto` shares its
        // relay mechanism with `tailcall` (confirmed directly in
        // `TclNRYieldToObjCmd`'s own source comment, see the module
        // comment above), which likewise carries no `TAINT_SINK`.
        //
        // Deliberately no `TAINT_SOURCE` either: the list `yieldto`
        // returns comes from whatever arguments a same-trust-level Tcl
        // caller supplies when it next resumes this coroutine — not a
        // read across an external trust boundary the way
        // `gets`/`read`/`exec`/`socket` are — mirroring `yield`'s own
        // resumption value, which carries neither trait.
        //
        // `NOT_PROC_FACTORY`: an ordinary call like `yieldto dispatch key
        // {status ok}` matches the `HEAD NAME ARGS BODY` four-token shape
        // the proc-factory signature scanner looks for, exactly like
        // `tailcall dispatch key {status ok}` does — both are variadic
        // "command name plus its own arguments" dispatchers, so both need
        // this trait to keep the scanner from mistaking a plain `yieldto`
        // call for a `proc`-defining wrapper call.
        traits: Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::NOT_PROC_FACTORY
            | Traits::COROUTINE_PRIMITIVE
            | Traits::TRANSFERS_CONTROL,
        arity: Arity::at_least(1),
        // "The return value of the yieldto call will be the list of
        // arguments passed to the context command" (Tcl coroutine(n)) —
        // always list-shaped, not a bare string, whenever this call does
        // return.
        //
        // Deliberately no `return_elements: ListOfArgs`: that fact means
        // "the result is exactly this call's own trailing argument words"
        // (`list a b c` ⇒ `{a b c}`), but the list `yieldto` returns is
        // built from a *different*, later, decoupled invocation's
        // arguments (whoever next resumes this coroutine), never from
        // this call's own `arg ...` words — `ListOfArgs` would misdescribe
        // it.
        return_type: Some(TclType::List),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Suspend the current coroutine, ceding execution to another command until resumed.",
            synopsis: &["yieldto command ?arg ...?"],
            snippet: "Suspends the currently running coroutine and, instead of yielding a plain value, cedes execution to command — resolved in the coroutine's own current namespace, then invoked with the trailing arg words as its own arguments verbatim. command may be any command, including another coroutine's own context command (letting two or more coroutines transfer control directly between each other) or return, which `yieldto return -level 0 $value` uses to build a yield that a caller can resume with more than one value. Once this coroutine's own context command is next called — with any number of arguments — the yieldto call itself completes, and its result is the list of exactly those arguments; unpack it with lassign or similar. Calling yieldto outside a coroutine raises \"yieldto can only be called in a coroutine\"; calling it after the coroutine's owning namespace has already been deleted raises \"yieldto called in deleted namespace\".",
            source: "Tcl coroutine(n)",
            examples: "proc yieldm {value} {\n    yieldto return -level 0 $value\n}\n\nproc juggler {name target {value \"\"}} {\n    if {$value eq \"\"} {\n        set value [yield [info coroutine]]\n    }\n    while {$value ne \"\"} {\n        puts \"$name : $value\"\n        set value [string range $value 0 end-1]\n        lassign [yieldto $target $value] value\n    }\n}\ncoroutine j1 juggler Larry [coroutine j2 juggler Curly [coroutine j3 juggler Moe j1]] \"Nyuck!Nyuck!Nyuck!\"",
            return_value: "The list of arguments passed when this coroutine's own context command is next called to resume it.",
        }),
        forms: FORMS,
        // `yieldto command ?arg ...?` — arg 0 is the command ceded to,
        // invoked with the trailing args appended verbatim (a variable
        // count). Declaring it a command prefix makes references /
        // go-to-definition / rename see the command named in a `yieldto`
        // call the same as a direct call. `Unknown` (not `AtLeast(0)`),
        // for exactly the reason `tailcall`'s identical-shaped
        // `command_prefixes` entry gives: those trailing words are
        // ordinary sibling arguments of the `yieldto` call itself, not
        // extras the runtime appends the way `coroinject`'s two extra
        // words are.
        command_prefixes: &[(0, AppendedArity::Unknown)],
        ..CommandSpec::DEFAULT
    }
}
