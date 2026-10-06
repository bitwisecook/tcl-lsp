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

proc run {label script} {proc probe {} $script; puts [list $label [catch probe result] $result]}
run retarget {set x OLD; set z NEW; upvar 0 x a; upvar 0 a b; upvar 0 z a; list $a $b}
run materialise {upvar 0 a(k) y; list [info exists a] [array exists a] [info exists y]}
run rootWrite {set a(k) OLD; upvar 0 a(k) y; unset a; set y NEW; list $a(k) $y}
run self {upvar 0 x x}
run cycle {upvar 0 a b; upvar 0 b a}
