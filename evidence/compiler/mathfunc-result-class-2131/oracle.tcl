foreach expression {ceil(3.14) int(3.14) trunc(2.5) isnan(1.0) signbit(-1.0)} {
    if {[catch {set value [expr $expression]} message]} {
        puts "$expression ERROR"
    } else {
        puts "$expression value=$value int=[string is integer -strict $value] double=[string is double -strict $value] boolean=[string is boolean -strict $value]"
    }
}
