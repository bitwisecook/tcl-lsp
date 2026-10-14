puts [list provider [info patchlevel]]
namespace eval N {}
proc p {} {
    set {$literal} SIGIL
    set {[literal]} BRACKET
    set {} EMPTY
    set N::x QUALIFIED
    list ${$literal} ${[literal]} ${} [set N::x]
}
puts [list values [p]]
puts [list namespace_value [namespace eval N {info exists x}]]
