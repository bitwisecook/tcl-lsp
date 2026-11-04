# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
set data abc
set values [string length $data]
set have_rep [expr {[llength [info commands ::tcl::unsupported::representation]] != 0}]
puts [list patchlevel [info patchlevel] have_rep $have_rep]
if {$have_rep} {puts [list before [::tcl::unsupported::representation $values]]}
set visited {}
set command foreach
$command value $values {lappend visited $value}
if {$have_rep} {puts [list after [::tcl::unsupported::representation $values]]}
puts [list visited $visited]
