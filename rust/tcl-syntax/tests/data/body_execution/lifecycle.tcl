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
::tcltest::outputChannel [file join $::oracle_fixture_root tcltest-output.log]
proc runCase {label argv} {
    set events {}
    set x unset
    set code [catch {eval [linsert $argv 0 ::tcltest::test $label {}]} result]
    puts [list $label code $code events $events x $x result $result]
}
set setup {lappend events setup; set x ready}
set body {lappend events body; set x}
set cleanup {lappend events cleanup; lappend events $x}
runCase normal [list -setup $setup -body $body -cleanup $cleanup -result ready]
runCase reordered [list -cleanup $cleanup -body $body -setup $setup -result ready]
runCase setupError [list -setup {lappend events setup; set x partial; error setup-error} -body $body -cleanup $cleanup]
runCase setupReturn [list -setup {lappend events setup; return setup-return} -body $body -cleanup $cleanup]
runCase bodyReturn [list -setup $setup -body {lappend events body; return returned} -cleanup $cleanup -result returned]
runCase bodyError [list -setup $setup -body {lappend events body; error body-error} -cleanup $cleanup -returnCodes error -result body-error]
runCase cleanupError [list -setup $setup -body $body -cleanup {lappend events cleanup; error cleanup-error} -result ready]
runCase cleanupReturn [list -setup $setup -body $body -cleanup {lappend events cleanup; return cleanup-return} -result ready]
runCase skipped [list -constraints definitely-not-present -setup $setup -body $body -cleanup $cleanup]
runCase duplicateBody [list -body {lappend events first; set x first} -body {lappend events last; set x last} -result last]
runCase duplicateSetup [list -setup {lappend events first; set x first} -body $body -setup $setup -cleanup $cleanup -result ready]
runCase unknownOption [list -invalid {lappend events invalid} -setup $setup -body $body -cleanup $cleanup]
runCase oddArgv [list -setup $setup -body]
runCase legacy [list {lappend events body; set x legacy} legacy]
runCase singleOptionsList [list [list -setup $setup -body $body -cleanup $cleanup -result ready]]
proc ::tcltest::SetupTest {script} {uplevel 1 {lappend events setupHook; set x hook}}
proc ::tcltest::CleanupTest {script} {uplevel 1 {lappend events cleanupHook}}
runCase overriddenHooks [list -setup $setup -body $body -cleanup $cleanup -result hook]
proc ::tcltest::EvalTest {script} {uplevel 1 {lappend events bodyHook; set x body-hook}}
runCase overriddenAllHooks [list -setup $setup -body $body -cleanup $cleanup -result body-hook]
rename ::tcltest::SetupTest {}
rename ::tcltest::CleanupTest {}
rename ::tcltest::EvalTest {}
set ::constraintSideEffect 0
runCase computedConstraint [list -constraints {[set ::constraintSideEffect 1]} -body {lappend events body; set x body} -result body]
puts [list computedConstraint-global-write $::constraintSideEffect]
runCase putsRebinding [list -body {lappend events [namespace origin ::puts]; puts observed} -output observed\n]
