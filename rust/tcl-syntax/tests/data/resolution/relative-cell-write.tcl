namespace eval N::R {set x 9}
namespace eval N {
    proc p {} {set R::x 10; puts $::N::R::x}
}
N::p
