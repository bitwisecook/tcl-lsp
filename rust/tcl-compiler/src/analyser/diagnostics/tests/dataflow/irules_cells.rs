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

//! IRULE4005: the racy `static::` write.

use super::*;

#[test]
fn analyse_irule4005_racy_static_emitted_for_per_request_writes() {
    // ``static::counter`` written in HTTP_REQUEST and read
    // in HTTP_RESPONSE — both per-request events; the
    // cross-event flow is racy ⇒ IRULE4005 fires.
    let mut a = Analyser::new();
    let r = a.analyse(
        "when HTTP_REQUEST { incr static::counter }\n\
             when HTTP_RESPONSE { log local0. \"$static::counter\" }",
        "f5-irules",
    );
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::Irule4005),
        "IRULE4005 expected for racy static cross-event flow; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_irule4005_no_emit_for_rule_init_writes() {
    // ``static::config`` written in RULE_INIT is racy-safe
    // (RULE_INIT runs once at iRule load) — IRULE4005 must
    // not fire.
    let mut a = Analyser::new();
    let r = a.analyse(
        "when RULE_INIT { set static::config 1 }\n\
             when HTTP_REQUEST { log local0. \"$static::config\" }",
        "f5-irules",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::Irule4005),
        "IRULE4005 must not fire for RULE_INIT writes; got {:?}",
        r.diagnostics,
    );
}
