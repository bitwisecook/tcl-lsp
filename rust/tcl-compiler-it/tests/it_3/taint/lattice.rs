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

//! Lattice joins: taint with mitigation colours, and the newer colours.

// Lattice join semantics (`taint_join` → `TaintLattice::join`).
//
// Pure lattice algebra: taint is a may-have (union), mitigating colours are a
// must-have (intersection). No tclsh analogue — this is the analyser's internal
// data structure.
pub(super) mod taint_join {
    use super::super::*;

    fn lat(colour: TaintColour) -> TaintLattice {
        TaintLattice::from_colours(colour)
    }
    const UNTAINTED: TaintColour = TaintColour::empty();
    const TAINTED: TaintColour = TaintColour::TAINTED;

    fn join(a: TaintColour, b: TaintColour) -> TaintLattice {
        lat(a).join(lat(b))
    }

    #[test]
    fn untainted_untainted() {
        crate::install_surfaces();
        assert!(!join(UNTAINTED, UNTAINTED).is_tainted());
    }

    #[test]
    fn tainted_untainted() {
        crate::install_surfaces();
        assert!(join(TAINTED, UNTAINTED).is_tainted());
    }

    #[test]
    fn untainted_tainted() {
        crate::install_surfaces();
        assert!(join(UNTAINTED, TAINTED).is_tainted());
    }

    #[test]
    fn tainted_tainted() {
        crate::install_surfaces();
        assert!(join(TAINTED, TAINTED).is_tainted());
    }

    #[test]
    fn path_prefixed_join_preserves_colour() {
        crate::install_surfaces();
        let p = TAINTED | TaintColour::PATH_PREFIXED;
        let r = join(p, p);
        assert!(r.is_tainted());
        assert!(r.colours.contains(TaintColour::PATH_PREFIXED));
    }

    #[test]
    fn path_prefixed_join_with_generic_loses_colour() {
        crate::install_surfaces();
        let r = join(TAINTED | TaintColour::PATH_PREFIXED, TAINTED);
        assert!(r.is_tainted());
        assert!(!r.colours.contains(TaintColour::PATH_PREFIXED));
    }

    #[test]
    fn path_prefixed_join_with_untainted() {
        crate::install_surfaces();
        // Joining PATH with untainted returns PATH unchanged (untainted is the
        // join *identity*), so PATH_PREFIXED survives.
        let r = join(TAINTED | TaintColour::PATH_PREFIXED, UNTAINTED);
        assert!(r.is_tainted());
        // FIXED: clean is now the join identity (not a mitigation annihilator),
        // so PATH_PREFIXED survives.
        assert!(r.colours.contains(TaintColour::PATH_PREFIXED));
    }

    #[test]
    fn ip_join_preserves_colour() {
        crate::install_surfaces();
        let ip = TAINTED | TaintColour::IP_ADDRESS;
        let r = join(ip, ip);
        assert!(r.is_tainted());
        assert!(r.colours.contains(TaintColour::IP_ADDRESS));
    }

    #[test]
    fn ip_join_with_generic_loses_colour() {
        crate::install_surfaces();
        let r = join(TAINTED | TaintColour::IP_ADDRESS, TAINTED);
        assert!(r.is_tainted());
        assert!(!r.colours.contains(TaintColour::IP_ADDRESS));
    }

    #[test]
    fn port_join_preserves_colour() {
        crate::install_surfaces();
        let port = TAINTED | TaintColour::PORT;
        let r = join(port, port);
        assert!(r.is_tainted());
        assert!(r.colours.contains(TaintColour::PORT));
    }

    #[test]
    fn fqdn_join_with_untainted() {
        crate::install_surfaces();
        let r = join(TAINTED | TaintColour::FQDN, UNTAINTED);
        assert!(r.is_tainted());
        // FIXED: clean is the join identity, so FQDN survives.
        assert!(r.colours.contains(TaintColour::FQDN));
    }

    #[test]
    fn different_colours_lose_both() {
        crate::install_surfaces();
        // IP_ADDRESS & PORT = 0 (different flags) → join keeps neither.
        let r = join(
            TAINTED | TaintColour::IP_ADDRESS,
            TAINTED | TaintColour::PORT,
        );
        assert!(r.is_tainted());
        assert!(!r.colours.contains(TaintColour::IP_ADDRESS));
        assert!(!r.colours.contains(TaintColour::PORT));
    }
}

// Join semantics for the extended colour set.
mod lattice_join_new_colours {
    use super::super::*;

    fn lat(colour: TaintColour) -> TaintLattice {
        TaintLattice::from_colours(colour)
    }
    const TAINTED: TaintColour = TaintColour::TAINTED;
    fn join(a: TaintColour, b: TaintColour) -> TaintLattice {
        lat(a).join(lat(b))
    }
    fn t(c: TaintColour) -> TaintColour {
        TAINTED | c
    }

    #[test]
    fn crlf_free_self() {
        crate::install_surfaces();
        let r = join(t(TaintColour::CRLF_FREE), t(TaintColour::CRLF_FREE));
        assert!(r.is_tainted() && r.colours.contains(TaintColour::CRLF_FREE));
    }

    #[test]
    fn crlf_free_with_generic_loses() {
        crate::install_surfaces();
        let r = join(t(TaintColour::CRLF_FREE), TAINTED);
        assert!(r.is_tainted() && !r.colours.contains(TaintColour::CRLF_FREE));
    }

    #[test]
    fn crlf_free_with_untainted() {
        crate::install_surfaces();
        let r = join(t(TaintColour::CRLF_FREE), TaintColour::empty());
        assert!(r.is_tainted());
        // FIXED: clean is the join identity, so CRLF_FREE survives.
        assert!(r.colours.contains(TaintColour::CRLF_FREE));
    }

    #[test]
    fn shell_atom_self() {
        crate::install_surfaces();
        let r = join(t(TaintColour::SHELL_ATOM), t(TaintColour::SHELL_ATOM));
        assert!(r.is_tainted() && r.colours.contains(TaintColour::SHELL_ATOM));
    }

    #[test]
    fn shell_atom_with_generic_loses() {
        crate::install_surfaces();
        let r = join(t(TaintColour::SHELL_ATOM), TAINTED);
        assert!(!r.colours.contains(TaintColour::SHELL_ATOM));
    }

    #[test]
    fn list_canonical_self() {
        crate::install_surfaces();
        let r = join(
            t(TaintColour::LIST_CANONICAL),
            t(TaintColour::LIST_CANONICAL),
        );
        assert!(r.colours.contains(TaintColour::LIST_CANONICAL));
    }

    #[test]
    fn regex_literal_with_html_escaped_loses_both() {
        crate::install_surfaces();
        let r = join(t(TaintColour::REGEX_LITERAL), t(TaintColour::HTML_ESCAPED));
        assert!(!r.colours.contains(TaintColour::REGEX_LITERAL));
        assert!(!r.colours.contains(TaintColour::HTML_ESCAPED));
    }

    #[test]
    fn path_normalised_self() {
        crate::install_surfaces();
        let r = join(
            t(TaintColour::PATH_NORMALISED),
            t(TaintColour::PATH_NORMALISED),
        );
        assert!(r.colours.contains(TaintColour::PATH_NORMALISED));
    }

    #[test]
    fn header_token_safe_self() {
        crate::install_surfaces();
        let r = join(
            t(TaintColour::HEADER_TOKEN_SAFE),
            t(TaintColour::HEADER_TOKEN_SAFE),
        );
        assert!(r.colours.contains(TaintColour::HEADER_TOKEN_SAFE));
    }

    #[test]
    fn html_escaped_self() {
        crate::install_surfaces();
        let r = join(t(TaintColour::HTML_ESCAPED), t(TaintColour::HTML_ESCAPED));
        assert!(r.colours.contains(TaintColour::HTML_ESCAPED));
    }

    #[test]
    fn url_encoded_self() {
        crate::install_surfaces();
        let r = join(t(TaintColour::URL_ENCODED), t(TaintColour::URL_ENCODED));
        assert!(r.colours.contains(TaintColour::URL_ENCODED));
    }

    #[test]
    fn url_encoded_with_generic_loses() {
        crate::install_surfaces();
        let r = join(t(TaintColour::URL_ENCODED), TAINTED);
        assert!(!r.colours.contains(TaintColour::URL_ENCODED));
    }

    #[test]
    fn multi_colour_join_preserves_shared() {
        crate::install_surfaces();
        let both = t(TaintColour::CRLF_FREE | TaintColour::URL_ENCODED);
        let r = join(both, both);
        assert!(r.colours.contains(TaintColour::CRLF_FREE));
        assert!(r.colours.contains(TaintColour::URL_ENCODED));
    }

    #[test]
    fn multi_colour_partial_overlap() {
        crate::install_surfaces();
        let a = t(TaintColour::CRLF_FREE | TaintColour::URL_ENCODED);
        let r = join(a, t(TaintColour::CRLF_FREE));
        assert!(r.colours.contains(TaintColour::CRLF_FREE));
        assert!(!r.colours.contains(TaintColour::URL_ENCODED));
    }
}
