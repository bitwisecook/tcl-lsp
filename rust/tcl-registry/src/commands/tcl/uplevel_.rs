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

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uplevel ?level? arg ?arg ...?",
}];

/// Whether `word` is *literally* an `uplevel` frame level.
///
/// Mirrors C Tcl's `TclObjGetFrame` first-character dispatch: an argument
/// is consumed as a level iff it begins with `#` (absolute frame, `#0`) or
/// a digit (relative frame, `1`). A literal level is the frame selector
/// even with no script following it (`uplevel 1` alone is a wrong-#args
/// error, but `1` is still a level, not a command named `1`).
fn word_is_literal_level(word: &str) -> bool {
    matches!(word.as_bytes().first(), Some(&b) if b == b'#' || b.is_ascii_digit())
}

/// Whether `word` is a *substituted* level selector — `$lvl` or
/// `[expr {$n-1}]`.
///
/// Its runtime value can't be inspected from source, so it only counts as
/// a level when a script word follows it: `uplevel $lvl {…}` shifts frame,
/// but a lone `uplevel $body` is an implicit-level-1 script whose body *is*
/// that argument. The arg-role layer only needs to know a level is
/// *present* — enough to place the body word one slot later so its braced
/// script recurses.
fn word_is_dynamic_level(word: &str) -> bool {
    word.starts_with('$') || (word.starts_with('[') && word.ends_with(']'))
}

/// Index of the first *script* word in an `uplevel` argument list — `1`
/// when a leading `level` word is present, else `0`.
fn uplevel_script_start(args: &[&str]) -> usize {
    match args.first() {
        Some(w) if word_is_literal_level(w) => 1,
        // A substituted level only separates from the script when a script
        // word follows (`uplevel $lvl {…}`); a lone `uplevel $body` is a body.
        Some(w) if args.len() >= 2 && word_is_dynamic_level(w) => 1,
        _ => 0,
    }
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
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uplevel",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::EVALUATES_CODE
            | Traits::TAINT_SINK
            | Traits::UNSAFE
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::DYNAMIC_EVAL_BODY,
        arity: Arity::at_least(1),
        // The body runs in another stack frame (the `level`), not the
        // caller's, so its variable references belong to that frame — mark
        // the body `Structural` so SSA skips it when scanning the enclosing
        // block's dataflow (it is still recursed for highlighting / its own
        // scope), exactly as `proc` / `namespace eval` bodies are.
        body_kind: BodyKind::Structural,
        arg_role_resolver: Some(uplevel_arg_roles),
        lowering_hook: Some(crate::hooks::LoweringHookId::Uplevel),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Execute a script in a different stack frame",
            synopsis: &["uplevel ?level? arg ?arg ...?"],
            snippet: "All of the arg arguments are concatenated as if they had been passed to concat; the result is then evaluated in the variable context indicated by level.",
            source: "Tcl man page uplevel.n",
            examples: "",
            return_value: "",
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
