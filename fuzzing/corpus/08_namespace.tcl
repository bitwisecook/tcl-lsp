# Namespaces
namespace eval myns {
    variable counter 0

    proc increment {} {
        variable counter
        incr counter
        return $counter
    }

    proc get {} {
        variable counter
        return $counter
    }
}

myns::increment
myns::increment
set val [myns::get]
