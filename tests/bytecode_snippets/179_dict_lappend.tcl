proc test {} {
    set d [dict create items {}]
    dict lappend d items "one"
    dict lappend d items "two"
    dict lappend d items "three"
    return $d
}
