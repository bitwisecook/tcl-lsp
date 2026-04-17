# 13: incr dialect difference — Tcl 8.4 vs 8.5+
#
# In Tcl 8.4, ``incr`` on an uninitialised variable raises:
#   can't read "x": no such variable
#
# In Tcl 8.5+, it silently treats the variable as 0.
#
# To test: change the dialect directive below between tcl8.4 and tcl8.6
# and observe that W210 fires only under 8.4.

# tcl-dialect: tcl8.6

proc counter {} {
    # No W210 in 8.5+: incr safely treats x as 0.
    # W210 in 8.4:     incr requires x to exist.
    incr x
    return $x
}

# The safe alternative that works in all Tcl versions:
proc counter_safe {} {
    set x 0
    incr x
    return $x
}
