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

//! `unload` — unload a shared library extension.
use crate::prelude::*;

// `unload` does not exist before Tcl 8.5 — introduced by TIP 100 (George
// Petasis, 2003). Confirmed three independent ways: Tcl 8.4's
// TclCmd/contents.htm command index lists `load` but never `unload`;
// Tcl 8.4's own load.n manpage says outright "It is not possible to
// unload or reload a package"; and a live fetch of both
// tcl8.4/TclCmd/unload.html and the .htm swap returns the site's generic
// "URL Not Found" page (HTTP 200, the same not-found template used
// site-wide for a missing page) rather than a manpage. Once introduced,
// unload's three-line SYNOPSIS and its three switches are unchanged
// through 9.1 (raw manpage HTML diffed line-by-line for every adjacent
// version pair: 8.5→8.6, 8.6→9.0, 9.0→9.1) — the only textual deltas are:
// the second argument's name, `packageName` in 8.5's synopsis, renamed
// `prefix` from 8.6 on with no change in grammar or behaviour; the
// auto-guessed prefix (used when the argument is omitted or empty)
// additionally strips a `tcl9` infix from Tcl 9.0 on, alongside the
// pre-existing `lib` prefix strip (unload libtcl9foo.so now guesses
// `Foo`, not `Tcl9foo`); the C unload-hook prototype's type renamed
// `Tcl_PackageUnloadProc` (8.5/8.6) to `Tcl_LibraryUnloadProc` (9.0/9.1),
// a C-API-only change that never touches the Tcl-level command; and Tcl
// 9.1 adds one documentation-only paragraph instructing a library's
// unload procedure to free its own commands/callbacks/handlers/
// resources, again with no Tcl-level command behaviour it changes.
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "unload ?switches? fileName ?prefix? ?interp?",
    dialects: Some(DialectSet::TCL85_PLUS),
}];

// All three switches exist, unchanged, in every Tcl version that has
// `unload` at all (8.5-9.1); gated to TCL85_PLUS individually — see the
// note on `dialects` in `spec()` below for why the command-level gate
// can't say the same today.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-nocomplain",
        value: OptionValue::flag(),
        detail: "Suppresses all error messages; unload never reports an error when this switch is given.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-keeplibrary",
        value: OptionValue::flag(),
        detail: "Prevents unload from issuing the operating system call that detaches the library from the process; the library's own unload procedure still runs and the reference counts are still updated.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "Marks the end of switches; the following argument is treated as fileName even if it begins with -.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// Command spec for `unload`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unload",
        // On the version axis this ought to be `Some(DialectSet::TCL85_PLUS)`
        // per the FORMS comment above — unload is genuinely absent from Tcl
        // 8.4. It is left at the broader `ALL_TCL` because that group is
        // also what bans it from iRules: `unload` is one of the K36322151
        // bans (iRules' TMM sandbox has no dynamic-linking surface), and a
        // banned command is now excluded simply by carrying an explicit
        // `dialects` group with no `IRULES` bit. `ALL_TCL` never intersects
        // the bare IRULES mask, so `unload` falls out of iRules by plain
        // intersection — no subtractive disable list. That contract is
        // pinned by `tcl-registry/tests/dialect_profile.rs`'s
        // `irules_banned_commands_lack_the_irules_bit`, which asserts every
        // banned name's spec lacks the `IRULES` bit. Narrowing this field
        // to `TCL85_PLUS` would keep the ban intact (that gate lacks the
        // IRULES bit too — `dict`/`lassign`/`apply`/`lmap`/`coroutine` are
        // the version-precise end state, excluded from iRules by their own
        // version gate alone), so the true 8.5+ floor is instead carried by
        // FORMS above (which nothing else in the test suite depends on),
        // and the per-option `dialects` below are set precisely rather than
        // inheriting this field's overly-permissive lower bound.
        dialects: Some(DialectSet::ALL_TCL),
        // BYTE_COMPILED: `unload` is a recognised core Tcl builtin — the
        // same literal-command-word convention `load` (its direct
        // sibling, same manpage family) already carries.
        traits: Traits::SAFE_INTERP_HIDDEN | Traits::BYTE_COMPILED,
        // Positional-argument count only: 1 (fileName alone) to 3
        // (fileName + prefix/packageName + interp), unchanged across
        // every version that has `unload` (8.5-9.1). The three switches
        // are skipped before positionals are counted, the same as
        // `load`'s -global/-lazy/--.
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Unload a shared library extension previously loaded with load.",
            synopsis: &[
                "unload ?switches? fileName",
                "unload ?switches? fileName prefix",
                "unload ?switches? fileName prefix interp",
            ],
            snippet: "Added in Tcl 8.5 (TIP 100); unload does not exist in Tcl 8.4. fileName must be exactly the string that was passed to load when the library was first loaded. prefix (named packageName in Tcl 8.5's synopsis; renamed prefix from 8.6 on, with no change in meaning) names the library and is used to compute the name of its unload procedure — Pfx_Unload in a trusted interpreter, or Pfx_SafeUnload in a safe one, where Pfx is prefix with its first letter upper-cased and the rest lower-cased. When prefix is omitted or empty, Tcl guesses it from the last path element of fileName: strip a leading \"lib\", then (from Tcl 9.0) a following \"tcl9\", then title-case what remains. interp defaults to the interpreter the unload command itself runs in. Tcl tracks separate reference counts for a library in trusted versus safe interpreters, and only detaches it from the process once both counts reach zero. The library must cooperate by exporting the matching unload procedure — one that doesn't, or a platform/build with library unloading disabled or unsupported, makes unload raise an error; -nocomplain suppresses every such error instead. -keeplibrary still runs the library's unload procedure and updates the reference counts, but skips the actual operating-system call that detaches the library from the process. unload can never remove a library that is statically linked into the application, and fileName may be empty only when prefix is given explicitly. Loading the same file under two different fileName spellings loads it into the process twice, since Tcl tracks each fileName string separately — unloading one spelling leaves the same code resident under the other, and a library the operating system has silently detached behind Tcl's own bookkeeping is unsafe to unload.",
            source: "Tcl unload(n)",
            examples: "load [file join [pwd] libfoo[info sharedlibextension]] Foo\nunload [file join [pwd] libfoo[info sharedlibextension]] Foo\nunload -nocomplain notloaded.so ;# swallow the error instead of raising it",
            return_value: "The empty string on success. If the library's own unload procedure fails, unload's result is the error message that procedure set on the interpreter.",
        }),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
