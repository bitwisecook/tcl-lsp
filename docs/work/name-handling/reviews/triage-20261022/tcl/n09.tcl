proc p {} {return GLOBAL}
namespace eval lib {proc p {} {return LIB}}
namespace eval n {
    namespace path ::lib
    proc c {} {return [p]}
}
puts [n::c]
