namespace eval cfg {}
set ::cfg:::dir /colon-run
set ::cfg:: /empty-tail
puts [set ::cfg::dir]
puts [set ::cfg:::]
puts [info exists ::cfg]
namespace eval : {
    variable dir /colon-namespace
    variable selected $dir
    namespace eval inner {
        variable dir /nested-colon-namespace
        variable selected $dir
    }
    puts $selected
    namespace eval inner {puts $selected}
}
puts [info exists :::::selected]
