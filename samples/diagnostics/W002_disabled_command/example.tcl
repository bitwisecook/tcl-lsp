# W002: Command disabled in active dialect profile
# tcl-dialect: tcl8.6
# Source: SpiceGenTcl/src/generalClasses.tcl (oo::configurable, lremove)
#         SpiceGenTcl/src/ltspice/specElementsClassesLtspice.tcl (oo::abstract)
#
# These commands are Tcl 9.0-only features. The LSP warns because its
# default dialect profile targets Tcl 8.6 where they don't exist.
# In Tcl 9.0 they work fine.
#
# Expected: tclsh 9.0 runs without error; tclsh 8.6 fails.

# --- oo::configurable (Tcl 9.0+) ---
if {![catch {oo::configurable create Greeter {
    property greeting -get {
        return "Hello, [my greeting]!"
    }
    constructor {g} {
        my configure -greeting $g
    }
}}]} {
    set g [Greeter new "World"]
    puts "oo::configurable: [$g configure -greeting]"
    $g destroy
} else {
    puts "oo::configurable: not available (pre-9.0)"
}

# --- lremove (Tcl 9.0+) ---
if {[info commands lremove] ne ""} {
    set items {a b c d e}
    set result [lremove $items 1 3]
    puts "lremove: $result"
    # expected: a c e
} else {
    puts "lremove: not available (pre-9.0)"
}

puts "W002 test complete"
