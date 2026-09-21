# Fixture for the "preview version:" regressions.
#
# `after <ms>` is a millisecond delay, not an unknown subcommand -> no W001.
after 200 {puts "Hello world!"}

# A fully-qualified global read is never "read before set" -> no W210,
# even though ::myVar is defined in another file.
proc foo {} {
    puts $::myVar
}

# An unknown third-party package may load Tk internally -> no W120 on
# the Tk commands below.
package require myTkPackage
if {[tk windowingsystem] eq "aqua"} {
    # nothing
} else {
    set fpixels [winfo fpixels . 1i]
}

# A nested [expr] that is an argument to another command is NOT in an
# expression context -> no W114 on the inner [expr].
proc myCmd {arg1} {
    return $arg1
}
if {[myCmd [expr {1 + 1}]]} {
    # nothing
}

# `lassign` destructures a list into per-element locals. Each target
# holds a list *element*, not the `List` value the command returns, so the
# arithmetic below must NOT draw S100 "list intrep used in arithmetic".
set point [list 1 2 3]
lassign $point px py pz
puts [expr {$px + $py + $pz}]

# A genuine error that must appear exactly ONCE: too many arguments.
set var 10 10
