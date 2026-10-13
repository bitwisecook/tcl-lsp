set x ROOT
namespace eval :: {
    puts [list root-namespace $x]
    proc p {} {
        set x PROCEDURE
        uplevel #0 {puts [list root-frame $x]}
        uplevel 0 {puts [list current-frame $x]}
    }
}
p
