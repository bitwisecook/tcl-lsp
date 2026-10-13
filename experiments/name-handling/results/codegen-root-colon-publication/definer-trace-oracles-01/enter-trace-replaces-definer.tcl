proc shadow args {return SHADOW}
proc replace_definer {command operation} {
    trace remove execution ::proc enter ::replace_definer
    rename ::proc ::stock_proc
    rename ::shadow ::proc
}
trace add execution ::proc enter ::replace_definer
set result [proc :audit {} {return ORIGINAL}]
puts [list definition-result $result installed [info commands :audit]]
if {[llength [info commands :audit]]} {puts [list body [:audit]]}
puts [list next-definition [proc :later {} {return LATER}] installed-later [info commands :later]]
