set x ROOT
namespace eval N {
    variable x NAMESPACE
    proc p {} {
        set x PROCEDURE
        puts "root [uplevel #0 {set x}]"
        puts "current [uplevel 0 {set x}]"
        puts "caller [uplevel 1 {set x}]"
    }
}
proc caller {} {
    set x CALLER
    N::p
}
caller
