namespace eval ::writer {
    proc assign {name} {upvar 1 $name cell; set cell changed}
    proc forward {prefix} {uplevel 1 $prefix}
    proc caller {} {
        set {a b} original
        forward [namespace code [list ::dispatch {a b}]]
        list [set {a b}] [namespace eval ::writer {info exists {a b}}]
    }
}
interp alias {} ::dispatch {} ::writer::assign
::writer::caller
