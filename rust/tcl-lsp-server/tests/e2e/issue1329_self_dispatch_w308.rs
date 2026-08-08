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

//! Issue #1329 — bareword `my <method>` is a dispatch site, and issue #1330
//! — the ranges every `CmdCommandSite`-anchored diagnostic reports.
//!
//! Both are span-and-wire tests on purpose. #1329's diagnostic did not exist
//! at all before the fix, so "it fires" is a real assertion; #1330's *did*
//! fire and only its `range` was wrong, so a test that merely checked for
//! the code would have passed against the bug. Every assertion here pins the
//! exact `range` the editor receives.

use crate::common::{Lsp, unique_uri};
use serde_json::Value;

/// The ticket's own repro (#1329), plus the two shapes that must stay
/// silent beside it: a real method, and `my variable` — `oo::object`'s
/// unexported member, which only `my` can reach.
const SRC_1329: &str = "\
oo::class create test {
    method animTick {} { return {} }
    method anim {} {
        my nosuchmethod
        my animTick
        my variable state
        return {}
    }
}
";

/// A `[cmd] method` dispatch inside a proc body — the shape whose W308 the
/// incremental path reported at the *body fragment's* offsets (#1330).
const SRC_1330: &str = "\
oo::class create Dog {
    method bark {} { return {} }
}
proc handler {} {
    [Dog new] badmethod
}
";

/// Every diagnostic carrying `code`, as `(code, start_line, start_char,
/// end_line, end_char, message)`.
fn diags_at(lsp: &mut Lsp, uri: &str, src: &str, code: &str) -> Vec<(u64, u64, u64, u64, String)> {
    lsp.open_ready(uri, src)
        .iter()
        .filter(|d| d.get("code").and_then(Value::as_str) == Some(code))
        .map(|d| {
            let r = &d["range"];
            (
                r["start"]["line"].as_u64().unwrap_or(u64::MAX),
                r["start"]["character"].as_u64().unwrap_or(u64::MAX),
                r["end"]["line"].as_u64().unwrap_or(u64::MAX),
                r["end"]["character"].as_u64().unwrap_or(u64::MAX),
                d["message"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

/// TP (#1329) — `my nosuchmethod` draws exactly one W308, on the wire, and
/// its range covers the method word alone: line 3, `nosuchmethod` at
/// columns 11..23.
#[test]
fn bareword_my_unknown_method_draws_w308_on_the_method_word() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let found = diags_at(&mut lsp, &uri, SRC_1329, "W308");
    assert_eq!(
        found.len(),
        1,
        "exactly one W308 expected — `my animTick` and `my variable state` \
         must stay silent; got {found:?}"
    );
    let (sl, sc, el, ec, msg) = &found[0];
    assert_eq!(
        (*sl, *sc, *el, *ec),
        (3, 11, 3, 23),
        "W308 must underline `nosuchmethod` alone; got {found:?}"
    );
    assert!(
        msg.contains("nosuchmethod") && msg.contains("test"),
        "message must name the method and the enclosing class; got {msg:?}"
    );
}

/// FP guard (#1329) — nothing in the same document draws W307 either. The
/// `my` head is a dispatch keyword, not a non-literal command name, so
/// recording it as a dispatch site must not leak into W307's population.
#[test]
fn bareword_my_draws_no_w307() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let found = diags_at(&mut lsp, &uri, SRC_1329, "W307");
    assert!(
        found.is_empty(),
        "`my <method>` must never draw W307: {found:?}"
    );
}

/// #1330 — the W308 on a `[cmd] method` dispatch *inside a proc body*,
/// which is the shape the LSP's per-item analysis isolates and re-bases.
/// Before the fix the reported range was computed against the body
/// fragment's own offsets and landed on unrelated text near the top of the
/// file; it must cover `badmethod` on line 4, columns 14..23.
#[test]
fn cmd_dispatch_w308_range_covers_the_method_word() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let found = diags_at(&mut lsp, &uri, SRC_1330, "W308");
    assert_eq!(found.len(), 1, "one W308 expected; got {found:?}");
    let (sl, sc, el, ec, _) = &found[0];
    assert_eq!(
        (*sl, *sc, *el, *ec),
        (4, 14, 4, 23),
        "W308 must underline `badmethod` alone; got {found:?}"
    );
}

/// #1330's other half — a diagnostic anchored on the `CmdCommandSite`'s
/// *head* span rather than its method span. E001 for a bare `[Dog new]`
/// with no method word used to stop one byte short, underlining `[Dog new`
/// and leaving the `]` outside the squiggle; it must cover the whole
/// substitution, columns 0..9.
#[test]
fn bare_cmd_dispatch_e001_range_covers_the_closing_bracket() {
    let src = "oo::class create Dog {\n    method bark {} { return {} }\n}\n[Dog new]\n";
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let found = diags_at(&mut lsp, &uri, src, "E001");
    assert_eq!(found.len(), 1, "one E001 expected; got {found:?}");
    let (sl, sc, el, ec, _) = &found[0];
    assert_eq!(
        (*sl, *sc, *el, *ec),
        (3, 0, 3, 9),
        "E001 must underline the whole `[Dog new]` head; got {found:?}"
    );
}

/// #1330, third consumer — W307 on an unresolvable `[cmd] method` head is
/// anchored on the same head span and was short by the same byte.
#[test]
fn cmd_dispatch_w307_range_covers_the_closing_bracket() {
    let src = "[getcmd] doit\n";
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let found = diags_at(&mut lsp, &uri, src, "W307");
    assert_eq!(found.len(), 1, "one W307 expected; got {found:?}");
    let (sl, sc, el, ec, _) = &found[0];
    assert_eq!(
        (*sl, *sc, *el, *ec),
        (0, 0, 0, 8),
        "W307 must underline the whole `[getcmd]` head; got {found:?}"
    );
}
