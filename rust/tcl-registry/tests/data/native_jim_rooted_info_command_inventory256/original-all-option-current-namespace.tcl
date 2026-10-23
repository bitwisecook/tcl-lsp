proc ::r2286_global {} {return GLOBAL}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
    set code [catch {lsort [::info commands -all r2286*]} result]
    list $code $result
}
