# The variables of code C compiles though it never runs are compiled locals
# (8.6 reads a literal index apart in a script compiled in the frame): every
# clause of a `for`, the commands after a `return`, the bodies of an `if` or
# `while` whose test is no literal constant; a literal constant `if` or
# `while` test leaves the script it never runs uncompiled.
proc L {} { list string range abcdefghijkl 0 [set e end-2147483649] }
set shapes {
    {for {} {0} {set r 1} {}}
    {for {} {0} {} {set r 1}}
    {set x 0; if {$x} {set r 1}}
    {if {1 == 2} {set r 1}}
    {if {0} {set r 1}}
    {if {false} {set r 1}}
    {if 0 {} else {set r 1}}
    {if 1 {} else {set r 1}}
    {while {1 == 2} {set r 1}}
    {while {false} {set r 1}}
    {for {} {false} {set r 1} {}}
    {if {0 && $b} {set r 1}}
    {return; set r 1}
}
set i 0
set out ""
foreach shape $shapes {
    incr i
    proc p$i {b} "if {\$b eq {}} { $shape }; eval {catch \$b r ;# $i}; eval \$b"
    append out [expr {[p$i [L]] eq {} ? {-} : {C}}]
}
puts $out
