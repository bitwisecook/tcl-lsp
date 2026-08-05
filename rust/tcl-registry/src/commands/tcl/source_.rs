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

//! `source` — evaluate a file or resource as a Tcl script.
//!
//! Tcl 8.4's SYNOPSIS documents three forms: the universal `source
//! fileName`, plus two forms restricted to classic (pre-OS X) Mac OS,
//! `source -rsrc resourceName ?fileName?` and `source -rsrcid resourceId
//! ?fileName?`, for sourcing code out of a Macintosh resource fork.
//! Neither Mac form survives into any later fetched SYNOPSIS — dropped
//! once Tcl 8.5 moved on from the classic-Mac build. `tcl-lsp` does not
//! model classic Mac OS as a dialect of its own, so the two forms are
//! recorded in `FORMS` below (gated `TCL84`) for hover/completion only,
//! not as `OptionSpec`s — see the `arity` field's own comment for why.
//! Tcl 8.4 has no `-encoding` option at all.
//!
//! Tcl 8.5 replaced the Mac forms with `-encoding encodingName`
//! (unchanged in wording through 9.1) and newly documented that a
//! leading BOM is ignored for a "unicode" encoding, and that omitting
//! `-encoding` assumes the platform's *system* encoding. Tcl 8.6's
//! source.htm is textually identical to 8.5's source.html (same two
//! forms, same BOM wording "(utf-8, unicode)", same system-encoding
//! default). Tcl 9.0 keeps the same two forms and the same `-encoding`
//! option, but its prose shifts in two ways: the BOM paragraph spells
//! out the unicode-family encodings explicitly, "(utf-8, utf-16,
//! ucs-2)", instead of 8.5/8.6's vaguer "(utf-8, unicode)"; and — the one
//! genuine behavioural delta — omitting `-encoding` now assumes utf-8
//! unconditionally rather than the system encoding. Tcl 9.1's source.html
//! is textually identical to 9.0's, the same "no 9.1-specific delta"
//! shape `return_.rs`/`eval_.rs` document for their own commands.
//!
//! Every fetched version (8.4 through 9.1) documents the same
//! end-of-file behaviour, word for word: reading stops at the first
//! ASCII `^Z` (`\x1a`) byte on every platform, letting a sourced file
//! carry arbitrary trailing data after its Tcl script (a "scripted
//! document") — a restriction `read`/`gets` do not share.
//!
//! `source` is one of the K36322151 bans (no real per-request filesystem
//! in the iRules TMM data plane), so it is unreachable under `f5-irules`:
//! its `dialects` group below is `ALL_TCL` (no `IRULES` bit), so it never
//! intersects the bare `IRULES` availability mask — the ban falls straight
//! out of that omission, with no separate disable list, the same `ALL_TCL`
//! convention `open`/`cd` rely on (see their own specs).

use crate::prelude::*;

// Tcl 8.4's SYNOPSIS documents `source -rsrc resourceName ?fileName?` and
// `source -rsrcid resourceId ?fileName?` alongside the plain `source
// fileName` form — both restricted to classic (pre-OS X) Mac OS, both
// absent from every later version's SYNOPSIS. They are recorded here,
// gated `TCL84`, purely for hover/completion; see the `arity` comment on
// `spec()` for why they are not modelled as `OptionSpec`s, and note this
// codebase's own bytecode-VM `cmd_source` (`tcl-vm/src/command.rs`)
// implements only the plain and `-encoding` forms, never `-rsrc`/
// `-rsrcid`. Tcl 8.5 dropped both Mac forms outright and added
// `-encoding encodingName`, unchanged in wording through 9.1 — only its
// omitted-default behaviour shifts (see the `-encoding` `OptionSpec`
// below).
const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "source fileName",
        dialects: None,
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "source -encoding encodingName fileName",
        dialects: Some(DialectSet::TCL85_PLUS),
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "source -rsrc resourceName ?fileName?",
        dialects: Some(DialectSet::TCL84),
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "source -rsrcid resourceId ?fileName?",
        dialects: Some(DialectSet::TCL84),
    },
];

// `source`'s own action is a definite file read (never a write — it
// never modifies fileName itself). What the sourced script's own
// top-level commands then do is unenumerable from the raw argument
// words — exactly the same "unknowable statically" situation `eval`'s
// and `apply`'s specs face — so a second `Unknown`/no-reads/no-writes
// placeholder entry sits alongside the concrete `FileIo` read rather
// than folding an invented write onto it.
const SIDE_EFFECTS: &[SideEffect] = &[
    SideEffect {
        target: SideEffectTarget::FileIo,
        reads: true,
        writes: false,
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

/// Command spec for `source`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "source",
        // Core Tcl 8.4-9.1 (present in every fetched manpage, only its
        // SYNOPSIS/option set shifts — see the module doc comment and
        // FORMS above). Unavailable under `f5-irules` because its
        // `dialects` group is `ALL_TCL` (no `IRULES` bit) — `source` is
        // one of the K36322151 bans, alongside `open`/`cd`/`exec`/`file`/
        // `glob`/… (no real per-request filesystem in the TMM data
        // plane) — so it never intersects the bare `IRULES` availability
        // mask, the same `ALL_TCL` convention `open`/`cd` rely on. Adding
        // the `IRULES` bit here would re-admit `source` under `f5-irules`,
        // the exact trap this omission avoids; there is no separate
        // disable list.
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::SOURCES_FILE
            // Evaluates another file's script in this interpreter, so that
            // file's commands can call back into whatever this one defines
            // — the signal `tcl_compiler::unit_scope` needs to stop
            // treating this file's own call sites as the complete caller
            // set (issue #977).
            | Traits::LOADS_EXTERNAL_UNIT
            | Traits::DYNAMIC_EVAL_BODY
            | Traits::SAFE_INTERP_HIDDEN
            // fileName is unconditionally read and evaluated as Tcl code:
            // attacker influence over the path lets a caller redirect
            // execution to a file of its own choosing — the exact
            // downstream hazard `cd`'s own spec names `source` as a
            // victim of ("lets the caller redirect every later
            // relative-path operation (`source`, `open`, `exec`,
            // `glob`, …) to a directory of its choosing"). Shaped like
            // `apply` (a reference to code — a path — rather than "the
            // whole tail is literally concatenated script text"), so
            // this pairs with `TAINT_SINK` alone, deliberately not
            // `EVALUATES_CODE` — see `apply.rs`'s comment on that exact
            // distinction, which also keeps `source` off the
            // `eval`/`uplevel`-style hardcoded dynamic-barrier shortcut
            // in `tcl_compiler::side_effects::classify_side_effects` (it
            // keys on `EVALUATES_CODE`, not `TAINT_SINK`), so this
            // spec's own `SIDE_EFFECTS` above is the operative
            // classification there.
            | Traits::TAINT_SINK
            // Moving a `source` call across a proc boundary changes
            // behaviour: the sourced file's top-level commands run in
            // whichever frame calls it — a top-level `set` inside the
            // file creates or overwrites a *local* of the calling proc,
            // never a frame of its own — exactly like `eval`, not like
            // `apply`/`proc` (which do open a fresh frame). Drives the
            // frame-sensitivity union
            // (`CommandRegistry::is_frame_sensitive`), the inline-proc
            // code action's safety decline, and the memory-clobber /
            // may-escape classification `eval`/`uplevel` already get
            // from this same trait.
            | Traits::CREATES_BARRIER,
        // Positional-only count: always exactly 1 (fileName). Every
        // fetched SYNOPSIS (8.4 through 9.1) documents only two shapes —
        // `source fileName` (1 word) and, 8.5+, `source -encoding
        // encodingName fileName` (3 words) — with no valid 2-word or
        // 4-or-more-word call; this project's own bytecode-VM `cmd_source`
        // (`tcl-vm/src/command.rs`) implements exactly that same
        // 1-or-3-word shape and rejects everything else, so
        // `source somefile.tcl otherfile.tcl` is just as invalid as
        // `source -encoding` alone. The registry's arity checker skips a
        // recognised *leading* option (`-encoding`, declared below,
        // gated `TCL85_PLUS`) together with its value word before
        // counting positionals, so both accepted shapes reduce to this
        // same single positional. Tcl 8.4's SYNOPSIS additionally allows
        // an *optional* trailing fileName after `-rsrc`/`-rsrcid`
        // (classic Mac OS resource-fork sourcing, not modelled as an
        // `OptionSpec` — see FORMS above); this `exact(1)` does not
        // capture that 0-or-1 shape for the 8.4-only Mac forms, a known,
        // deliberate gap given how obscure and platform-locked that
        // syntax is (and that this project's own VM never implements
        // it).
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        options: const {
            &[OptionSpec {
                name: "-encoding",
                value: OptionValue::value("encodingName"),
                detail: "The character encoding of fileName's contents. Added in Tcl 8.5 — Tcl 8.4's source has no -encoding option. When omitted, Tcl 8.5 and 8.6 assume the platform's system encoding (see encoding system); Tcl 9.0 and 9.1 instead always assume utf-8.",
                dialects: Some(DialectSet::TCL85_PLUS),
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        hover: Some(HoverSnippet {
            summary: "Evaluate a file or resource as a Tcl script.",
            synopsis: &["source fileName", "source -encoding encodingName fileName"],
            snippet: "Reads fileName from disk and evaluates its contents as a Tcl script in the caller's own context — inside a proc body, the file's top-level commands run in that proc's frame exactly as if they had been typed there directly, so a top-level `set` in the file can create or overwrite a local variable of the calling proc. The command's own result is the result of the last command executed in the file; if the file raises an error, source propagates it unchanged. A return at the file's top level does not unwind past source itself — it only ends the file early, and source returns normally with the returned value, letting a sourced file guard itself with an early return (e.g. a compatibility check) without also unwinding the caller. Inside the sourced file, info script reports the path currently being sourced.\n\nReading stops at the first ASCII ^Z (0x1A) byte on every platform, so a Tcl script can be followed by arbitrary binary or data content in the same file (a \"scripted document\"); read and gets have no such restriction.\n\nSince Tcl 8.5, -encoding selects the character encoding fileName is decoded with, and a leading byte-order mark is silently stripped whenever the effective encoding is a Unicode one. When -encoding is omitted, Tcl 8.5 and 8.6 fall back to the platform's system encoding; Tcl 9.0 and 9.1 instead always assume utf-8. Tcl 8.4 has no -encoding option at all, and instead supports two Macintosh-only forms, source -rsrc resourceName ?fileName? and source -rsrcid resourceId ?fileName?, for sourcing code out of a classic Mac OS resource fork; both were dropped in Tcl 8.5.\n\n**Security**: fileName is read and executed unconditionally, so attacker influence over the path lets a caller redirect execution to a file of its own choosing — treat a non-literal fileName the same as eval of arbitrary code, and validate it against a known-good set before sourcing.",
            source: "Tcl source(n)",
            examples: "source config.tcl\nsource -encoding utf-8 helpers.tcl\n\nforeach lib {utils.tcl widgets.tcl} {\n    source $lib\n}",
            return_value: "The result of the last command executed in the sourced file, or the value a top-level return in that file supplied (without unwinding past source itself). Propagates any error the script raises.",
        }),
        forms: FORMS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Source),
        ..CommandSpec::DEFAULT
    }
}
