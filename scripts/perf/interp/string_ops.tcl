# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
proc run {} {
    set s "the quick brown fox"
    set n 0
    for {set i 0} {$i < @N@} {incr i} {
        set t [string range $s 4 [expr {[string length $s] - 5}]]
        incr n [string first "brown" $t]
        append t $i
    }
    return $n
}
run
