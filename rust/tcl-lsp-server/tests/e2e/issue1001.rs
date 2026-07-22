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

//! End-to-end coverage for issue #1001, against the real, packaged native
//! server: W129 (the Safe Base diagnostic) missing a hidden command reached
//! only through `[...]` bracket-substitution indirection.
//!
//! The unit tests in `tcl-compiler`'s `analyser::handlers::tests` module
//! (suffixed `_1001`) exercise every indirection path exhaustively —
//! including the pervasive `[list apply {...} $x]` deferred-command idiom
//! (`package ifneeded ... [list apply {...} $dir]`) — via `Analyser::analyse`
//! (the whole-file walk). This file confirms the fix reaches real clients
//! through the full, *incremental* LSP diagnostics pipeline
//! (`analyse_per_item`, what the live server actually uses), for every
//! indirection path **except** one reached through an `apply` (or any
//! other proc/method) body: `analyse_per_item`'s shell/body-pass split
//! defers every such body to an isolated second pass that carries no
//! `safe_interp_stack` snapshot at all — a pre-existing gap unrelated to
//! bracket-substitution (it loses a *directly-written* hidden call the
//! same way), pinned separately by
//! `safe_interp_w129_lost_across_per_item_deferred_proc_body_1001` in
//! `tcl-compiler`, not fixed here (see that test's doc for why).

use serde_json::Value;

use crate::common::{Lsp, unique_uri};

fn diagnostic_codes(lsp: &mut Lsp, uri: &str) -> Vec<String> {
    lsp.await_diagnostics(uri)
        .iter()
        .filter_map(|d| d.get("code").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

/// `package ifneeded`'s deferred script built with `[list source $file]`
/// directly (no `apply` / no proc body involved, so this reaches the real
/// server's incremental pipeline in full) — the reported issue's second
/// bullet: `list` command-quoting a hidden command straight.
#[test]
fn safe_interp_w129_reaches_list_quoted_hidden_command_in_package_ifneeded_1001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "interp create -safe s\n\
               interp eval s {\n\
                   package ifneeded myPackage 1.0 [list source pkg.tcl]\n\
               }\n";
    lsp.open_ready(&uri, src);
    let codes = diagnostic_codes(&mut lsp, &uri);
    assert!(
        codes.iter().any(|c| c == "W129"),
        "a `[list source ...]` deferred script in `package ifneeded` warns: {codes:?}"
    );
}

/// The same idiom in an `ArgRole::CommandPrefix` position (`trace add
/// variable ... command`) rather than `package ifneeded`'s `Body` position —
/// a different registry role reaching the same resolution.
#[test]
fn safe_interp_w129_reaches_list_quoted_hidden_command_in_command_prefix_1001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "interp create -safe s\n\
               interp eval s { trace add variable x write [list source pkg.tcl] }\n";
    lsp.open_ready(&uri, src);
    let codes = diagnostic_codes(&mut lsp, &uri);
    assert!(
        codes.iter().any(|c| c == "W129"),
        "a `[list source ...]` trace command-prefix warns: {codes:?}"
    );
}

/// FP guard (mirrors #954's `set data [list apply {...} value]`
/// non-invocation case): the same list-quoting shape sitting in ordinary
/// `set` data (never a `Body`/`LambdaLiteral`/`CommandPrefix` argument
/// position) must not draw W129.
#[test]
fn safe_interp_w129_does_not_flag_list_quoted_hidden_command_in_plain_data_1001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "interp create -safe s\n\
               interp eval s {\n\
                   set data [list source pkg.tcl]\n\
               }\n";
    lsp.open_ready(&uri, src);
    let codes = diagnostic_codes(&mut lsp, &uri);
    assert!(
        !codes.iter().any(|c| c == "W129"),
        "a `[list source ...]` value that is only ever stored must not warn: {codes:?}"
    );
}

/// A direct nested `[...]` bracket substitution (no `list`-quoting at all)
/// is an *immediate* invocation, wherever it appears — `set x [source
/// b.tcl]` inside a safe interpreter must warn exactly like a bare
/// top-level `source b.tcl` statement would.
#[test]
fn safe_interp_w129_reaches_direct_nested_bracket_substitution_1001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "interp create -safe s\ninterp eval s { set x [source b.tcl] }\n";
    lsp.open_ready(&uri, src);
    let codes = diagnostic_codes(&mut lsp, &uri);
    assert!(
        codes.iter().any(|c| c == "W129"),
        "a directly nested `[source ...]` substitution warns: {codes:?}"
    );
}

/// `{*}[list source $file]` as the whole statement: `{*}` expansion splices
/// `list`'s result into this statement's own argv, so the command's
/// effective head becomes `source`.
#[test]
fn safe_interp_w129_reaches_expand_list_quoted_head_1001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "interp create -safe s\ninterp eval s { {*}[list source b.tcl] }\n";
    lsp.open_ready(&uri, src);
    let codes = diagnostic_codes(&mut lsp, &uri);
    assert!(
        codes.iter().any(|c| c == "W129"),
        "`{{*}}[list source ...]` used as the whole statement warns on the \
         effective (expanded) head: {codes:?}"
    );
}

/// A safe command wrapped the same list-quoted-apply way must not warn —
/// this fix widens W129's recall, it must not start flagging ordinary,
/// allowed calls just because they are reached via `[list apply ...]`.
#[test]
fn safe_interp_w129_does_not_flag_safe_command_via_list_quoted_apply_1001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "interp create -safe s\n\
               interp eval s { after idle [list apply {x { puts $x }} val] }\n";
    lsp.open_ready(&uri, src);
    let codes = diagnostic_codes(&mut lsp, &uri);
    assert!(
        !codes.iter().any(|c| c == "W129"),
        "a safe command (`puts`) reached via list-quoted apply must not warn: {codes:?}"
    );
}
