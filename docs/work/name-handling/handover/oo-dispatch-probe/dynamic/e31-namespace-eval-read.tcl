set x 1
namespace eval ::n { set x 2 }
puts [catch {set ::n::x} m]:$m ;# BAD on 8.x only: the write went to ::x
