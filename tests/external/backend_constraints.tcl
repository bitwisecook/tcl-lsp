# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# This overlay may exclude only platform identity, unavailable host
# capabilities, and C-only internal-representation probes. Ordinary Tcl
# semantics are never backend constraints.

set runtime [expr {
    [info exists ::tcl_platform(runtime)] ? $::tcl_platform(runtime) : "c"
}]
set wasm [expr {
    [info exists ::tcl_platform(wasm)] ? $::tcl_platform(wasm) : ""
}]
set wasi [expr {
    [info exists ::tcl_platform(wasi)] ? $::tcl_platform(wasi) : ""
}]
set ebpf [expr {
    [info exists ::tcl_platform(ebpf)] ? $::tcl_platform(ebpf) : ""
}]

::tcltest::testConstraint rustBackend [expr {$runtime in {bytecode treewalk ebpf}}]
::tcltest::testConstraint bytecodeRuntime [expr {$runtime eq "bytecode"}]
::tcltest::testConstraint treewalkRuntime [expr {$runtime eq "treewalk"}]
::tcltest::testConstraint wasmBackend [expr {$wasm ne ""}]
::tcltest::testConstraint notWasm [expr {$wasm eq ""}]
::tcltest::testConstraint wasiPreview1 [expr {$wasi in {preview1 0.1}}]
::tcltest::testConstraint ebpfBackend [expr {$ebpf ne ""}]

# Each exclusion is one explicit backend boundary. Broad language-semantic
# stems such as set-*, expr-*, proc-*, and namespace-* are forbidden here.
set exclusions {}
if {$runtime in {bytecode treewalk}} {
    # C's platform-1.1 asserts the exact stock array-key set. Rust adds its
    # documented backend-introspection keys, so the assertion is inapplicable.
    lappend exclusions platform-1.1
}
if {![llength [info commands socket]]} {
    lappend exclusions socket-*
}
if {![llength [info commands exec]] || $wasi ne "" || $ebpf ne ""} {
    lappend exclusions exec-*
}
if {![info exists ::tcl_platform(threaded)] || !$::tcl_platform(threaded)} {
    lappend exclusions thread-* async-*
}
if {$ebpf ne ""} {
    lappend exclusions fCmd-* fileSystem-*
}

# C-only object/list/dict/array representation probes already carry upstream
# test-command constraints (testobj, testlistrep, and peers). Do not duplicate
# them with semantic stem globs: their absent commands make them skip exactly.
if {[llength $exclusions]} {
    ::tcltest::configure -skip [concat [::tcltest::configure -skip] $exclusions]
}

unset runtime wasm wasi ebpf exclusions
