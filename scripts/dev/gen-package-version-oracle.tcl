#!/usr/bin/env tclsh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# gen-package-version-oracle.tcl — regenerate the pinned package-version
# corpora that `rust/tcl-dialect/tests/package_version_oracle.rs` checks the
# comparator port against (issue #1090).
#
# Usage:
#   tclsh8.6 scripts/dev/gen-package-version-oracle.tcl version \
#       > rust/tcl-dialect/tests/data/package_version_oracle.txt.body
#   tclsh8.6 scripts/dev/gen-package-version-oracle.tcl select \
#       > rust/tcl-dialect/tests/data/package_select_oracle.txt.body
#
# Run it under **every** supported interpreter and diff the outputs before
# updating the pinned files: they are expected to be byte-identical on 8.6.14
# and 9.0.4, and a divergence is itself the finding.  The `#`-comment header of
# each pinned file is hand-maintained; this script emits only the data rows.

# --- version / requirement grids -------------------------------------------

# Ordinary releases: majors, minors, patch levels, and alpha/beta prereleases.
set versions {
    0 1 1.0 1.0.0 1.2 1.2.0 1.2.3 1.3 1.10 1.2.10 2 2.0 2.3 10.0
    1.2a1 1.2b1 1.3a1 1.3b1 1.3b2 1.2a2 0.9 9.0 8.6 8.6.14 9.0.4
    1.0a1 1.0b1 2.0a1 1.2.3a1 1.2.3b4 3.1 3.0.1 0.0.1 100.200.300
}

# Numeric components that overflow every fixed integer width, plus the
# leading-zero spellings C strips: 32-bit, i64 and u64 boundaries, forty-digit
# components, and a big component behind an alpha marker.  C Tcl compares digit
# strings (length, then lexicographic, after skipping leading zeros) rather
# than computing a numeric value, so all of these must order exactly.
set bigs {
    4294967295 4294967296
    9223372036854775807 9223372036854775808 9223372036854775809
    18446744073709551615 18446744073709551616
    0000000000000000000000005 5 0005
    99999999999999999999999999999999999999999
    100000000000000000000000000000000000000000
    1111111111111111111111111111111111111111
    1111111111111111111111111111111111111112
    1.9223372036854775807 1.9223372036854775808
    100000000000000000000 20000000000000000000
    1.0a9223372036854775808 1.0a9223372036854775807
}

# Every requirement form: bare minimum, open-ended, half-open range, and the
# degenerate `v-v` range `package require -exact` builds.
set reqs {
    1.2 1.2- 1.2-2.0 1.2-1.3 2 2- 0- 1.3b1 1.3b1- 1.2a1-1.3
    1.0-2.0 1.2.3 9 8.5-9.0 1.10 1.2-1.2 2.0-2.0 1.2.0-1.2.0
    1.3b1-1.3b1 0 1 10.0-  1.2.10 8.6-9.0 2.3-2.3 1.0.0-1.0.0
}

set bigreqs {
    9223372036854775807 9223372036854775808
    9223372036854775808-9223372036854775808
    9223372036854775807-9223372036854775808
    18446744073709551616- 0005-0005
    1111111111111111111111111111111111111111
}

# Inputs the interpreter rejects outright.
set badversions {"" . a 1. .1 1..2 1a 1a1b2 1.2a 1-2 x1 " 1.2" 01.02}
set badreqs {"" - -1.2 1.2- 1.2-1.3-1.4 a-b 1.2-x}

# --- emitters ---------------------------------------------------------------

proc emit_version {} {
    global versions bigs reqs bigreqs badversions badreqs
    foreach grid [list $versions $bigs] {
        foreach a $grid {
            foreach b $grid {
                puts "vcompare\t$a\t$b\t[package vcompare $a $b]"
            }
        }
    }
    foreach {grid rs} [list $versions $reqs $bigs $bigreqs] {
        foreach v $grid {
            foreach r $rs {
                puts "vsatisfies\t$v\t$r\t[package vsatisfies $v $r]"
            }
        }
    }
    foreach s $badversions {
        puts "malformed-version\t$s\t[catch {package vsatisfies $s 1.2}]"
    }
    foreach r $badreqs {
        set rc [catch {package vsatisfies 1.2 $r} res]
        puts "malformed-req\t$r\t$rc\t[if {$rc} {set _ {}} else {set _ $res}]"
    }
}

# One selection trial: register each `providers` version as a
# `package ifneeded widget V …` in a fresh child interpreter (in the listed
# order), optionally set `package prefer`, then run `package require {*}$args`
# and report the version that actually loaded.
proc trial {prefer providers reqargs} {
    set i [interp create]
    $i eval [list set providers $providers]
    $i eval [list set reqargs $reqargs]
    $i eval [list set prefer $prefer]
    $i eval {
        if {$prefer ne ""} { package prefer $prefer }
        foreach v $providers {
            package ifneeded widget $v [list package provide widget $v]
        }
        set rc [catch {package require {*}$reqargs} res]
    }
    set rc [$i eval {set rc}]
    set res [$i eval {set res}]
    interp delete $i
    set p [expr {$prefer eq "" ? "default" : "$prefer"}]
    puts "select\t$p\t$providers\t$reqargs\t[if {$rc} {set _ ERROR} else {set _ $res}]"
}

proc emit_select {} {
    foreach {prefer providers reqargs} {
        {} {1.5 2.3}            {widget}
        {} {2.3 1.5}            {widget}
        {} {1.5 2.3}            {widget 1.2}
        {} {1.5 2.3}            {widget 2.0}
        {} {1.5 2.3 2.0}        {widget 2.0}
        {} {1.5 2.3}            {-exact widget 2.0}
        {} {1.5 2.0 2.3}        {-exact widget 2.0}
        {} {1.5 2.0.0 2.3}      {-exact widget 2.0}
        {} {1.0 1.0.0}          {-exact widget 1.0}
        {} {2.0a1 2.0}          {-exact widget 2.0}
        {} {2.0a1}              {-exact widget 2.0}
        {} {1.5 2.3}            {widget 1.2-2.0}
        {} {1.5 1.9 2.3}        {widget 1.2-2.0}
        {} {1.2 1.3b1}          {widget}
        latest {1.2 1.3b1}      {widget}
        stable {1.2 1.3b1}      {widget}
        {} {1.3b1}              {widget}
        {} {1.2 1.3b1}          {widget 1.2}
        latest {1.2 1.3b1}      {widget 1.2}
        {} {1.5 2.3}            {widget 3.0}
        {} {1.5 2.3}            {widget 2.5}
        {} {1.5 2.3}            {widget 1.2 2.0}
        {} {1.2.3 1.2.10}       {widget 1.2}
        {} {1.5 2.3}            {widget 2.0-}
        latest {1.2 1.2b1 1.3a1 1.3} {widget}
        stable {1.2 1.2b1 1.3a1 1.3} {widget}
        {} {2.3 2.3}            {widget}
        {} {1.2a1 1.2}          {widget 1.2}
        latest {1.2a1 1.2}      {widget 1.2}
        latest {1.2a1}          {widget 1.2}
        {} {1.2a1}              {widget 1.2}
        {} {1.0 2.0 3.0}        {widget}
        {} {10.0 9.0}           {widget}
        {} {1.2 1.10}           {widget 1.2}
        {} {9223372036854775807 9223372036854775808}   {widget}
        {} {9223372036854775808 9223372036854775807}   {widget}
        {} {9223372036854775807 9223372036854775808}   {-exact widget 9223372036854775807}
        {} {9223372036854775807 9223372036854775808}   {-exact widget 9223372036854775808}
        {} {18446744073709551615 18446744073709551616} {widget}
        {} {1111111111111111111111111111111111111111 1111111111111111111111111111111111111112} {widget}
        {} {0005 5}             {widget}
        {} {1.9223372036854775807 1.9223372036854775808} {widget 1.0}
    } {
        trial $prefer $providers $reqargs
    }
}

switch -exact -- [lindex $argv 0] {
    version { emit_version }
    select  { emit_select }
    default {
        puts stderr "usage: [info script] version|select"
        exit 2
    }
}
