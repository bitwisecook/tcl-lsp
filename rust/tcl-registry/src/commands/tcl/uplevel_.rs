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

//! `uplevel` — execute a script in a different stack frame.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::surface;
use tcl_dialect::model::Family;

/// `uplevel`'s script can do literally anything once evaluated in the
/// target frame — set variables, open files, spawn processes — none of
/// which the compiler can enumerate from the raw argument words alone, so
/// this deliberately declares `Unknown`/no-reads/no-writes as an
/// "unknowable statically" placeholder rather than asserting a specific
/// effect the command doesn't actually have — the same idiom `eval`'s spec
/// uses (and cites), for the same reason.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    ..SideEffect::DEFAULT
}];

// `uplevel ?level? arg ?arg ...?` — synopsis, arity, and the core
// level/script semantics are byte-for-byte identical across the fetched
// Tcl 8.4, 8.5, 8.6, 9.0, and 9.1 manpages: no option or flag was ever
// added, removed, or renamed, so this is the command's only form,
// unrestricted by dialect or version. (iRules also enables it — the spec
// below carries the `IRULES` bit explicitly (`ALL_TCL.union(IRULES)`), so
// it resolves under the bare `IRULES` mask — and no sibling dialect
// directory — irules/iapps/tmsh/expect/tk/itcl/eda_* — declares any
// override or extra form for it, so every dialect that hosts a real Tcl
// core carries it unmodified, exactly like `eval`.)
//
// The DESCRIPTION prose itself has two textual deltas across versions,
// neither of which changes the grammar above: (1) the 8.5+ manpages
// additionally credit `apply` (TIP 194, added in 8.5) alongside
// `namespace eval` as a command that pushes a call frame for `uplevel`/
// `upvar` to count — 8.4's manpage, predating `apply`, mentions only
// `namespace eval`; (2) the "level cannot be omitted" clause reads
// "starts with a digit or #" in the 8.4/8.5/8.6 manpages but "is an
// integer or starts with #" in the 9.0/9.1 manpages. That wording change
// *is* a behavioural change, and it lands in 9.0 — run, not read (issue
// #1069; both directions pinned in
// `tcl_registry::frame_effect::FrameLevelWord::LeadingProbe`):
//
//   uplevel -1  {oops}   9.0.4: bad level "-1"             8.6.14: invalid command name "-1"
//   uplevel 1.0 {oops}   9.0.4: invalid command name "1.0" 8.6.14: bad level "1.0"
//
// i.e. 8.6 still commits a digit-led word to the level slot and dispatches
// a negative one as a command, exactly as its manpage says, while 9.0
// tries the whole word against `Tcl_GetIntFromObj` first. Both spellings
// are errors under both, so no working script is shaped differently; the
// level *value* grammar (`FrameLevel::parse`) is identical on both.
// 9.1's uplevel.html is byte-for-byte identical to 9.0's (bar doc-anchor
// line-number IDs) — no 9.1-specific delta exists for this command.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "uplevel ?level? arg ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// This command's own frame-crossing grammar, so the arg-role resolver and
/// `CommandSpec::frame_effect` below cannot drift apart.
const FRAME_EFFECT: FrameEffectSpec = FrameEffectSpec {
    level_word: FrameLevelWord::LeadingProbe,
    layout: FrameArgLayout::ScriptInSelectedFrame,
};

/// Index of the first *script* word in an `uplevel` argument list — `1`
/// when a leading `level` word is present, else `0`.
///
/// Delegates to [`FrameEffectSpec::level_word_len`], the single level-word
/// rule; this file previously carried a private first-byte `#`-or-digit
/// sniff, which rejected `+1`, `-0`, `-1`, `" 1"`, `0x1`, and `0b1` — all of
/// which C Tcl consumes as levels (issue #1069).  An
/// [`ArgRoleResolver`](crate::hooks) carries no dialect, so the
/// version-invariant reading is the one asked for here: it abstains on the two
/// spellings where the releases disagree (`-1`, `1.0`), leaving them as script
/// text rather than claiming the next word is a body — both are hard errors on
/// both interpreters, so no working script is shaped differently.  See
/// [`FrameLevelWord::LeadingProbe`] for the pinned per-release matrix.
fn uplevel_script_start(args: &[&str]) -> usize {
    FRAME_EFFECT.level_word_len(args)
}

/// Dynamic arg-role resolver for `uplevel ?level? arg ?arg ...?`.
///
/// `uplevel` evaluates a script — the concatenation of its trailing
/// arguments — in the stack frame named by an optional leading `level`
/// word, so the script's position is data-dependent and can't be a fixed
/// [`CommandSpec::arg_roles`] entry (unlike `eval`, whose body is always
/// arg 0). Marks the first script word [`ArgRole::Body`] so the
/// semantic-token layer, the green-tree descent, and every other
/// registry-driven body consumer recurse it as a real script instead of
/// rendering it as an opaque string (issue #837). Only a braced word
/// actually recurses — each consumer keeps its own `Str`-token guard — so
/// a bare `$body` / command-substitution body stays a value here and is
/// resolved by the compiler's const-lattice lowering instead.
fn uplevel_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let start = uplevel_script_start(args);
    u8::try_from(start)
        .ok()
        .filter(|&i| (i as usize) < args.len())
        .map(|i| vec![(i, ArgRole::Body)])
        .unwrap_or_default()
}

/// Command spec for `uplevel`.
///
/// Stable across every fetched version: Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
/// all document the same command under the same single form. The
/// level-word *detection* dispatch is the one per-release delta — see the
/// comment above `FORMS` and [`FrameLevelWord::LeadingProbe`].
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uplevel",
        // Present and unrestricted — `uplevel` carries the `IRULES` bit
        // explicitly (`ALL_TCL.union(IRULES)`), resolving under the bare
        // `IRULES` mask — a pure control-flow primitive with no
        // filesystem/process/network access of its own, so every dialect
        // that hosts a real Tcl core (irules, iapps, tmsh, the EDA shells,
        // expect, tk, itcl) carries it unmodified, exactly like `eval`. Its
        // own `unsafe_command: true` flags it as a context-escalation risk
        // inside iRules (IRULE2003) — a usage warning about a real,
        // available command, not an availability gate.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::EVALUATES_CODE
            | Traits::TAINT_SINK
            | Traits::UNSAFE
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::DYNAMIC_EVAL_BODY
            | Traits::EVALUATES_IN_SHIFTED_FRAME
            | Traits::SCRIPT_CONCATENATES_ARGS,
        arity: Arity::at_least(1),
        // The body runs in another stack frame (the `level`), not the
        // caller's, so its variable references belong to that frame — mark
        // the body `Structural` so SSA skips it when scanning the enclosing
        // block's dataflow (it is still recursed for highlighting / its own
        // scope), exactly as `proc` / `namespace eval` bodies are.
        body_kind: BodyKind::Structural,
        arg_role_resolver: Some(uplevel_arg_roles),
        // `uplevel ?level? arg …` — the level word is probed from the
        // leading word's text (`Tcl_UplevelObjCmd` → `TclObjGetFrame`), and
        // the script it runs belongs to the frame that word selects, not to
        // the frame the call is written in.  `uplevel_script_start` above is
        // the same rule expressed for the arg-role layer.
        frame_effect: Some(FRAME_EFFECT),
        lowering_hook: Some(crate::hooks::LoweringHookId::Uplevel),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Execute a script in a different stack frame",
            synopsis: &["uplevel ?level? arg ?arg ...?"],
            snippet: "All of the arg words are concatenated as if passed to concat, and the result is evaluated as a script in the stack frame named by level — with the invoking procedure itself removed from the call stack for the duration, so a command further up sees its own caller's variables, not this uplevel call's immediate caller. level is a relative integer distance up the calling stack (default 1, the immediate caller) or an absolute frame number written #N (#0 is the global/top-level frame, where only global variables are visible); level cannot be omitted when the first arg word itself looks like a level, since Tcl would otherwise be unable to tell the two apart. uplevel #0 is the standard idiom for running a script in the global namespace regardless of how deeply nested the caller is. Besides ordinary procedure calls, namespace eval always pushes a call frame of its own, and apply (Tcl 8.5+) does too, so each nested namespace eval or apply body counts as one more level for uplevel (and upvar) to walk past. info level reports the current call depth for a script that needs to compute a level dynamically.",
            source: "Tcl uplevel(n)",
            examples: "proc do {body while condition} {\n    if {$while ne \"while\"} {\n        error \"required word missing\"\n    }\n    set conditionCmd [list expr $condition]\n    while {1} {\n        uplevel 1 $body\n        if {![uplevel 1 $conditionCmd]} {\n            break\n        }\n    }\n}\n\n# Set a variable in the global namespace regardless of calling context\nproc setGlobal {name value} {\n    uplevel #0 [list set $name $value]\n}",
            return_value: "The result of evaluating the concatenated arg words as a script in the target stack frame, or the error it raises — propagated as uplevel's own result, exactly as if that script had been entered directly at that stack level.",
        }),
        // A `LIST_CANONICAL` value preserves element
        // boundaries and suppresses T100.
        taint_sink_safe_colour: Some(TaintColour::LIST_CANONICAL),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        xc_translatable: Some(false),
        unsafe_command: true,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Uplevel),
        ..CommandSpec::DEFAULT
    }
}
