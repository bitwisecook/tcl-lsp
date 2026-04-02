# W304: Missing '--' option terminator before substituted input
# Source: SpiceGenTcl/src/generalClasses.tcl (lsearch, regexp, switch)
#         SpiceGenTcl/src/ngspice/netlistParserClassNgspice.tcl
#
# The diagnostic should ideally be a tristate:
#   OFF     - value structurally cannot start with '-'
#   POSSIBLE - value comes from user input, could start with '-'
#   ALWAYS  - value is known to start with '-'
#
# Of 24 W304 sites in SpiceGenTcl, 19 are false positives because the
# value structurally cannot begin with '-'.
#
# Expected: tclsh 9.0 runs without error.

# ---------------------------------------------------------------
# CASE 1: OFF - value structurally cannot start with '-'
# These should NOT fire W304 (currently false positives).
# ---------------------------------------------------------------

# 1a. switch on argparse output with fixed value set
# Source: generalClasses.tcl:749 - $action is always add/get/set/delete
set action "get"
switch $action {
    add     { set r "adding" }
    get     { set r "getting" }
    set     { set r "setting" }
    delete  { set r "deleting" }
    default { set r "unknown" }
}
puts "switch fixed values: $r"

# 1b. regexp on netlist lines that always start with '.'
# Source: netlistParserClassNgspice.tcl:103
set netlist_line ".include mylib.spice"
if {[regexp {^\.include\s+(.+)} $netlist_line -> filename]} {
    puts "regexp on netlist: $filename"
}

# 1c. lsearch on parsed data - elements are SPICE tokens, not options
# Source: netlistParserClassNgspice.tcl:320
set tokens {tran 1n 100n uic}
puts "lsearch on tokens: [lsearch -exact $tokens uic]"

# 1d. HTTP path always starts with '/'
set path "/api/v1/data"
if {[regexp {^/api/(.+)} $path -> rest]} {
    puts "regexp on path: $rest"
}

# 1e. simulation vector names like v(node), i(src)
# Source: generalClasses.tcl:2197
set vecname "v(anode)"
if {[regexp -nocase {^v\(([^)]+)\)$} $vecname -> node]} {
    puts "regexp on vector name: $node"
}

# 1f. switch on netlist element types: r, c, l, v, i, d, ...
# Source: netlistParserClassNgspice.tcl:201
set elemtype "r"
switch $elemtype {
    r { set r "resistor" }
    c { set r "capacitor" }
    v { set r "voltage source" }
    default { set r "other" }
}
puts "switch on element type: $r"

# ---------------------------------------------------------------
# CASE 2: POSSIBLE - value could start with '-'
# These SHOULD fire W304.
# ---------------------------------------------------------------

# 2a. list iteration over user-provided items
# Source: generalClasses.tcl:210
proc find_in_list {haystack needle} {
    # $needle is user input - could be "-exact" or "-verbose"
    set idx [lsearch $haystack $needle]
    return $idx
}
set items {apple -verbose banana -debug cherry}
puts "lsearch no --: [find_in_list $items -verbose]"

# 2b. with '--': safe regardless of content
proc find_in_list_safe {haystack needle} {
    set idx [lsearch -- $haystack $needle]
    return $idx
}
puts "lsearch with --: [find_in_list_safe $items -verbose]"

# 2c. regexp validating user string that might start with '-'
# Source: generalClasses.tcl:2958
set user_input "hello"
if {[regexp -- {^[a-zA-Z_]\w*$} $user_input]} {
    puts "regexp on user input: valid identifier"
}

# 2d. switch on user-provided parameter
set user_opt "value"
switch -- $user_opt {
    value   { puts "switch with --: matched value" }
    default { puts "switch with --: default" }
}

# ---------------------------------------------------------------
# CASE 3: ALWAYS - value known to start with '-'
# '--' is mandatory here.
# ---------------------------------------------------------------

# 3a. constructed option strings: "-$name"
# Source: generalClasses.tcl constructs "-$name" from parameters
set name "Wall"
set opt "-$name"
switch -- $opt {
    -Wall   { set r "all warnings" }
    -O2     { set r "optimise" }
    default { set r "unknown flag" }
}
puts "switch on -\$name: $r"

# 3b. searching for option-like strings in a list
set flags {-O2 -Wall -Werror -pedantic}
puts "lsearch for flag: [lsearch -- $flags -Wall]"

# 3c. regexp matching option-like patterns
set flag "-verbose"
if {[regexp -- {^-(\w+)$} $flag -> flagname]} {
    puts "regexp on flag: $flagname"
}

puts "W304 test complete"
