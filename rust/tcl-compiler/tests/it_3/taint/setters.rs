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

//! IRULE3101 setter constraints, including the normalised-path constraint.

// Setter constraints (IRULE3101) — HTTP::uri / HTTP::path set to a value not
// provably starting with `/`. f5-dialect.
mod setter_constraints {
    use super::super::*;

    #[test]
    fn literal_slash_prefix_clean() {
        assert_eq!(
            of_code("HTTP::uri /newpath", IR, "IRULE3101"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn literal_no_slash_warns() {
        let ws = of_code("HTTP::uri newpath", IR, "IRULE3101");
        assert_eq!(ws.len(), 1);
        assert!(ws[0].sink_command.contains("HTTP::uri"));
    }

    #[test]
    fn path_prefixed_var_clean() {
        // f5-dialect: $uri carries PATH_PREFIXED (HTTP::uri getter).
        assert_eq!(
            of_code("set uri [HTTP::uri]\nHTTP::uri $uri", IR, "IRULE3101"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn generic_tainted_var_warns() {
        let ws = of_code("set x [HTTP::payload]\nHTTP::uri $x", IR, "IRULE3101");
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn http_path_literal_clean() {
        assert_eq!(
            of_code("HTTP::path /foo/bar", IR, "IRULE3101"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn http_path_literal_warns() {
        let ws = of_code("HTTP::path relative/path", IR, "IRULE3101");
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn path_to_path_clean() {
        assert_eq!(
            of_code("set p [HTTP::path]\nHTTP::path $p", IR, "IRULE3101"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    /// #2055: a path the lattice proves at the setter is checked as a
    /// literal is — `set p /a; HTTP::path $p` is clean — and a proven value
    /// without the `/` still warns.
    #[test]
    fn irule3101_reads_the_proven_path() {
        assert_eq!(
            of_code("set p /a\nHTTP::path $p", IR, "IRULE3101"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
        let ws = of_code("set p a\nHTTP::path $p", IR, "IRULE3101");
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "p");
    }

    /// A computed operand reads as the constant the lattice proves:
    /// `[HTTP::uri] starts_with $p` over `set p [string range /api/v1 0 3]`
    /// is IRULE3103, as the literal `/api` is, and the same test over a
    /// header value is not.
    #[test]
    fn irule3103_reads_a_proven_operand() {
        let proven = "set p [string range /api/v1 0 3]\n\
                      if {[HTTP::uri] starts_with $p} { pool api }";
        assert_eq!(of_code(proven, IR, "IRULE3103").len(), 1);
        let unknown = "set p [HTTP::header value X-Prefix]\n\
                       if {[HTTP::uri] starts_with $p} { pool api }";
        assert_eq!(
            of_code(unknown, IR, "IRULE3103"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn untainted_var_warns() {
        // Untainted var without a known `/` prefix → can't prove safety → warn.
        let ws = of_code("set x \"something\"\nHTTP::uri $x", IR, "IRULE3101");
        assert_eq!(ws.len(), 1);
    }
}

// PATH_NORMALISED / PATH_PREFIXED satisfy
// the IRULE3101 setter constraint. f5-dialect.
mod path_normalised_setter_constraint {
    use super::super::*;

    #[test]
    fn normalised_uri_suppresses_irule3101() {
        // TMM has no `file`; its normaliser is the getter's `-normalized` flag.
        let source = "when HTTP_REQUEST {\nset norm [HTTP::uri -normalized]\nHTTP::uri $norm\n}";
        assert_eq!(
            of_code(source, IR, "IRULE3101"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn generic_taint_still_fires_irule3101() {
        let source = "set raw [HTTP::query]\nHTTP::uri $raw";
        assert_ne!(
            of_code(source, IR, "IRULE3101"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn path_prefixed_still_suppresses_irule3101() {
        let source = "set path [HTTP::path]\nHTTP::uri $path";
        assert_eq!(
            of_code(source, IR, "IRULE3101"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}
