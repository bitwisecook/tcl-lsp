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

//! The iRules-specific checks and the event lifecycle.

// IRULE2002 / IRULE2003 / IRULE3102 / IRULE1002 — F5 iRules checks.
//
// Dialect-gated to f5-irules (silent under plain Tcl). IRULE2002: deprecated
// command → modern replacement. IRULE2003: unsafe command (Error). IRULE3102:
// HTTP getters should use -normalized. IRULE1002: unknown event name.
pub(super) mod irules_checks {
    use super::super::*;

    #[test]
    fn deprecated_command_suggests_replacement() {
        crate::install_surfaces();
        let ds = of_code("client_addr", IR, "IRULE2002");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("deprecated"));
        assert!(ds[0].0.contains("IP::client_addr"));
        assert_eq!(ds[0].1, Severity::Warning);
        assert!(
            of_code("redirect \"http://example.com\"", IR, "IRULE2002")[0]
                .0
                .contains("HTTP::redirect")
        );
        assert!(
            of_code("set uri [http_uri]", IR, "IRULE2002")[0]
                .0
                .contains("HTTP::uri")
        );
    }

    #[test]
    fn deprecated_not_flagged_under_plain_tcl() {
        crate::install_surfaces();
        assert!(!fires("set x 1", D, "IRULE2002"));
        assert!(!fires("client_addr", D, "IRULE2002"));
    }

    #[test]
    fn unsafe_command_is_error() {
        crate::install_surfaces();
        let ds = of_code("uplevel 1 {set x 1}", IR, "IRULE2003");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("unsafe"));
        assert_eq!(ds[0].1, Severity::Error);
        assert_eq!(count("history", IR, "IRULE2003"), 1);
        // Not checked under plain Tcl.
        assert!(!fires("uplevel 1 {set x 1}", D, "IRULE2003"));
    }

    #[test]
    fn http_getter_should_be_normalized() {
        crate::install_surfaces();
        let ds = of_code("set p [HTTP::path]", IR, "IRULE3102");
        assert_ne!(
            ds,
            [] as [(
                std::string::String,
                tcl_compiler::analyser::Severity,
                std::vec::Vec<std::string::String>
            ); 0]
        );
        assert!(ds[0].0.contains("HTTP::path -normalized"));
        assert_eq!(ds[0].1, Severity::Warning);
        // Already normalized / setter form / plain Tcl → clean.
        assert!(!fires("set p [HTTP::path -normalized]", IR, "IRULE3102"));
        assert!(!fires("HTTP::path /lowercase/path", IR, "IRULE3102"));
        assert!(!fires("set p [HTTP::path]", D, "IRULE3102"));
    }

    #[test]
    fn unknown_event_warns() {
        crate::install_surfaces();
        let ds = of_code("when FAKE_EVENT {puts hi}", IR, "IRULE1002");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("Unknown iRules event"));
        assert_eq!(ds[0].1, Severity::Warning);
        assert!(!fires("when HTTP_REQUEST {puts hi}", IR, "IRULE1002"));
        assert!(!fires("when FAKE_EVENT {puts hi}", D, "IRULE1002"));
    }
}

// Unified event lifecycle.
//
// The three states are independently reportable: an event deprecated but
// still present draws IRULE1003 naming its deprecating release, while one
// retired before the target release draws IRULE1002 instead — retirement
// outranks deprecation, and the surviving range is described with the
// exclusive upper bound.
mod event_lifecycle {
    use super::super::*;

    #[test]
    fn a_deprecated_but_present_event_names_its_deprecating_release() {
        crate::install_surfaces();
        let ds = of_code("when AUTH_SUCCESS {puts hi}", IR, "IRULE1003");
        assert_eq!(
            ds.len(),
            1,
            "{:?}",
            codes("when AUTH_SUCCESS {puts hi}", IR)
        );
        assert!(ds[0].0.contains("9.4.0"), "{:?}", ds[0].0);
        // Still available, so it is not reported as retired.
        assert!(!fires("when AUTH_SUCCESS {puts hi}", IR, "IRULE1002"));
    }

    #[test]
    fn a_retired_event_is_reported_retired_not_deprecated() {
        crate::install_surfaces();
        // The classic XML parser events are gone from BIG-IP 10.0.0, which is
        // well before any modern target release.
        let src = "when XML_BEGIN_DOCUMENT {puts hi}";
        assert!(fires(src, IR, "IRULE1002"), "{:?}", codes(src, IR));
        assert!(
            !fires(src, IR, "IRULE1003"),
            "retirement outranks deprecation: {:?}",
            codes(src, IR)
        );
        let ds = of_code(src, IR, "IRULE1002");
        assert!(
            ds[0].0.contains("not including"),
            "the retiring release is exclusive: {:?}",
            ds[0].0
        );
    }

    #[test]
    fn a_current_event_draws_neither() {
        crate::install_surfaces();
        for code in ["IRULE1002", "IRULE1003"] {
            assert!(!fires("when HTTP_REQUEST {puts hi}", IR, code));
            // XML_CONTENT_BASED_ROUTING is *not* part of the retired classic
            // XML set — a flat boolean must not lump it in with them.
            assert!(
                !fires("when XML_CONTENT_BASED_ROUTING {puts hi}", IR, code),
                "{code}: {:?}",
                codes("when XML_CONTENT_BASED_ROUTING {puts hi}", IR)
            );
        }
    }
}
