proc check {a b} {
    set total [expr $a + $b]
    if {$total == "0"} {
        return empty
    }
    return $total
}
