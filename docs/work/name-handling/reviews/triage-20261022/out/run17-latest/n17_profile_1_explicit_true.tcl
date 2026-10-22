namespace eval mylib {}
proc mylib::strlen {subject} {string length $subject}
proc ::f {} {
    set y 5
    return $y
}
puts [f]
