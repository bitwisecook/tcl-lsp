set x global
namespace eval ::n { set x nsevalset }
namespace eval ::n { set y $x ;# READ
}
puts [info exists ::n::x]:$::x:$::n::y
