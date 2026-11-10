# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
proc run {} {
    set d [dict create a 1 b 2 c 3]
    set n 0
    for {set i 0} {$i < @N@} {incr i} {
        dict set d k [expr {$i & 15}]
        dict incr d a
        incr n [dict get $d k]
        if {[dict exists $d c]} {incr n}
    }
    return $n
}
run
