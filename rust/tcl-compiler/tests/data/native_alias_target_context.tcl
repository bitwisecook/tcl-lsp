proc :target {} {return COLON}
proc target {} {return ROOT}
if {[llength [info commands alias]]} {
    alias :link :target
    alias contextual target
} else {
    interp alias {} :link {} :target
    interp alias {} contextual {} target
}
puts [list RELATIVE [catch {:link} result] $result]
puts [list PRINTED [catch {:::target} result] $result]
namespace eval N {
    proc target {} {return LOCAL}
    puts [list CALLER [catch {contextual} result] $result]
}
