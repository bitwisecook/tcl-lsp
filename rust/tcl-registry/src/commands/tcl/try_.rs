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

//! `try` — trap and process errors and exceptions.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

// `body`, every handler `script`, and `finally` can do literally anything
// (`cmd_try` in `tcl-vm/src/cmd_try.rs` evaluates each one with
// `vm.eval_source`), so — exactly like `catch`, its structural cousin —
// the static spec can only declare the generic worst case rather than a
// specific `SideEffectTarget`.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

// `try body ?handler...? ?finally script?` is `try`'s only documented
// form, unchanged since its introduction: Tcl 8.6.18's try.htm, Tcl
// 9.0.4's try.html, and Tcl 9.1b0's try.html all give this identical
// one-line SYNOPSIS verbatim (the 9.0/9.1 pages additionally grow a
// third EXAMPLES entry over 8.6's two, with no synopsis or grammar
// change alongside it). `try` has no manpage at all in Tcl 8.4 or
// 8.5 — both `tcl-lang.org/man/tcl{8.4,8.5}/TclCmd/try.html` serve a
// genuine "URL Not Found" page (HTTP 200 with a soft-404 body, not a
// redirect quirk — the same pattern `throw_.rs` documents for
// `throw`), consistent with `try` being a Tcl 8.6 addition (TIP 329,
// alongside `throw`). `surface: None` here inherits the command's own
// `TCL86_PLUS` gate below, so this single entry already correctly
// excludes 8.4/8.5 and every dialect pinned to an 8.4/8.5 base.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "try body ?handler...? ?finally script?",
    ..FormSpec::DEFAULT
}];

/// Whether a handler-body word is the literal `-` fallthrough marker
/// (issue #703). Tcl recognises a body of `-` by string value, so the
/// braced `{-}` and quoted `"-"` forms — which evaluate to the same
/// string — are equally fallthroughs. Role-resolver callers may pass
/// the word either stripped (`-`) or brace/quote-inclusive (`{-}`),
/// so one layer of matched `{}`/`""` is stripped before comparing.
pub(crate) fn is_dash_fallthrough(arg: &str) -> bool {
    let stripped = arg
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .or_else(|| arg.strip_prefix('"').and_then(|s| s.strip_suffix('"')))
        .unwrap_or(arg);
    stripped == "-"
}

/// Dynamic arg role resolver for `try`/`on`/`trap`/`finally`.
///
/// The structural keyword words (`on`/`trap`/`finally`) carry
/// `ArgRole::Keyword` so the semantic-token layer highlights them as
/// keywords rather than strings.
fn try_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let mut roles = Vec::new();
    if !args.is_empty() {
        roles.push((0, ArgRole::Body));
    }
    let mut i: usize = 1;
    let push_keyword = |roles: &mut Vec<(u8, ArgRole)>, index: usize| {
        if let Ok(idx) = u8::try_from(index) {
            roles.push((idx, ArgRole::Keyword));
        }
    };
    while i < args.len() {
        let kw = args[i];
        if kw == "finally" && i + 1 < args.len() {
            push_keyword(&mut roles, i);
            if let Ok(idx) = u8::try_from(i + 1) {
                roles.push((idx, ArgRole::Body));
            }
            i += 2;
        } else if (kw == "on" || kw == "trap") && i + 3 < args.len() {
            push_keyword(&mut roles, i);
            // A handler body of literal `-` is a fallthrough marker (shares the
            // next handler's body, like `switch`); it is not a script, so it
            // gets no BODY role.
            if !is_dash_fallthrough(args[i + 3])
                && let Ok(idx) = u8::try_from(i + 3)
            {
                roles.push((idx, ArgRole::Body));
            }
            i += 4;
        } else {
            i += 1;
        }
    }
    roles
}

/// The word immediately after `body` (index 1), when present, is always
/// the head of the first handler clause or a bare `finally` — every
/// later clause-head position shifts with how many 4-word `on`/`trap`
/// clauses precede it, so index 1 is the one spot [`CommandSpec::arg_values`]'s
/// fixed-index model can describe exactly (`cmd_try`'s own
/// `parse_clauses` rejects anything else there with `bad handler type
/// "X": must be finally, on, or trap`). Every occurrence — not just this
/// first one — still gets `ArgRole::Keyword` from [`try_arg_roles`]
/// above, which does not depend on position.
const FIRST_CLAUSE_KEYWORD_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "on",
        detail: "on code variableList script — matches an exact completion code: ok, error, return, break, continue, or an integer.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "trap",
        detail: "trap pattern variableList script — matches an error whose -errorcode has pattern as a leading prefix.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "finally",
        detail: "finally script — always runs last, whether body succeeded, errored, or was otherwise handled.",
        ..ArgValue::DEFAULT
    },
];

/// Command spec for `try`.
///
/// `try` does not exist before Tcl 8.6 (see the version note on
/// `FORMS` above); Tcl 8.6.18, 9.0.4, and 9.1b0 document identical
/// grammar, handler forms, and semantics — no option, form, or
/// behavioural delta of any kind across those three releases. The
/// 9.0/9.1 manpages add a third EXAMPLES entry (a `finally`-guarded
/// `read` inside a proc that also `return`s) over 8.6's two, but that
/// is new example prose, not a semantic change: a plain `return`
/// inside `body` has always propagated outward once `finally` has
/// run, on 8.6 as much as 9.0+.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "try",
        // `FRAMELESS_RUNTIME` deliberately absent: unlike `throw`/`error`/
        // `return` (which build their completion directly with no `vm`
        // touch), `cmd_try` evaluates `body`, every handler `script`, and
        // `finally` through `vm.eval_source` — a genuine eval fallback —
        // so a call to `try` needs a real frame in the callee. `try` is
        // correctly absent from the
        // `frameless_runtime_covers_the_audited_allow_list` test in
        // `tcl-registry/src/registry.rs`.
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::NEVER_INLINE_BODY
            | Traits::BRANCH_SELECTED_BODY,
        // `TCL86_PLUS`, via the mask-intersection rule
        // `CommandSpec::supports_dialect` / `ProfileQueries::is_available`,
        // already resolves availability correctly for every non-core dialect
        // with no extra gate needed: `f5-irules`'s profile point is the iRules
        // family alone (an embedded Tcl 8.4.6 core), and
        // `f5-iapps`/`f5-tmsh`/the Quartus/Mentor/Xilinx EDA shells all mask
        // in `TCL85` (their documented Tcl base) — none of those five
        // intersect `TCL86_PLUS`, so `try` is correctly unavailable in all six
        // (`tcl-dialect/src/profile.rs`). `f5-bigip`'s mask carries no Tcl
        // release at all (a config-file surface, not a Tcl command surface),
        // so it is unaffected either way. Expect and the Cadence/Synopsys EDA
        // shells mask in `TCL86`, and BPF masks in `TCL90` — all four
        // intersect `TCL86_PLUS`, so `try` correctly resolves there too (each
        // embeds a real 8.6+/9.0+ core with no disable list of its own
        // touching it — confirmed by grepping `irules/`, `iapps/`, `expect/`,
        // `eda_*/`, `tk/`, and `itcl/` for `"try"`: no hits at all, so no
        // dialect overrides or bans it).
        surface: Some(SpecSurface::TCL86_PLUS),
        // Minimum 1 (`body` is mandatory — `try` with no arguments is
        // "wrong # args: should be \"try body ?handler ...? ?finally
        // script?\"", confirmed in `cmd_try`'s own `USAGE` string); no
        // fixed maximum, since any number of `on`/`trap` handler clauses
        // may precede an optional trailing `finally`. The real grammar is
        // a clause chain (each handler exactly 4 words, `finally` exactly
        // 2 and only as the very last clause) finer than a flat
        // `min..=max` range can express — `cmd_try`'s own `parse_clauses`
        // (`tcl-vm/src/cmd_try.rs`) enforces that shape at runtime with
        // dedicated "wrong # args to on/trap/finally clause" messages;
        // this floor is the coarse static bound the generic arity check
        // uses.
        arity: Arity::at_least(1),
        arg_role_resolver: Some(try_arg_roles),
        lowering_hook: Some(crate::hooks::LoweringHookId::Try),
        inline_codegen_hook: Some(crate::hooks::InlineCodegenHookId::Try),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Trap and process errors and exceptions",
            synopsis: &["try body ?handler...? ?finally script?"],
            snippet: "Evaluates body, then dispatches to at most one matching handler clause, tried in the order written, based on how body completed. Each clause is either \"on code variableList script\" — matching an exact completion code, where code is ok, error, return, break, or continue, or the equivalent integer 0 through 4 — or \"trap pattern variableList script\" — matching only when body raised an error whose -errorcode option has pattern as a leading prefix, compared element-wise with inter-word spacing normalised in both (not as a raw string prefix). An empty pattern (trap {}) therefore matches any error, so on error and trap {} are equivalent, and an earlier on error masks every later trap. A handler's variableList holds up to two variable names: when the first is present and non-empty it receives body's own result (its error message, on an error), and when the second is present and non-empty it receives the options dictionary as of that same moment. A handler script that is exactly \"-\" falls through to the next handler's script, just like a switch body of \"-\" — the last handler may not itself be \"-\". Once a handler has run, or none matched, an optional trailing finally script always runs — even when body or the handler raised an error, and irrespective of which handler, if any, matched. A finally that completes normally discards its own result and lets the handler's (or body's, when none matched) result and completion code propagate outward unchanged, so a plain return inside body still returns from the enclosing procedure once finally has run; a finally that does not complete normally replaces that outcome with its own instead. Whenever body, a handler, or finally raises a new exception while superseding an earlier one, the superseded exception's options dictionary is recorded on the new one under its -during key. try does not exist before Tcl 8.6, where it was added alongside throw.",
            source: "Tcl try(n)",
            examples: "# Guarantee cleanup regardless of outcome\nset f [open /some/file/name a]\ntry {\n    puts $f \"some message\"\n} finally {\n    close $f\n}\n\n# Differentiate error causes by their -errorcode\ntry {\n    set f [open /some/file/name r]\n} trap {POSIX EISDIR} {} {\n    puts \"it's a directory\"\n} trap {POSIX ENOENT} {} {\n    puts \"it doesn't exist\"\n}\n\n# finally still runs even though the body returns\nproc readFile {filename} {\n    set f [open $filename r]\n    try {\n        return [read $f]\n    } finally {\n        close $f\n    }\n}",
            return_value: "The result and completion code of whichever handler matched, or of body itself when no handler matched, once finally (if present) has completed normally; a finally that does not itself complete with code ok replaces this outcome instead.",
        }),
        forms: FORMS,
        arg_values: &[(1, FIRST_CLAUSE_KEYWORD_VALUES)],
        closed_value_args: &[1],
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Try),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_indices(args: &[&str]) -> Vec<u8> {
        let mut idx: Vec<u8> = try_arg_roles(args)
            .into_iter()
            .filter(|(_, role)| *role == ArgRole::Body)
            .map(|(i, _)| i)
            .collect();
        idx.sort_unstable();
        idx
    }

    #[test]
    fn dash_handler_body_gets_no_body_role() {
        // Issue #703: a `-` fallthrough handler body is not a script, so it
        // must carry no `ArgRole::Body` (mirrors `switch`). Index layout for
        // `try <body> on ok result - trap NONE result <body>`:
        //   0 body, 1 on, 2 ok, 3 result, 4 `-`, 5 trap, 6 NONE, 7 result, 8 body
        let args = [
            "{...}", "on", "ok", "result", "-", "trap", "NONE", "result", "{...}",
        ];
        let indices = body_indices(&args);
        assert!(!indices.contains(&4), "`-` body must get no Body role");
        assert!(indices.contains(&0), "try body keeps Body role");
        assert!(indices.contains(&8), "real handler body keeps Body role");
    }

    #[test]
    fn braced_and_quoted_dash_body_get_no_body_role() {
        // The braced `{-}` / quoted `"-"` forms evaluate to the same string
        // and are equally fallthroughs.
        for dash in ["{-}", "\"-\""] {
            let args = ["{...}", "on", "ok", "a", dash, "trap", "NONE", "b", "{...}"];
            assert!(
                !body_indices(&args).contains(&4),
                "{dash} body must get no Body role",
            );
        }
    }

    #[test]
    fn ordinary_handler_body_keeps_body_role() {
        let args = ["{...}", "on", "error", "msg", "{puts $msg}"];
        assert!(body_indices(&args).contains(&4));
    }
}
