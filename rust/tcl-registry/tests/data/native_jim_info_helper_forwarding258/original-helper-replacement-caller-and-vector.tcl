if {[catch {::info body {namespace info}}]} {
    list NOT_APPLICABLE
} else {
    proc ::r2286_global {} {return GLOBAL}
    namespace eval ::r2286_holder {
        proc r2286_local {} {return LOCAL}
    }
    proc {namespace info} args {list HELPER [uplevel 1 {namespace current}] $args}
    namespace eval ::r2286_holder {
        set ordinary [info commands -all r2286*]
        set rooted [::info commands -all r2286*]
        set direct [lsort [::info -nons commands -all r2286*]]
        list $ordinary $rooted $direct
    }
}
