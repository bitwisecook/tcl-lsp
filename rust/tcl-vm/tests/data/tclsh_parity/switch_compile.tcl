# Which forms of switch each release compiles: their bodies' variables are
# compiled locals of the procedure (8.6 reads a literal index apart in a
# script compiled in its frame), and an error in a compiled body leaves no
# frame of the switch's own; 8.4 compiles none.
proc L {} { list string range abcdefghijkl 0 [set e end-2147483649] }
set shapes {
    {switch a a {set r 1}}
    {switch -- a a {set r 1}}
    {switch -exact a a {set r 1}}
    {switch a {a {set r 1}}}
    {switch -- a {a {set r 1}}}
    {switch a a {set r 1} b {}}
    {switch -glob a a {set r 1}}
    {switch -regexp a a {set r 1}}
    {switch -exact -- a a {set r 1}}
    {switch a a - b {set r 1}}
    {switch -nocase a a {set r 1}}
    {switch -nocase -- a {a {set r 1}}}
    {switch -glob -nocase -- a {a {set r 1}}}
    {switch -regexp -matchvar m a {a {set r 1}}}
    {switch $b {a {set r 1}}}
    {switch -- $b a {set r 1}}
    {switch a {a {set r 1} default {}}}
    {switch -glob -- a {a* {set r 1}}}
    {switch -regexp -- a {^a {set r 1}}}
    {switch -- a {a - b {set r 1}}}
}
set i 0
set out ""
foreach shape $shapes {
    incr i
    proc p$i {b} "if {\$b eq {}} { $shape }; eval {catch \$b r ;# $i}; eval \$b"
    append out [expr {[p$i [L]] eq {} ? {-} : {C}}]
}
puts $out
proc info_of {script} {
    catch {uplevel 1 $script}
    string map [list \n { | }] $::errorInfo
}
proc q1 {} { switch a a {error x} }
proc q2 {} { switch -- a {a {error y}} }
proc q3 {} { switch -glob -- abc {a* {error w}} }
proc q4 {} { switch -exact a a {error v} }
proc q5 {} { switch b a - b - c {error u} }
foreach q {q1 q2 q3 q4 q5} {
    puts [info_of $q]
}
puts [info_of {switch a a {error z}}]
puts [info_of {switch -- a {a {error t}}}]
