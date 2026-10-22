proc p {} {uplevel 1 {set result GLOBAL}}
namespace eval n {
    proc p {} {uplevel 1 {set result LOCAL}; return}
    proc c {} {set result INIT; p; return $result}
}
puts [n::c]
