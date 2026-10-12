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

proc check {label script} {puts [list $label [catch {uplevel 1 $script} result] $result]}
check nestedCatch {proc p {} {catch {set x 1} msg; set x}; p}
check uplevelZero {proc p {} {set x 1; uplevel 0 {set x 2}; set x}; p}
check ownFrameLambda {proc p {} {set x caller; apply {{} {set x}}}; p}
check catchPartial {proc p {} {catch {set x 1; error err; set x 2} msg; set x}; p}
check catchPreWrite {proc p {} {catch {error err; set x 2} msg; set x}; p}
check catchUnset {proc p {} {set x 1; catch {unset x; error err} msg; set x}; p}
check returnCaught {proc p {} {catch {set x 1; return stop; set x 2} msg; list $msg $x}; p}
check deferredCallback {proc p {} {set x 1; after 0 {set ::result 2}; set x}; p}
