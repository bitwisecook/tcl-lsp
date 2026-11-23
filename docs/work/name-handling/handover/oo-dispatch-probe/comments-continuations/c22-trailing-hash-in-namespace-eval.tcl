namespace eval ns {
    variable v 1 ;# comment
    # proc hidden {} {}
    proc shown {} {return shown}
}
puts [info commands ::ns::*]
