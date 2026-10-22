namespace eval mylib {}
proc mylib::strlen {subject} {string length $subject}
proc ::f {} {
    set y [mylib::strlen abcde]
}
puts [f]
