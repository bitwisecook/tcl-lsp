namespace eval ::N {
    variable value ready
    proc get {} {variable value; set marker LINKED; return ready}
}
puts [list RESULT [::N::get]]
