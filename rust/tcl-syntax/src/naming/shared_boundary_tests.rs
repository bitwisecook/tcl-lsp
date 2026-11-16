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

use super::*;

#[test]
fn written_command_parts_keep_empty_tail_and_counted_components() {
    for (written, absolute, qualifiers, simple) in [
        (b"".as_slice(), false, vec![], b"".as_slice()),
        (b":".as_slice(), false, vec![], b":".as_slice()),
        (b"::::".as_slice(), true, vec![], b"".as_slice()),
        (
            b"::a:::b::".as_slice(),
            true,
            vec![b"a".as_slice(), b"b"],
            b"".as_slice(),
        ),
        (b"a:b".as_slice(), false, vec![], b"a:b".as_slice()),
        (
            b"q:::p".as_slice(),
            false,
            vec![b"q".as_slice()],
            b"p".as_slice(),
        ),
        (
            b"q:\0::\xff".as_slice(),
            false,
            vec![b"q:\0".as_slice()],
            b"\xff".as_slice(),
        ),
    ] {
        assert_eq!(
            written_command_parts(written),
            WrittenCommandParts {
                absolute,
                qualifiers,
                simple
            },
            "{written:?}",
        );
    }
}

#[test]
fn written_separator_spans_retain_complete_runs_and_counted_units() {
    // Implementation contract: naming.grammar.shared-written-name-boundaries
    // docs/design/analysis/name-resolution-proofs/shared-written-name-boundaries.md
    // Counted source geometry only; no native extent or name identity.
    for (written, separator, tail) in [
        (b"".as_slice(), None, b"".as_slice()),
        (b":".as_slice(), None, b":".as_slice()),
        (b"a:b".as_slice(), None, b"a:b".as_slice()),
        (b":::".as_slice(), Some(0..3), b"".as_slice()),
        (b"::a:::b".as_slice(), Some(3..6), b"b".as_slice()),
        (b"a:::".as_slice(), Some(1..4), b"".as_slice()),
        (b"a\0x:::b".as_slice(), Some(3..6), b"b".as_slice()),
        (b"a::b\0c".as_slice(), Some(1..3), b"b\0c".as_slice()),
        ("é:::λ".as_bytes(), Some(2..5), "λ".as_bytes()),
    ] {
        assert_eq!(written_separator_extent(written), separator);
        assert_eq!(written_command_tail(written), tail);
    }
}

#[test]
fn key_suffixes_preserve_constructed_colon_segments_without_lookup_authority() {
    // naming.analysis.procedure-summary-candidate-keys
    // docs/design/analysis/name-resolution-proofs/procedure-summary-candidate-keys.md
    // Pure constructed labels only, never Native namespace interpretation.
    assert_eq!(
        key_suffixes("::a::b::p"),
        ["::a::b::p", "a::b::p", "b::p", "p"]
    );
    assert_eq!(key_suffixes(":::::p"), [":::::p", ":::p", "p"]);
    assert_eq!(
        key_suffixes("::a:::::p"),
        ["::a:::::p", "a:::::p", ":::p", "p"]
    );
    assert_eq!(key_suffixes("::::::"), ["::::::", "::::", ":"]);
    assert_eq!(key_suffixes("::a::"), ["::a::", "a::"]);
    assert_eq!(key_suffixes("::"), ["::"]);
    assert_eq!(key_suffixes(""), [""]);
    assert_eq!(
        key_suffixes("::é::雪\0p"),
        ["::é::雪\0p", "é::雪\0p", "雪\0p"]
    );
}
