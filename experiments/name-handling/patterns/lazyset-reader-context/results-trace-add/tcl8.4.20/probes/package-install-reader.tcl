# Authored caller around the unchanged pinned Tcllib lazyset implementation.
source [file join [file dirname [info script]] .. upstream lazyset.tcl]
namespace eval ::install {
    proc produce {fixed name} {return [list install $fixed $name]}
    proc read {} {
        upvar #0 ::install::cell {reader alias}
        set {reader alias}
    }
}
namespace eval ::reader {
    proc produce {fixed name} {return [list reader $fixed $name]}
    proc read {} {
        upvar #0 ::install::cell {reader alias}
        set {reader alias}
    }
}
namespace eval ::install {
    ::lazyset::variable -array false cell [list produce {a b}]
}
::install::read
