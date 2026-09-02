# T5: break/continue/return arriving through nested constructs and through
# a called command (dynamic completion code).
proc find {needle haystack} {
    set i 0
    foreach h $haystack {
        if {$h eq $needle} { return $i }
        incr i
    }
    return -1
}
puts [find c {a b c d}]
puts [find z {a b c d}]
set out {}
foreach i {1 2 3 4 5} {
    switch $i {
        2 { continue }
        4 { break }
    }
    lappend out $i
}
puts $out
set n 0
while 1 { if {[incr n] >= 3} break }
puts $n
