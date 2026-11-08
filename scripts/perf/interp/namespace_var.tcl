# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
namespace eval ns {variable v 0}
proc ns::bump {} {variable v; incr v}
for {set i 0} {$i < @N@} {incr i} {ns::bump}
