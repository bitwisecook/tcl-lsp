# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
proc run {} {
    set l {a b c d e f g h}
    set n 0
    for {set i 0} {$i < @N@} {incr i} {
        set m $l
        lappend m $i
        lset m 2 $i
        incr n [llength $m]
        set x [lindex $m 5]
    }
    return $n
}
run
