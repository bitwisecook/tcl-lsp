# Adds one. \
proc hidden {} {}
proc add1 {x} {
    return [expr {$x + 1}]
}
# Doubles {x}, unbalanced } in a doc comment
proc dbl {x} { expr {$x*2} }
puts [add1 1]:[dbl 2]:[info commands hidden]
