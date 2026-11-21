set x 1
namespace eval ::n { set x 2 }
puts $x:[info exists ::n::x]
