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

//! Minifying the braced clause list of a `switch`-shaped command.

use super::{MinifyEnv, minify_body};

// case-list (clause list) handling

/// One element of a braced clause list, with enough shape to re-emit
/// it exactly as written.
struct CaseElement {
    /// Interior byte range in the list content (delimiters stripped).
    pub(super) value: std::ops::Range<usize>,
    /// Written `{…}`-braced.
    braced: bool,
    /// Written `"…"`-quoted.
    quoted: bool,
}

/// Split the content of a braced clause list into elements with the
/// central Tcl **list** grammar ([`tcl_syntax::list::find_element`]).
/// Returns `None` when the content is not a well-formed list — the
/// caller must then leave the original text untouched.
fn case_list_elements(inner: &str) -> Option<Vec<CaseElement>> {
    let bytes = inner.as_bytes();
    let mut out = Vec::new();
    let mut scan = 0usize;
    loop {
        match tcl_syntax::list::find_element(inner, scan) {
            Ok(Some(el)) => {
                let quoted = !el.braced
                    && el.value.start > 0
                    && bytes.get(el.value.start - 1) == Some(&b'"');
                let next = el.next;
                out.push(CaseElement {
                    value: el.value,
                    braced: el.braced,
                    quoted,
                });
                if next <= scan {
                    break;
                }
                scan = next;
            }
            Ok(None) => break,
            Err(_) => return None,
        }
    }
    Some(out)
}

/// Re-emit a clause-list element exactly as written (original
/// delimiters and interior spelling preserved).
fn case_element_text<'s>(inner: &'s str, el: &CaseElement) -> &'s str {
    if el.braced || el.quoted {
        // `value` strips the delimiters; the closer sits at `value.end`.
        &inner[(el.value.start - 1)..=el.value.end]
    } else {
        &inner[el.value.clone()]
    }
}

/// Minify the content of a `case_list` command's braced clause list.
///
/// A braced case list is a Tcl **list**, not a script: `#` and `;`
/// are ordinary pattern characters there (C Tcl's
/// `TclNRSwitchObjCmd` splits it with `TclListObjGetElements`,
/// `generic/tclCmdMZ.c`), so the content is decoded with the central
/// list grammar, never the script lexer: a script lexer drops a valid `#`
/// pattern and its body as a "comment" (tclsh 9.0.4: `switch # { # {puts
/// matched} default {puts default} }` prints `matched`).
///
/// Only `{…}`-braced **body** elements are recursively minified (their
/// content is literal script text); every flag, pattern, fall-through
/// `-`, and non-braced body is re-emitted exactly as written.  The
/// original text is preserved untouched when the content is not a
/// well-formed list or when any clause is missing its body (Tcl
/// errors on the odd-length list and the error message quotes the
/// original).
pub(super) fn minify_case_list(
    inner: &str,
    cl: &tcl_registry::CaseListSpec,
    env: MinifyEnv<'_>,
    depth: u32,
    bodies: &[tcl_compiler::registry_invocation::OriginalSourceScriptBody],
) -> String {
    let Some(elements) = case_list_elements(inner) else {
        return inner.to_owned();
    };
    if elements.is_empty() {
        return String::new();
    }

    // Walk clauses with the shared registry-derived shape (clause flags may
    // precede a pattern — Expect's `-re` / `-timeout 5`).  In particular,
    // this resolves the same unique abbreviations as the compiler, folding,
    // and semantic-token walkers.
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: cl.clause_flags,
        clause_value_flags: cl.clause_value_flags,
    };

    let mut parts: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < elements.len() {
        let mut options_ended = false;
        // Leading clause flags (and their value words).
        while !shape.clause_flags.is_empty()
            && i < elements.len()
            && !options_ended
            && !elements[i].braced
        {
            let flag = &elements[i];
            let word = &inner[flag.value.clone()];
            if !word.starts_with('-') {
                break;
            }
            let Some(canonical_flag) = shape.resolve_flag(word) else {
                // An option-shaped word that is neither canonical nor a
                // unique abbreviation is an Expect error.  Retain the
                // source rather than accidentally reclassifying its words.
                return inner.to_owned();
            };
            parts.push(case_element_text(inner, flag).to_owned());
            i += 1;
            // Read off `cl`'s own descriptor, like `CaseListSpec::inline_clauses`
            // does for the compiler — a future case-list command's own
            // end-of-clause-options flag (if it even has one) is whatever its
            // pack declares, not necessarily "--".
            if cl.clause_end_options_flag == Some(canonical_flag) {
                options_ended = true;
            } else if shape.flag_takes_value(canonical_flag) && i < elements.len() {
                parts.push(case_element_text(inner, &elements[i]).to_owned());
                i += 1;
            }
        }
        // Pattern + body.  A trailing pattern with no body is a malformed
        // list ("extra switch pattern with no body") — preserve the
        // original so the runtime error (and its quoted text) is
        // unchanged.
        if i >= elements.len() {
            break;
        }
        if i + 1 >= elements.len() {
            return inner.to_owned();
        }
        let (pattern, body) = (&elements[i], &elements[i + 1]);
        parts.push(case_element_text(inner, pattern).to_owned());
        let body_src = &inner[body.value.clone()];
        if body.braced
            && env
                .original_region(body_src)
                .is_some_and(|span| bodies.iter().any(|body| body.content_span() == span))
        {
            let minified = minify_body(
                body_src,
                env,
                depth,
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
            );
            parts.push(format!("{{{minified}}}"));
        } else {
            parts.push(case_element_text(inner, body).to_owned());
        }
        i += 2;
    }
    parts.join(" ")
}
