proc ::r2286_global {} {return GLOBAL}
proc {::r2286 spaced} {} {return SPACE}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
}
set code [catch {::info commands r2286*} result]
if {$code} {list $code $result} else {list $code [lsort $result]}
