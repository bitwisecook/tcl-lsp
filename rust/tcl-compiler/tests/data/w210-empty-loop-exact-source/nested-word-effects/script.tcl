set loadRc [catch {namespace eval ::nested {
    proc operation {} {upvar 1 x cell; set cell 7; return {}}
    proc p {} {foreach y [operation] {}; return $x}
    error fail
}} loadResult]
set callRc [catch {::nested::p} callResult]
puts [list load $loadRc $loadResult call $callRc $callResult]
