puts "version [info patchlevel]"
puts "has-apply [llength [info commands apply]]"
if {![llength [info commands apply]]} {exit 0}
namespace eval Target {variable lambda {{} {return ROOT_TARGET}}}
namespace eval N {
    namespace eval Target {variable lambda {{} {return LOCAL_TARGET}}}
    proc p {} {
        puts "qualified-root [uplevel #0 {apply $Target::lambda}]"
        puts "qualified-current [uplevel 0 {apply $Target::lambda}]"
        puts "qualified-caller [uplevel 1 {apply $Target::lambda}]"
    }
}
namespace eval M {
    namespace eval Target {variable lambda {{} {return CALLER_TARGET}}}
    proc caller {} {N::p}
}
M::caller
set lambda {{} {return ROOT_BARE}}
proc missing {} {apply $lambda}
puts "bare-root-in-procedure [catch {missing} result] $result"
proc outer {} {
    set lambda {{} {return OUTER_BARE}}
    proc inner {} {apply $lambda}
    inner
}
puts "outer-local-in-inner [catch {outer} result] $result"
