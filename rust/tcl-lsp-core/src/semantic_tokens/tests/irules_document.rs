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

//! iRules event names and BIG-IP object references.

use super::*;

#[test]
fn event_name_classified_as_event() {
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let ks = kinds(
        "when HTTP_REQUEST {\n  set x 1\n}\n",
        tcl_dialect::DialectProfile::irules(),
        &registry,
    );
    assert!(
        ks.contains(&(TokenKind::Event as u32)),
        "expected an event token; got {ks:?}"
    );
}

#[test]
fn malformed_when_body_does_not_classify_an_event_declaration() {
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    for source in [
        "when HTTP_REQUEST bare_body\n",
        "when HTTP_REQUEST \"quoted body\"\n",
        "when HTTP_REQUEST { unterminated",
    ] {
        let ks = kinds(source, tcl_dialect::DialectProfile::irules(), &registry);
        assert!(
            !ks.contains(&(TokenKind::Event as u32)),
            "malformed declaration colored HTTP_REQUEST as an event: {source:?}; {ks:?}"
        );
    }
}

#[test]
fn bigip_object_ref_token_in_irules_body() {
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    // `pool web_pool` inside a multi-line `when` body → `object`.
    for dialect in ["f5-irules", "irules", "tcl-irule"] {
        let ks = kinds(
            "when HTTP_REQUEST {\n  pool web_pool\n}\n",
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            &registry,
        );
        assert!(
            ks.contains(&(TokenKind::Object as u32)),
            "{dialect}: {ks:?}"
        );
    }
}

#[test]
fn bigip_object_ref_not_emitted_in_plain_tcl() {
    // The object overlay is iRules-only.
    let ks = kinds("when HTTP_REQUEST {\n  pool web_pool\n}\n", tcl(), &reg());
    assert!(!ks.contains(&(TokenKind::Object as u32)), "{ks:?}");
}

#[test]
fn is_event_name_matches_event_shape() {
    assert!(is_event_name("HTTP_REQUEST"));
    assert!(is_event_name("CLIENT_ACCEPTED"));
    assert!(!is_event_name("lowercase"));
    assert!(!is_event_name("X")); // single char — needs 2+
}
