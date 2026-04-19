#!/usr/bin/env tclsh
# reference_disasm_84.tcl — Generate reference bytecode disassembly from Tcl 8.4.
#
# Tcl 8.4 lacks tcl::unsupported::disassemble, so we use tcl_traceCompile
# to capture bytecode as it compiles.  The trace output goes to stderr.
#
# Usage: tclsh reference_disasm_84.tcl <snippets_dir>
#
# For each .tcl file in <snippets_dir>, outputs:
#
#   === <filename> ===
#   <tcl_traceCompile 2 output>
#   === END ===

set tcl_traceCompile 2

if {$argc != 1} {
    puts stderr "Usage: tclsh reference_disasm_84.tcl <snippets_dir>"
    exit 1
}

set dir [lindex $argv 0]
foreach f [lsort [glob -directory $dir *.tcl]] {
    set fd [open $f r]
    set source [read $fd]
    close $fd

    set filename [file tail $f]
    puts "=== $filename ==="
    flush stdout

    # Compile and execute the snippet in a child interp to capture trace
    # output.  tcl_traceCompile output goes to stderr of the process.
    set child [interp create]
    $child eval {set tcl_traceCompile 2}
    catch {$child eval $source}
    interp delete $child

    puts "=== END ==="
    flush stdout
}
