namespace eval ::nh {}
proc {::nh::two words} {fixed value} {list $fixed $value}
set target {::nh::two words}
set prefix [list $target {a b}]
eval [linsert $prefix end {c d}]

