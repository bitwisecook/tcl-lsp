set traceEvents {}
proc watch {label args} {global traceEvents; lappend traceEvents $label}
namespace eval a: {namespace eval b {
    set x LEFT
    set a(k) LEFT_ELEM
    trace add variable x write [list watch LEFT]
    set ::leftName [namespace current]
}}
namespace eval a {namespace eval :b {
    set x RIGHT
    set a(k) RIGHT_ELEM
    trace add variable x write [list watch RIGHT]
    set ::rightName [namespace current]
}}
puts [list display $leftName $rightName]
puts [namespace eval a: {namespace eval b {upvar 0 x y; list values $x $a(k) $y}}]
puts [namespace eval a {namespace eval :b {upvar 0 x y; list values $x $a(k) $y}}]
namespace eval a: {namespace eval b {set x CHANGED}}
puts [list traces $traceEvents]
puts [namespace eval a {namespace eval :b {list unaffected $x $a(k)}}]
