# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https:#github.com/bitwisecook>
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
# along with this program.  If not, see <https:#www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set core [lsort [info commands ::*]]
package require tcltest
set bodies {}
foreach proc [info procs ::tcltest::*] {lappend bodies [info body $proc]}
set sf [open [lindex $argv 2] r]; lappend bodies [read $sf]; close $sf
set f [open [lindex $argv 0] w]; puts $f $bodies; close $f
set f [open [lindex $argv 1] w]; puts $f $core; close $f
