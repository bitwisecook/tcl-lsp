# Fixture for the DiagnosticTag suite (issue #1333).
#
# Line 5 (1-based): `c` is declared but never read — W214, tagged Unnecessary, so the
# identifier renders faded in the editor.
proc greet {a b c} {
    return [expr {$a + $b}]
}
greet 1 2 3

# Line 12 (1-based): `spare` is set and never read — W211, also tagged Unnecessary.
proc total {items} {
    set spare 0
    return [llength $items]
}
total {a b}

# Line 20 (1-based): `never` is read before it is ever set — W210. A real defect, so it
# must NOT be faded; greying it out would hide it.
proc report {} {
    return $never
}
report
