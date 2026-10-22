namespace eval A {proc touch {v} {upvar 1 $v x; set x 1}}
namespace eval B {proc touch {v} {return $v}; puts [touch plain]; puts [info exists plain]}
