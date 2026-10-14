package require struct::set
package require struct::list
proc inc {} {set s {}; struct::set include s a; return $s}
proc exc {} {set s {a b}; struct::set exclude s a; return $s}
proc add {} {set s {a}; struct::set add s {b}; return $s}
proc sub {} {set s {a b}; struct::set subtract s {a}; return $s}
proc shf {} {set l {a b c}; struct::list shift l; return $l}
proc asg {} {set x 0; struct::list assign {1 2} x y; return $x}
foreach p {inc exc add sub shf asg} {puts "$p: [$p]"}
