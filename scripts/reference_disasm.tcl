#!/usr/bin/env tclsh
# reference_disasm.tcl — Generate reference bytecode disassembly from tclsh.
#
# Usage: tclsh reference_disasm.tcl <snippets_dir>
#
# Reads every .tcl file in <snippets_dir>.  Each file contains a single
# Tcl script snippet.  For each, it outputs:
#
#   === <filename> ===
#   <disassembly output from tcl::unsupported::disassemble script>
#   === END ===
#
# When a snippet defines a proc (detected by the "proc" command at the
# top level), the proc's disassembly is also emitted after the script's.

proc emit_disasm {filename source} {
    puts "=== $filename ==="

    # Disassemble the script itself
    if {[catch {set asm [tcl::unsupported::disassemble script $source]} err]} {
        puts "ERROR: $err"
        puts "=== END ==="
        return
    }
    puts $asm

    # If the script defines procs, disassemble them too.
    # We eval the script in a child interp to discover proc names.
    set child [interp create -safe]
    if {![catch {$child eval $source}]} {
        foreach cmd [$child eval {info procs}] {
            if {![catch {set pasm [$child eval [list tcl::unsupported::disassemble proc $cmd]]}]} {
                puts ""
                puts $pasm
            }
        }
    }
    interp delete $child

    puts "=== END ==="
}

if {$argc != 1} {
    puts stderr "Usage: tclsh reference_disasm.tcl <snippets_dir>"
    exit 1
}

set dir [lindex $argv 0]
foreach f [lsort [glob -directory $dir *.tcl]] {
    set fd [open $f r]
    set source [read $fd]
    close $fd
    emit_disasm [file tail $f] $source
}
