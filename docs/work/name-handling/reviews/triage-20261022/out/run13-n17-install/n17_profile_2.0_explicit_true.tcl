namespace eval mylib {}
proc mylib::strlen {subject} {string length $subject}
proc ::f {} {
    set y [mylib::strlen abcde]
    return $y
}
puts [f]
