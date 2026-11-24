# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

# Compare every distinct usage line in the matrix against its source
# synopsis and, for C Tcl, the corresponding VM and runtime messages.
foreach {root baseline outputFile} $argv break
set input [open $baseline r]
while {[gets $input row] >= 0} {
    if {[string match "#*" $row]} { continue }
    foreach {lane patches backend command sub verdict reference reason} [split $row "\t"] break
    if {$reason eq ""} { error [list "usage allowance needs a reason" $row] }
    foreach patch [split $patches ,] {
        set allowed([list $lane $patch $backend $command $sub $verdict $reference]) 1
    }
}
close $input

proc check {path lane patch backend} {
    upvar 1 allowed allowed differences differences
    set input [open $path r]
    gets $input
    while {[gets $input row] >= 0} {
        if {$backend eq "registry"} {
            foreach {command sub count reference synopsis diagnostic} [split $row "\t"] break
            set expected $reference
            if {![regexp {^wrong # args: should be "(.*)"$} $reference -> expected]} {
                regsub {^usage: } $reference {} expected
            }
            if {$expected eq $synopsis} { continue }
            set verdict different-synopsis
            if {$synopsis eq ""} { set verdict missing-synopsis }
        } else {
            foreach {command sub count verdict reference synopsis status answer} [split $row "\t"] break
            if {$verdict eq "match"} { continue }
        }
        set key [list $lane $patch $backend $command $sub $verdict $reference]
        set differences($key) [expr {![info exists allowed($key)]}]
    }
    close $input
}

foreach dir [lsort [glob -directory $root -types d -- *-*]] {
    set id [file tail $dir]
    set splitAt [string first - $id]
    set lane [string range $id 0 [expr {$splitAt - 1}]]
    set patch [string range $id [expr {$splitAt + 1}] end]
    check [file join $dir usage.tsv] $lane $patch registry
    if {$lane eq "tcl"} {
        foreach backend {vm runtime} {
            check [file join $dir usage-$backend.tsv] $lane $patch $backend
        }
    }
}

set output [open $outputFile w]
puts $output [join {lane patch backend command subcommand verdict {reference usage} unreviewed} "\t"]
set regressions 0
foreach key [lsort [array names differences]] {
    puts $output [join [concat $key $differences($key)] "\t"]
    incr regressions $differences($key)
}
close $output
if {$regressions > 0} {
    error "$regressions new usage differences. See $outputFile"
}
