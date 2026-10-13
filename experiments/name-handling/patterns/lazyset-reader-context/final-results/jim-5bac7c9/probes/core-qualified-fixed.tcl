# Authored trace callback reduction; does not source or adapt lazyset.
namespace eval ::install {
    variable cell original
    proc produce {fixed name} {return [list install $fixed $name]}
    proc read {} {
        upvar #0 ::install::cell {reader alias}
        set {reader alias} changed
        return $::observed
    }
}
namespace eval ::reader {
    proc produce {fixed name} {return [list reader $fixed $name]}
    proc read {} {
        upvar #0 ::install::cell {reader alias}
        set {reader alias} changed
        return $::observed
    }
}
proc ::trace_callback {prefix name1 name2 operation} {
    set ::observed [uplevel 1 [linsert $prefix end $name1]]
}
namespace eval ::install {
    trace variable cell w [list ::trace_callback [list ::install::produce {a b}]]
}
::reader::read
