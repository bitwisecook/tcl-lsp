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

//! `unknown` — handle attempts to use non-existent commands.
//!
//! The SYNOPSIS (`unknown cmdName ?arg arg ...?`) is byte-for-byte
//! identical, and every documented step of the default implementation's
//! resolution order is the same in substance, across every fetched
//! manpage from Tcl 8.4 through 9.1 — confirmed by a direct diff of the
//! fetched HTML: 8.6, 9.0, and 9.1 are byte-for-byte identical bodies
//! (bar the version-number banner and doc-anchor ids), and 9.0/9.1 are
//! in turn identical to each other. This is one form, unrestricted by
//! dialect or version; the genuine deltas are textual/behavioural
//! refinements within it, not new options or forms:
//!
//! - Tcl 8.5 added the per-namespace `namespace unknown` handler
//!   registration (TIP 181) and rewrote the handler-lookup paragraph to
//!   match: Tcl 8.4's DESCRIPTION says only that Tcl "checks for the
//!   existence of a command named unknown"; 8.5 onward instead describes
//!   looking for "an unknown handler for the current namespace",
//!   defaulting to a command named `::unknown`, resolved against the
//!   current namespace and then the global namespace.
//! - Tcl 8.6 added the clause "and Tcl is run interactively" to the
//!   auto-exec step's gating ("If the auto-load fails and Tcl is run
//!   interactively then unknown calls auto_execok…"); the Tcl 8.4 and
//!   8.5 manpages document the same step with no such qualifier.
//! - Tcl 8.6 also added "unknown" itself to the KEYWORDS list (a
//!   doc-indexing change only, no behavioural content).
//! - The manpage's own EXAMPLE chains to the saved-off original handler
//!   with `uplevel 1 [list _original_unknown {*}$args]`; Tcl 8.4's
//!   manpage instead shows `{expand}$args` in that spot, which is not
//!   real Tcl 8.4 syntax — the `{*}` expansion word is itself a Tcl 8.5
//!   addition (TIP 157) — so this reads as a documentation artefact of
//!   that late-life 8.4.x doc patch rather than working code, and is not
//!   reproduced in this spec's own `examples`.
//! - Cosmetic-only wording tweaks not reflected above: "doesn't exist"
//!   (8.4) vs "does not exist" (8.5+), "can't" (8.4) vs "cannot" (8.5+),
//!   and `namespace` joining the SEE ALSO list from 8.5 (tracking the
//!   new `namespace unknown` cross-reference).
//!
//! `unknown` is one of the K36322151 bans: it is named explicitly in the
//! *profile's* subtractive `IRULES_DISABLED_COMMANDS` list
//! (`tcl-dialect/src/profile.rs`), alongside the filesystem/process
//! primitives its own default implementation depends on to do anything
//! useful (`auto_load`, `auto_execok`, `source`, `exec`, `file`, `glob`,
//! `open`, `namespace`, `package`, …) — the iRules TMM sandbox has no
//! default `unknown` at all. `dialects: None` on the spec below is still
//! correct, not an oversight: the ban is enforced by that subtractive
//! list rather than a `CommandSpec`-level restriction, the same
//! mechanism `source`/`auto_load`/`auto_execok` rely on (see their own
//! specs). No other dialect profile (Expect, the EDA vendor shells,
//! tmsh, iApps, BPF) lists `unknown` in its `disabled_commands`, so it
//! stays reachable there.
//!
//! Attempted, for extra rigour beyond the manpage text, to cross-check
//! the auto-exec/interactive-only wording against the real
//! `library/init.tcl` source across versions; the source-tarball fetch
//! (`codeload.github.com`) was blocked by this session's egress policy
//! (403, "not allowed by your organization's egress policy"), so that
//! comparison was not made — the version note above states only what
//! the fetched manpage text itself says, not an inference about the
//! underlying C/Tcl behaviour.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    // Identical synopsis in every fetched manpage, Tcl 8.4 through 9.1.
    synopsis: "unknown cmdName ?arg arg ...?",
    dialects: None,
}];

// `unknown`'s own documented steps touch interpreter state directly: it
// reads the command table to notice cmdName isn't defined, and its
// default implementation then (1) calls auto_load, which reads library
// index files from disk and, on success, defines the missing command —
// mirroring `auto_load`'s own FileIo-read + ProcDefinition-write
// declaration; or (2) calls auto_execok and exec's a same-named external
// program, mirroring `exec`'s own Process declaration. Whatever the
// *eventually executed* command then does — the loaded proc, the
// exec'd program, or the history-/abbreviation-resolved existing
// command — is unenumerable from the raw argument words, exactly the
// "unknowable statically" situation `eval`'s and `source`'s specs face,
// so a further `Unknown`/no-reads/no-writes placeholder sits alongside
// the concrete effects rather than folding an invented write onto them.
const SIDE_EFFECTS: &[SideEffect] = &[
    SideEffect {
        target: SideEffectTarget::InterpState,
        reads: true,
        writes: false,
        connection_side: ConnectionSide::None,
        dialects: None,
    },
    SideEffect {
        target: SideEffectTarget::FileIo,
        reads: true,
        writes: false,
        connection_side: ConnectionSide::None,
        dialects: None,
    },
    SideEffect {
        target: SideEffectTarget::ProcDefinition,
        reads: false,
        writes: true,
        connection_side: ConnectionSide::None,
        dialects: None,
    },
    SideEffect {
        target: SideEffectTarget::Process,
        reads: true,
        writes: true,
        connection_side: ConnectionSide::None,
        dialects: None,
    },
    SideEffect {
        target: SideEffectTarget::Unknown,
        reads: false,
        writes: false,
        connection_side: ConnectionSide::None,
        dialects: None,
    },
];

/// Command spec for `unknown`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unknown",
        // Universal core Tcl 8.4-9.1 (present, arity- and
        // synopsis-unchanged, in every fetched manpage — see the module
        // doc comment). `dialects: None` here is still correct despite
        // the iRules ban documented above: that ban is enforced by the
        // profile's subtractive `IRULES_DISABLED_COMMANDS` list, not a
        // `CommandSpec`-level restriction — folding it into a dialect
        // mask here would be redundant with, not a substitute for, that
        // list (and would risk re-admitting it if the list ever changed
        // shape). See `source_.rs`/`auto_load.rs`/`auto_execok.rs` for
        // the identical reasoning on their own specs.
        dialects: None,
        traits: Traits::CREATES_BARRIER
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::OVERRIDABLE_LIBRARY_PROC
            // cmdName reaching `unknown` from tainted input can end up
            // passed straight to `exec` via the documented auto-exec
            // step (see the module doc comment) — the same
            // code-execution hazard `exec`'s own spec flags, reached
            // one level indirectly through a failed dispatch rather
            // than a direct call.
            | Traits::TAINT_SINK
            // A core Tcl fallback the interpreter dispatches to purely
            // by the literal name `unknown` (or a namespace's
            // registered `namespace unknown` handler) — load-bearing as
            // a literal command word in the sense `Traits::BYTE_COMPILED`
            // already covers for `eval`/`source`/`rename`, so the
            // minifier must not rewrite a call to it through a `$var`
            // alias.
            | Traits::BYTE_COMPILED,
        // `cmdName ?arg arg ...?` — at least 1 argument, unbounded
        // above. Identical in every fetched manpage, Tcl 8.4 through
        // 9.1.
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Handle attempts to use non-existent commands",
            synopsis: &["unknown cmdName ?arg arg ...?"],
            snippet: "Invoked automatically whenever a script calls a command that does not exist. The default implementation is a library procedure Tcl defines when it initialises an interpreter; you can replace it outright (rename the original aside, then define your own unknown), or, since Tcl 8.5, register a handler for just one namespace with namespace unknown (TIP 181) instead of touching the global ::unknown. A safe interpreter has no default unknown implementation at all — the auto-loading and auto-exec steps below need filesystem and process access a safe interpreter does not have.\n\nWhen dispatch fails, Tcl looks for an unknown handler for the current namespace — by default a command named ::unknown — and, if found, invokes it with the fully-substituted cmdName and args of the call that failed; its result becomes the result of that original call, and an interpreter with no such handler at all raises a plain error instead. Tcl 8.4 documents this lookup more simply, as checking for the existence of \"a command named unknown\", with no per-namespace handler concept; the current-namespace-then-global search and the namespace unknown registration both date from Tcl 8.5.\n\nThe default handler tries, in order: auto_load, to locate cmdName in a library index and define it — succeeding, it re-runs the original call unchanged; failing that, auto_execok to see whether cmdName names an external program or shell builtin, running it (with all of args) via exec if so — Tcl 8.6 onward documents this step as also requiring that Tcl itself is running interactively, a qualifier the 8.4/8.5 manpages do not state; and, only when the original call was made at top level outside of any script, csh-style history substitution for !!, !event, and ^old^new, followed by expanding cmdName as a unique abbreviation of an existing command. Setting the global variable auto_noload skips the auto-load step; auto_noexec skips the auto-exec step. If none of this resolves the command, unknown raises an error.\n\n**Security**: because a failed auto-load falls through to running cmdName as an external program, letting attacker-influenced data reach cmdName unvalidated can result in arbitrary process execution — treat a dynamically-built command name the same as an argument to exec.\n\niRules provides no unknown at all: it is one of the TMM sandbox's banned commands, alongside the filesystem/process primitives (auto_load, auto_execok, source, exec, file, glob, open, namespace, package, …) its default implementation depends on.",
            source: "Tcl unknown(n)",
            examples: "# Log every command-not-found failure, then chain to Tcl's original handler.\nrename unknown _original_unknown\nproc unknown {args} {\n    puts stderr \"WARNING: unknown command: $args\"\n    uplevel 1 [list _original_unknown {*}$args]\n}\n\n# Give one namespace its own handler instead of replacing ::unknown (Tcl 8.5+)\nnamespace eval ::mystuff {\n    namespace unknown ::mystuff::missingCommand\n    proc missingCommand {cmdName args} {\n        return -code error \"::mystuff has no command \\\"$cmdName\\\"\"\n    }\n}",
            return_value: "The result of whichever resolution step actually ran the command — the auto-loaded procedure, the auto-exec'd external program, or the history- or abbreviation-resolved existing command — becomes unknown's own result. Raises an error when cmdName cannot be resolved by any of them.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
