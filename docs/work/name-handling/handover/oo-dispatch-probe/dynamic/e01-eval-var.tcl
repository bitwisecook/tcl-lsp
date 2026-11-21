proc p {} {
    set code {set x 1}
    eval $code
    return $x
}
puts [p]
proc q {} { set code {set y [nosuchcmd]}; return [catch {eval $code} m]:$m } ;# BAD unknown command in data
puts [q]
