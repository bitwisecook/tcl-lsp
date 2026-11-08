# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
set g 0
proc bump {} {global g; incr g}
for {set i 0} {$i < @N@} {incr i} {bump}
