proc p {} {
    set d {a 1 b 2}
    dict with d { set z [expr {$a + $b}] }
    return $d
}
