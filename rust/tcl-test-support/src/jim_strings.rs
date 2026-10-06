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

//! Shared original Jim string conformance specimens.
//!
//! Each engine runs these sources against the independently discovered pinned
//! Jim oracle. The specimens supply no handler, compiler or storage authority.

/// Byte equality, numeric-unit ordering, limited/nocase options and simple case mapping.
pub const JIM_COMPARISON_AND_CASE_SCRIPTS: &[&str] = &[
    "set a [binary format H* ff];set b [binary format H* c3bf];list [string equal $a $b] [string compare $a $b] [string equal -nocase $a $b]",
    "string equal -length 0 a b",
    "string compare -length -1 a b",
    "string equal -length 1 ab ac",
    "string compare -nocase ß SS",
    "string equal -nocase 𐐨 𐐀",
    "string equal - a A",
    "string equal -n a A",
    "string equal -l 1 a A",
    "string equal -length BAD a A",
    "set s [binary format H* ff];string toupper $s",
    "set s [binary format H* c39f];string toupper $s",
    "set s [binary format H* c4b0];string tolower $s",
    "set s [binary format H* c7b3c4b0ff];string totitle $s",
    "set s [binary format H* 410042];string tolower $s",
    "set s [binary format H* eda080];string toupper $s",
    "set s [binary format H* f09090a8];string toupper $s",
    "set s [binary format H* c341c3a9];set r [string range $s 0 1];string compare $r $s",
];

/// Native first/last searches, retained character counts and integer-expression repetition.
pub const JIM_SEARCH_AND_REPEAT_SCRIPTS: &[&str] = &[
    "string first a abc",
    "string first a abc 1",
    "string first {} abc",
    "string first a abc -1",
    "string last a a",
    "string last a a end",
    "string last ab abc 1",
    "string last a aaa 1",
    "string last a aaa -1",
    "set a [binary format H* ff];set b [binary format H* c3bf];list [string first $a $b] [string last $a $b]",
    "set a [binary format H* bf];set b [binary format H* c3bf];list [string first $a $b] [string last $a $b]",
    "set s [binary format H* c341c3a9];set r [string range $s 0 1];list [string first A $r] [string last A $r]",
    "set s [binary format H* 410042];set n [binary format H* 00];list [string first $n $s] [string last $n $s]",
    "set s [binary format H* eda08058eda080];set n [binary format H* eda080];list [string first $n $s] [string last $n $s]",
    "set s [binary format H* ff];string repeat $s 2",
    "set s [binary format H* ff];string repeat $s {1+1}",
];

/// Pinned glob bracket/star rules, raw numeric units, surrogate encodings and embedded NULs.
/// Native decodes beyond owned storage are tested through the separate
/// host-refusal guard rather than treated as admitted guest observations.
pub const JIM_GLOB_SCRIPTS: &[&str] = &[
    "string match ? {}",
    "string match {?*} {}",
    "string match {*?} {}",
    "string match * {}",
    "string match -bogus a a",
    "string match - a a",
    "string match {} {}",
    "string match {} a",
    "string match {[]]} ]",
    "string match {[a} a",
    "string match {[a} ab",
    "string match {[a-]} ^",
    "string match [binary format H* ff] [binary format H* c3bf]",
    "string match -nocase [binary format H* ff] [binary format H* c5b8]",
    "string match [binary format H* bf] [binary format H* c3bf]",
    "string match {?} [binary format H* eda080]",
    "string match [binary format H* eda080] [binary format H* eda080]",
    "string match [binary format H* 410042] [binary format H* 410042]",
    "string match {A?B} [binary format H* 410042]",
    "string match [binary format H* 5c0041] [binary format H* 5c0041]",
    "string match -nocase SS [binary format H* c39f]",
    "set s [binary format H* c341c3a9];set r [string range $s 0 1];string match {?A?} $r",
];

/// Native trim sets, backward malformed-unit scans and shared/unshared count receipts.
pub const JIM_TRIM_SCRIPTS: &[&str] = &[
    "set s [binary format H* c341c3a92020];set r [string range $s 0 3];set t [string trimright $r];binary scan $t H* h;list $h [string length $t] [string length $r]",
    "set s [binary format H* c341c3a92020];set t [string trimright [string range $s 0 3]];binary scan $t H* h;list $h [string length $t]",
    "set s [binary format H* c341c3a92020];set t [string trim [string range $s 0 3]];binary scan $t H* h;list $h [string length $t]",
    "set s [binary format H* 20c341c3a92020];set t [string trim [string range $s 0 4]];binary scan $t H* h;list $h [string length $t]",
    "set s [binary format H* c341c3a92020];set r [string range $s 0 3];set t [string trimleft $r];binary scan $t H* h;list $h [string length $t]",
    "set s [binary format H* c341c3a9];set r [string range $s 0 1];set t [string trimright $r];binary scan $t H* h;list $h [string length $t]",
    "set t [string trim [binary format H* 0b0c00c2a0c2852020090a0d]];binary scan $t H* h;list $h [string length $t]",
    "set t [string trim [binary format H* 004100]];binary scan $t H* h;list $h [string length $t]",
    "set t [string trim [binary format H* ff41ff] [binary format H* c3bf]];binary scan $t H* h;list $h [string length $t]",
    "set t [string trimleft [binary format H* c3bf41ff] [binary format H* ff]];binary scan $t H* h;list $h [string length $t]",
    "set t [string trimright [binary format H* c3bf41ff] [binary format H* c3bf]];binary scan $t H* h;list $h [string length $t]",
    "set t [string trim [binary format H* eda08041eda080] [binary format H* eda080]];binary scan $t H* h;list $h [string length $t]",
    "set t [string trim [binary format H* 2020]];binary scan $t H* h;list $h [string length $t]",
    "set t [string trim [binary format H* ff20] {}];binary scan $t H* h;list $h [string length $t]",
    "set t [string trimright [binary format H* ffbf] {}];binary scan $t H* h;list $h [string length $t]",
    "set t [string trimright [binary format H* 41bfbf] {}];binary scan $t H* h;list $h [string length $t]",
    "set t [string trimleft [binary format H* ffbf] {}];binary scan $t H* h;list $h [string length $t]",
    "set t [string trimright [binary format H* ffbf] [binary format H* ff]];binary scan $t H* h;list $h [string length $t]",
];
