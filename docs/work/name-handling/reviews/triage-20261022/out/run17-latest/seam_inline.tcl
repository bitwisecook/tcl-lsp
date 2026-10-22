proc p {} {return GLOBAL}
namespace eval n {
    proc p {} {return LOCAL}
    proc c {} {return [p]}
}
puts [n::c]
