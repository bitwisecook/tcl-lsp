namespace eval edge {}
proc edge::test {input} {set a x; set a:b BAD; set arr(x:b) good; set arr(BAD) bad; return $arr($a:b)}
proc ::f {} {
    set y [edge::test abcde]
    return $y
}
puts [f]
