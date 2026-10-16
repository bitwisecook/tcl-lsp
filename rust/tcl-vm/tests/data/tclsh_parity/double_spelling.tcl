# A double's string rep: Tcl 8.4 prints %.12g (tcl_precision 12), 8.5 on the
# shortest digits that read back.
puts [expr {1.0/3}]
puts [expr 1.0/7]
puts [expr {1e-5}]
puts [expr {1e15 + 0.3}]
puts [expr {123456789012.5}]
puts [expr {999999999999.5}]
puts [expr {-0.0}]
puts [expr {0.1 + 0.2}]
puts [expr {1e300 * 10}]
puts [expr {pow(2.0,64)}]
puts [expr {double(1)/3 * 3}]
puts [list [expr {sqrt(2)}] [expr {exp(1)}] [expr {acos(-1)}]]
set x [expr {2.0/3}]
puts "$x [string length $x]"
append s [expr {1/3.0}] |
puts $s
binary scan [binary format d3 {0.1 0.25 3.0}] d3 values
puts $values
puts [expr {1.0471975511965979}]
puts 1.0471975511965979
puts [format %.17g [expr {1.0/3}]]
proc p {a} {
    set r [expr {$a / 7.0}]
    lappend l $r [expr {$r * 7}] [expr {sin($a)}]
    return $l
}
puts [p 1]
puts [p 22]
set v [expr {1.0/3}]
set w [expr {$v * 1}]
puts "[format %.17g $w] [expr {$v * 3 == 1}]"
