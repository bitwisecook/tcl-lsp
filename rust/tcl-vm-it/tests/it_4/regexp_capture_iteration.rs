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

//! Which iteration a capture inside a quantifier reports, against tclsh 8.4
//! through 9.1. 8.4/8.5 split `x{0,n}` into a prefix and a capturing final
//! `x`; 8.6 on build an iteration node whose cut follows the operand's own
//! preference, and with backreferences the extent is settled before the rest
//! is tried. `r13`'s `pref-3`, `(a+|b+)*c` on "aabbc", is `aabbc b` on 8.4 and
//! 8.5 and `aabbc bb` from 8.6.

use crate::common;

use tcl_dialect::TclVersion;

const ITERATION_CAPTURES: &str = r"foreach {re s} {
 {(a+|b+)*c} aabbc
 {(a+|b+)*} aabb
 {(a+|b+)+c} aabbc
 {(a|b)*c} aabbc
 {(ab|a)*c} ababc
 {(a+)*} aaa
 {(a+)*b} aaab
 {(b+|a+)*c} aabbc
 {(a+|b+)*c} bbaac
 {(a+|b+){2}c} aabbc
 {(a+|b+){0,3}c} aabbc
 {(a+|b+)*?c} aabbc
 {x(a+|b+)*c} xaabbc
 {((a+)|(b+))*c} aabbc
 {(a*|b*)*c} aabbc
 {(a+|b+)*(c)} aabbc
 {(.)*} abc
 {(..)*} abcd
 {(a+?|b+)*c} aabbc
 {(?:(a+|b+))*c} aabbc
 {(a+|b+)*c} aabbbc
 {(a+|b+)*c} abbbc
 {(a+?|b+?)*?c} aabbc
 {(a+?|b+?)*c} aabbc
 {(a+?)*?c} aaac
 {(a+?)*c} aaac
 {(a+)*?c} aaac
 {(ab)*?c} ababc
 {(ab)*?} abab
 {(a|ab)*?c} aabc
 {(a*?|b)*c} aabc
 {(a+|b+){0,2}?c} aabbc
 {(a+|b+){0,3}c} aabbbc
 {(a+|b+){1,3}?c} aabbc
 {((a+)|(b+))*?c} aabbc
 {(a+|b+)*?(c)} aabbc
 {(a)*} {}
 {(a*)*} {}
 {(a*)*?} {}
 {(a*)+} aaa
 {(a+|b+)*\1} aabbbb
 {(a+|b+)*?\1c} aabbbbc
 {(a|b)*\1} abb
 {(a*)*\1x} aax
 {(a+)*(b)\2} aabb
 {x((a+|b+)*)y} xaabby
 {(a+|b+)*?c|z} aabbc
 {.*(a+|b+)*c} aabbc
 {(?:(a+)|b)*c} abac
 {(a|ab)*c} aabc
 {(a|ab)c} abc
 {(a|ab)+?c} aabc
 {(?:x)(a|ab)*?c} xaabc
 {(ab|a)*?c} aabc
 {(a|ab){0,5}?c} aabc
 {(a|ab)*?} aab
 {(a|ab|ac)*?d} aacd
 {(a|ab)*} aab
 {(a|ab)} ab
 {(a|ab)$} ab
 {(a|ab)(a|ab)} aab
 {(?:a|ab)(a|ab)} aab
 {(a|ab){2}} aab
 {(a|ab)+} aab
 {(a|ab)*} ab
 {(ab|a)*} aab
 {(ab|a)} ab
 {x(ab|a)} xab
 {(a|xy)*} axy
} { puts [list $re $s [catch {regexp -inline -indices $re $s} m] $m] }";

const PREF_3: &str = "puts [regexp -inline {(a+|b+)*c} aabbc]";

#[test]
fn iteration_captures_match_every_release() {
    for oracle in tcl_test_support::required_tclshs(&TclVersion::ALL)
        .expect("matching original C interpreters")
    {
        let expected = if oracle.version.regex_star_is_iteration() {
            "aabbc bb"
        } else {
            "aabbc b"
        };
        assert_eq!(
            common::oracle_output(&oracle.path, PREF_3),
            expected,
            "{}",
            oracle.patchlevel
        );
        assert_eq!(
            common::vm_output(PREF_3, oracle.version.dialect_profile_name()),
            expected,
            "{}: compiled",
            oracle.patchlevel
        );
        assert_eq!(
            common::vm_output(ITERATION_CAPTURES, oracle.version.dialect_profile_name()),
            common::oracle_output(&oracle.path, ITERATION_CAPTURES),
            "{}: compiled",
            oracle.patchlevel
        );
    }
}
