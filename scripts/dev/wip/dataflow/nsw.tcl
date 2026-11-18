proc bump {name {by 1}} {upvar 1 $name v; incr v $by}
proc c {} {bump ::g; return $::g}
namespace eval ns {variable v 1; proc w {} {variable v; bump v}}
puts [c]
puts [ns::w]
