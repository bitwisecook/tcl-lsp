# match variables written by regexp/regsub/switch/scan and then read
proc a {s} {
    if {[regexp {^(\w+)=(\w+)$} $s -> k v]} { return "$k:$v" }
    return none
}
proc b {s} {
    regexp {(\d+)} $s all num
    return $num
}
proc c {s} {
    set n [regsub -all {x} $s y out]
    return "$n $out"
}
proc d {s} {
    switch -regexp -matchvar mv -- $s {
        {(a)(b)} { return [lindex $mv 1] }
    }
    return no
}
proc e {s} {
    regexp -indices {b} $s where
    return $where
}
proc f {s} {
    foreach {all x} [regexp -all -inline {(\d)} $s] { lappend out $x }
    return $out
}
puts [a k=v] ; puts [b x42] ; puts [c axbx] ; puts [d ab] ; puts [e abc] ; puts [f a1b2]
