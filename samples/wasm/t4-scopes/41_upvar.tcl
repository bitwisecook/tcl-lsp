# T4: upvar makes the caller's cell observable - caller must materialise it.
proc incrby {varName amount} {
    upvar 1 $varName v
    incr v $amount
}
proc swap {a b} {
    upvar 1 $a x $b y
    lassign [list $y $x] x y
}
set n 10
incrby n 5
puts $n
set p 1; set q 2
swap p q
puts "$p $q"
