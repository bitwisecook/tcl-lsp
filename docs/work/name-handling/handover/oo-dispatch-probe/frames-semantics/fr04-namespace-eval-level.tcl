proc p {} { namespace eval ::n { set ::lv [info level] }; return $::lv }
puts [p]
proc q {} { set loc qlocal; namespace eval ::n { uplevel 1 {set fromns 1} }; return [info exists fromns] }
puts [q]
