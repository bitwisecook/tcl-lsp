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

//! Native binary value conversion and decoder observations shared by engines.

/// Actual string/byte-array conversion contrasts, including unavailable codec
/// commands on older C and Jim. The wrapper observes code and original result.
pub const BINARY_VALUE_SCRIPTS: &[&str] = &[
    "set c [catch {set x [binary format a* \\u20ac]; binary scan $x H* h; set h} r]; list $c $r",
    "set c [catch {set x [binary format a* \\u00e9]; binary scan $x H* h; set h} r]; list $c $r",
    "set c [catch {binary encode hex \\u20ac} r]; list $c $r",
    "set c [catch {binary encode hex \\u00e9} r]; list $c $r",
    "set c [catch {binary decode hex \\u20ac} r]; list $c $r",
    "set c [catch {binary decode hex \\u00e9} r]; list $c $r",
    "set c [catch {binary decode hex [binary format H* e9]} r]; list $c $r",
    "set c [catch {set d [binary format H* e9]; set ignored [string length $d]; binary decode hex $d} r]; list $c $r",
    "set c [catch {binary scan \\u20ac H* h;set h} r]; list $c $r",
];

/// C8.6+ decoder switches, missing padding, strict tails and uuencode short
/// inputs. Each script preserves native result bytes and error codes.
pub const BINARY_DECODER_SCRIPTS: &[&str] = &[
    "catch {binary decode base64 \"YQ\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode base64 -strict \"YQ\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode base64 \"YQ==\\n\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode base64 -strict \"YQ==\\n\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode base64 \"Y\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode base64 -strict \"Y\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode base64 \"YQ==junk\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode base64 -strict \"YQ==junk\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode uuencode \"!\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode uuencode -strict \"!\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode uuencode \"!80``\\n\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode uuencode -strict \"!80``\\n\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode uuencode \"!8\\n\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode uuencode -strict \"!8\\n\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode uuencode \"z\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode uuencode -strict \"z\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode uuencode \"!8~``\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
    "catch {binary decode uuencode -strict \"!8~``\"} r o; list [dict get $o -code] [binary encode hex $r] [expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : {}}]",
];
