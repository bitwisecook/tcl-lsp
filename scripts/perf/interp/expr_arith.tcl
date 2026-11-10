# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
proc run {} {
    set x 1
    for {set i 0} {$i < @N@} {incr i} {
        set x [expr {($x * 31 + $i) % 1000003 - ($i >> 2) + ($i & 7)}]
    }
    return $x
}
run
