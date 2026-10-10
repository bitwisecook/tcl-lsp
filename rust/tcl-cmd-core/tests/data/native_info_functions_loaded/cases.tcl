proc row {name script} {
    set code [catch {uplevel 1 $script} result]
    set length [string length $result]
    set units {}
    for {set index 0} {$index < $length} {incr index} {
        scan [string index $result $index] %c unit
        lappend units $unit
    }
    puts [list $name $code $length $units]
}
row fixed-or-script-sin {info functions sin}
row caller-local-function {
    namespace eval ::InfoScope085 {
        namespace eval tcl::mathfunc {proc local085 {} {return LOCAL}}
        info functions local085
    }
}
row global-local-merge {
    namespace eval ::tcl::mathfunc {proc shared085 {} {return GLOBAL}}
    namespace eval ::InfoScope085 {
        namespace eval tcl::mathfunc {
            proc shared085 {} {return LOCAL}
            proc other085 {} {return OTHER}
        }
        lsort [info functions *085]
    }
}
row opaque-function-pattern {
    set tail "opaque085[binary format H* ff]"
    proc ::tcl::mathfunc::$tail {} {return OPAQUE}
    info functions $tail
}
row raw-zero-function-pattern {info functions [binary format H* 73696e00ff]}
row encoded-zero-function-pattern {info functions [binary format H* 73696ec080ff]}
row loaded-root {info loaded {}}
row loaded-nested-child {
    interp create InfoParent085
    interp eval InfoParent085 {interp create InfoChild085}
    info loaded {InfoParent085 InfoChild085}
}
row loaded-original-raw-zero-child {
    set child [binary format H* 496e666f5a65726f30383500ff]
    interp create $child
    info loaded $child
}
row loaded-invalid-list {info loaded \{}
row loaded-missing-child {info loaded MissingInfo085}
row functions-observe-apply-helper {
    set hasApply [llength [info commands ::apply]]
    if {$hasApply} {rename ::apply ::SavedInfoApply085}
    proc ::apply args {error APPLY_HELPER085}
    set code [catch {info functions sin} result]
    rename ::apply {}
    if {$hasApply} {rename ::SavedInfoApply085 ::apply}
    if {$code} {error $result}
    set result
}
