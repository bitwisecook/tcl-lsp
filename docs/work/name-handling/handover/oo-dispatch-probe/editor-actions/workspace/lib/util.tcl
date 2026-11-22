namespace eval ::util {
    namespace export scale
    proc scale {value factor} {
        set result [expr {$value * $factor}]
        return $result
    }
}
