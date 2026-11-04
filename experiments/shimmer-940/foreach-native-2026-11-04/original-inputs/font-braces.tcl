# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
set fontSizes {10.0 12.0 16.0 24.0}
set valueName fontSizes
set have_rep [expr {[llength [info commands ::tcl::unsupported::representation]] != 0}]
puts [list patchlevel [info patchlevel] have_rep $have_rep]
if {$have_rep} {puts [list before [::tcl::unsupported::representation $fontSizes]]}
set visited {}
foreach size $fontSizes {lappend visited $size}
if {$have_rep} {puts [list after [::tcl::unsupported::representation $fontSizes]]}
puts [list visited $visited]
