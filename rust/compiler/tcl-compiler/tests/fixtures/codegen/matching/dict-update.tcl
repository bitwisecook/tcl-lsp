proc p {} {
    set d {a 1 b 2}
    dict update d a x b y { set z [expr {$x + $y}] }
    return $d
}
