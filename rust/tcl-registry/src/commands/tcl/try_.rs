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

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "try body ?handler...? ?finally script?",
}];

/// Whether a handler-body word is the literal `-` fallthrough marker
/// (issue #703). Tcl recognises a body of `-` by string value, so the
/// braced `{-}` and quoted `"-"` forms — which evaluate to the same
/// string — are equally fallthroughs. Role-resolver callers may pass
/// the word either stripped (`-`) or brace/quote-inclusive (`{-}`),
/// so one layer of matched `{}`/`""` is stripped before comparing.
fn is_dash_fallthrough(arg: &str) -> bool {
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

/// Command spec for `try`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "try",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        arg_role_resolver: Some(try_arg_roles),
        lowering_hook: Some(crate::hooks::LoweringHookId::Try),
        inline_codegen_hook: Some(crate::hooks::InlineCodegenHookId::Try),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Trap and process errors and exceptions",
            synopsis: &["try body ?handler...? ?finally script?"],
            snippet: "This command executes the script body and, depending on what the outcome of that script is (normal exit, error, or some other exceptional result), runs a handler script to deal with the case.",
            source: "Tcl man page try.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
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
