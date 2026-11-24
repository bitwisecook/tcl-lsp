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

# Run in C Tcl, with each engine observation in a fresh process. An engine
# that lacks this command or terminates while evaluating it still has a row.
foreach {inputFile outputFile engine release quiet} $argv break
set input [open $inputFile r]
set output [open $outputFile w]
set fixture [file join [file dirname $outputFile] [file tail $outputFile].tcl]
set launch [list exec timeout 10 $engine]
if {$quiet} { lappend launch --quiet }
lappend launch --tcl-version $release $fixture

puts $output [join {command subcommand argc verdict {reference usage} {registry synopsis} {process status} {engine answer}} "\t"]
gets $input
while {[gets $input row] >= 0} {
    foreach {command sub count usage synopsis diagnostic} [split $row "\t"] break
    set key [list $command $sub $usage]
    if {[info exists observed($key)]} { continue }
    set observed($key) 1
    set words [list $command]
    if {$sub ne ""} { lappend words $sub }
    for {set n 0} {$n < $count} {incr n} { lappend words __definition_probe__ }
    set script [open $fixture w]
    puts $script [list set invocation $words]
    puts $script {set result [catch $invocation message]}
    puts $script {puts [list $result $message]}
    close $script
    set status [catch {eval $launch} answer]
    set verdict mismatch
    if {$status == 0 && ![catch {set actualCode [lindex $answer 0]; set actualMessage [lindex $answer 1]}]
        && $actualCode eq "1"} {
        if {$actualMessage eq $usage} {
            set verdict match
        } elseif {[string match {invalid command name*} $actualMessage]} {
            set verdict unsupported
        }
    }
    if {$status != 0} {
        set failure [lindex $::errorCode 0]
        if {$failure eq "CHILDKILLED" || $failure eq "CHILDSTATUS"} {
            set answer [list $failure [lindex $::errorCode 2] $answer]
        }
    }
    set answer [string map [list "\\" "\\\\" "\t" {\t} "\n" {\n} "\r" {\r}] $answer]
    puts $output [join [list $command $sub $count $verdict $usage $synopsis $status $answer] "\t"]
}
close $input
close $output
file delete $fixture
