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

::tcltest::configure -verbose {}
::tcltest::outputChannel [file join $::oracle_fixture_root repetition-output.log]
puts "REPEATS=[expr {[llength [info commands ::tcltest::testIterations]] != 0}]"
if {[llength [info commands ::tcltest::testIterations]]} {
    foreach count {0 1 3} {
        ::tcltest::testIterations $count
        set events {}
        ::tcltest::test repetition d -setup {lappend events setup} -body {lappend events body; return ok} -cleanup {lappend events cleanup} -result ok
        puts "COUNT=$count EVENTS=$events"
    }
    ::tcltest::testIterations 3
    set events {}
    ::tcltest::test iteration-hook d -setup {
        lappend events setup
        proc ::tcltest::EvalTest {script} {uplevel 1 {lappend events hook; return ok}}
    } -body {lappend events body; return ok} -cleanup {lappend events cleanup} -result ok
    puts "HOOK=$events"
}
