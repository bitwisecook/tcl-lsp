# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
oo::class create Counter {
    variable n
    constructor {} {set n 0}
    method bump {by} {incr n $by}
}
proc run {} {
    set c [Counter new]
    for {set i 0} {$i < @N@} {incr i} {$c bump 2}
    $c destroy
}
run
