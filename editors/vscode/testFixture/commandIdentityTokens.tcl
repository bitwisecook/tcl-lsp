# Issue #1185 — a command head's grammar follows its effective identity, not
# its spelling.  Every line below is real, working Tcl (verified on tclsh
# 9.0.4 and 8.6.16).
#
# 1. The explicitly global spelling resolves to the same command.
::foreach {aa bb} {1 2} {
    puts $aa
}
puts [::format {%08x} 42]

# 2. A static `interp alias` makes the new name behave as the target.
interp alias {} myformat {} format
puts [myformat {%08x} 42]

# 3. A static `rename` moves the identity to the new name …
rename upvar myupvar
proc reader {} {
    myupvar 1 outer local
    return $local
}

# 4. … and a top-level `proc` takes a built-in's name over entirely, so the
#    shadowed spelling is an ordinary user command from here on.
proc scan {args} {
    return USER
}
puts [scan {%d} 7]
