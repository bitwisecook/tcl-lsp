proc foo {} {
    set unused "value"
    set y [expr $aaa + $bbb]
    return $y
}
