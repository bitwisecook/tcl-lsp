set ::hits 0
proc plain {} {set result [catch {incr ::hits}]}
proc nested {} {set result [catch {catch {incr ::hits}}]}
plain
puts [list catch $::hits]
nested
puts [list nested $::hits]
if {[llength [info commands try]]} {
    set ::done 0
    proc protected {} {set result [try {incr ::hits} finally {incr ::done}]}
    protected
    puts [list try $::hits $::done]
}
