set x ROOT
namespace eval N {proc p {} {set x LOCAL; uplevel #0 {puts $x}; uplevel 0 {puts $x}}}
N::p
