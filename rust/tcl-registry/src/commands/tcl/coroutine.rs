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

//! `coroutine` — create a coroutine.
//
// Added in Tcl 8.6 (TIP 328): tcl-lang.org's 8.4 and 8.5 TclCmd trees have
// no coroutine.n page (a soft-404 "URL Not Found" page under a 200 status)
// — confirmed absent, not merely undocumented. The 8.6/9.0/9.1 manpages
// document an identical `coroutine name command ?arg...?` synopsis; 9.0
// additionally documents the sibling `coroinject`/`coroprobe` commands
// (their own registry specs), not present in 8.6.
use crate::prelude::*;

// `coroutine` both (a) creates the named command in the interpreter's
// command table — an `InterpState` write, matching `yield`/`yieldto`/
// `tailcall`'s sibling specs — and (b) immediately runs `command ?arg...?`
// up to its first `yield`/`yieldto` or its own return, which (like
// `apply`'s lambda body) can do anything the invoked command can do.
// Unlike `proc`, whose body only executes on a later call, this dynamic
// portion happens synchronously inside the `coroutine` invocation itself,
// so it is folded into this command's own side effects rather than
// deferred to a callee.
const SIDE_EFFECTS: &[SideEffect] = &[
    SideEffect {
        target: SideEffectTarget::InterpState,
        writes: true,
        ..SideEffect::DEFAULT
    },
    SideEffect {
        ..SideEffect::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "coroutine name command ?arg...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroutine",
        traits: Traits::NOT_PROC_FACTORY | Traits::BYTE_COMPILED | Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Name)],
        // `coroutine NAME cmd ?arg …?` creates the command NAME
        // (`TclNRCoroutineObjCmd`, `tclBasic.c`) — later calls to a literal
        // NAME resolve to the coroutine, not an unknown command.
        defines_command_at: Some(0),
        // Arg 1 is the command the coroutine runs, invoked with the trailing
        // args appended. The appended count is `Unknown` (never
        // arity-checked) rather than a fixed count: after the first call,
        // `name ?value...?` can resume the suspended context with any
        // number of values, which become the result of whichever `yield`/
        // `yieldto` is currently suspended — not a statically bounded
        // arity, exactly like `coroinject`/`coroprobe`'s callback prefix.
        command_prefixes: &[(1, AppendedArity::Unknown)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Create a named coroutine and run it up to its first suspension or completion.",
            synopsis: &["coroutine name command ?arg...?"],
            snippet: "Creates a new command called name backed by a coroutine context, then immediately calls command ?arg...? inside it — command is resolved in the namespace coroutine itself was called from, but once running it starts in the global namespace with no caller stack frames above it (uplevel/upvar cannot reach past the coroutine's own frames). Inside the context, yield suspends execution and its value becomes the result of this coroutine call (or of whichever call resumed the coroutine); yieldto instead cedes control to another command, and the resuming call's arguments come back as a list. If command returns or raises an exception without ever yielding, name is deleted immediately and its result (or error) becomes the result of this call. name can also be removed early with `rename name {}`, which fires any pending variable-deletion traces inside the coroutine's frames; `info coroutine` reports the name of the currently running coroutine. From Tcl 9.0, a suspended coroutine's state can additionally be inspected with coroprobe or have a command queued for its next resumption with coroinject.",
            source: "Tcl man page coroutine(n)",
            examples: "proc counter {} {\n    set i 0\n    while 1 {\n        yield $i\n        incr i\n    }\n}\ncoroutine next counter\nputs [next]\nputs [next]\nrename next {}",
            return_value: "The value passed to the coroutine's first yield or yieldto call, or command's own result if it returns without ever yielding.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
