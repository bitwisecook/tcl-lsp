# W301: uplevel with an unbraced script argument — double substitution risk
# Source: SpiceGenTcl/src/generalClasses.tcl:2756
#
# Like eval, uplevel with an unbraced script causes substitution in
# the current scope before the script runs in the caller's scope.
# This can cause double substitution bugs.
#
# Expected: tclsh runs — shows the difference between braced and unbraced.

proc inner_unbraced {script} {
    set x "inner"
    uplevel 1 $script
}

proc inner_braced {script} {
    set x "inner"
    uplevel 1 {set result "from braced uplevel"}
}

set x "outer"

# Unbraced: $x is substituted in inner_unbraced's scope (double-sub)
inner_unbraced {puts "uplevel sees x=$x"}

# The script argument was already substituted, so uplevel runs it
# in the caller's scope with the value from the callee's scope
set script {puts "script x=$x"}
inner_unbraced $script

puts "W301 test complete"
