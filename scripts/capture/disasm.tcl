#!/usr/bin/env tclsh
# disasm.tcl — Generate reference bytecode disassembly from tclsh.
#
# Usage: tclsh disasm.tcl <snippet.tcl>
#
# Reads one .tcl snippet and outputs:
#
#   === <filename> ===
#   <disassembly output from tcl::unsupported::disassemble script>
#   <the disassembly of each proc the snippet defines, in name order>
#   === END ===

proc emit_disasm {filename source} {
    puts "=== $filename ==="

    # Disassemble the script itself
    if {[catch {set asm [tcl::unsupported::disassemble script $source]} err]} {
        puts "ERROR: $err"
        puts "=== END ==="
        return
    }
    puts $asm

    # If the script defines procs, disassemble them too. They are defined in
    # this, the parent interpreter: Tcl compiles the compact second pass
    # (no INST_START_CMD) only where Tcl_GetParent is NULL (tclCompile.c), so a
    # child's bodies would differ. One snippet runs per process, so nothing
    # is shared; only the procs the snippet defines are emitted, in name order
    # as our listing has them.
    set before [info procs ::*]
    if {![catch {uplevel #0 $source}]} {
        set defined {}
        foreach cmd [info procs ::*] {
            if {$cmd ni $before} {
                lappend defined $cmd
            }
        }
        foreach cmd [lsort $defined] {
            if {![catch {set pasm [tcl::unsupported::disassemble proc $cmd]}]} {
                puts ""
                puts $pasm
            }
        }
    }

    puts "=== END ==="
}

if {$argc != 1} {
    puts stderr "Usage: tclsh disasm.tcl <snippet.tcl>"
    exit 1
}

set f [lindex $argv 0]
set fd [open $f r]
set source [read $fd]
close $fd
emit_disasm [file tail $f] $source
